//! ESP32-S3 MVP Firmware
//!
//! Implements the MVP per docs/software_design.md:
//! - Boot init: time, GPIO, I2C, basic presence scans
//! - I2C mux PCA9545A (0x70) split + per-channel IP6557 module bring-up deferral
//! - Front-panel TCA6408A (0x21) presence check
//! - Power input subsystem MVP: INA226-based input qualification and 10s status log

#![no_std]
#![no_main]

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use defmt::{error, info, warn};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
// Note: use fully-qualified trait calls for embedded-hal to avoid unused-import lints under clippy -D warnings
use esp_backtrace as _;
use esp_hal::gpio::{DriveMode, Input, Level, Output, OutputConfig, Pull};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::spi::Mode as SpiMode;
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;
use sc8815::registers::Register as ScReg;
use sc8815::{CellCount, DeadTime, DeviceConfiguration, OperatingMode, SwitchingFrequency};
// Shared I2C bus infrastructure
mod power_in;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Receiver;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
// use embassy_sync::signal::Signal; // not used on this branch
use static_cell::StaticCell;
mod fan;
mod front_panel;

// No global mutex in MVP

// We manually drive PCA9545A (0x70) via async I2C writes

// INA226 is handled inside power_in task

// Legacy SC8815/SW2303 support is kept only for historical fallback code paths.
use sc8815::registers::constants as sc8815_const;
use sw2303::registers::constants as sw2303_const;
use sw2303::registers::{constants as swc, Register as SwReg};
use xca9545a_async as pca9545;
// Display driver
use embedded_graphics::{
    mono_font::{ascii::FONT_7X13_BOLD, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
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

// 各通道初始化完成标志：当前仅在旧子板回退路径中使用。
static CH_READY0: Signal<CriticalSectionRawMutex, bool> = Signal::new();
static CH_READY1: Signal<CriticalSectionRawMutex, bool> = Signal::new();
static CH_READY2: Signal<CriticalSectionRawMutex, bool> = Signal::new();
static CH_READY3: Signal<CriticalSectionRawMutex, bool> = Signal::new();

// Latch channel readiness for UI without blocking on Signal::wait
static CH_RDY: [AtomicBool; 4] = [
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
];
// Scan completion flag per channel (true once initial scan concludes, even if module bring-up is deferred)
static CH_SCAN_DONE: [AtomicBool; 4] = [
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
];
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
const PIN_I2C_RESET: u8 = 35; // open-drain, low to reset I2C peripherals

#[allow(dead_code)]
const PIN_IN_EN: u8 = 41; // TPS2490 enable (high = on)
#[allow(dead_code)]
const PIN_IN_PG: u8 = 42; // TPS2490 PG (open drain, high = good)

// Output module channel enable lines (MCU direct drive, high = enable)
#[allow(dead_code)]
const PIN_EN1: u8 = 17;
#[allow(dead_code)]
const PIN_EN2: u8 = 18;
#[allow(dead_code)]
const PIN_EN3: u8 = 39;
#[allow(dead_code)]
const PIN_EN4: u8 = 40;

// USB channel mux control (CH442E)
// UCM_DIN: route select, low => connect selected HUB downstream lane to MCU
// UCM_DCE: CH442E EN#, low => enable mux
#[allow(dead_code)]
const PIN_UCM_DIN: u8 = 33;
#[allow(dead_code)]
const PIN_UCM_DCE: u8 = 34;

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
// SW2303 探测退避与重试（VBUS 就绪后再等待其上电）
const SW2303_DETECT_BACKOFF_MS: u64 = 150; // 每次间隔 150ms
const SW2303_DETECT_RETRIES: u8 = 20; // 最多重试 20 次（总计 ~3.0s）
                                      // Policy update: SW2303 必须在线，否则判定通道异常并关闭功率级
const ALLOW_SC8815_WITHOUT_SW2303: bool = false;
// Periodic monitor
const SC8815_MONITOR_INTERVAL_MS: u64 = 1000;
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
    // Project uses 4S pack on SC8815 daughter board; ensure VBAT monitor ratio is 12.5x
    // so that VBAT up to ~36V is within ADC range and calculations are correct.
    config.battery.cell_count = CellCount::Cells4S;
    config.current_limits.rs1_mohm = SC8815_RS1_MOHM;
    config.current_limits.rs2_mohm = SC8815_RS2_MOHM;
    config.current_limits.ibus_limit_ma = SC8815_IBUS_LIMIT_MA;
    config.current_limits.ibat_limit_ma = SC8815_IBAT_LIMIT_MA;
    config.power.operating_mode = OperatingMode::OTG;
    config.power.switching_frequency = SwitchingFrequency::Freq450kHz;
    config.power.dead_time = DeadTime::Ns80;
    // Do not set VINREG or VBUS_RATIO in firmware per project policy.
    config
}

// Minimal SC8815 status register address for ACK probe (per sc8815-rs README)
const SC8815_STATUS_REG_ADDR: u8 = 0x17;

// SC8815 detect retries to handle delayed device readiness (total ~2s)
const SC8815_DETECT_INTERVAL_MS: u64 = 50; // ms between attempts
const SC8815_DETECT_TOTAL_MS: u64 = 300; // overall grace per channel (faster: 6x50ms)
const SC8815_DETECT_RETRIES: u8 = (SC8815_DETECT_TOTAL_MS / SC8815_DETECT_INTERVAL_MS) as u8; // 40

fn legacy_sc8815_path_enabled() -> bool {
    false
}

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

// ===== UI: Dashboard renderer (160x50) =====

#[derive(Copy, Clone, Debug, Default)]
struct PortSample {
    connected: bool,
    // millivolts and milliamps for convenience
    vbus_mv: u32,
    ich_ma: u32,
    ui_state: UiPortState,
}

impl PortSample {
    fn power_mw(&self) -> u32 {
        ((self.vbus_mv as u64 * self.ich_ma as u64) / 1000) as u32
    }
}

const UI_BG_GRAY: Rgb565 = Rgb565::new(31, 63, 31); // pure white background for max contrast
const UI_BORDER: Rgb565 = Rgb565::new(0, 0, 0);
const UI_V_YELLOW: Rgb565 = Rgb565::new(31, 45, 0); // darker amber for better contrast on white
const UI_I_RED: Rgb565 = Rgb565::new(31, 0, 0); // vivid red
const UI_W_GREEN: Rgb565 = Rgb565::new(0, 42, 0); // darker green for contrast

// UI states per column
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
enum UiPortState {
    #[default]
    Initializing,
    PowerBlocked, // NOVIN
    Deferred,     // PEND
    Disconnected, // DISC
    Closed,       // OFF
    Overcurrent,  // CC
    Normal,
}

// Embed icon masks (ASCII '0'/'1' bitmaps), authoritative assets per spec
static ICON_DISC_32: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/rivet-icons_close-circle-solid.raw"
));
static ICON_OFF_32: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fluent_plug-disconnected-16-filled.raw"
));
static ICON_CC_32: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fa7-solid_closed-captioning.raw"
));

