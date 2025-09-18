//! ESP32-S3 MVP Firmware
//!
//! Implements the MVP per docs/software_design.md:
//! - Boot init: time, GPIO, I2C, basic presence scans
//! - I2C mux PCA9545A (0x70) split + per-channel device ACK checks (SC8815/SW2303)
//! - Front-panel TCA6408A (0x20) presence check
//! - Power input subsystem MVP: INA226-based input qualification and 10s status log

#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU8, Ordering};
use defmt::{error, info, warn};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::gpio::{Input, Level, Output, Pull};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;
use sc8815::registers::Register as ScReg;
// Shared I2C bus infrastructure
use core::fmt::Write as _;
mod power_in;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Receiver;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

// No global mutex in MVP

// We manually drive PCA9545A (0x70) via async I2C writes

// INA226 is handled inside power_in task

// Use SC8815/SW2303 crates only for their default I2C addresses
use sc8815::registers::constants as sc8815_const;
use sw2303::registers::constants as sw2303_const;

esp_bootloader_esp_idf::esp_app_desc!();

// Provide a global timestamp for defmt logs (milliseconds since boot)
defmt::timestamp!("{=u64} ms", {
    esp_hal::time::Instant::now()
        .duration_since_epoch()
        .as_millis()
});

// Type alias for the async I2C bus and a global container to share it
type I2cBus = I2c<'static, esp_hal::Async>;
type SharedI2cBus = Mutex<CriticalSectionRawMutex, I2cBus>;
static I2C_BUS: StaticCell<SharedI2cBus> = StaticCell::new();

// Global target state: open/closed (intent)
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
enum PowerSwitchTarget {
    Open,
    Closed,
}

// Global intent per docs/software_design.md 0.1
static PWR_SW_TARGET: AtomicU8 = AtomicU8::new(PowerSwitchTarget::Open as u8);

// No shared snapshot; status task logs directly

// I2C shared in init (blocking). We do not share across tasks for MVP; INA226 is accessed in tasks

// Board-specific pins per docs/esp32-s3fh4r2_gpio_assignment_guide.md
const PIN_I2C_SDA: u8 = 8;
const PIN_I2C_SCL: u8 = 9;
const PIN_I2C_INT: u8 = 16;
const PIN_I2C_RESET: u8 = 38; // open-drain, low to reset I2C peripherals

const PIN_IN_EN: u8 = 41; // TPS2490 enable (high = on)
const PIN_IN_PG: u8 = 42; // TPS2490 PG (open drain, high = good)

// SC8815 PSTOP control lines (active-low enable)
const PIN_PSTOP1: u8 = 17;
const PIN_PSTOP2: u8 = 18;
const PIN_PSTOP3: u8 = 39;
const PIN_PSTOP4: u8 = 40;

// INA226 shunt value (ohms) from docs: 5 mΩ
const SHUNT_RESISTANCE_OHMS: f32 = 0.005;

// Qualification thresholds (docs/software_design.md)
const VIN_MIN_V: f32 = 9.0;
const VIN_MAX_V: f32 = 24.0;
const I_IDLE_MAX_A: f32 = 0.010; // 10 mA

// SC8815 VBUS readiness requirements
const SC8815_VBUS_READY_MV: u16 = 4000;
const SC8815_VBUS_READY_CONSECUTIVE: u8 = 2;
const SC8815_VBUS_READY_INTERVAL_MS: u64 = 50;

// Minimal SC8815 status register address for ACK probe (per sc8815-rs README)
const SC8815_STATUS_REG_ADDR: u8 = 0x17;

// SC8815 detect retries to handle delayed device readiness (total ~2s)
const SC8815_DETECT_INTERVAL_MS: u64 = 50; // ms between attempts
const SC8815_DETECT_TOTAL_MS: u64 = 2000; // overall grace per channel
const SC8815_DETECT_RETRIES: u8 = (SC8815_DETECT_TOTAL_MS / SC8815_DETECT_INTERVAL_MS) as u8; // 40

