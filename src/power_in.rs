use defmt::{info, warn};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::{task, SpawnError, Spawner};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver};
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use embedded_hal_async::i2c::I2c;
use esp_hal::gpio::{Input, Output};
use ina226_tp as ina226;

use crate::I2cBus;

const INA226_ADDR: u8 = 0x44;
// INA226 shunt voltage LSB = 2.5 uV per datasheet and crate constants.
const INA226_SHUNT_LSB_V: f32 = 2.5e-6;

pub type InaOp<'a, I2C> = ina226::INA226<&'a mut I2C, ina226::Operational>;

#[derive(Copy, Clone)]
pub struct Limits {
    pub vin_min_v: f32,
    pub vin_max_v: f32,
    pub idle_current_max_a: f32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            vin_min_v: 9.0,
            vin_max_v: 24.0,
            idle_current_max_a: 0.010,
        }
    }
}

#[derive(Copy, Clone, Default)]
pub struct Status {
    pub vin_v: f32,
    pub i_a: f32,
    pub pg_good: bool,
    pub vin_on: bool,
}

static STATUS_CH: Channel<CriticalSectionRawMutex, Status, 8> = Channel::new();
static VIN_ON_SIG: Signal<CriticalSectionRawMutex, bool> = Signal::new();

pub fn status_receiver() -> Receiver<'static, CriticalSectionRawMutex, Status, 8> {
    STATUS_CH.receiver()
}

pub fn vin_on_signal() -> &'static Signal<CriticalSectionRawMutex, bool> {
    &VIN_ON_SIG
}

pub fn spawn(
    spawner: &Spawner,
    bus: &'static Mutex<CriticalSectionRawMutex, I2cBus>,
    in_en: Output<'static>,
    in_pg: Input<'static>,
    shunt_res_ohms: f32,
    limits: Limits,
) -> Result<(), SpawnError> {
    spawner.spawn(task(bus, in_en, in_pg, shunt_res_ohms, limits))
}

#[task]
async fn task(
    bus: &'static Mutex<CriticalSectionRawMutex, I2cBus>,
    mut in_en: Output<'static>,
    in_pg: Input<'static>,
    shunt_res_ohms: f32,
    limits: Limits,
) {
    // Ensure power path is held open until qualification passes.
    in_en.set_low();

    let mut i2c = I2cDevice::new(bus);
    let mut ina = {
        let mut dev = ina226::INA226::new(None);
        dev.set_ina_address(INA226_ADDR);
        dev.initialize(&mut i2c)
            .await
            .unwrap_or_else(|_| panic!("INA226 init failed at 0x{:02X}", INA226_ADDR))
    };

    configure_ina(&mut ina, shunt_res_ohms).await;

    let ok = qualify_startup(&mut ina, shunt_res_ohms, limits).await;
    if ok {
        in_en.set_high();
    } else {
        in_en.set_low();
        warn!("pwr.in:qual failed; keep switch open");
    }

    let mut vin_on_state = wait_vin_on(&mut ina, &in_pg, limits, 50, 40).await;
    VIN_ON_SIG.signal(vin_on_state.vin_on);

    loop {
        let status = sample_status(
            &mut ina,
            &in_pg,
            shunt_res_ohms,
            limits,
            vin_on_state.vin_on,
        )
        .await;
        info!(
            "pwr.in:stat vin={}V i={}A pg={} vin_on={}",
            status.vin_v,
            status.i_a,
            if status.pg_good { "good" } else { "bad" },
            if status.vin_on { "true" } else { "false" }
        );
        if status.vin_on && !vin_on_state.vin_on {
            info!("pwr.in:vin_on=true vin={}V pg=good", status.vin_v);
            VIN_ON_SIG.signal(true);
        }
        vin_on_state.vin_on |= status.vin_on;

        STATUS_CH.send(status).await;
        Timer::after(Duration::from_secs(10)).await;
    }
}

