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
use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
use esp_hal::gpio::{Input, Level, Output, Pull};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;
use sc8815::registers::Register as ScReg;

// No global mutex in MVP

// We manually drive PCA9545A (0x70) via async I2C writes

// INA226 minimal async helpers (manual register reads)

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
const PIN_VIN_ADC: u8 = 4; // ADC1_CH3

// USB HUB port power control (active-low enable) — not toggled in MVP except default disable
const PIN_CE1: u8 = 17;
const PIN_CE2: u8 = 18;
const PIN_CE3: u8 = 39;
const PIN_CE4: u8 = 40;

// INA226 default addresses: docs prefer 0x40, field note suggests 0x44; probe both
const INA226_ADDR_PRIMARY: u8 = 0x44;
const INA226_ADDR_FALLBACK: u8 = 0x40;

// INA226 shunt value (ohms) from docs: 5 mΩ
const SHUNT_RESISTANCE_OHMS: f32 = 0.005;

// ADC scaling: 11:1 divider (100k + 10k)
const VIN_ADC_DIV: f32 = 11.0;

// Qualification thresholds (docs/software_design.md)
const VIN_MIN_V: f32 = 9.0;
const VIN_MAX_V: f32 = 24.0;
const I_IDLE_MAX_A: f32 = 0.010; // 10 mA

// Anomaly detection thresholds (status task)
const ADC_INA_RATIO_MIN: f32 = 0.60;
const ADC_INA_DELTA_MAX_V: f32 = 3.0;

// SC8815 VBUS readiness requirements
const SC8815_VBUS_READY_MV: u16 = 4000;
const SC8815_VBUS_READY_CONSECUTIVE: u8 = 2;
const SC8815_VBUS_READY_INTERVAL_MS: u64 = 50;

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

async fn ina226_read_bus_mv<I2C: embedded_hal_async::i2c::I2c>(
    i2c: &mut I2C,
    addr: u8,
) -> Result<f32, I2C::Error> {
    // Bus voltage register 0x02, 16-bit BE, LSB = 1.25mV
    let mut buf = [0u8; 2];
    i2c.write_read(addr, &[0x02], &mut buf).await?;
    let raw = u16::from_be_bytes(buf) as u32;
    Ok(raw as f32 * 1.25)
}