async fn sc8815_ack<I2C: embedded_hal_async::i2c::I2c>(
    i2c: &mut I2C,
    addr: u8,
) -> (bool, &'static str) {
    let mut b = [0u8; 1];
    if i2c
        .write_read(addr, &[SC8815_STATUS_REG_ADDR], &mut b)
        .await
        .is_ok()
    {
        return (true, "wr_rd");
    }
    if i2c.read(addr, &mut b).await.is_ok() {
        return (true, "rd");
    }
    if i2c.write(addr, &[]).await.is_ok() {
        return (true, "addr");
    }
    (false, "no")
}

async fn i2c_scan_found<I2C: embedded_hal_async::i2c::I2c>(i2c: &mut I2C) -> heapless::Vec<u8, 16> {
    let mut found: heapless::Vec<u8, 16> = heapless::Vec::new();
    let mut tmp = [0u8; 1];
    for addr in 0x03u8..0x78u8 {
        if embedded_hal_async::i2c::I2c::read(i2c, addr, &mut tmp)
            .await
            .is_ok()
        {
            let _ = found.push(addr);
            if found.is_full() {
                break;
            }
        }
        Timer::after(Duration::from_millis(1)).await;
    }
    found
}

async fn tca6408a_present<I2C: embedded_hal_async::i2c::I2c>(i2c: &mut I2C) -> bool {
    let mut buf = [0u8; 1];
    i2c.write_read(0x20, &[0x00], &mut buf).await.is_ok()
}

async fn pca9545a_select<I2C: embedded_hal_async::i2c::I2c>(
    i2c: &mut I2C,
    ch: u8,
) -> Result<(), I2C::Error> {
    let mask = 1u8 << (ch & 0x03);
    i2c.write(0x70, &[mask]).await
}

async fn ack_scan_vin_off(bus: &'static SharedI2cBus, sc_addr: u8) {
    for ch in 0u8..4u8 {
        let mut i2c_scan = I2cDevice::new(bus);
        let _ = pca9545a_select(&mut i2c_scan, ch).await;
        Timer::after(Duration::from_millis(2)).await;
        let mut present = false;
        let mut method = "no";
        let mut tries: u8 = 0;
        for attempt in 0..SC8815_DETECT_RETRIES {
            let (ok, m) = sc8815_ack(&mut i2c_scan, sc_addr).await;
            tries = attempt + 1;
            if ok {
                present = true;
                method = m;
                break;
            }
            Timer::after(Duration::from_millis(SC8815_DETECT_INTERVAL_MS)).await;
        }
        info!(
            "i2c.scan: ch={} sc8815_ack={} via={} tries={} vin_on=false",
            ch,
            if present { "yes" } else { "no" },
            method,
            tries
        );
    }
}