async fn sample_status<I2C: I2c>(
    ina: &mut InaOp<'_, I2C>,
    in_pg: &Input<'_>,
    shunt_res_ohms: f32,
    limits: Limits,
    vin_on_state: bool,
) -> Status {
    let vin_v = ina.read_voltage().await as f32;
    let vshunt_v = read_signed_shunt_voltage(ina).await;
    let i_a = vshunt_v / shunt_res_ohms;
    let pg_good = in_pg.is_high();
    let vin_range_ok = (limits.vin_min_v..=limits.vin_max_v).contains(&vin_v);

    Status {
        vin_v,
        i_a,
        pg_good,
        vin_on: vin_on_state || (pg_good && vin_range_ok),
    }
}

async fn configure_ina<I2C: I2c>(ina: &mut InaOp<'_, I2C>, shunt_res_ohms: f32) {
    use ina226::{InaAverage, InaMode, InaVbusct, InaVshct};
    ina.set_ina_mode(InaMode::ShuntAndBusContinuous)
        .set_ina_average(InaAverage::_16)
        .set_ina_vbusct(InaVbusct::_1_1_ms)
        .set_ina_vscht(InaVshct::_1_1_ms)
        .commit()
        .await;
    ina.set_ina_calibration(shunt_res_ohms as f64, 10.0f64)
        .commit()
        .await;
}

async fn qualify_startup<I2C: I2c>(
    ina: &mut InaOp<'_, I2C>,
    shunt_res_ohms: f32,
    limits: Limits,
) -> bool {
    let mut ok = false;
    for _ in 0..5 {
        let vbus_v = ina.read_voltage().await as f32;
        let vshunt_v = read_signed_shunt_voltage(ina).await;
        let ishunt_a = vshunt_v / shunt_res_ohms;
        let range_ok = (limits.vin_min_v..=limits.vin_max_v).contains(&vbus_v);
        let current_ok = ishunt_a.abs() <= limits.idle_current_max_a;
        info!(
            "pwr.in:qual vbus={}V i={}A range_ok={} current_ok={}",
            vbus_v, ishunt_a, range_ok, current_ok
        );
        if range_ok && current_ok {
            ok = true;
            break;
        }
        Timer::after(Duration::from_millis(20)).await;
    }
    ok
}

#[allow(dead_code)]
struct VinOnResult {
    vin_on: bool,
    last_vbus_v: f32,
    last_pg_good: bool,
}

async fn wait_vin_on<I2C: I2c>(
    ina: &mut InaOp<'_, I2C>,
    in_pg: &Input<'_>,
    limits: Limits,
    interval_ms: u64,
    max_iters: u32,
) -> VinOnResult {
    let mut vin_on = false;
    let mut last_vbus_v = 0.0f32;
    let mut last_pg_good = false;

    for _ in 0..max_iters {
        let pg_good = in_pg.is_high();
        let vbus_v = ina.read_voltage().await as f32;
        last_vbus_v = vbus_v;
        last_pg_good = pg_good;
        let range_ok = (limits.vin_min_v..=limits.vin_max_v).contains(&vbus_v);
        if pg_good && range_ok {
            vin_on = true;
            break;
        }
        Timer::after(Duration::from_millis(interval_ms)).await;
    }

    if vin_on {
        info!("pwr.in:vin_on=true vin={}V pg=good", last_vbus_v);
    } else {
        warn!(
            "pwr.in:vin_on=false vin={}V pg={}",
            last_vbus_v,
            if last_pg_good { "good" } else { "bad" }
        );
    }

    VinOnResult {
        vin_on,
        last_vbus_v,
        last_pg_good,
    }
}

async fn read_signed_shunt_voltage<I2C: I2c>(ina: &mut InaOp<'_, I2C>) -> f32 {
    // INA226 shunt register is a signed 16-bit value (two's complement); convert before scaling.
    let raw = ina.read_raw_shunt_voltage().await;
    let signed = i16::from_be_bytes(raw.to_be_bytes());
    signed as f32 * INA226_SHUNT_LSB_V
}
