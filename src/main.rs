//! ESP32-S3 MVP Firmware
//!
//! Implements the MVP per docs/software_design.md:
//! - Boot init: time, GPIO, I2C, basic presence scans
//! - I2C mux PCA9545A (0x70) split + per-channel device ACK checks (SC8815/SW2303)
//! - Front-panel TCA6408A (0x21) presence check
//! - Power input subsystem MVP: INA226-based input qualification and 10s status log

#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU8, Ordering};
use defmt::{error, info, warn};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
// Note: use fully-qualified trait calls for embedded-hal to avoid unused-import lints under clippy -D warnings
use esp_backtrace as _;
use esp_hal::gpio::{Input, Level, Output, Pull};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::spi::Mode as SpiMode;
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;
use sc8815::registers::Register as ScReg;
use sc8815::{
    CellCount, DeadTime, DeviceConfiguration, OperatingMode, SwitchingFrequency, VoltagePerCell,
};
// Shared I2C bus infrastructure
mod power_in;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Receiver;
use embassy_sync::mutex::Mutex;
// use embassy_sync::signal::Signal; // not used on this branch
use static_cell::StaticCell;
mod fan;
mod front_panel;

// No global mutex in MVP

// We manually drive PCA9545A (0x70) via async I2C writes

// INA226 is handled inside power_in task

// Use SC8815/SW2303 crates only for their default I2C addresses
use sc8815::registers::constants as sc8815_const;
use sw2303::registers::constants as sw2303_const;
use sw2303::registers::{constants as swc, Register as SwReg};
use xca9545a_async as pca9545;
// Display driver
use embedded_graphics::{
    pixelcolor::Rgb565, prelude::*, primitives::PrimitiveStyle, primitives::Rectangle,
};
// esp-hal Output has inherent set_low/set_high; no trait import needed
use embedded_hal_async::spi::{Operation as SpiOp, SpiBus as Eh1SpiBus, SpiDevice as Eh1SpiDevice};
use gc9d01::{Config as DisplayConfig, Orientation, Timer as Gc9d01Timer, GC9D01};

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
static mut I2C_BUS_REF: Option<&'static SharedI2cBus> = None;
// No per-channel statics; channels are lightweight views over the shared bus

// （已按要求撤销 TCA6408A 虚拟 GPIO 实现）

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
#[allow(dead_code)]
const PIN_I2C_SDA: u8 = 8;
#[allow(dead_code)]
const PIN_I2C_SCL: u8 = 9;
#[allow(dead_code)]
const PIN_I2C_INT: u8 = 16;
#[allow(dead_code)]
const PIN_I2C_RESET: u8 = 38; // open-drain, low to reset I2C peripherals

#[allow(dead_code)]
const PIN_IN_EN: u8 = 41; // TPS2490 enable (high = on)
#[allow(dead_code)]
const PIN_IN_PG: u8 = 42; // TPS2490 PG (open drain, high = good)

// SC8815 PSTOP control lines via MCU-side PSTOP_CTL (board-inverted to module PSTOP)
// MCU: PSTOP_CTL high -> module PSTOP low (enable)
#[allow(dead_code)]
const PIN_PSTOP_CTL1: u8 = 17;
#[allow(dead_code)]
const PIN_PSTOP_CTL2: u8 = 18;
#[allow(dead_code)]
const PIN_PSTOP_CTL3: u8 = 39;
#[allow(dead_code)]
const PIN_PSTOP_CTL4: u8 = 40;

// INA226 shunt value (ohms) from docs: 5 mΩ
const SHUNT_RESISTANCE_OHMS: f32 = 0.005;

// Qualification thresholds (see docs/software_design.md §2)
// 开发阶段临时放宽：将 VIN 下限改为 4.5V 以便在 5V 供电/风扇台架下进行功能验证；
// 量产应恢复到 9.0V 以避免 5–8V 区间误判上电（请在合入前复位该值）。
const VIN_MIN_V: f32 = 4.5;
const VIN_MAX_V: f32 = 24.0;
const I_IDLE_MAX_A: f32 = 0.010; // 10 mA

// SC8815 VBUS readiness requirements
const SC8815_VBUS_READY_MV: u16 = 4000;
const SC8815_VBUS_READY_CONSECUTIVE: u8 = 2;
const SC8815_VBUS_READY_INTERVAL_MS: u64 = 50;

const SC8815_OUTPUT_VOLTAGE_MV: u16 = 5000;
const SC8815_IBUS_LIMIT_MA: u16 = 5000;
const SC8815_IBAT_LIMIT_MA: u16 = 6000;
const SC8815_RS1_MOHM: u16 = 5;
const SC8815_RS2_MOHM: u16 = 5;

// ===== Display pin mapping (assumed; please confirm) =====
// SPI2 SCLK/MOSI to panel; MISO unused.
// DC uses MCU GPIO (data/command). Backlight not managed here.
// 回退实现：MCU 直接控制 CS/RES，与示例一致
const PIN_LCD_SCLK: u8 = 12;
const PIN_LCD_MOSI: u8 = 11;
const PIN_LCD_DC: u8 = 10;
const PIN_LCD_CS_GPIO: u8 = 13;
const PIN_LCD_RST_GPIO: u8 = 14;
const PIN_LCD_BLK_GPIO: u8 = 15;