fn fmt_v(buf: &mut heapless::String<8>, mv: u32) {
    use core::fmt::Write as _;
    buf.clear();
    let v = mv as f32 / 1000.0;
    if v < 10.0 {
        let _ = write!(buf, "{:.2}V", v);
    } else {
        let _ = write!(buf, "{:.1}V", v);
    }
}

fn fmt_i(buf: &mut heapless::String<8>, ma: u32) {
    use core::fmt::Write as _;
    buf.clear();
    if ma >= 1000 {
        let a = ma as f32 / 1000.0;
        let _ = write!(buf, "{:.2}A", a);
    } else {
        let _ = write!(buf, "{}mA", ma);
    }
}

fn fmt_w(buf: &mut heapless::String<8>, mw: u32) {
    use core::fmt::Write as _;
    buf.clear();
    if mw >= 1000 {
        let w = mw as f32 / 1000.0;
        let _ = write!(buf, "{:.1}W", w);
    } else {
        let _ = write!(buf, "{}mW", mw);
    }
}

fn draw_centered_text<D: embedded_graphics::draw_target::DrawTarget<Color = Rgb565>>(
    disp: &mut D,
    col_cx: i32,
    y: i32,
    text: &str,
    style: MonoTextStyle<'_, Rgb565>,
    adv_x: i32,
) {
    let w = (text.len() as i32) * adv_x;
    let x = col_cx - (w / 2);
    let _ = Text::with_baseline(text, Point::new(x, y), style, Baseline::Top).draw(disp);
}

fn draw_centered_text_with_outline<
    D: embedded_graphics::draw_target::DrawTarget<Color = Rgb565>,
>(
    disp: &mut D,
    col_cx: i32,
    y: i32,
    text: &str,
    style: MonoTextStyle<'_, Rgb565>,
    adv_x: i32,
) {
    let outline = MonoTextStyle::new(style.font, UI_BORDER);
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        draw_centered_text(disp, col_cx + dx, y + dy, text, outline, adv_x);
    }
    draw_centered_text(disp, col_cx, y, text, style, adv_x);
}

fn draw_dashboard_frame<D: embedded_graphics::draw_target::DrawTarget<Color = Rgb565>>(
    disp: &mut D,
    samples: &[PortSample; 4],
) {
    // Background
    let _ = Rectangle::new(Point::new(0, 0), Size::new(160, 50))
        .into_styled(PrimitiveStyle::with_fill(UI_BG_GRAY))
        .draw(disp);

    // Outer border
    let _ = Rectangle::new(Point::new(0, 0), Size::new(160, 50))
        .into_styled(PrimitiveStyle::with_stroke(UI_BORDER, 1))
        .draw(disp);

    // Column separators
    for x in [40i32, 80, 120] {
        let _ = Rectangle::new(Point::new(x, 0), Size::new(1, 50))
            .into_styled(PrimitiveStyle::with_fill(UI_BORDER))
            .draw(disp);
    }

    // Column centers
    let centers = [20i32, 60, 100, 140];

    // Rows with larger bold font (no header): y = 2, 16, 30 (tight spacing)
    let rows_y = [2i32, 16, 30];
    // Helpers to draw 1-bit masks
    let draw_mask_32 = |disp: &mut D, left: i32, top: i32, mask: &str, color: Rgb565| {
        for (yy, line) in mask.lines().enumerate() {
            let bytes = line.as_bytes();
            for (xx, &b) in bytes.iter().enumerate() {
                if b == b'1' {
                    let _ = Rectangle::new(
                        Point::new(left + xx as i32, top + yy as i32),
                        Size::new(1, 1),
                    )
                    .into_styled(PrimitiveStyle::with_fill(color))
                    .draw(disp);
                }
            }
        }
    };
    let draw_mask_scaled =
        |disp: &mut D, left: i32, top: i32, mask: &str, dw: usize, dh: usize, color: Rgb565| {
            let lines: heapless::Vec<&str, 40> = mask.lines().collect();
            for oy in 0..dh {
                let sy = (oy * 32) / dh;
                let line = *lines.get(sy).unwrap_or(&"");
                let bytes = line.as_bytes();
                for ox in 0..dw {
                    let sx = (ox * 32) / dw;
                    let b = *bytes.get(sx).unwrap_or(&b'0');
                    if b == b'1' {
                        let _ = Rectangle::new(
                            Point::new(left + ox as i32, top + oy as i32),
                            Size::new(1, 1),
                        )
                        .into_styled(PrimitiveStyle::with_fill(color))
                        .draw(disp);
                    }
                }
            }
        };

    for (col, cx) in centers.iter().enumerate() {
        let s = samples[col];
        match s.ui_state {
            UiPortState::Disconnected => {
                // 32x32 DISC icon + label; no values
                draw_mask_32(disp, *cx - 16, 2, ICON_DISC_32, UI_BORDER);
                draw_centered_text(
                    disp,
                    *cx,
                    36,
                    "DISC",
                    MonoTextStyle::new(&FONT_7X13_BOLD, UI_BORDER),
                    7,
                );
            }
            UiPortState::Deferred => {
                let dash = MonoTextStyle::new(&FONT_7X13_BOLD, UI_BORDER);
                draw_centered_text(disp, *cx, rows_y[0], "--", dash, 7);
                draw_centered_text(disp, *cx, rows_y[1], "PEND", dash, 7);
                draw_centered_text(disp, *cx, rows_y[2], "--", dash, 7);
            }
            UiPortState::PowerBlocked => {
                let dash = MonoTextStyle::new(&FONT_7X13_BOLD, UI_BORDER);
                draw_centered_text(disp, *cx, rows_y[0], "--", dash, 7);
                draw_centered_text(disp, *cx, rows_y[1], "NOVIN", dash, 7);
                draw_centered_text(disp, *cx, rows_y[2], "--", dash, 7);
            }
            UiPortState::Closed => {
                // 32x32 OFF icon + label
                draw_mask_32(disp, *cx - 16, 2, ICON_OFF_32, UI_BORDER);
                draw_centered_text(
                    disp,
                    *cx,
                    36,
                    "OFF",
                    MonoTextStyle::new(&FONT_7X13_BOLD, UI_BORDER),
                    7,
                );
            }
            UiPortState::Overcurrent => {
                // Show V/I only; CC icon (24x24) centered on power row; hide W
                let mut buf: heapless::String<8> = heapless::String::new();
                let v_style = MonoTextStyle::new(&FONT_7X13_BOLD, UI_V_YELLOW);
                let i_style = MonoTextStyle::new(&FONT_7X13_BOLD, UI_I_RED);
                fmt_v(&mut buf, s.vbus_mv);
                draw_centered_text_with_outline(disp, *cx, rows_y[0], &buf, v_style, 7);
                fmt_i(&mut buf, s.ich_ma);
                draw_centered_text_with_outline(disp, *cx, rows_y[1], &buf, i_style, 7);
                // CC icon 24x24 at top=26 (26..49)
                draw_mask_scaled(disp, *cx - 12, 26, ICON_CC_32, 24, 24, UI_BORDER);
            }

            UiPortState::Initializing => {
                // Three lines of "--"
                let dash = MonoTextStyle::new(&FONT_7X13_BOLD, UI_BORDER);
                draw_centered_text(disp, *cx, rows_y[0], "--", dash, 7);
                draw_centered_text(disp, *cx, rows_y[1], "--", dash, 7);
                draw_centered_text(disp, *cx, rows_y[2], "--", dash, 7);
            }
            UiPortState::Normal => {
                let mut buf: heapless::String<8> = heapless::String::new();
                let v_style = MonoTextStyle::new(&FONT_7X13_BOLD, UI_V_YELLOW);
                let i_style = MonoTextStyle::new(&FONT_7X13_BOLD, UI_I_RED);
                let w_style = MonoTextStyle::new(&FONT_7X13_BOLD, UI_W_GREEN);
                fmt_v(&mut buf, s.vbus_mv);
                draw_centered_text_with_outline(disp, *cx, rows_y[0], &buf, v_style, 7);
                fmt_i(&mut buf, s.ich_ma);
                draw_centered_text_with_outline(disp, *cx, rows_y[1], &buf, i_style, 7);
                fmt_w(&mut buf, s.power_mw());
                draw_centered_text_with_outline(disp, *cx, rows_y[2], &buf, w_style, 7);
            }
        }
    }

    // Power bars: y=45..48 (4 px tall) for clearer visibility
    let bar_y = 45i32;
    let bar_h = 4u32;
    let bar_w = 34u32;
    let bar_xs = [3i32, 43, 83, 123];
    const MAX_WATT: f32 = 30.0; // fallback max when negotiation not available
    for (i, bx) in bar_xs.iter().enumerate() {
        // Outline (hidden for DISC/CLOSED/CC)
        match samples[i].ui_state {
            UiPortState::Deferred
            | UiPortState::PowerBlocked
            | UiPortState::Disconnected
            | UiPortState::Closed
            | UiPortState::Overcurrent => {}
            _ => {
                let _ = Rectangle::new(Point::new(*bx, bar_y), Size::new(bar_w, bar_h))
                    .into_styled(PrimitiveStyle::with_stroke(UI_BORDER, 1))
                    .draw(disp);
            }
        }
        // Fill only in Normal
        if matches!(samples[i].ui_state, UiPortState::Normal) {
            let mw = samples[i].power_mw();
            let w = (mw as f32) / 1000.0;
            let frac = (w / MAX_WATT).clamp(0.0, 1.0);
            let fwf = frac * (bar_w as f32 - 2.0);
            let fill_w = if fwf <= 0.0 { 0 } else { (fwf + 0.5) as u32 }; // round to nearest
            if fill_w > 0 {
                let _ = Rectangle::new(
                    Point::new(*bx + 1, bar_y + 1),
                    Size::new(fill_w, bar_h.saturating_sub(2)),
                )
                .into_styled(PrimitiveStyle::with_fill(UI_W_GREEN))
                .draw(disp);
            }
        }
    }
}