async fn ina226_read_shunt_uv<I2C: embedded_hal_async::i2c::I2c>(
    i2c: &mut I2C,
    addr: u8,
) -> Result<f32, I2C::Error> {
    // Shunt voltage register 0x01, signed 16-bit, LSB = 2.5uV
    let mut buf = [0u8; 2];
    i2c.write_read(addr, &[0x01], &mut buf).await?;
    let raw = i16::from_be_bytes(buf) as i32;
    Ok(raw as f32 * 2.5)
}

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

    // USB port enables default disabled (active-low -> drive high)
    let mut ce1 = Output::new(
        p.GPIO17,
        Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );
    let mut ce2 = Output::new(
        p.GPIO18,
        Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );
    let mut ce3 = Output::new(
        p.GPIO39,
        Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );
    let mut ce4 = Output::new(
        p.GPIO40,
        Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );
    // Keep variables used
    ce1.set_high();
    ce2.set_high();
    ce3.set_high();
    ce4.set_high();

    info!("init.hw: chip=ESP32-S3 i2c=ok sda=GPIO8 scl=GPIO9");

    // Publish power-on intent first (only intent; actual switch controlled after qualification)
    PWR_SW_TARGET.store(PowerSwitchTarget::Closed as u8, Ordering::Relaxed);

    // Upstream TCA6408A presence check (0x20) using async I2C — runs immediately after publishing intent
    let mut i2c = I2c::new(p.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(p.GPIO8)
        .with_scl(p.GPIO9)
        .into_async();
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

    // INA226 address probe by reading bus voltage register
    let ina_addr = match ina226_read_bus_mv(&mut i2c, INA226_ADDR_PRIMARY).await {
        Ok(_) => INA226_ADDR_PRIMARY,
        Err(_) => match ina226_read_bus_mv(&mut i2c, INA226_ADDR_FALLBACK).await {
            Ok(_) => INA226_ADDR_FALLBACK,
            Err(_) => {
                warn!("pwr.in: ina226=offline addr=0x44/0x40");
                0
            }
        },
    };

    // ADC configuration for VIN sampling
    let mut adc_cfg = AdcConfig::new();
    let mut vin_pin = adc_cfg.enable_pin(p.GPIO4, Attenuation::_11dB);
    let mut adc1 = Adc::new(p.ADC1, adc_cfg);

    // Startup qualification: read INA226 up to 5 times, 20 ms apart
    if ina_addr != 0 {
        let mut ok_to_close = false;
        for _ in 0..5 {
            let v_mv = ina226_read_bus_mv(&mut i2c, ina_addr).await.unwrap_or(0.0);
            let vbus_v = v_mv / 1000.0;
            let sh_uv = ina226_read_shunt_uv(&mut i2c, ina_addr)
                .await
                .unwrap_or(0.0);
            let ishunt_a = (sh_uv / 1_000_000.0) / SHUNT_RESISTANCE_OHMS;
            let range_ok = vbus_v >= VIN_MIN_V && vbus_v <= VIN_MAX_V;
            let current_ok = ishunt_a.abs() <= I_IDLE_MAX_A;
            info!(
                "pwr.in:qual vbus={}V i={}A range_ok={} current_ok={}",
                vbus_v, ishunt_a, range_ok, current_ok
            );
            if range_ok && current_ok {
                ok_to_close = true;
                break;
            }
            Timer::after(Duration::from_millis(20)).await;
        }

        if ok_to_close {
            in_en.set_high();
        } else {
            warn!("pwr.in:qual failed; keep switch open");
            in_en.set_low();
        }
    }

    // Wait until VIN is considered ON per docs: INA226 within range AND PG good
    let mut vin_on = false;
    let mut last_vbus_v = 0.0f32;
    let mut last_pg_good = false;
    if ina_addr != 0 {
        for _ in 0..40 {
            // up to ~2s
            let pg_good = in_pg.is_high();
            let v_mv = ina226_read_bus_mv(&mut i2c, ina_addr).await.unwrap_or(0.0);
            let vbus_v = v_mv / 1000.0;
            last_vbus_v = vbus_v;
            last_pg_good = pg_good;
            let range_ok = vbus_v >= VIN_MIN_V && vbus_v <= VIN_MAX_V;
            if pg_good && range_ok {
                vin_on = true;
                break;
            }
            Timer::after(Duration::from_millis(50)).await;
        }
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

    // After VIN ON, scan SC8815 and conditionally init SW2303 per channel
    let sc_addr = sc8815_const::DEFAULT_ADDRESS;
    let sw_addr = sw2303_const::DEFAULT_ADDRESS;
    if vin_on {
        info!("i2c.scan:start vin_on=true");
        for ch in 0u8..4u8 {
            let _ = pca9545a_select(&mut i2c, ch).await;

            // Probe SC8815 by reading a status register
            let mut sc_ok = false;
            // SC8815 driver owns I2C; temporarily move and release back after use
            let mut sc_drv = sc8815::SC8815::new(i2c, sc_addr);
            let sc_present = sc_drv.read_register(ScReg::Status).await.is_ok();
            if sc_present {
                // Initialize SC8815 per design
                if sc_drv.init().await.is_ok() {
                    let _ = sc_drv.set_otg_mode(true).await;
                    let _ = sc_drv.set_vbus_internal_voltage(5000, 1).await;
                    match ch {
                        0 => ce1.set_low(),
                        1 => ce2.set_low(),
                        2 => ce3.set_low(),
                        3 => ce4.set_low(),
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
            i2c = sc_drv.release();

            // Probe/init SW2303 if SC8815 VBUS ready
            let mut sw_ok = false;
            if sc_ok {
                // Read SW2303 device ID register (0x00) for online detection
                let mut id_buf = [0u8; 1];
                let online = embedded_hal_async::i2c::I2c::write_read(
                    &mut i2c,
                    sw_addr,
                    &[0x00],
                    &mut id_buf,
                )
                .await
                .is_ok();
                if online {
                    let mut sw = sw2303::SW2303::new(&mut i2c, sw_addr);
                    if sw.init().await.is_ok() {
                        sw_ok = true;
                    }
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
        }
    } else {
        warn!("pwr.in: vin_on=false; skip module scan");
    }

    // Spawn periodic status task
    spawner.spawn(status_task(ina_addr)).ok();

    // Main loop idle
    loop {
        Timer::after(Duration::from_secs(60)).await;
    }
}

#[embassy_executor::task]
async fn status_task(ina_addr: u8) {
    // Recreate peripherals locally
    let p = unsafe { esp_hal::peripherals::Peripherals::steal() };
    let mut i2c = I2c::new(p.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(p.GPIO8)
        .with_scl(p.GPIO9)
        .into_async();
    let mut adc_cfg = AdcConfig::new();
    let mut vin_pin = adc_cfg.enable_pin(p.GPIO4, Attenuation::_11dB);
    let mut adc1 = Adc::new(p.ADC1, adc_cfg);
    let in_pg = Input::new(
        p.GPIO42,
        esp_hal::gpio::InputConfig::default().with_pull(Pull::Up),
    );

    loop {
        // Read INA226
        let mut vin_v = core::f32::NAN;
        let mut i_a = core::f32::NAN;
        if ina_addr != 0 {
            match ina226_read_bus_mv(&mut i2c, ina_addr).await {
                Ok(mv) => vin_v = mv / 1000.0,
                Err(_) => warn!("pwr.in:read warn=ina_vbus"),
            }
            match ina226_read_shunt_uv(&mut i2c, ina_addr).await {
                Ok(uv) => i_a = (uv / 1_000_000.0) / SHUNT_RESISTANCE_OHMS,
                Err(_) => warn!("pwr.in:read warn=ina_shunt"),
            }
        }

        // Read ADC
        let vin_adc_v = match nb::block!(adc1.read_oneshot(&mut vin_pin)) {
            Ok(raw) => {
                // Raw -> volts: esp-hal ADC units are not directly in mV; without calibration, treat as proportion within 0..4095 -> 0..3.3V
                // For MVP, approximate using 12-bit full-scale mapping.
                let v = (raw as f32) * (3.3 / 4095.0);
                v * VIN_ADC_DIV
            }
            Err(_) => {
                warn!("pwr.in:read warn=vin_adc");
                core::f32::NAN
            }
        };

        // PG
        let pg_good = in_pg.is_high();

        // Capture target state (intent) and actual (by PG)
        let sw_state = match PWR_SW_TARGET.load(Ordering::Relaxed) {
            x if x == PowerSwitchTarget::Closed as u8 => PowerSwitchTarget::Closed,
            _ => PowerSwitchTarget::Open,
        };
        let sw_intent = match sw_state {
            PowerSwitchTarget::Open => "off",
            PowerSwitchTarget::Closed => "on",
        };
        let sw_actual = if pg_good { "on" } else { "off" };

        // Anomaly note when closed & pg good
        let mut note: &str = "";
        if sw_state == PowerSwitchTarget::Closed
            && pg_good
            && vin_v.is_finite()
            && vin_adc_v.is_finite()
        {
            let ratio = vin_adc_v / vin_v;
            let delta = (vin_v - vin_adc_v).abs();
            if ratio < ADC_INA_RATIO_MIN || delta > ADC_INA_DELTA_MAX_V {
                // Limited formatting to keep within single line
                note = "anom: vin_adc<<ina_v";
            }
        }

        // Log per spec (use on/off; print both intent and actual)
        info!(
            "pwr.in:stat vin={}V i={}A sw_intent={} sw_actual={} pg={}{}{}",
            vin_v,
            i_a,
            sw_intent,
            sw_actual,
            if pg_good { "good" } else { "bad" },
            if note.is_empty() { "" } else { " " },
            note
        );

        Timer::after(Duration::from_secs(10)).await;
    }
}