// TCA6408A 地址仅用于存在性检测日志
const TCA6408_ADDR: u8 = 0x21;

fn sc8815_default_device_config() -> DeviceConfiguration {
    let mut config = DeviceConfiguration::default();
    config.battery.cell_count = CellCount::Cells4S;
    config.battery.voltage_per_cell = VoltagePerCell::Mv4200;
    config.battery.use_internal_setting = true;
    config.current_limits.rs1_mohm = SC8815_RS1_MOHM;
    config.current_limits.rs2_mohm = SC8815_RS2_MOHM;
    config.current_limits.ibus_limit_ma = SC8815_IBUS_LIMIT_MA;
    config.current_limits.ibat_limit_ma = SC8815_IBAT_LIMIT_MA;
    config.power.operating_mode = OperatingMode::OTG;
    config.power.switching_frequency = SwitchingFrequency::Freq450kHz;
    config.power.dead_time = DeadTime::Ns80;
    config.power.vinreg_voltage_mv = SC8815_OUTPUT_VOLTAGE_MV;
    config.trickle_charging = false;
    config.charging_termination = false;
    config.use_ibus_for_charging = false;
    config
}

// Minimal SC8815 status register address for ACK probe (per sc8815-rs README)
const SC8815_STATUS_REG_ADDR: u8 = 0x17;

// SC8815 detect retries to handle delayed device readiness (total ~2s)
const SC8815_DETECT_INTERVAL_MS: u64 = 50; // ms between attempts
const SC8815_DETECT_TOTAL_MS: u64 = 300; // overall grace per channel (faster: 6x50ms)
const SC8815_DETECT_RETRIES: u8 = (SC8815_DETECT_TOTAL_MS / SC8815_DETECT_INTERVAL_MS) as u8; // 40

fn sc_err_tag<E: core::fmt::Debug>(err: &sc8815::error::Error<E>) -> &'static str {
    use sc8815::error::Error::*;
    match err {
        I2c(_) => "i2c",
        InvalidRegisterOrParameter => "invalid_reg",
        InvalidParameter => "invalid_param",
        DeviceNotResponding => "no_device",
        Timeout => "timeout",
        PowerConfigError => "power_config",
        InitializationFailed => "init_failed",
        InvalidDeviceState => "bad_state",
        OvercurrentDetected => "overcurrent",
        OvervoltageDetected => "overvoltage",
        ThermalProtection => "thermal",
        BatteryError => "battery",
        ChargingError => "charging",
    }
}

struct ChannelDevice {
    bus: &'static SharedI2cBus,
    mask: u8,
}

impl ChannelDevice {
    fn new(bus: &'static SharedI2cBus, mask: u8) -> Self {
        Self { bus, mask }
    }
}

impl embedded_hal_async::i2c::ErrorType for ChannelDevice {
    type Error = esp_hal::i2c::master::Error;
}

impl embedded_hal_async::i2c::I2c for ChannelDevice {
    async fn transaction(
        &mut self,
        address: u8,
        operations: &mut [embedded_hal::i2c::Operation<'_>],
    ) -> Result<(), Self::Error> {
        let mut guard = self.bus.lock().await;
        embedded_hal_async::i2c::I2c::write(&mut *guard, 0x70, &[self.mask]).await?;
        embedded_hal_async::i2c::I2c::transaction(&mut *guard, address, operations).await
    }

    async fn write(&mut self, address: u8, write: &[u8]) -> Result<(), Self::Error> {
        let mut guard = self.bus.lock().await;
        embedded_hal_async::i2c::I2c::write(&mut *guard, 0x70, &[self.mask]).await?;
        embedded_hal_async::i2c::I2c::write(&mut *guard, address, write).await
    }

    async fn read(&mut self, address: u8, read: &mut [u8]) -> Result<(), Self::Error> {
        let mut guard = self.bus.lock().await;
        embedded_hal_async::i2c::I2c::write(&mut *guard, 0x70, &[self.mask]).await?;
        embedded_hal_async::i2c::I2c::read(&mut *guard, address, read).await
    }

    async fn write_read(
        &mut self,
        address: u8,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), Self::Error> {
        let mut guard = self.bus.lock().await;
        embedded_hal_async::i2c::I2c::write(&mut *guard, 0x70, &[self.mask]).await?;
        embedded_hal_async::i2c::I2c::write_read(&mut *guard, address, write, read).await
    }
}

fn mux_channel(ch: u8) -> ChannelDevice {
    let bus = unsafe { I2C_BUS_REF.expect("I2C bus not initialized") };
    let idx = (ch & 0x03) as usize;
    let mask = 1u8 << idx;
    ChannelDevice::new(bus, mask)
}

async fn sc8815_ack<I2C: embedded_hal_async::i2c::I2c>(
    i2c: &mut I2C,
    addr: u8,
) -> (bool, &'static str) {
    let mut b = [0u8; 1];
    if embedded_hal_async::i2c::I2c::write_read(i2c, addr, &[SC8815_STATUS_REG_ADDR], &mut b)
        .await
        .is_ok()
    {
        return (true, "wr_rd");
    }
    if embedded_hal_async::i2c::I2c::read(i2c, addr, &mut b)
        .await
        .is_ok()
    {
        return (true, "rd");
    }
    if embedded_hal_async::i2c::I2c::write(i2c, addr, &[])
        .await
        .is_ok()
    {
        return (true, "addr");
    }
    (false, "no")
}

// NOTE: no arbitrary address enumeration; only probe known devices.

// 按项目要求：不在固件中操作 TCA6408A（不扫描/不复位/不拉 CS），RES/CS 由外部电路或其它控制实体负责。

// Embassy timer adapter for the display driver
struct DisplayTimer;
impl Gc9d01Timer for DisplayTimer {
    async fn after_millis(ms: u64) {
        Timer::after(Duration::from_millis(ms)).await;
    }
}

// Minimal Eh1-async SpiDevice wrapper over esp-hal async SPI bus.
// CS is permanently held low by TCA6408A (P6), so we don't toggle CS here.
struct SimpleSpiDev<'a, BUS> {
    bus: BUS,
    cs: Option<Output<'a>>,
}