// No extra helpers for INA226 per request; use driver inline at call sites

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let p = esp_hal::init(esp_hal::Config::default());
    info!("app.start");

    // Initialize the embassy time driver
    let timg0 = TimerGroup::new(p.TIMG0);
    esp_hal_embassy::init(timg0.timer0);
    info!("init.time: embassy-timer=ok");

    // GPIO prepare
    // I2C reset (use push-pull for MVP), default high (released)
    let mut i2c_reset = Output::new(
        p.GPIO38,
        Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );
    // Briefly assert low then release
    i2c_reset.set_low();
    // small blocking delay via timer (1 ms)
    Timer::after(Duration::from_millis(5)).await;
    i2c_reset.set_high();
    Timer::after(Duration::from_millis(5)).await;

    // IN_EN default off
    let mut in_en = Output::new(p.GPIO41, Level::Low, esp_hal::gpio::OutputConfig::default());
    // PG input
    let in_pg = Input::new(
        p.GPIO42,
        esp_hal::gpio::InputConfig::default().with_pull(Pull::Up),
    );

    // PSTOP lines default disabled (active-low -> drive high)
    let mut pstop1 = Output::new(
        p.GPIO17,
        Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );
    let mut pstop2 = Output::new(
        p.GPIO18,
        Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );
    let mut pstop3 = Output::new(
        p.GPIO39,
        Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );
    let mut pstop4 = Output::new(
        p.GPIO40,
        Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );
    // Keep variables used
    pstop1.set_high();
    pstop2.set_high();
    pstop3.set_high();
    pstop4.set_high();

    info!("init.hw: chip=ESP32-S3 i2c=ok sda=GPIO8 scl=GPIO9");

    // Publish power-on intent first (only intent; actual switch controlled after qualification)
    PWR_SW_TARGET.store(PowerSwitchTarget::Closed as u8, Ordering::Relaxed);

    // Upstream TCA6408A presence check (0x20) using async I2C — runs immediately after publishing intent
    // Initialize I2C0 once and share via Mutex + I2cDevice
    let i2c_hw = I2c::new(p.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(p.GPIO8)
        .with_scl(p.GPIO9)
        .into_async();
    let bus = I2C_BUS.init(Mutex::new(i2c_hw));
    let mut i2c = I2cDevice::new(bus);
    let tca_online = tca6408a_present(&mut i2c).await;
    if tca_online {
        info!("i2c.front: tca6408a=online addr=0x20");
    } else {
        warn!("i2c.front: tca6408a=offline addr=0x20");
    }

    // I2C mux PCA9545A presence via reading control register (REG 0x00)
    let mut mux_reg = [0u8; 1];
    if embedded_hal_async::i2c::I2c::write_read(&mut i2c, 0x70, &[0x00], &mut mux_reg)
        .await
        .is_ok()
    {
        info!("i2c.mux: ok addr=0x70 parts=4");
    } else {
        error!("i2c.mux: err=init addr=0x70");
        panic!("PCA9545A not found");
    }

    // Spawn power input task: handles INA init/qualification/VIN_ON/periodic status
    power_in::spawn(
        &spawner,
        bus,
        in_en,
        in_pg,
        SHUNT_RESISTANCE_OHMS,
        power_in::Limits {
            vin_min_v: VIN_MIN_V,
            vin_max_v: VIN_MAX_V,
            idle_current_max_a: I_IDLE_MAX_A,
        },
    )
    .expect("spawn power_in task");

    // Spawn a main-side subscriber to print status logs from Channel
    #[embassy_executor::task]
    async fn power_in_log_task(
        mut rx: Receiver<
            'static,
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            power_in::Status,
            8,
        >,
    ) {
        loop {
            let s = rx.receive().await;
            info!(
                "pwr.in:stat(main) vin={}V i={}A pg={} vin_on={}",
                s.vin_v,
                s.i_a,
                if s.pg_good { "good" } else { "bad" },
                if s.vin_on { "true" } else { "false" }
            );
        }
    }
    spawner
        .spawn(power_in_log_task(power_in::status_receiver()))
        .expect("spawn power_in_log_task");

    // Wait for VIN_ON signal before scanning SC8815 modules; fallback to ACK-only scan when false
    let vin_on = power_in::vin_on_signal().wait().await;
    if !vin_on {
        warn!("pwr.in: vin_on=false; skip module init; do ack-scan only");
        let sc_addr = sc8815_const::DEFAULT_ADDRESS;
        ack_scan_vin_off(bus, sc_addr).await;
        return;
    }

    // After VIN ON, scan SC8815 and conditionally init SW2303 per channel
    let sc_addr = sc8815_const::DEFAULT_ADDRESS;
    let sw_addr = sw2303_const::DEFAULT_ADDRESS;
    info!("i2c.scan:start vin_on=true");
    for ch in 0u8..4u8 {
        let mut i2c_scan = I2cDevice::new(bus);
        let mut mux_reg = [0u8; 1];
        let select_ok = pca9545a_select(&mut i2c_scan, ch).await.is_ok();
        let ctrl_read_ok =
            embedded_hal_async::i2c::I2c::write_read(&mut i2c_scan, 0x70, &[0x00], &mut mux_reg)
                .await
                .is_ok();
        info!(
            "i2c.mux: ch={} select={} ctrl_read={} reg=0x{:02X}",
            ch,
            if select_ok { "ok" } else { "err" },
            if ctrl_read_ok { "ok" } else { "err" },
            mux_reg[0]
        );
        // small settle time after mux switch to let downstream devices wake
        Timer::after(Duration::from_millis(2)).await;

        // Pre-probe ACK on expected address with retries.
        let mut sc_ack = false;
        let mut ack_method = "no";
        let mut tries: u8 = 0;
        for attempt in 0..SC8815_DETECT_RETRIES {
            if attempt > 0 {
                let _ = pca9545a_select(&mut i2c_scan, ch).await;
                Timer::after(Duration::from_millis(2)).await;
            }
            let (ok, method) = sc8815_ack(&mut i2c_scan, sc_addr).await;
            tries = attempt + 1;
            if ok {
                sc_ack = true;
                ack_method = method;
                break;
            }
            Timer::after(Duration::from_millis(SC8815_DETECT_INTERVAL_MS)).await;
        }

        // Probe SC8815 by reading a status register (only if ACK)
        let mut sc_ok = false;
        // SC8815 driver owns I2C; temporarily move and release back after use
        let mut sc_drv = sc8815::SC8815::new(i2c_scan, sc_addr);
        let sc_present = sc_ack && sc_drv.read_register(ScReg::Status).await.is_ok();
        if sc_present {
            // Initialize SC8815 per design
            if sc_drv.init().await.is_ok() {
                let _ = sc_drv.set_otg_mode(true).await;
                let _ = sc_drv.set_vbus_internal_voltage(5000, 1).await;
                match ch {
                    0 => pstop1.set_low(),
                    1 => pstop2.set_low(),
                    2 => pstop3.set_low(),
                    3 => pstop4.set_low(),
                    _ => {}
                }
                // Require consecutive VBUS readings above threshold
                let mut consecutive = 0u8;
                for _ in 0..40 {
                    // ~2s with 50ms intervals
                    if let Ok(meas) = sc_drv.get_adc_measurements().await {
                        if meas.vbus_mv >= SC8815_VBUS_READY_MV {
                            consecutive += 1;
                            if consecutive >= SC8815_VBUS_READY_CONSECUTIVE {
                                info!(
                                    "pwr.sc8815: ch={} vbus_ready=true vbus={}mV",
                                    ch, meas.vbus_mv
                                );
                                sc_ok = true;
                                break;
                            }
                        } else {
                            consecutive = 0;
                        }
                    }
                    Timer::after(Duration::from_millis(SC8815_VBUS_READY_INTERVAL_MS)).await;
                }
            }
        }
        // release I2C back from driver
        i2c_scan = sc_drv.release();

        // Probe/init SW2303 if SC8815 VBUS ready
        let mut sw_ok = false;
        if sc_ok {
            // Read SW2303 device ID register (0x00) for online detection
            let mut id_buf = [0u8; 1];
            let online = embedded_hal_async::i2c::I2c::write_read(
                &mut i2c_scan,
                sw_addr,
                &[0x00],
                &mut id_buf,
            )
            .await
            .is_ok();
            if online {
                let mut sw = sw2303::SW2303::new(&mut i2c_scan, sw_addr);
                if sw.init().await.is_ok() {
                    sw_ok = true;
                }
            }
        }

        // Always report ACK result to aid debug
        info!(
            "i2c.scan: ch={} mux_reg=0x{:02X} sc8815_ack={} via={} tries={}",
            ch,
            mux_reg[0],
            if sc_ack { "yes" } else { "no" },
            ack_method,
            tries
        );

        if !sc_ack {
            let found = i2c_scan_found(&mut i2c_scan).await;
            if !found.is_empty() {
                let mut line: heapless::String<128> = heapless::String::new();
                for (i, a) in found.iter().enumerate() {
                    let _ = if i == 0 {
                        write!(line, "0x{:02X}", a)
                    } else {
                        write!(line, ",0x{:02X}", a)
                    };
                }
                info!("i2c.scan: ch={} found=[{}]", ch, line.as_str());
            } else {
                info!("i2c.scan: ch={} found=[]", ch);
            }
        }

        if sc_ok && sw_ok {
            info!("i2c.scan: ch={} sc8815=online sw2303=online", ch);
        } else {
            info!(
                "i2c.scan: ch={} sc8815={} sw2303={}",
                ch,
                if sc_ok { "online" } else { "offline" },
                if sw_ok { "online" } else { "offline" }
            );
            if sc_ok && !sw_ok {
                warn!(
                    "i2c.scan: ch={} anomaly=true reason=\"module-incomplete\"",
                    ch
                );
            } else if sc_ok ^ sw_ok {
                error!("i2c.scan: ch={} anomaly=true reason=\"pair-mismatch\"", ch);
            }
        }

        if !(sc_ok && sw_ok) {
            match ch {
                0 => pstop1.set_high(),
                1 => pstop2.set_high(),
                2 => pstop3.set_high(),
                3 => pstop4.set_high(),
                _ => {}
            }
        }
    }
}