fn draw_dashboard_mock<D: embedded_graphics::draw_target::DrawTarget<Color = Rgb565>>(
    disp: &mut D,
) {
    let samples = [
        PortSample {
            connected: false,
            vbus_mv: 0,
            ich_ma: 0,
            ui_state: UiPortState::Disconnected,
        },
        PortSample {
            connected: true,
            vbus_mv: 9000,
            ich_ma: 2500,
            ui_state: UiPortState::Overcurrent,
        },
        PortSample {
            connected: false,
            vbus_mv: 0,
            ich_ma: 0,
            ui_state: UiPortState::Closed,
        },
        PortSample {
            connected: false,
            vbus_mv: 0,
            ich_ma: 0,
            ui_state: UiPortState::Initializing,
        },
    ];
    draw_dashboard_frame(disp, &samples);
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
    // Firmware actively asserts the shared RESET# net low first, then releases it as open-drain.
    let mut i2c_reset = Output::new(p.GPIO35, Level::Low, OutputConfig::default());
    Timer::after(Duration::from_millis(5)).await;
    i2c_reset.set_high();
    i2c_reset.apply_config(&OutputConfig::default().with_drive_mode(DriveMode::OpenDrain));
    Timer::after(Duration::from_millis(5)).await;

    // IN_EN default off
    let in_en = Output::new(p.GPIO41, Level::Low, esp_hal::gpio::OutputConfig::default());
    // PG input
    let in_pg = Input::new(
        p.GPIO42,
        esp_hal::gpio::InputConfig::default().with_pull(Pull::Up),
    );

    // Front-panel INT pin will be initialized only if panel is present.

    // EN1~EN4 default disabled (drive low)
    let mut en1 = Output::new(p.GPIO17, Level::Low, esp_hal::gpio::OutputConfig::default());
    let mut en2 = Output::new(p.GPIO18, Level::Low, esp_hal::gpio::OutputConfig::default());
    let mut en3 = Output::new(p.GPIO39, Level::Low, esp_hal::gpio::OutputConfig::default());
    let mut en4 = Output::new(p.GPIO40, Level::Low, esp_hal::gpio::OutputConfig::default());
    // Keep variables used
    en1.set_low();
    en2.set_low();
    en3.set_low();
    en4.set_low();

    // CH442E routing defaults are held by external pulldowns; leave GPIO33/34 high-z
    // until a higher-level routing policy takes ownership.

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
        // Initial frame using mock states, then UI will refresh below
        draw_dashboard_mock(&mut disp);
        let _ = disp.flush().await;
        info!("lcd.draw: dashboard first frame ready");
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
        // 延后到 VIN_ON 再开始以避免上电早期 I2C read_err 噪声
        let _ = power_in::vin_on_signal().wait().await;
        info!("sw2303.ch0: vin_on=true; waiting ch0_ready");
        // 再等待通道0完成一次 SC8815+SW2303 初始化，避免在配置之前频繁探测
        let _ = CH_READY0.wait().await;
        info!("sw2303.ch0: ch0_ready=true; start telemetry");
        let sw_addr = swc::DEFAULT_ADDRESS;
        let mut diag_left: u8 = 10;
        let mut prev_online: bool = false;
        loop {
            // 若 SC8815 的 VBUS 尚未就绪，则暂缓 SW2303 轮询，避免无谓的 I2C 错误
            {
                use sc8815::registers::constants as sc_c;
                use sc8815::registers::Register as ScReg;
                let mut i2c_sc = mux_channel(0);
                let mut sc = sc8815::SC8815::new(&mut i2c_sc, sc_c::DEFAULT_ADDRESS);
                if let Ok((vh, vl)) = sc.read_consecutive_registers(ScReg::VbusFbValue).await {
                    if let Some(v) =
                        sc8815::driver::AdcCalculations::calculate_voltage_mv(vh, vl, 0)
                    {
                        if v < SC8815_VBUS_READY_MV {
                            if diag_left > 0 {
                                warn!("sw2303.ch0: wait vbus={}mV<{}mV", v, SC8815_VBUS_READY_MV);
                                diag_left = diag_left.saturating_sub(1);
                            }
                            Timer::after(Duration::from_millis(SC8815_VBUS_READY_INTERVAL_MS))
                                .await;
                            continue;
                        }
                    }
                }
            }
            let mut i2c_ch0 = mux_channel(0);
            let mut sw = sw2303::SW2303::new(&mut i2c_ch0, sw_addr);

            // Read 8-bit ADC VBUS (datasheet: 7.5*16 mV/bit)
            let vbus_mv = async {
                match sw.read_register(SwReg::AdcVbus).await {
                    Ok(v8) => {
                        let mv_per_lsb = swc::adc::VBUS_FACTOR_MV * 16.0; // 7.5*16 mV
                        if diag_left > 0 {
                            info!("sw2303.ch0: adc.vbus8 raw=0x{:02X}", v8);
                        }
                        Some((v8 as f32) * mv_per_lsb)
                    }
                    Err(_) => {
                        if diag_left > 0 {
                            warn!("sw2303.ch0: adc.vbus8 read_err");
                            diag_left = diag_left.saturating_sub(1);
                        }
                        None
                    }
                }
            }
            .await;

            // Read SW2303 ICH from 8-bit register (0x33), 50 mA/LSB
            let ich_ma: Option<f32> = match sw.read_register(SwReg::AdcIch).await {
                Ok(v8) => {
                    if diag_left > 0 {
                        info!("sw2303.ch0: adc.ich8 raw=0x{:02X}", v8);
                    }
                    Some(v8 as f32 * swc::adc::ICH_FACTOR_MA)
                }
                Err(_) => {
                    if diag_left > 0 {
                        warn!("sw2303.ch0: adc.ich8 read_err");
                        diag_left = diag_left.saturating_sub(1);
                    }
                    None
                }
            };

            let online = match sw.is_sink_device_connected().await {
                Ok(b) => b,
                Err(_) => {
                    if diag_left > 0 {
                        warn!("sw2303.ch0: status3 read_err");
                        diag_left = diag_left.saturating_sub(1);
                    }
                    false
                }
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

                    // Derive a simple source-active flag from VBUS level (use 4.5V threshold)
                    let src_active = v_mv >= 4500;
                    match sc_ibus {
                        Some(ibus_sc) => {
                            let delta = ibus_sc as i32 - i_ma as i32;
                            info!(
                                "sw2303.ch0: sink_online={} src_active={} vbus={}mV ich_sw={}mA ibus_sc={}mA delta={}mA",
                                if online { "true" } else { "false" },
                                if src_active { "true" } else { "false" },
                                v_mv,
                                i_ma,
                                ibus_sc,
                                delta
                            );
                        }
                        None => {
                            info!(
                                "sw2303.ch0: sink_online={} src_active={} vbus={}mV ich_sw={}mA ibus_sc=na",
                                if online { "true" } else { "false" },
                                if src_active { "true" } else { "false" },
                                v_mv,
                                i_ma
                            );
                        }
                    }

                    if diag_left > 0 {
                        // Dump key SW2303 status/ctrl registers for validation
                        if let Ok(s0) = sw.get_system_status0().await {
                            info!("sw2303.ch0: sys0=0b{:08b}", s0.bits());
                        }
                        if let Ok(s3) = sw.get_system_status3().await {
                            info!("sw2303.ch0: sys3=0b{:08b}", s3.bits());
                        }
                        if let Ok(fc) = sw.get_fast_charging_status().await {
                            info!("sw2303.ch0: fastchg=0b{:08b}", fc.bits());
                        }
                        if let Ok(cc) = sw.read_register(SwReg::ConnectionControl).await {
                            info!("sw2303.ch0: reg14(conn)=0x{:02X}", cc);
                        }
                        diag_left = diag_left.saturating_sub(1);
                    }
                }
                _ => {
                    // 使用驱动包装的读以避免借用冲突
                    match sw.read_register(SwReg::SystemStatus3).await {
                        Ok(v) => info!("sw2303.ch0: probe 0x0D ok raw=0x{:02X}", v),
                        Err(_) => {
                            warn!(
                                "sw2303.ch0: read_failed online={} probe_err",
                                if online { "true" } else { "false" }
                            );
                            diag_left = diag_left.saturating_sub(1);
                        }
                    }
                }
            }

            // 一次性上线摘要：检测到 online 从 false->true 时打印全面状态
            if online && !prev_online {
                if let Ok(s0) = sw.get_system_status0().await {
                    info!("sw2303.ch0: online.sys0=0b{:08b}", s0.bits());
                }
                if let Ok(s3) = sw.get_system_status3().await {
                    info!("sw2303.ch0: online.sys3=0b{:08b}", s3.bits());
                }
                if let Ok(fc) = sw.get_fast_charging_status().await {
                    info!("sw2303.ch0: online.fastchg=0b{:08b}", fc.bits());
                }
                if let Ok(cc) = sw.read_register(SwReg::ConnectionControl).await {
                    info!("sw2303.ch0: online.conn=0x{:02X}", cc);
                }
                // 兼容当前 sw2303 crate 版本：使用 8-bit ADC 并换算
                if let Ok(v8) = sw.read_register(SwReg::AdcVbus).await {
                    let v_mv = (v8 as f32 * swc::adc::VBUS_FACTOR_MV * 16.0) as u32;
                    info!("sw2303.ch0: online.vbus8={}mV (raw=0x{:02X})", v_mv, v8);
                }
                if let Ok(i8) = sw.read_register(SwReg::AdcIch).await {
                    let i_ma = (i8 as f32 * swc::adc::ICH_FACTOR_MA) as u32;
                    info!("sw2303.ch0: online.ich8={}mA (raw=0x{:02X})", i_ma, i8);
                }
            }
            prev_online = online;

            Timer::after(Duration::from_secs(1)).await;
        }
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

    if !legacy_sc8815_path_enabled() {
        let mut vin_on = power_in::vin_on_state();
        if vin_on {
            info!("i2c.scan:start vin_on=true backend=ip6557");
        } else {
            warn!("pwr.in: vin_on=false; keep ip6557 modules disabled");
        }
        let mut deferred = [false; 4];
        let mut disc = [false; 4];
        let mut probe_failures = [0u8; 4];
        let mut deferred_refresh_countdown = 0u8;

        loop {
            let vin_on_now = power_in::vin_on_state();
            if vin_on_now && !vin_on {
                info!("i2c.scan:start vin_on=true backend=ip6557");
                deferred_refresh_countdown = 0;
            } else if !vin_on_now && vin_on {
                warn!("pwr.in: vin_on=false; keep ip6557 modules disabled");
                deferred = [false; 4];
                disc = [false; 4];
                probe_failures = [0u8; 4];
                deferred_refresh_countdown = 0;
            }
            vin_on = vin_on_now;

            if vin_on {
                let refresh_deferred = deferred_refresh_countdown == 0;
                for ch in 0u8..4u8 {
                    let idx = ch as usize;
                    if deferred[idx] && !refresh_deferred {
                        continue;
                    }

                    let mux_mask = 1u8 << (ch & 0x03);
                    let mut select_ok = false;
                    let mut ctrl = [0u8; 1];
                    for _ in 0..3 {
                        let mut upstream = I2cDevice::new(bus);
                        if embedded_hal_async::i2c::I2c::write_read(
                            &mut upstream,
                            pca9545::DEFAULT_ADDRESS,
                            &[mux_mask],
                            &mut ctrl,
                        )
                        .await
                        .is_ok()
                            && (ctrl[0] & mux_mask) != 0
                        {
                            select_ok = true;
                            break;
                        }
                        Timer::after(Duration::from_millis(20)).await;
                    }

                    if select_ok {
                        probe_failures[idx] = 0;
                        disc[idx] = false;
                        deferred[idx] = true;
                        info!("i2c.mux: ch={} select=ok ctrl=0x{:02X}", ch, ctrl[0]);
                        info!(
                            "pwr.mod: ch={} backend=ip6557 init=deferred reason=\"bringup-pending\"",
                            ch
                        );
                    } else {
                        probe_failures[idx] = probe_failures[idx].saturating_add(1);
                        if probe_failures[idx] >= 3 {
                            if !disc[idx] {
                                warn!("i2c.mux: ch={} select=err reg=0x{:02X}", ch, mux_mask);
                            }
                            disc[idx] = true;
                        }
                    }
                }
                deferred_refresh_countdown = if refresh_deferred {
                    10
                } else {
                    deferred_refresh_countdown.saturating_sub(1)
                };
            }

            let mut view: [PortSample; 4] = [
                PortSample {
                    connected: false,
                    vbus_mv: 0,
                    ich_ma: 0,
                    ui_state: UiPortState::Initializing,
                },
                PortSample {
                    connected: false,
                    vbus_mv: 0,
                    ich_ma: 0,
                    ui_state: UiPortState::Initializing,
                },
                PortSample {
                    connected: false,
                    vbus_mv: 0,
                    ich_ma: 0,
                    ui_state: UiPortState::Initializing,
                },
                PortSample {
                    connected: false,
                    vbus_mv: 0,
                    ich_ma: 0,
                    ui_state: UiPortState::Initializing,
                },
            ];
            let target = if PWR_SW_TARGET.load(Ordering::Relaxed) == (PowerSwitchTarget::Open as u8)
            {
                PowerSwitchTarget::Open
            } else {
                PowerSwitchTarget::Closed
            };
            for ch in 0u8..4u8 {
                let idx = ch as usize;
                if target == PowerSwitchTarget::Open {
                    view[idx].ui_state = UiPortState::Closed;
                } else if !vin_on {
                    view[idx].ui_state = UiPortState::PowerBlocked;
                } else if deferred[idx] {
                    view[idx].ui_state = UiPortState::Deferred;
                } else if disc[idx] {
                    view[idx].ui_state = UiPortState::Disconnected;
                } else {
                    view[idx].ui_state = UiPortState::Initializing;
                }
            }
            draw_dashboard_frame(&mut disp, &view);
            let _ = disp.flush().await;
            Timer::after(Duration::from_millis(500)).await;
        }
    }

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
        let _sc_vbus_last = 0u16;
        // SC8815 driver owns I2C; temporarily move and release back after use
        let mut sc_drv = sc8815::SC8815::new(i2c_scan, sc_addr);
        if sc_ack {
            match sc_drv.read_register(ScReg::Status).await {
                Ok(v) => {
                    sc_present = true;
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
            // Initialize SC8815 per design (keep EN low until init succeeds)
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
                // Do not print init-time readbacks; proceed to one-shot configuration
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
                }

                // OTG mode, switching frequency, dead time, etc. are applied via configure_device()

                // Skip ratio readback during init

                // Configure SC8815 first while the downstream board gate stays disabled.
                if sc_startup_ok {
                    if let Err(e) = sc_drv.set_vbus_external_reference(615).await {
                        warn!("pwr.sc8815: ch={} vbus_set_err={}", ch, sc_err_tag(&e));
                        sc_startup_ok = false;
                    }
                    if let Err(e) = sc_drv.set_adc_conversion(true).await {
                        warn!("pwr.sc8815: ch={} adc_start_err={}", ch, sc_err_tag(&e));
                        sc_startup_ok = false;
                    }
                    if let Err(e) = sc_drv.set_otg_mode(true).await {
                        warn!("pwr.sc8815: ch={} otg_err={}", ch, sc_err_tag(&e));
                        sc_startup_ok = false;
                    }
                }

                if sc_startup_ok {
                    match ch {
                        0 => en1.set_high(),
                        1 => en2.set_high(),
                        2 => en3.set_high(),
                        3 => en4.set_high(),
                        _ => {}
                    }
                    Timer::after(Duration::from_millis(5)).await;
                }
                if sc_startup_ok {
                    {
                        use sc8815::registers::Register as ScReg;
                        // Post-EN, read-only verification of key registers and ADC raw
                        let ctrl1 = sc_drv.read_register(ScReg::Ctrl1Set).await.ok();
                        let ctrl0 = sc_drv.read_register(ScReg::Ctrl0Set).await.ok();
                        let ctrl3 = sc_drv.read_register(ScReg::Ctrl3Set).await.ok();
                        let status = sc_drv.read_register(ScReg::Status).await.ok();
                        let ratio = sc_drv.read_register(ScReg::Ratio).await.ok();
                        let vi_hi = sc_drv.read_register(ScReg::VbusrefISet).await.ok();
                        let vi_lo = sc_drv.read_register(ScReg::VbusrefISet2).await.ok();
                        let ve_hi = sc_drv.read_register(ScReg::VbusrefESet).await.ok();
                        let ve_lo = sc_drv.read_register(ScReg::VbusrefESet2).await.ok();
                        let (vbus_h, vbus_l) = sc_drv
                            .read_consecutive_registers(ScReg::VbusFbValue)
                            .await
                            .unwrap_or((0, 0));

                        let vref_i_mv = vi_hi.zip(vi_lo).map(|(hi, lo)| {
                            let lsb: u16 = ((hi as u16) << 2) | ((lo as u16) >> 6);
                            (lsb + 1) * 2
                        });
                        let vref_e_mv = ve_hi.zip(ve_lo).map(|(hi, lo)| {
                            let lsb: u16 = ((hi as u16) << 2) | ((lo as u16) >> 6);
                            (lsb + 1) * 2
                        });

                        let vbus_12 = sc8815::driver::AdcCalculations::calculate_voltage_mv(
                            vbus_h, vbus_l, 0,
                        )
                        .unwrap_or(0);
                        let vbus_5 = sc8815::driver::AdcCalculations::calculate_voltage_mv(
                            vbus_h, vbus_l, 1,
                        )
                        .unwrap_or(0);

                        let fb_sel_is_ext = ctrl1.map(|c| (c & 0x10) != 0).unwrap_or(false);
                        let vbus_ratio_is_5x = ratio.map(|r| (r & 0x01) != 0).unwrap_or(false);
                        let vbat_ratio_is_5x = ratio.map(|r| (r & 0x02) != 0).unwrap_or(false);
                        let en_otg = ctrl0.map(|c| (c & 0x80) != 0).unwrap_or(false);
                        let ad_start = ctrl3.map(|c| (c & 0x20) != 0).unwrap_or(false);
                        let dis_short_fb = ctrl3.map(|c| (c & 0x04) != 0).unwrap_or(false);
                        let vbus_short = status.map(|s| (s & 0x08) != 0).unwrap_or(false);

                        // For debugging: also print raw register bytes for VBUSREF_I/EB (hi/lo)
                        let vi_hi_b: u8 = vi_hi.unwrap_or(0);
                        let vi_lo_b: u8 = vi_lo.unwrap_or(0);
                        let ve_hi_b: u8 = ve_hi.unwrap_or(0);
                        let ve_lo_b: u8 = ve_lo.unwrap_or(0);

                        info!(
                            "pwr.sc8815: ch={} verify fb_sel={} ratio={} vbat_ratio={} en_otg={} ad_start={} dis_sfb={} stat.vbus_short={} vref_i={}mV vref_e={}mV reg.vref_i=0x{:02X}/0x{:02X} reg.vref_e=0x{:02X}/0x{:02X} adc.vbus(12.5x/5x)={}/{}mV",
                            ch,
                            if fb_sel_is_ext { "external" } else { "internal" },
                            if vbus_ratio_is_5x { "5x" } else { "12.5x" },
                            if vbat_ratio_is_5x { "5x" } else { "12.5x" },
                            if en_otg { "on" } else { "off" },
                            if ad_start { "on" } else { "off" },
                            if dis_short_fb { "on" } else { "off" },
                            if vbus_short { "yes" } else { "no" },
                            vref_i_mv.unwrap_or(0),
                            vref_e_mv.unwrap_or(0),
                            vi_hi_b,
                            vi_lo_b,
                            ve_hi_b,
                            ve_lo_b,
                            vbus_12,
                            vbus_5
                        );
                    }
                    // No init-time sampling; treat startup OK when prior steps succeeded
                    sc_ok = sc_startup_ok;
                } else {
                    warn!("pwr.sc8815: ch={} startup_aborted", ch);
                }
            }
        }
        // release I2C back from driver
        i2c_scan = sc_drv.release();

        // Spawn periodic VBUS monitor and optional auto-calibration for ch0 only
        if ch == 0 && sc_ok {
            #[embassy_executor::task]
            async fn sc8815_ch0_monitor_task() {
                let mut i2c = mux_channel(0);
                let mut prev_vbus_short: Option<bool> = None;
                loop {
                    let mut sc = sc8815::SC8815::new(&mut i2c, sc8815_const::DEFAULT_ADDRESS);
                    let meas = sc.get_adc_measurements().await.ok();
                    // Also poll status for fault visibility
                    let vbus_short = sc.is_vbus_short_fault().await.ok();
                    // SW2303 quick read on the same bus/cadence
                    let (sw_vbus_mv, sw_ich_ma, sw_sink) = {
                        let mut sw = sw2303::SW2303::new(&mut i2c, swc::DEFAULT_ADDRESS);
                        let v8 = sw.read_register(SwReg::AdcVbus).await.ok();
                        let i8 = sw.read_register(SwReg::AdcIch).await.ok();
                        let sink = sw.is_sink_device_connected().await.ok();
                        let v_mv = v8.map(|v| (v as f32 * swc::adc::VBUS_FACTOR_MV) as u32);
                        let i_ma = i8.map(|i| (i as f32 * swc::adc::ICH_FACTOR_MA) as u32);
                        (v_mv, i_ma, sink)
                    };
                    if let Some(m) = meas {
                        match (sw_vbus_mv, sw_ich_ma) {
                            (Some(sv), Some(si)) => info!(
                                "pwr.sc8815: ch=0 stat vbus={}mV ibus={}mA vbat={}mV ibat={}mA sw.vbus={}mV sw.ich={}mA sink={}",
                                m.vbus_mv,
                                m.ibus_ma,
                                m.vbat_mv,
                                m.ibat_ma,
                                sv,
                                si,
                                if sw_sink.unwrap_or(false) { "true" } else { "false" }
                            ),
                            _ => info!(
                                "pwr.sc8815: ch=0 stat vbus={}mV ibus={}mA vbat={}mV ibat={}mA sw=na sink={}",
                                m.vbus_mv,
                                m.ibus_ma,
                                m.vbat_mv,
                                m.ibat_ma,
                                if sw_sink.unwrap_or(false) { "true" } else { "false" }
                            ),
                        }
                        // no auto-calibration in application layer
                    }
                    if let Some(short) = vbus_short {
                        match prev_vbus_short {
                            None => {
                                if short {
                                    warn!("pwr.sc8815: ch=0 fault vbus_short=1 (sticky)");
                                }
                            }
                            Some(prev) if prev != short => {
                                if short {
                                    warn!("pwr.sc8815: ch=0 fault vbus_short=1 (sticky)");
                                } else {
                                    info!("pwr.sc8815: ch=0 fault cleared vbus_short=0");
                                }
                            }
                            _ => {}
                        }
                        prev_vbus_short = Some(short);
                    }
                    Timer::after(Duration::from_millis(SC8815_MONITOR_INTERVAL_MS)).await;
                }
            }
            spawner
                .spawn(sc8815_ch0_monitor_task())
                .expect("spawn sc8815_ch0_monitor_task");
        }

        // Wait for SC8815 VBUS ready, then probe/init SW2303
        let mut sw_ok = false;
        if sc_ok {
            // Gate on VBUS >= SC8815_VBUS_READY_MV for SC8815_VBUS_READY_CONSECUTIVE samples
            let mut consec_ok: u8 = 0;
            let mut attempts: u16 = 0;
            // upper bound ~ (25 * 50ms) = 1250ms
            while consec_ok < SC8815_VBUS_READY_CONSECUTIVE && attempts < 25 {
                attempts += 1;
                // Reborrow I2C for SC8815 one-shot read
                {
                    use sc8815::registers::Register as ScReg;
                    let mut sc_chk = sc8815::SC8815::new(i2c_scan, sc_addr);
                    let (vbus_h, vbus_l) = sc_chk
                        .read_consecutive_registers(ScReg::VbusFbValue)
                        .await
                        .unwrap_or((0, 0));
                    let vbus_mv =
                        sc8815::driver::AdcCalculations::calculate_voltage_mv(vbus_h, vbus_l, 0)
                            .unwrap_or(0);
                    i2c_scan = sc_chk.release();
                    if vbus_mv >= SC8815_VBUS_READY_MV {
                        consec_ok = consec_ok.saturating_add(1);
                    } else {
                        consec_ok = 0;
                    }
                    if consec_ok < SC8815_VBUS_READY_CONSECUTIVE {
                        Timer::after(Duration::from_millis(SC8815_VBUS_READY_INTERVAL_MS)).await;
                    } else {
                        info!("pwr.sc8815: ch={} vbus_ready=true vbus={}mV", ch, vbus_mv);
                    }
                }
            }
            if consec_ok < SC8815_VBUS_READY_CONSECUTIVE {
                warn!(
                    "pwr.sc8815: ch={} vbus_ready=timeout th={}mV",
                    ch, SC8815_VBUS_READY_MV
                );
            }
            // Detect SW2303 by reading SystemStatus3 (0x0D).
            // 给予更长的就绪宽限：每 150ms 重试，最多 12 次（~1.8s）。
            let mut tried: u8 = 0;
            let mut detected = false;
            let mut detect_reg: u8 = 0x00;
            loop {
                let mut sw_detect = sw2303::SW2303::new(&mut i2c_scan, sw_addr);
                // 优先 SystemStatus3(0x0D)
                if sw_detect.get_system_status3().await.is_ok() {
                    if sw_detect.init().await.is_ok() {
                        detected = true;
                        detect_reg = 0x0D;
                    }
                } else if sw_detect.read_register(SwReg::AdcVbus).await.is_ok() {
                    // 次选 AdcVbus(0x31)
                    if sw_detect.init().await.is_ok() {
                        detected = true;
                        detect_reg = 0x31;
                    }
                } else if sw_detect
                    .read_register(SwReg::ConnectionControl)
                    .await
                    .is_ok()
                {
                    // 兜底 ConnectionControl(0x14)
                    if sw_detect.init().await.is_ok() {
                        detected = true;
                        detect_reg = 0x14;
                    }
                }
                if detected || tried >= SW2303_DETECT_RETRIES {
                    break;
                }
                tried = tried.saturating_add(1);
                if tried == 1 {
                    info!("pwr.sc8815: ch={} wait sw2303_ready ...", ch);
                }
                Timer::after(Duration::from_millis(SW2303_DETECT_BACKOFF_MS)).await;
            }
            sw_ok = detected;
            if sw_ok {
                info!(
                    "pwr.sc8815: ch={} sw2303_ready via reg=0x{:02X}",
                    ch, detect_reg
                );
            } else {
                warn!(
                    "pwr.sc8815: ch={} sw2303_ready=timeout total={}ms",
                    ch,
                    (SW2303_DETECT_RETRIES as u64) * SW2303_DETECT_BACKOFF_MS
                );
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
            match ch {
                0 => {
                    CH_READY0.signal(true);
                    CH_RDY[0].store(true, Ordering::Relaxed);
                }
                1 => {
                    CH_READY1.signal(true);
                    CH_RDY[1].store(true, Ordering::Relaxed);
                }
                2 => {
                    CH_READY2.signal(true);
                    CH_RDY[2].store(true, Ordering::Relaxed);
                }
                3 => {
                    CH_READY3.signal(true);
                    CH_RDY[3].store(true, Ordering::Relaxed);
                }
                _ => {}
            }
            CH_SCAN_DONE[ch as usize].store(true, Ordering::Relaxed);
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
                // Address is fixed per hardware; no address scan.
            } else if sc_ok ^ sw_ok {
                error!("i2c.scan: ch={} anomaly=true reason=\"pair-mismatch\"", ch);
            }
            // Mark scan done to allow UI to switch from INIT to DISC
            CH_SCAN_DONE[ch as usize].store(true, Ordering::Relaxed);
        }

        // 按新策略：只要 SW2303 不在线也要关闭该通道功率级
        if !sc_ok || (!sw_ok && !ALLOW_SC8815_WITHOUT_SW2303) {
            match ch {
                0 => en1.set_low(),
                1 => en2.set_low(),
                2 => en3.set_low(),
                3 => en4.set_low(),
                _ => {}
            }
        }
    }

    // 仅在扫描完成后再启动 SW2303 遥测，避免早期干扰
    info!("sw2303.ch0: spawn");
    spawner
        .spawn(sw2303_ch0_telemetry_task())
        .expect("spawn sw2303_ch0_telemetry_task");

    // 其他通道的 SW2303 遥测（同样在各自 ready 后启动）
    #[embassy_executor::task]
    async fn sw2303_ch1_telemetry_task() {
        info!("sw2303.ch1: task_start");
        let _ = power_in::vin_on_signal().wait().await;
        info!("sw2303.ch1: vin_on=true; waiting ch1_ready");
        let _ = CH_READY1.wait().await;
        info!("sw2303.ch1: ch1_ready=true; start telemetry");
        let sw_addr = swc::DEFAULT_ADDRESS;
        let mut diag_left: u8 = 5;
        let mut prev_online = false;
        loop {
            let mut i2c = mux_channel(1);
            let mut sw = sw2303::SW2303::new(&mut i2c, sw_addr);
            let vbus_mv = async {
                match sw.read_register(SwReg::AdcVbus).await {
                    Ok(v8) => {
                        if diag_left > 0 {
                            info!("sw2303.ch1: adc.vbus8 raw=0x{:02X}", v8);
                        }
                        Some((v8 as f32) * swc::adc::VBUS_FACTOR_MV * 16.0)
                    }
                    Err(_) => None,
                }
            }
            .await;
            let ich_ma: Option<f32> = match sw.read_register(SwReg::AdcIch).await {
                Ok(v8) => {
                    if diag_left > 0 {
                        info!("sw2303.ch1: adc.ich8 raw=0x{:02X}", v8);
                    }
                    Some(v8 as f32 * swc::adc::ICH_FACTOR_MA)
                }
                Err(_) => None,
            };
            let online = sw.is_sink_device_connected().await.unwrap_or(false);
            if let (Some(v), Some(i)) = (vbus_mv, ich_ma) {
                info!(
                    "sw2303.ch1: sink_online={} vbus={}mV ich_sw={}mA",
                    if online { "true" } else { "false" },
                    v as u32,
                    i as u32
                );
                if diag_left > 0 {
                    if let Ok(s3) = sw.get_system_status3().await {
                        info!("sw2303.ch1: sys3=0b{:08b}", s3.bits());
                    }
                    if let Ok(s0) = sw.get_system_status0().await {
                        info!("sw2303.ch1: sys0=0b{:08b}", s0.bits());
                    }
                    diag_left = diag_left.saturating_sub(1);
                }
            }
            if online && !prev_online {
                info!("sw2303.ch1: online");
            }
            prev_online = online;
            Timer::after(Duration::from_secs(1)).await;
        }
    }
    spawner
        .spawn(sw2303_ch1_telemetry_task())
        .expect("spawn sw2303_ch1_telemetry_task");

    #[embassy_executor::task]
    async fn sw2303_ch2_telemetry_task() {
        info!("sw2303.ch2: task_start");
        let _ = power_in::vin_on_signal().wait().await;
        info!("sw2303.ch2: vin_on=true; waiting ch2_ready");
        let _ = CH_READY2.wait().await;
        info!("sw2303.ch2: ch2_ready=true; start telemetry");
        let sw_addr = swc::DEFAULT_ADDRESS;
        let mut i2c = mux_channel(2);
        let mut sw = sw2303::SW2303::new(&mut i2c, sw_addr);
        loop {
            let online = sw.is_sink_device_connected().await.unwrap_or(false);
            if let Ok(v8) = sw.read_register(SwReg::AdcVbus).await {
                let v = (v8 as f32 * swc::adc::VBUS_FACTOR_MV * 16.0) as u32;
                info!(
                    "sw2303.ch2: sink_online={} vbus={}mV",
                    if online { "true" } else { "false" },
                    v
                );
            }
            Timer::after(Duration::from_secs(1)).await;
        }
    }
    spawner
        .spawn(sw2303_ch2_telemetry_task())
        .expect("spawn sw2303_ch2_telemetry_task");

    #[embassy_executor::task]
    async fn sw2303_ch3_telemetry_task() {
        info!("sw2303.ch3: task_start");
        let _ = power_in::vin_on_signal().wait().await;
        info!("sw2303.ch3: vin_on=true; waiting ch3_ready");
        let _ = CH_READY3.wait().await;
        info!("sw2303.ch3: ch3_ready=true; start telemetry");
        let sw_addr = swc::DEFAULT_ADDRESS;
        let mut i2c = mux_channel(3);
        let mut sw = sw2303::SW2303::new(&mut i2c, sw_addr);
        loop {
            let online = sw.is_sink_device_connected().await.unwrap_or(false);
            if let Ok(v8) = sw.read_register(SwReg::AdcVbus).await {
                let v = (v8 as f32 * swc::adc::VBUS_FACTOR_MV * 16.0) as u32;
                info!(
                    "sw2303.ch3: sink_online={} vbus={}mV",
                    if online { "true" } else { "false" },
                    v
                );
            }
            Timer::after(Duration::from_secs(1)).await;
        }
    }
    spawner
        .spawn(sw2303_ch3_telemetry_task())
        .expect("spawn sw2303_ch3_telemetry_task");
    // === UI periodic refresh loop (2 Hz) ===
    let sw_addr = swc::DEFAULT_ADDRESS;
    loop {
        // Derive per-port samples
        let mut view: [PortSample; 4] = [
            PortSample {
                connected: false,
                vbus_mv: 0,
                ich_ma: 0,
                ui_state: UiPortState::Initializing,
            },
            PortSample {
                connected: false,
                vbus_mv: 0,
                ich_ma: 0,
                ui_state: UiPortState::Initializing,
            },
            PortSample {
                connected: false,
                vbus_mv: 0,
                ich_ma: 0,
                ui_state: UiPortState::Initializing,
            },
            PortSample {
                connected: false,
                vbus_mv: 0,
                ich_ma: 0,
                ui_state: UiPortState::Initializing,
            },
        ];
        let target = if PWR_SW_TARGET.load(Ordering::Relaxed) == (PowerSwitchTarget::Open as u8) {
            PowerSwitchTarget::Open
        } else {
            PowerSwitchTarget::Closed
        };
        for ch in 0u8..4u8 {
            let idx = ch as usize;
            // OFF takes precedence (global intent in MVP)
            if target == PowerSwitchTarget::Open {
                view[idx].ui_state = UiPortState::Closed;
                continue;
            }
            // Not yet ready -> INIT
            if !CH_RDY[idx].load(Ordering::Relaxed) {
                // If initial scan is done but channel not ready => treat as disconnected
                if CH_SCAN_DONE[idx].load(Ordering::Relaxed) {
                    view[idx].ui_state = UiPortState::Disconnected;
                } else {
                    view[idx].ui_state = UiPortState::Initializing;
                }
                continue;
            }
            // Module online: sample SW2303 (best-effort)
            let mut i2c = mux_channel(ch);
            let mut sw = sw2303::SW2303::new(&mut i2c, sw_addr);
            view[idx].connected = true;
            // Read 8-bit ADCs
            let v_mv = match sw.read_register(SwReg::AdcVbus).await {
                Ok(v8) => (v8 as f32 * swc::adc::VBUS_FACTOR_MV * 16.0) as u32,
                Err(_) => 0,
            };
            let i_ma = match sw.read_register(SwReg::AdcIch).await {
                Ok(i8) => (i8 as f32 * swc::adc::ICH_FACTOR_MA) as u32,
                Err(_) => 0,
            };
            view[idx].vbus_mv = v_mv;
            view[idx].ich_ma = i_ma;
            // Overcurrent heuristic；否则正常显示（包括空载 0mA/0W 情况）
            if i_ma >= (SC8815_IBUS_LIMIT_MA as u32) {
                view[idx].ui_state = UiPortState::Overcurrent;
            } else {
                view[idx].ui_state = UiPortState::Normal;
            }
        }
        // Draw and flush
        draw_dashboard_frame(&mut disp, &view);
        let _ = disp.flush().await;
        Timer::after(Duration::from_millis(500)).await;
    }
}