impl<'a, BUS> embedded_hal::spi::ErrorType for SimpleSpiDev<'a, BUS>
where
    BUS: Eh1SpiBus<Error = esp_hal::spi::Error>,
{
    type Error = esp_hal::spi::Error;
}

impl<'a, BUS> Eh1SpiDevice for SimpleSpiDev<'a, BUS>
where
    BUS: Eh1SpiBus<Error = esp_hal::spi::Error>,
{
    async fn transaction(&mut self, ops: &mut [SpiOp<'_, u8>]) -> Result<(), Self::Error> {
        if let Some(cs) = self.cs.as_mut() {
            cs.set_low();
        }
        for op in ops.iter_mut() {
            match op {
                SpiOp::Write(w) => {
                    self.bus.write(w).await?;
                }
                SpiOp::Read(_r) => {
                    // Not used by this driver path
                    // If needed, implement via self.bus.read
                    // For safety, return Ok without action
                }
                SpiOp::Transfer(_r, _w) => {}
                SpiOp::TransferInPlace(_b) => {}
                SpiOp::DelayNs(_ns) => {}
            }
        }
        if let Some(cs) = self.cs.as_mut() {
            cs.set_high();
        }
        Ok(())
    }
}

// Initialize SPI + GC9D01 and draw a chessboard pattern (inlined in main where pins are available)

// NoopPin for RST placeholder: real reset handled via TCA6408 P5
pub struct NoopPin;
impl embedded_hal::digital::ErrorType for NoopPin {
    type Error = core::convert::Infallible;
}
impl embedded_hal::digital::OutputPin for NoopPin {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

async fn ack_scan_vin_off(sc_addr: u8) {
    for ch in 0u8..4u8 {
        let mut i2c_scan = mux_channel(ch);
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
        let mux_mask = 1u8 << (ch & 0x03);
        info!(
            "i2c.scan: ch={} mux_reg=0x{:02X} sc8815_ack={} via={} tries={} vin_on=false",
            ch,
            mux_mask,
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
    let in_en = Output::new(p.GPIO41, Level::Low, esp_hal::gpio::OutputConfig::default());
    // PG input
    let in_pg = Input::new(
        p.GPIO42,
        esp_hal::gpio::InputConfig::default().with_pull(Pull::Up),
    );

    // Front-panel INT pin will be initialized only if panel is present.

    // PSTOP_CTL lines default disabled (drive low => board-inverted PSTOP=high -> module disabled)
    let mut pstop_ctl1 = Output::new(p.GPIO17, Level::Low, esp_hal::gpio::OutputConfig::default());
    let mut pstop_ctl2 = Output::new(p.GPIO18, Level::Low, esp_hal::gpio::OutputConfig::default());
    let mut pstop_ctl3 = Output::new(p.GPIO39, Level::Low, esp_hal::gpio::OutputConfig::default());
    let mut pstop_ctl4 = Output::new(p.GPIO40, Level::Low, esp_hal::gpio::OutputConfig::default());
    // Keep variables used
    pstop_ctl1.set_low();
    pstop_ctl2.set_low();
    pstop_ctl3.set_low();
    pstop_ctl4.set_low();

    info!("init.hw: chip=ESP32-S3 i2c=ok sda=GPIO8 scl=GPIO9");

    // Publish power-on intent first (only intent; actual switch controlled after qualification)
    PWR_SW_TARGET.store(PowerSwitchTarget::Closed as u8, Ordering::Relaxed);

    // Upstream TCA6408A presence check (0x21) using async I2C — runs immediately after publishing intent
    // Initialize I2C0 once and share via Mutex + I2cDevice
    let i2c_hw = I2c::new(p.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(p.GPIO8)
        .with_scl(p.GPIO9)
        .into_async();
    let bus = I2C_BUS.init(Mutex::new(i2c_hw));
    // MCU 直接控制 CS/RES（回退实现）
    info!("lcd.ctrl: cs,res via MCU GPIO");

    // Setup SPI2 and display. CS/RES 由 TCA6408A 控制（本固件不介入）。
    let spi_bus = Spi::new(
        p.SPI2,
        SpiConfig::default()
            .with_frequency(esp_hal::time::Rate::from_hz(10_000_000))
            .with_mode(SpiMode::_0),
    )
    .unwrap()
    .with_sck(match PIN_LCD_SCLK {
        12 => p.GPIO12,
        _ => p.GPIO12,
    })
    .with_mosi(match PIN_LCD_MOSI {
        11 => p.GPIO11,
        _ => p.GPIO11,
    })
    .into_async();

    let dc = match PIN_LCD_DC {
        10 => Output::new(p.GPIO10, Level::Low, esp_hal::gpio::OutputConfig::default()),
        _ => Output::new(p.GPIO10, Level::Low, esp_hal::gpio::OutputConfig::default()),
    };
    // Backlight on (high)
    let mut blk = match PIN_LCD_BLK_GPIO {
        15 => Output::new(p.GPIO15, Level::Low, esp_hal::gpio::OutputConfig::default()),
        _ => Output::new(p.GPIO15, Level::Low, esp_hal::gpio::OutputConfig::default()),
    };
    blk.set_high();

    const LOGICAL_W: usize = 160;
    const LOGICAL_H: usize = 50;
    let mut fb_buf: [Rgb565; LOGICAL_W * LOGICAL_H] = [Rgb565::BLACK; LOGICAL_W * LOGICAL_H];
    let fb: &mut [Rgb565] = &mut fb_buf;

    // 用 MCU CS 脚包一层 SpiDevice，事务内拉低/释放
    let cs = match PIN_LCD_CS_GPIO {
        13 => Output::new(
            p.GPIO13,
            Level::High,
            esp_hal::gpio::OutputConfig::default(),
        ),
        _ => Output::new(
            p.GPIO13,
            Level::High,
            esp_hal::gpio::OutputConfig::default(),
        ),
    };
    let spi_dev = SimpleSpiDev {
        bus: spi_bus,
        cs: Some(cs),
    };
    let cfg = DisplayConfig {
        width: LOGICAL_W as u16,
        height: LOGICAL_H as u16,
        orientation: Orientation::Landscape,
        rgb: false,
        inverted: false,
        dx: 15,
        dy: 0,
    };
    let rst = match PIN_LCD_RST_GPIO {
        14 => Output::new(
            p.GPIO14,
            Level::High,
            esp_hal::gpio::OutputConfig::default(),
        ),
        _ => Output::new(
            p.GPIO14,
            Level::High,
            esp_hal::gpio::OutputConfig::default(),
        ),
    };
    let mut disp: GC9D01<_, _, _, DisplayTimer> = GC9D01::new(cfg, spi_dev, dc, rst, fb);
    info!("lcd.init: start panel_160x50 mode (fallback MCU CS/RST)");
    if let Err(_e) = disp.init().await {
        warn!("lcd.init: failed (fallback)");
    } else {
        // draw chessboard
        let _ = disp.clear(Rgb565::BLACK);
        let bw = 10u16;
        let bh = 10u16;
        let nx = (LOGICAL_W as u16).div_ceil(bw);
        let ny = (LOGICAL_H as u16).div_ceil(bh);
        for r in 0..ny {
            for c in 0..nx {
                let color = if (r + c) % 2 == 0 {
                    Rgb565::WHITE
                } else {
                    Rgb565::BLACK
                };
                let x = c * bw;
                let y = r * bh;
                let w = core::cmp::min(bw, LOGICAL_W as u16 - x);
                let h = core::cmp::min(bh, LOGICAL_H as u16 - y);
                let _ = Rectangle::new(
                    Point::new(x as i32, y as i32),
                    Size::new(w as u32, h as u32),
                )
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(&mut disp);
            }
        }
        let _ = disp.flush().await;
        info!("lcd.draw: chessboard done (fallback)");
    }

    // I2C mux PCA9545A presence check via async driver
    let mut pca = pca9545::Pca9545a::new(I2cDevice::new(bus), pca9545::DEFAULT_ADDRESS);
    match pca.get_channel_status().await {
        Ok(status) => info!("i2c.mux: ok addr=0x70 parts=4 status=0x{:02X}", status),
        Err(_) => {
            error!("i2c.mux: err=init addr=0x70");
            panic!("PCA9545A not found");
        }
    }
    // Record global bus reference for channel views
    unsafe {
        I2C_BUS_REF = Some(bus);
    }

    // Probe front-panel presence and conditionally enable related features
    if front_panel::is_present(bus).await {
        info!("i2c.front: tca6408a=online addr=0x{:02X}", TCA6408_ADDR);
        let int_pin = Input::new(
            p.GPIO16,
            esp_hal::gpio::InputConfig::default().with_pull(Pull::Up),
        );
        // Spawn front-panel task to log falling edges on P0..P4 from TCA6408A
        front_panel::spawn(&spawner, bus, int_pin).expect("spawn front_panel task");
    } else {
        warn!(
            "i2c.front: tca6408a=offline addr=0x{:02X}; disable related features",
            TCA6408_ADDR
        );
    }

    // Spawn SW2303 CH0 telemetry task (prints once per second)
    #[embassy_executor::task]
    async fn sw2303_ch0_telemetry_task() {
        info!("sw2303.ch0: task_start");
        let sw_addr = swc::DEFAULT_ADDRESS;
        let mut diag_left: u8 = 3;
        loop {
            let mut i2c_ch0 = mux_channel(0);
            let mut sw = sw2303::SW2303::new(&mut i2c_ch0, sw_addr);

            // Read 12-bit ADC VBUS
            let vbus_mv = async {
                let adc_cfg_before = sw.read_register(SwReg::AdcConfig).await.ok();
                if sw
                    .write_register(SwReg::AdcConfig, swc::adc::ADC_SELECT_VBUS)
                    .await
                    .is_ok()
                {
                    let adc_cfg_after = sw.read_register(SwReg::AdcConfig).await.ok();
                    if let Ok(h) = sw.read_register(SwReg::AdcDataHigh).await {
                        if let Ok(l) = sw.read_register(SwReg::AdcDataLow).await {
                            let raw12 = (((h as u16) << 4) | ((l & 0x0F) as u16)) as u32;
                            if diag_left > 0 {
                                info!(
                                    "sw2303.ch0: adc.vbus cfg_bef={:?} cfg_aft={:?} raw_h=0x{:02X} raw_l=0x{:02X} raw12={}",
                                    adc_cfg_before, adc_cfg_after, h, l, raw12
                                );
                            }
                            return Some(raw12 as f32 * swc::adc::VBUS_FACTOR_MV);
                        }
                    }
                }
                None
            }
            .await;

            // Read 12-bit ADC ICH (3.125 mA/LSB)
            let ich_ma = async {
                if sw
                    .write_register(SwReg::AdcConfig, swc::adc::ADC_SELECT_ICH)
                    .await
                    .is_ok()
                {
                    // Allow a brief conversion time after channel select
                    Timer::after(Duration::from_micros(500)).await;
                    if let Ok(h) = sw.read_register(SwReg::AdcDataHigh).await {
                        if let Ok(l) = sw.read_register(SwReg::AdcDataLow).await {
                            let raw12 = (((h as u16) << 4) | ((l & 0x0F) as u16)) as u32;
                            let ich_factor_ma_12 = swc::adc::ICH_FACTOR_MA / 16.0;
                            return Some(raw12 as f32 * ich_factor_ma_12);
                        }
                    }
                }
                None
            }
            .await;

            let online = match sw.is_sink_device_connected().await {
                Ok(b) => b,
                Err(_) => false,
            };

            match (vbus_mv, ich_ma) {
                (Some(v), Some(i)) => {
                    let v_mv: u32 = v as u32;
                    let i_ma: u32 = i as u32;

                    // Read SC8815 IBUS on CH0 for cross-check (ratio=3x, RS1=5mΩ)
                    let sc_addr = sc8815_const::DEFAULT_ADDRESS;
                    let i2c_sc = mux_channel(0);
                    let mut sc = sc8815::SC8815::new(i2c_sc, sc_addr);
                    let sc_ibus = sc.read_ibus_current(2, SC8815_RS1_MOHM).await.ok();

                    // Also read SW2303 8-bit ICH value for cross-check (always)
                    let ich8_ma_opt: Option<u32> = match sw.read_register(SwReg::AdcIch).await {
                        Ok(v8) => Some((v8 as f32 * swc::adc::ICH_FACTOR_MA) as u32),
                        Err(_) => None,
                    };
                    // Derive a simple source-active flag from VBUS level
                    let src_active = v_mv >= 4500;
                    match (sc_ibus, ich8_ma_opt) {
                        (Some(ibus_sc), Some(i8)) => {
                            let delta = ibus_sc as i32 - i_ma as i32;
                            info!(
                                "sw2303.ch0: sink_online={} src_active={} vbus={}mV ich_sw12={}mA ich_sw8={}mA ibus_sc={}mA delta={}mA",
                                if online { "true" } else { "false" },
                                if src_active { "true" } else { "false" },
                                v_mv,
                                i_ma,
                                i8,
                                ibus_sc,
                                delta
                            );
                        }
                        (Some(ibus_sc), None) => {
                            let delta = ibus_sc as i32 - i_ma as i32;
                            info!(
                                "sw2303.ch0: sink_online={} src_active={} vbus={}mV ich_sw12={}mA ich_sw8=na ibus_sc={}mA delta={}mA",
                                if online { "true" } else { "false" },
                                if src_active { "true" } else { "false" },
                                v_mv,
                                i_ma,
                                ibus_sc,
                                delta
                            );
                        }
                        (None, Some(i8)) => {
                            info!(
                                "sw2303.ch0: sink_online={} src_active={} vbus={}mV ich_sw12={}mA ich_sw8={}mA ibus_sc=na",
                                if online { "true" } else { "false" },
                                if src_active { "true" } else { "false" },
                                v_mv,
                                i_ma,
                                i8
                            );
                        }
                        (None, None) => {
                            info!(
                                "sw2303.ch0: sink_online={} src_active={} vbus={}mV ich_sw12={}mA ich_sw8=na ibus_sc=na",
                                if online { "true" } else { "false" },
                                if src_active { "true" } else { "false" },
                                v_mv,
                                i_ma
                            );
                        }
                    }

                    if diag_left > 0 {
                        diag_left = diag_left.saturating_sub(1);
                    }
                }
                _ => {
                    warn!(
                        "sw2303.ch0: read_failed online={} ",
                        if online { "true" } else { "false" }
                    );
                }
            }

            Timer::after(Duration::from_secs(1)).await;
        }
    }
    info!("sw2303.ch0: spawn");
    spawner
        .spawn(sw2303_ch0_telemetry_task())
        .expect("spawn sw2303_ch0_telemetry_task");

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
        rx: Receiver<
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

    // Spawn fan control + tach task early: calibrate max RPM then 10/50/100% loop
    // Spawns regardless of VIN_ON to allow bench 5V fan tests.
    fan::spawn(&spawner, p.LEDC, p.PCNT, p.SENS, p.GPIO1, p.GPIO2, p.GPIO6)
        .expect("spawn fan task");

    // Wait for VIN_ON signal before scanning SC8815 modules; fallback to ACK-only scan when false
    let vin_on = power_in::vin_on_signal().wait().await;
    if !vin_on {
        warn!("pwr.in: vin_on=false; skip module init; do ack-scan only");
        let sc_addr = sc8815_const::DEFAULT_ADDRESS;
        ack_scan_vin_off(sc_addr).await;
        // Keep executor alive so background tasks (power_in, fan) run
        core::future::pending::<()>().await;
    }

    // After VIN ON, scan SC8815 and conditionally init SW2303 per channel
    let sc_addr = sc8815_const::DEFAULT_ADDRESS;
    let sw_addr = sw2303_const::DEFAULT_ADDRESS;
    info!("i2c.scan:start vin_on=true");
    for ch in 0u8..4u8 {
        let mut i2c_scan = mux_channel(ch);
        let mux_mask = 1u8 << (ch & 0x03);
        info!(
            "i2c.mux: ch={} select=ok ctrl_read=implicit reg=0x{:02X}",
            ch, mux_mask
        );
        Timer::after(Duration::from_millis(10)).await;

        // Pre-probe ACK on expected address with retries.
        let mut sc_ack = false;
        let mut ack_method = "no";
        let mut tries: u8 = 0;
        for attempt in 0..SC8815_DETECT_RETRIES {
            let (ok, method) = sc8815_ack(&mut i2c_scan, sc_addr).await;
            tries = attempt + 1;
            if ok {
                sc_ack = true;
                ack_method = method;
                break;
            }
            Timer::after(Duration::from_millis(SC8815_DETECT_INTERVAL_MS)).await;
        }
        if sc_ack {
            info!(
                "pwr.sc8815: ch={} ack=ok via={} tries={}",
                ch, ack_method, tries
            );
        } else {
            warn!(
                "pwr.sc8815: ch={} ack=fail via={} tries={}",
                ch, ack_method, tries
            );
        }

        // Probe SC8815 by reading a status register (only if ACK)
        let mut sc_ok = false;
        let mut sc_present = false;
        let mut sc_init_ok = false;
        let mut sc_status_err = None;
        let mut sc_init_err = None;
        let mut sc_vbus_last = 0u16;
        let mut sc_status_val: Option<u8> = None;
        // SC8815 driver owns I2C; temporarily move and release back after use
        let mut sc_drv = sc8815::SC8815::new(i2c_scan, sc_addr);
        if sc_ack {
            match sc_drv.read_register(ScReg::Status).await {
                Ok(v) => {
                    sc_present = true;
                    sc_status_val = Some(v);
                    info!("pwr.sc8815: ch={} status=0x{:02X}", ch, v);
                }
                Err(e) => {
                    sc_status_err = Some(sc_err_tag(&e));
                }
            }
            // Quick SW2303 diagnostics on same channel (once per scan)
            let mut i2c_sw = mux_channel(ch);
            let mut sw_dbg = sw2303::SW2303::new(&mut i2c_sw, sw_addr);
            if let Ok(s3) = sw_dbg.get_system_status3().await {
                info!("sw2303.ch{}: sys3=0b{:08b}", ch, s3.bits());
            }
            if let Ok(s0) = sw_dbg.get_system_status0().await {
                info!("sw2303.ch{}: sys0=0b{:08b}", ch, s0.bits());
            }
            if let Ok(fc) = sw_dbg.get_fast_charging_status().await {
                info!("sw2303.ch{}: fastchg=0b{:08b}", ch, fc.bits());
            }
            if let Ok(cc) = sw_dbg.read_register(SwReg::ConnectionControl).await {
                info!("sw2303.ch{}: reg14(conn)=0x{:02X}", ch, cc);
            }
            match sw_dbg.is_sink_device_connected().await {
                Ok(b) => info!(
                    "sw2303.ch{}: sink_online={}",
                    ch,
                    if b { "true" } else { "false" }
                ),
                Err(_) => warn!("sw2303.ch{}: sink_online=err", ch),
            }
        }
        if !sc_present {
            if let Some(tag) = sc_status_err {
                warn!(
                    "pwr.sc8815: ch={} status_read_err={} ack={}",
                    ch,
                    tag,
                    if sc_ack { "yes" } else { "no" }
                );
            }
        } else {
            // Initialize SC8815 per design (keep PSTOP_CTL low until init succeeds)
            match sc_drv.init().await {
                Ok(()) => {
                    sc_init_ok = true;
                }
                Err(e) => {
                    sc_init_err = Some(sc_err_tag(&e));
                }
            }
            if !sc_init_ok {
                if let Some(tag) = sc_init_err {
                    warn!("pwr.sc8815: ch={} init_err={}", ch, tag);
                }
            } else {
                info!(
                    "pwr.sc8815: ch={} init_ok status=0x{:02X}",
                    ch,
                    sc_status_val.unwrap_or(0x00)
                );
                match sc_drv.read_register(ScReg::Ctrl0Set).await {
                    Ok(v) => info!("pwr.sc8815: ch={} ctrl0=0x{:02X}", ch, v),
                    Err(e) => warn!("pwr.sc8815: ch={} ctrl0_err={}", ch, sc_err_tag(&e)),
                }
                match sc_drv.read_register(ScReg::Ctrl1Set).await {
                    Ok(v) => info!("pwr.sc8815: ch={} ctrl1=0x{:02X}", ch, v),
                    Err(e) => warn!("pwr.sc8815: ch={} ctrl1_err={}", ch, sc_err_tag(&e)),
                }
                match sc_drv.read_register(ScReg::Ctrl2Set).await {
                    Ok(v) => info!("pwr.sc8815: ch={} ctrl2=0x{:02X}", ch, v),
                    Err(e) => warn!("pwr.sc8815: ch={} ctrl2_err={}", ch, sc_err_tag(&e)),
                }
                match sc_drv.read_register(ScReg::Mask).await {
                    Ok(v) => info!("pwr.sc8815: ch={} mask=0x{:02X}", ch, v),
                    Err(e) => warn!("pwr.sc8815: ch={} mask_err={}", ch, sc_err_tag(&e)),
                }
                match sc_drv.read_register(ScReg::Ctrl3Set).await {
                    Ok(v) => info!("pwr.sc8815: ch={} ctrl3=0x{:02X}", ch, v),
                    Err(e) => warn!("pwr.sc8815: ch={} ctrl3_err={}", ch, sc_err_tag(&e)),
                }
                let sc_config = sc8815_default_device_config();
                let mut sc_startup_ok = true;

                let foldback_res = sc_drv.set_short_foldback_disable(true).await;
                if let Err(e) = foldback_res {
                    warn!("pwr.sc8815: ch={} foldback_err={}", ch, sc_err_tag(&e));
                    sc_startup_ok = false;
                } else {
                    info!("pwr.sc8815: ch={} foldback=disabled", ch);
                }

                let config_res = sc_drv.configure_device(&sc_config).await;
                if let Err(e) = config_res {
                    warn!("pwr.sc8815: ch={} config_err={}", ch, sc_err_tag(&e));
                    sc_startup_ok = false;
                } else {
                    info!(
                        "pwr.sc8815: ch={} config_applied otg={}mV ibus={}mA ibat={}mA",
                        ch, SC8815_OUTPUT_VOLTAGE_MV, SC8815_IBUS_LIMIT_MA, SC8815_IBAT_LIMIT_MA
                    );
                }

                let otg_res = sc_drv.set_otg_mode(true).await;
                if let Err(e) = otg_res {
                    warn!("pwr.sc8815: ch={} otg_mode_err={}", ch, sc_err_tag(&e));
                    sc_startup_ok = false;
                } else {
                    info!("pwr.sc8815: ch={} otg_mode=enabled", ch);
                }

                // Use external FB mode: set external reference (VBUSREF_E), select FB_SEL=external
                let vref_e_mv: u16 = 1200; // 1.2V external FB reference
                let vbus_res = sc_drv.set_vbus_external_reference(vref_e_mv).await;
                if let Err(e) = vbus_res {
                    warn!("pwr.sc8815: ch={} vbus_set_err={}", ch, sc_err_tag(&e));
                    sc_startup_ok = false;
                } else {
                    info!("pwr.sc8815: ch={} fb=external vref_e={}mV", ch, vref_e_mv);
                }

                match sc_drv.read_register(ScReg::Ratio).await {
                    Ok(v) => info!("pwr.sc8815: ch={} ratio_reg=0x{:02X}", ch, v),
                    Err(e) => warn!("pwr.sc8815: ch={} ratio_read_err={}", ch, sc_err_tag(&e)),
                }

                let adc_res = sc_drv.set_adc_conversion(true).await;
                if let Err(e) = adc_res {
                    warn!("pwr.sc8815: ch={} adc_start_err={}", ch, sc_err_tag(&e));
                    sc_startup_ok = false;
                } else {
                    info!("pwr.sc8815: ch={} adc=start", ch);
                }

                let pgate_res = sc_drv.set_pgate_control(true).await;
                if let Err(e) = pgate_res {
                    warn!("pwr.sc8815: ch={} pgate_err={}", ch, sc_err_tag(&e));
                    sc_startup_ok = false;
                } else {
                    info!("pwr.sc8815: ch={} pgate=enabled", ch);
                }
                if sc_startup_ok {
                    match ch {
                        0 => pstop_ctl1.set_high(),
                        1 => pstop_ctl2.set_high(),
                        2 => pstop_ctl3.set_high(),
                        3 => pstop_ctl4.set_high(),
                        _ => {}
                    }
                    Timer::after(Duration::from_millis(5)).await;
                    match sc_drv.read_register(ScReg::Ctrl3Set).await {
                        Ok(v) => info!("pwr.sc8815: ch={} ctrl3_after=0x{:02X}", ch, v),
                        Err(e) => warn!("pwr.sc8815: ch={} ctrl3_after_err={}", ch, sc_err_tag(&e)),
                    }
                    // Require consecutive VBUS readings above threshold
                    let mut consecutive = 0u8;
                    let mut sample_index = 0u8;
                    let mut vbus_min = u16::MAX;
                    let mut vbus_max = 0u16;
                    let mut ibus_last = 0u16;
                    let mut any_sample = false;
                    for _ in 0..40 {
                        // ~2s with 50ms intervals
                        match sc_drv.get_adc_measurements().await {
                            Ok(meas) => {
                                any_sample = true;
                                sc_vbus_last = meas.vbus_mv;
                                ibus_last = meas.ibus_ma;
                                if meas.vbus_mv < vbus_min {
                                    vbus_min = meas.vbus_mv;
                                }
                                if meas.vbus_mv > vbus_max {
                                    vbus_max = meas.vbus_mv;
                                }
                                info!(
                                    "pwr.sc8815: ch={} adc_sample={} vbus={}mV ibus={}mA",
                                    ch, sample_index, meas.vbus_mv, meas.ibus_ma
                                );
                                sample_index = sample_index.wrapping_add(1);
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
                            Err(e) => {
                                warn!("pwr.sc8815: ch={} adc_err={}", ch, sc_err_tag(&e));
                                break;
                            }
                        }
                        Timer::after(Duration::from_millis(SC8815_VBUS_READY_INTERVAL_MS)).await;
                    }
                    if !sc_ok {
                        let min_report = if any_sample { vbus_min } else { 0 };
                        let max_report = if any_sample { vbus_max } else { 0 };
                        let mut raw_vbus_lsb: Option<u32> = None;
                        let mut raw_ratio_reg: Option<u8> = None;
                        if let Ok(vh) = sc_drv.read_register(ScReg::VbusFbValue).await {
                            if let Ok(vl) = sc_drv.read_register(ScReg::VbusFbValue2).await {
                                let lsb = (4 * (vh as u32)) + (((vl >> 6) & 0x03) as u32) + 1;
                                raw_vbus_lsb = Some(lsb);
                            }
                        }
                        if let Ok(ratio) = sc_drv.read_register(ScReg::Ratio).await {
                            raw_ratio_reg = Some(ratio);
                        }
                        warn!(
                            "pwr.sc8815: ch={} vbus_ready=false last={}mV min={}mV max={}mV ibus={}mA raw_lsb={:?} ratio_reg={:?}",
                            ch,
                            sc_vbus_last,
                            min_report,
                            max_report,
                            ibus_last,
                            raw_vbus_lsb,
                            raw_ratio_reg
                        );
                        if let Some(lsb) = raw_vbus_lsb {
                            let mv_ratio5 = lsb * 10; // 2mV * 5x ratio
                            let mv_ratio12 = lsb * 25; // 2mV * 12.5x ratio
                            info!(
                                "pwr.sc8815: ch={} raw_estimate ratio5={}mV ratio12={}mV",
                                ch, mv_ratio5, mv_ratio12
                            );
                        }
                    }
                } else {
                    warn!("pwr.sc8815: ch={} startup_aborted", ch);
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
            mux_mask,
            if sc_ack { "yes" } else { "no" },
            ack_method,
            tries
        );

        // do not enumerate unknown addresses

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
                0 => pstop_ctl1.set_low(),
                1 => pstop_ctl2.set_low(),
                2 => pstop_ctl3.set_low(),
                3 => pstop_ctl4.set_low(),
                _ => {}
            }
        }
    }
}
