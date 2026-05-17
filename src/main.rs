//! ESP32-S3 MVP Firmware
//!
//! Implements the MVP per docs/software_design.md:
//! - Boot init: time, GPIO, I2C, basic presence scans
//! - Direct I2C bus + per-port output-module sensor checks
//! - Front-panel TCA6408A (0x21) presence check
//! - Power input subsystem MVP: INA226-based input qualification and 10s status log

#![no_std]
#![no_main]

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_time::{with_timeout, Duration, Timer};
// Note: use fully-qualified trait calls for embedded-hal to avoid unused-import lints under clippy -D warnings
use esp_backtrace as _;
use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
use esp_hal::gpio::{DriveMode, Flex, Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::spi::Mode as SpiMode;
use esp_hal::system;
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;
// Shared I2C bus infrastructure
mod boot_diag;
mod hub_sideband;
mod power_in;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Receiver;
use embassy_sync::mutex::Mutex;
// use embassy_sync::signal::Signal; // not used on this branch
use static_cell::StaticCell;
mod fan;
mod front_panel;
use boot_diag::{
    fault_label, outcome_label, state_label, BootFaultCode, BootOutcome, BootSelfCheckSnapshot,
    BootStage, GateDecision, SelfCheckItemState, SysCheck,
};

// No global mutex in MVP

// INA226 is handled inside power_in task

use ina226_tp as ina226;
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
static SENSOR_I2C_BUS: StaticCell<SharedI2cBus> = StaticCell::new();
static HUB_I2C_BUS: StaticCell<SharedI2cBus> = StaticCell::new();
static mut SENSOR_I2C_BUS_REF: Option<&'static SharedI2cBus> = None;
// Latch channel readiness for UI without blocking on Signal::wait
static CH_RDY: [AtomicBool; 4] = [
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
];
// Scan completion flag per channel (true once initial scan concludes, even if offline)
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

// I2C shared in init and reused by async tasks / runtime sampling.

// Board-specific pins per docs/esp32-s3fh4r2_gpio_assignment_guide.md
#[allow(dead_code)]
const PIN_I2C_SDA: u8 = 8;
#[allow(dead_code)]
const PIN_I2C_SCL: u8 = 9;
#[allow(dead_code)]
const PIN_HUB_I2C_SDA: u8 = 14;
#[allow(dead_code)]
const PIN_HUB_I2C_SCL: u8 = 13;
#[allow(dead_code)]
const PIN_I2C_INT: u8 = 16;
#[allow(dead_code)]
const PIN_I2C_RESET: u8 = 35; // mainboard active-low reset net; front-panel TCA reset is local pull-up

#[allow(dead_code)]
const PIN_IN_CE: u8 = 41; // high = force TPS2490 EN low/off, low = allow/on
#[allow(dead_code)]
const PIN_IN_PG: u8 = 42; // TPS2490 PG (open drain, high = good)

#[allow(dead_code)]
const PIN_USB_V1OK: u8 = 21; // ISOUSB211 side-1 power OK (high = upstream side powered)

// Output-module enable lines (V3): EN high = module enabled
#[allow(dead_code)]
const PIN_EN1: u8 = 17;
#[allow(dead_code)]
const PIN_EN2: u8 = 18;
#[allow(dead_code)]
const PIN_EN3: u8 = 39;
#[allow(dead_code)]
const PIN_EN4: u8 = 40;

// INA226 shunt value (ohms) from docs: 5 mΩ
const SHUNT_RESISTANCE_OHMS: f32 = 0.005;

// Qualification thresholds (see docs/software_design.md §2)
// 开发阶段临时放宽：将 VIN 下限改为 4.5V 以便在 5V 供电/风扇台架下进行功能验证；
// 量产应恢复到 9.0V 以避免 5–8V 区间误判上电（请在合入前复位该值）。
const VIN_MIN_V: f32 = 4.5;
const VIN_MAX_V: f32 = 24.0;
const I_IDLE_MAX_A: f32 = 0.010; // 10 mA
const INA226_SHUNT_LSB_V: f32 = 2.5e-6;

// Direct-bus V3 IP6557 modules are expected to be strapped to unique addresses.
// Channel 4 is the current validation target and is populated as INA226@0x43 + TMP112@0x4B.
const MODULE_INA226_ADDRS: [u8; 4] = [0x40, 0x41, 0x42, 0x43];
const MODULE_TMP112_ADDRS: [u8; 4] = [0x48, 0x49, 0x4A, 0x4B];
const MODULE_SENSOR_RETRY_MS: u64 = 50;
const MODULE_SENSOR_RETRIES: u8 = 6;

// ===== Display pin mapping (V3 netlist) =====
// SPI2 SCLK/MOSI to panel; MISO unused. Latest mainboard routes GPIO13/14 to hub I2C.
// BLK drives a panel-side P-channel gate and is therefore active-low.
const PIN_LCD_SCLK: u8 = 12;
const PIN_LCD_MOSI: u8 = 11;
const PIN_LCD_DC: u8 = 10;
const PIN_LCD_BLK_GPIO: u8 = 15;

const TCA6408_ADDR: u8 = 0x21;
const INPUT_INA226_ADDR: u8 = 0x44;
const PCA9545_ADDR: u8 = 0x70;
const I2C_RECOVERY_CLOCKS: u8 = 18;
const FRONT_PANEL_BOOT_PROBE_ATTEMPTS: u8 = 3;
const PORT_OCP_LOW_VBUS_MV: u32 = 3000;
const PORT_OCP_LOW_VBUS_MIN_CURRENT_MA: u32 = 100;
const PORT_OCP_HIGH_CURRENT_MA: u32 = 5300;
const PORT_OCP_RELEASE_SAFE_SAMPLES: u8 = 4;
const FRONT_DIAG_FRONT_MASK: u8 = 1 << 0;
const FRONT_DIAG_HUB_MASK: u8 = 1 << 1;
const FRONT_DIAG_INPUT_INA_MASK: u8 = 1 << 2;
const FRONT_DIAG_MUX_MASK: u8 = 1 << 3;

#[derive(Copy, Clone)]
struct I2cRecoveryReport {
    sda_released: bool,
    scl_released: bool,
    sda_initial: bool,
    scl_initial: bool,
    sda_final: bool,
    scl_final: bool,
}

fn module_addr_pair(ch: u8) -> (u8, u8) {
    let idx = (ch & 0x03) as usize;
    (MODULE_INA226_ADDRS[idx], MODULE_TMP112_ADDRS[idx])
}

async fn i2c_ack_probe<I2C: embedded_hal_async::i2c::I2c>(
    i2c: &mut I2C,
    addr: u8,
) -> (bool, &'static str) {
    let mut b = [0u8; 2];
    if embedded_hal_async::i2c::I2c::write_read(i2c, addr, &[0x00], &mut b)
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

async fn sample_module_ina226(ch: u8) -> Option<(u32, u32)> {
    let bus = unsafe { SENSOR_I2C_BUS_REF.expect("sensor I2C bus not initialized") };
    let i2c = I2cDevice::new(bus);
    let mut dev = ina226::INA226::new(None);
    let (ina_addr, _) = module_addr_pair(ch);
    dev.set_ina_address(ina_addr);
    let mut ina = dev.initialize(i2c).await.ok()?;
    let vbus_mv = (ina.read_voltage().await * 1000.0) as u32;
    let raw = ina.read_raw_shunt_voltage().await;
    let signed = i16::from_be_bytes(raw.to_be_bytes());
    let shunt_v = signed as f32 * INA226_SHUNT_LSB_V;
    let current_ma = ((shunt_v / SHUNT_RESISTANCE_OHMS).abs() * 1000.0) as u32;
    Some((vbus_mv, current_ma))
}

// Diagnostic scan is limited to board-relevant address windows.
async fn log_i2c_diag_scan(bus: &'static SharedI2cBus, reason: &'static str) {
    info!("i2c.diag: start reason={}", reason);
    for addr in 0x18u8..=0x27u8 {
        let mut i2c = I2cDevice::new(bus);
        let (ok, method) = i2c_ack_probe(&mut i2c, addr).await;
        if ok {
            info!("i2c.diag: addr=0x{:02X} online via={}", addr, method);
        }
    }
    for addr in 0x40u8..=0x4Bu8 {
        let mut i2c = I2cDevice::new(bus);
        let (ok, method) = i2c_ack_probe(&mut i2c, addr).await;
        if ok {
            info!("i2c.diag: addr=0x{:02X} online via={}", addr, method);
        }
    }
    for addr in 0x70u8..=0x77u8 {
        let mut i2c = I2cDevice::new(bus);
        let (ok, method) = i2c_ack_probe(&mut i2c, addr).await;
        if ok {
            info!("i2c.diag: addr=0x{:02X} online via={}", addr, method);
        }
    }
    info!("i2c.diag: end reason={}", reason);
}

async fn log_front_i2c_peer_matrix(
    sensor_bus: &'static SharedI2cBus,
    hub_bus: &'static SharedI2cBus,
    reason: &'static str,
    attempt: u8,
) -> u8 {
    let mut online_mask = 0u8;
    for (addr, label, mask, bus) in [
        (TCA6408_ADDR, "front_tca", FRONT_DIAG_FRONT_MASK),
        (INPUT_INA226_ADDR, "input_ina226", FRONT_DIAG_INPUT_INA_MASK),
        (PCA9545_ADDR, "pca9545", FRONT_DIAG_MUX_MASK),
    ]
    .map(|(addr, label, mask)| (addr, label, mask, sensor_bus))
    .into_iter()
    .chain(core::iter::once((
        hub_sideband::TCA6408_ADDR,
        "hub_tca",
        FRONT_DIAG_HUB_MASK,
        hub_bus,
    ))) {
        let mut i2c = I2cDevice::new(bus);
        let (ok, method) = i2c_ack_probe(&mut i2c, addr).await;
        if ok {
            online_mask |= mask;
        }
        info!(
            "i2c.front_diag: reason={} attempt={} label={} addr=0x{:02X} online={} via={}",
            reason,
            attempt,
            label,
            addr,
            if ok { "yes" } else { "no" },
            method
        );
    }
    online_mask
}

async fn probe_front_panel_tca(
    bus: &'static SharedI2cBus,
    attempt: u8,
    front_int_high: bool,
) -> bool {
    let mut i2c = I2cDevice::new(bus);
    let mut input = [0u8; 1];
    if embedded_hal_async::i2c::I2c::write_read(&mut i2c, TCA6408_ADDR, &[0x00], &mut input)
        .await
        .is_ok()
    {
        info!(
            "i2c.front_probe: attempt={} phase=input-read result=ok addr=0x{:02X} input=0x{:02X} int={}",
            attempt,
            TCA6408_ADDR,
            input[0],
            if front_int_high { "high" } else { "low" }
        );
        return true;
    }

    warn!(
        "i2c.front_probe: attempt={} phase=input-read result=fail addr=0x{:02X} int={}",
        attempt,
        TCA6408_ADDR,
        if front_int_high { "high" } else { "low" }
    );

    let mut i2c = I2cDevice::new(bus);
    let (ok, method) = i2c_ack_probe(&mut i2c, TCA6408_ADDR).await;
    warn!(
        "i2c.front_probe: attempt={} phase=ack-fallback result={} addr=0x{:02X} via={} int={}",
        attempt,
        if ok { "ack" } else { "nack" },
        TCA6408_ADDR,
        method,
        if front_int_high { "high" } else { "low" }
    );
    false
}

fn log_front_panel_failure_classification(report: I2cRecoveryReport, peer_mask: u8) {
    let peer_online = (peer_mask & !FRONT_DIAG_FRONT_MASK) != 0;
    let classification = if !report.sda_final || !report.scl_final {
        "bus_stuck_after_clear"
    } else if peer_online {
        "front_tca_only_offline"
    } else {
        "shared_i2c_offline_or_power"
    };
    warn!(
        "i2c.front_diag: classification={} sda_released={} scl_released={} sda_initial={} scl_initial={} sda_final={} scl_final={} peer_mask=0x{:02X}",
        classification,
        if report.sda_released { "high" } else { "low" },
        if report.scl_released { "high" } else { "low" },
        if report.sda_initial { "high" } else { "low" },
        if report.scl_initial { "high" } else { "low" },
        if report.sda_final { "high" } else { "low" },
        if report.scl_final { "high" } else { "low" },
        peer_mask
    );
}

async fn recover_i2c_bus(
    sda_pin: esp_hal::peripherals::GPIO8<'static>,
    scl_pin: esp_hal::peripherals::GPIO9<'static>,
) -> (Flex<'static>, Flex<'static>, I2cRecoveryReport) {
    let mut sda = Flex::new(sda_pin);
    let mut scl = Flex::new(scl_pin);
    let output_config = OutputConfig::default()
        .with_drive_mode(DriveMode::OpenDrain)
        .with_pull(Pull::Up);
    let input_config = InputConfig::default().with_pull(Pull::Up);

    sda.apply_input_config(&input_config);
    scl.apply_input_config(&input_config);
    sda.set_input_enable(true);
    scl.set_input_enable(true);
    Timer::after(Duration::from_micros(10)).await;
    let sda_released = sda.is_high();
    let scl_released = scl.is_high();
    info!(
        "i2c.recover: phase=release sda={} scl={}",
        if sda_released { "high" } else { "low" },
        if scl_released { "high" } else { "low" }
    );

    sda.apply_output_config(&output_config);
    scl.apply_output_config(&output_config);
    sda.set_high();
    scl.set_high();
    sda.set_output_enable(true);
    scl.set_output_enable(true);
    Timer::after(Duration::from_micros(10)).await;

    let sda_initial = sda.is_high();
    let scl_initial = scl.is_high();
    for _ in 0..I2C_RECOVERY_CLOCKS {
        scl.set_low();
        Timer::after(Duration::from_micros(50)).await;
        scl.set_high();
        Timer::after(Duration::from_micros(50)).await;
    }

    sda.set_low();
    Timer::after(Duration::from_micros(50)).await;
    scl.set_high();
    Timer::after(Duration::from_micros(50)).await;
    sda.set_high();
    Timer::after(Duration::from_micros(50)).await;

    info!(
        "i2c.recover: clocks={} sda_initial={} scl_initial={} sda_final={} scl_final={}",
        I2C_RECOVERY_CLOCKS,
        if sda_initial { "high" } else { "low" },
        if scl_initial { "high" } else { "low" },
        if sda.is_high() { "high" } else { "low" },
        if scl.is_high() { "high" } else { "low" }
    );
    let report = I2cRecoveryReport {
        sda_released,
        scl_released,
        sda_initial,
        scl_initial,
        sda_final: sda.is_high(),
        scl_final: scl.is_high(),
    };
    (sda, scl, report)
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum PortOcpDecision {
    None,
    LowVbus,
    HighCurrent,
}

fn port_overcurrent(vbus_mv: u32, current_ma: u32) -> PortOcpDecision {
    if current_ma > PORT_OCP_HIGH_CURRENT_MA {
        return PortOcpDecision::HighCurrent;
    }
    if vbus_mv < PORT_OCP_LOW_VBUS_MV && current_ma > PORT_OCP_LOW_VBUS_MIN_CURRENT_MA {
        return PortOcpDecision::LowVbus;
    }
    PortOcpDecision::None
}

fn ocp_reason_label(decision: PortOcpDecision) -> &'static str {
    match decision {
        PortOcpDecision::None => "none",
        PortOcpDecision::LowVbus => "low_vbus",
        PortOcpDecision::HighCurrent => "high_current",
    }
}

fn reset_reason_label(reason: Option<esp_hal::rtc_cntl::SocResetReason>) -> &'static str {
    use esp_hal::rtc_cntl::SocResetReason;

    match reason {
        Some(SocResetReason::ChipPowerOn) => "chip_power_on",
        Some(SocResetReason::CoreSw) => "core_sw",
        Some(SocResetReason::CoreDeepSleep) => "core_deep_sleep",
        Some(SocResetReason::CoreMwdt0) => "core_mwdt0",
        Some(SocResetReason::CoreMwdt1) => "core_mwdt1",
        Some(SocResetReason::CoreRtcWdt) => "core_rtc_wdt",
        Some(SocResetReason::CpuMwdt0) => "cpu_mwdt0",
        Some(SocResetReason::CpuSw) => "cpu_sw",
        Some(SocResetReason::CpuRtcWdt) => "cpu_rtc_wdt",
        Some(SocResetReason::SysBrownOut) => "sys_brown_out",
        Some(SocResetReason::SysRtcWdt) => "sys_rtc_wdt",
        Some(SocResetReason::CpuMwdt1) => "cpu_mwdt1",
        Some(SocResetReason::SysSuperWdt) => "sys_super_wdt",
        Some(SocResetReason::SysClkGlitch) => "sys_clk_glitch",
        Some(SocResetReason::CoreEfuseCrc) => "core_efuse_crc",
        Some(SocResetReason::CoreUsbUart) => "core_usb_uart",
        Some(SocResetReason::CoreUsbJtag) => "core_usb_jtag",
        Some(SocResetReason::CorePwrGlitch) => "core_pwr_glitch",
        None => "unknown",
    }
}

fn port_ocp_recovery_safe(vbus_mv: u32, current_ma: u32) -> bool {
    vbus_mv >= PORT_OCP_LOW_VBUS_MV && current_ma <= PORT_OCP_HIGH_CURRENT_MA
}

fn set_port_enable(ch: u8, enabled: bool, port_enables: &mut [Output<'static>; 4]) {
    let pin = &mut port_enables[(ch & 0x03) as usize];
    if enabled {
        pin.set_high();
    } else {
        pin.set_low();
    }
}

fn handle_front_key_event(
    event: front_panel::KeyEvent,
    selected_port: &mut usize,
    manual_enabled: &mut [bool; 4],
    port_enables: &mut [Output<'static>; 4],
) {
    match event {
        front_panel::KeyEvent::Left => {
            *selected_port = if *selected_port == 0 {
                3
            } else {
                *selected_port - 1
            };
            info!("front.ui: selected_port={}", *selected_port + 1);
        }
        front_panel::KeyEvent::Right => {
            *selected_port = (*selected_port + 1) % 4;
            info!("front.ui: selected_port={}", *selected_port + 1);
        }
        front_panel::KeyEvent::Center => {
            manual_enabled[*selected_port] = !manual_enabled[*selected_port];
            if !manual_enabled[*selected_port] {
                port_enables[*selected_port].set_low();
            }
            info!(
                "front.ui: ch={} manual_output={}",
                *selected_port + 1,
                if manual_enabled[*selected_port] {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        }
    }
}

// The front-panel TCA6408A is used for key input only; display CS/RES are MCU GPIOs.

// Embassy timer adapter for the display driver
struct DisplayTimer;
impl Gc9d01Timer for DisplayTimer {
    async fn after_millis(ms: u64) {
        Timer::after(Duration::from_millis(ms)).await;
    }
}

// Minimal Eh1-async SpiDevice wrapper over esp-hal async SPI bus.
// Latest mainboard has no MCU LCD CS pin; transactions run without CS toggling.
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
    selected: bool,
    // millivolts and milliamps for convenience
    vbus_mv: u32,
    ich_ma: u32,
    ui_state: UiPortState,
    pwren_enabled: bool,
    en_enabled: bool,
    ocp_latched: bool,
}

impl PortSample {
    fn power_mw(&self) -> u32 {
        ((self.vbus_mv as u64 * self.ich_ma as u64) / 1000) as u32
    }
}

const UI_BG_GRAY: Rgb565 = Rgb565::new(31, 63, 31); // pure white background for max contrast
const UI_BORDER: Rgb565 = Rgb565::new(0, 0, 0);
const UI_SELECTED: Rgb565 = Rgb565::new(0, 24, 31);
const UI_V_YELLOW: Rgb565 = Rgb565::new(31, 45, 0); // darker amber for better contrast on white
const UI_I_RED: Rgb565 = Rgb565::new(31, 0, 0); // vivid red
const UI_W_GREEN: Rgb565 = Rgb565::new(0, 42, 0); // darker green for contrast

// UI states per column
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
enum UiPortState {
    #[default]
    Initializing,
    Disconnected, // DISC
    Closed,       // OFF
    #[allow(dead_code)]
    Overcurrent, // CC
    Normal,
}

fn port_state_label(state: UiPortState) -> &'static str {
    match state {
        UiPortState::Initializing => "init",
        UiPortState::Disconnected => "disc",
        UiPortState::Closed => "off",
        UiPortState::Overcurrent => "cc",
        UiPortState::Normal => "ok",
    }
}

fn hub_power_mode_label(sideband_online: bool, upstream_powered: bool) -> &'static str {
    if !sideband_online {
        "offline"
    } else if upstream_powered {
        "host"
    } else {
        "standalone"
    }
}

fn hub_allows_output(
    sideband_online: bool,
    upstream_powered: bool,
    pwren_enabled: [bool; 4],
    idx: usize,
) -> bool {
    sideband_online && (!upstream_powered || pwren_enabled[idx])
}

fn apply_dashboard_control_state(
    samples: &mut [PortSample; 4],
    selected_port: usize,
    manual_enabled: [bool; 4],
    target: PowerSwitchTarget,
) {
    for (idx, sample) in samples.iter_mut().enumerate() {
        sample.selected = idx == selected_port;
        if !manual_enabled[idx] || target == PowerSwitchTarget::Open {
            sample.connected = false;
            sample.vbus_mv = 0;
            sample.ich_ma = 0;
            sample.ui_state = UiPortState::Closed;
        }
    }
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
    for (col, sample) in samples.iter().enumerate() {
        if sample.selected {
            let left = (col as i32) * 40 + 1;
            let _ = Rectangle::new(Point::new(left, 1), Size::new(38, 48))
                .into_styled(PrimitiveStyle::with_stroke(UI_SELECTED, 1))
                .draw(disp);
        }
    }

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
            UiPortState::Disconnected | UiPortState::Closed | UiPortState::Overcurrent => {}
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

fn draw_boot_self_check_frame<D: embedded_graphics::draw_target::DrawTarget<Color = Rgb565>>(
    disp: &mut D,
    snapshot: &BootSelfCheckSnapshot,
    ports_page: bool,
) {
    let _ = Rectangle::new(Point::new(0, 0), Size::new(160, 50))
        .into_styled(PrimitiveStyle::with_fill(UI_BG_GRAY))
        .draw(disp);
    let _ = Rectangle::new(Point::new(0, 0), Size::new(160, 50))
        .into_styled(PrimitiveStyle::with_stroke(UI_BORDER, 1))
        .draw(disp);

    let head = MonoTextStyle::new(&FONT_7X13_BOLD, UI_BORDER);
    let ok_style = MonoTextStyle::new(&FONT_7X13_BOLD, UI_W_GREEN);
    let warn_style = MonoTextStyle::new(&FONT_7X13_BOLD, UI_V_YELLOW);
    let err_style = MonoTextStyle::new(&FONT_7X13_BOLD, UI_I_RED);
    let rows = [14i32, 23, 32, 41];

    draw_centered_text(
        disp,
        34,
        2,
        if ports_page { "PORT" } else { "SYS" },
        head,
        7,
    );
    draw_centered_text(disp, 92, 2, outcome_label(snapshot.outcome), head, 7);
    draw_centered_text(disp, 137, 2, fault_label(snapshot.first_fault), head, 7);

    if ports_page {
        for (idx, slot) in snapshot.ports.iter().enumerate() {
            let y = rows[idx];
            let style = match slot.state {
                SelfCheckItemState::Ok => ok_style,
                SelfCheckItemState::Warn | SelfCheckItemState::Skipped => warn_style,
                SelfCheckItemState::Err | SelfCheckItemState::Fatal => err_style,
                SelfCheckItemState::Pending => head,
            };
            let mut label: heapless::String<8> = heapless::String::new();
            let mut state: heapless::String<12> = heapless::String::new();
            use core::fmt::Write as _;
            let _ = write!(label, "P{}", idx + 1);
            let _ = write!(state, "{}", state_label(slot.state));
            draw_centered_text(disp, 15, y, &label, head, 7);
            draw_centered_text(disp, 65, y, &state, style, 7);
            draw_centered_text(disp, 122, y, fault_label(slot.fault), style, 7);
        }
    } else {
        let sys = [
            ("VIN", snapshot.sys[0]),
            ("MUX", snapshot.sys[1]),
            ("PANEL", snapshot.sys[2]),
            ("FAN", snapshot.sys[3]),
        ];
        for (idx, (name, slot)) in sys.iter().enumerate() {
            let y = rows[idx];
            let style = match slot.state {
                SelfCheckItemState::Ok => ok_style,
                SelfCheckItemState::Warn | SelfCheckItemState::Skipped => warn_style,
                SelfCheckItemState::Err | SelfCheckItemState::Fatal => err_style,
                SelfCheckItemState::Pending => head,
            };
            draw_centered_text(disp, 20, y, name, head, 7);
            draw_centered_text(disp, 74, y, state_label(slot.state), style, 7);
            draw_centered_text(disp, 126, y, fault_label(slot.fault), style, 7);
        }
    }
}

async fn flush_boot_self_check<
    BUS: Eh1SpiBus<Error = esp_hal::spi::Error>,
    D: embedded_hal::digital::OutputPin<Error = core::convert::Infallible>,
    R: embedded_hal::digital::OutputPin<Error = core::convert::Infallible>,
>(
    disp: &mut GC9D01<'_, SimpleSpiDev<'_, BUS>, D, R, DisplayTimer>,
    snapshot: &BootSelfCheckSnapshot,
    ports_page: bool,
) {
    draw_boot_self_check_frame(disp, snapshot, ports_page);
    let _ = disp.flush().await;
}

async fn ack_scan_vin_off() {
    for ch in 0u8..4u8 {
        let mut i2c_scan =
            I2cDevice::new(unsafe { SENSOR_I2C_BUS_REF.expect("sensor I2C bus not initialized") });
        let (ina_addr, tmp_addr) = module_addr_pair(ch);
        let mut ina_ok = false;
        let mut ina_method = "no";
        let mut ina_tries: u8 = 0;
        let mut tmp_ok = false;
        let mut tmp_method = "no";
        let mut tmp_tries: u8 = 0;
        for attempt in 0..MODULE_SENSOR_RETRIES {
            let (ok, method) = i2c_ack_probe(&mut i2c_scan, ina_addr).await;
            ina_tries = attempt + 1;
            if ok {
                ina_ok = true;
                ina_method = method;
                break;
            }
            Timer::after(Duration::from_millis(MODULE_SENSOR_RETRY_MS)).await;
        }
        for attempt in 0..MODULE_SENSOR_RETRIES {
            let (ok, method) = i2c_ack_probe(&mut i2c_scan, tmp_addr).await;
            tmp_tries = attempt + 1;
            if ok {
                tmp_ok = true;
                tmp_method = method;
                break;
            }
            Timer::after(Duration::from_millis(MODULE_SENSOR_RETRY_MS)).await;
        }
        info!(
            "i2c.scan: ch={} ina226@0x{:02X}={} via={} tries={} tmp112@0x{:02X}={} via={} tries={} vin_on=false",
            ch,
            ina_addr,
            if ina_ok { "yes" } else { "no" },
            ina_method,
            ina_tries,
            tmp_addr,
            if tmp_ok { "yes" } else { "no" },
            tmp_method,
            tmp_tries
        );
    }
}

// No extra helpers for INA226 per request; use driver inline at call sites

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let p = esp_hal::init(esp_hal::Config::default());
    let reset_reason = system::reset_reason();
    info!("app.start");

    // Initialize the embassy time driver
    let timg0 = TimerGroup::new(p.TIMG0);
    esp_hal_embassy::init(timg0.timer0);
    info!("init.time: embassy-timer=ok");

    // GPIO prepare
    // Mainboard RESET# is MCU-driven. The front-panel TCA6408A reset pin is a
    // local fixed pull-up on the front-panel PCB and is not controlled here.
    let mut i2c_reset = Output::new(
        p.GPIO35,
        Level::Low,
        esp_hal::gpio::OutputConfig::default().with_drive_mode(DriveMode::PushPull),
    )
    .into_flex();
    i2c_reset.set_input_enable(true);
    info!("main.reset: gpio=GPIO35 mode=push-pull pulse=10ms release_wait=50ms");
    Timer::after(Duration::from_millis(10)).await;
    i2c_reset.set_high();
    Timer::after(Duration::from_millis(50)).await;
    let i2c_reset_released_high = i2c_reset.is_high();

    // IN_CE default off: high turns on the NMOS that pulls TPS2490 EN low.
    let in_ce = Output::new(
        p.GPIO41,
        Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );
    // PG input
    let in_pg = Input::new(
        p.GPIO42,
        esp_hal::gpio::InputConfig::default().with_pull(Pull::Up),
    );
    let usb_v1ok = Input::new(
        p.GPIO21,
        esp_hal::gpio::InputConfig::default().with_pull(Pull::Up),
    );
    info!(
        "hub.upstream: v1ok={} gpio=GPIO{}",
        if usb_v1ok.is_high() { "high" } else { "low" },
        PIN_USB_V1OK
    );
    let mut vin_adc_config = AdcConfig::new();
    let vin_adc_pin: power_in::VinAdcPin =
        vin_adc_config.enable_pin_with_cal(p.GPIO4, Attenuation::_11dB);
    let vin_adc = Adc::new(p.ADC1, vin_adc_config);

    let front_int_pin = Input::new(
        p.GPIO16,
        esp_hal::gpio::InputConfig::default().with_pull(Pull::Up),
    );

    // EN lines default disabled (drive low => module disabled)
    let mut port_enables = [
        Output::new(p.GPIO17, Level::Low, esp_hal::gpio::OutputConfig::default()),
        Output::new(p.GPIO18, Level::Low, esp_hal::gpio::OutputConfig::default()),
        Output::new(p.GPIO39, Level::Low, esp_hal::gpio::OutputConfig::default()),
        Output::new(p.GPIO40, Level::Low, esp_hal::gpio::OutputConfig::default()),
    ];
    for port_enable in port_enables.iter_mut() {
        port_enable.set_low();
    }

    info!("init.hw: chip=ESP32-S3 sensor_i2c=sdaGPIO8/sclGPIO9 hub_i2c=sdaGPIO14/sclGPIO13");

    // Publish power-on intent first (only intent; actual switch controlled after qualification)
    PWR_SW_TARGET.store(PowerSwitchTarget::Closed as u8, Ordering::Relaxed);

    // Recover a possible half-finished sensor/front-panel I2C transaction left by MCU-only resets.
    let (i2c_sda, i2c_scl, i2c_recovery) = recover_i2c_bus(p.GPIO8, p.GPIO9).await;
    // Initialize I2C0 on SDA1/SCL1 for input INA226, front panel, and output-module sensors.
    let i2c_hw = I2c::new(p.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(i2c_sda)
        .with_scl(i2c_scl)
        .into_async();
    let bus = SENSOR_I2C_BUS.init(Mutex::new(i2c_hw));

    // Initialize I2C1 on SDA0/SCL0 for the mainboard CH335F sideband expander.
    let hub_i2c_hw = I2c::new(p.I2C1, I2cConfig::default())
        .unwrap()
        .with_sda(p.GPIO14)
        .with_scl(p.GPIO13)
        .into_async();
    let hub_bus = HUB_I2C_BUS.init(Mutex::new(hub_i2c_hw));

    info!("lcd.ctrl: cs=none rst=board-reset");

    // Setup SPI2 and display. Latest mainboard exposes DC/MOSI/SCLK/BLK; CS is not MCU-driven,
    // and LCD reset follows the board-level RESET# net.
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
    // Backlight enable is active-low through the panel-side P-MOS gate.
    let mut blk = match PIN_LCD_BLK_GPIO {
        15 => Output::new(p.GPIO15, Level::Low, esp_hal::gpio::OutputConfig::default()),
        _ => Output::new(p.GPIO15, Level::Low, esp_hal::gpio::OutputConfig::default()),
    };
    blk.set_low();

    const LOGICAL_W: usize = 160;
    const LOGICAL_H: usize = 50;
    let mut fb_buf: [Rgb565; LOGICAL_W * LOGICAL_H] = [Rgb565::BLACK; LOGICAL_W * LOGICAL_H];
    let fb: &mut [Rgb565] = &mut fb_buf;

    let spi_dev = SimpleSpiDev {
        bus: spi_bus,
        cs: None,
    };
    let cfg = DisplayConfig {
        width: LOGICAL_W as u16,
        height: LOGICAL_H as u16,
        orientation: Orientation::LandscapeSwapped,
        rgb: false,
        inverted: false,
        dx: 15,
        dy: 0,
    };
    let rst = NoopPin;
    let mut disp: GC9D01<_, _, _, DisplayTimer> = GC9D01::new(cfg, spi_dev, dc, rst, fb);
    info!("lcd.init: start panel_160x50 mode (mcu cs/rst, landscape-swapped)");
    if let Err(_e) = disp.init().await {
        warn!("lcd.init: failed (fallback)");
    } else {
        info!("lcd.init: panel ready");
    }
    info!(
        "main.reset: release_level={} reset_reason={}",
        if i2c_reset_released_high {
            "high"
        } else {
            "low"
        },
        reset_reason_label(reset_reason)
    );

    let mut boot_snapshot = BootSelfCheckSnapshot::new();
    boot_snapshot.set_stage(BootStage::SelfCheck);
    info!("boot.stage: stage=self-check");
    flush_boot_self_check(&mut disp, &boot_snapshot, false).await;

    // Record global bus reference for channel views.
    unsafe {
        SENSOR_I2C_BUS_REF = Some(bus);
    }

    info!("i2c.topo: direct shared bus; mux probe skipped");
    info!("boot.check: name=mux state=skip fault=-");
    boot_snapshot.set_sys(
        SysCheck::Mux,
        SelfCheckItemState::Skipped,
        BootFaultCode::None,
    );
    flush_boot_self_check(&mut disp, &boot_snapshot, false).await;

    // Spawn power input task: handles INA init/qualification/VIN_ON/periodic status
    power_in::spawn(
        &spawner,
        bus,
        in_ce,
        in_pg,
        vin_adc,
        vin_adc_pin,
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

    let power_boot = power_in::bootstrap_signal().wait().await;
    info!(
        "boot.check: name=vin state={} fault={} vin={}V pg={}",
        state_label(power_boot.state),
        fault_label(power_boot.fault),
        power_boot.vin_v,
        if power_boot.pg_good { "good" } else { "bad" }
    );
    boot_snapshot.set_sys(SysCheck::Vin, power_boot.state, power_boot.fault);
    flush_boot_self_check(&mut disp, &boot_snapshot, false).await;

    info!(
        "main.reset: pre-tca-probe level={} reset_reason={}",
        if i2c_reset.is_high() { "high" } else { "low" },
        reset_reason_label(reset_reason)
    );
    let mut hub_ctrl = None;
    if power_boot.ready {
        let mut hub_i2c = I2cDevice::new(hub_bus);
        match hub_sideband::Controller::init(&mut hub_i2c).await {
            Ok(ctrl) => {
                match ctrl.snapshot(&mut hub_i2c).await {
                    Ok(s) => info!(
                        "hub.sideband: tca6408a=online addr=0x{:02X} init=ok input=0x{:02X} p1_pwren={} p2_pwren={} p3_pwren={} p4_pwren={}",
                        hub_sideband::TCA6408_ADDR,
                        s.input,
                        if s.pwren_enabled[0] { "on" } else { "off" },
                        if s.pwren_enabled[1] { "on" } else { "off" },
                        if s.pwren_enabled[2] { "on" } else { "off" },
                        if s.pwren_enabled[3] { "on" } else { "off" },
                    ),
                    Err(_) => warn!(
                        "hub.sideband: tca6408a=online addr=0x{:02X} init=ok read=fail",
                        hub_sideband::TCA6408_ADDR
                    ),
                }
                hub_ctrl = Some(ctrl);
            }
            Err(_) => warn!(
                "hub.sideband: tca6408a=offline addr=0x{:02X}; keep outputs closed",
                hub_sideband::TCA6408_ADDR
            ),
        }
        if hub_ctrl.is_none() {
            log_i2c_diag_scan(hub_bus, "hub-sideband-offline").await;
        }
    }

    let mut front_panel_online = false;
    let mut front_panel_peer_mask = 0u8;
    if power_boot.ready {
        boot_snapshot.set_sys(
            SysCheck::Front,
            SelfCheckItemState::Pending,
            BootFaultCode::None,
        );
        flush_boot_self_check(&mut disp, &boot_snapshot, false).await;

        // Current V3 hardware cannot hard-reset or power-cycle the front-panel
        // TCA6408A from the MCU. If bus-clear cannot recover 0x21, waiting here
        // forever leaves the product unusable. Future hardware with a routed
        // front-panel TCA RESET#/VCCP control should remove this degraded path
        // and recover the expander before runtime.
        for attempt in 0..FRONT_PANEL_BOOT_PROBE_ATTEMPTS {
            let attempt_no = attempt + 1;
            let front_int_high = front_int_pin.is_high();
            if probe_front_panel_tca(bus, attempt_no, front_int_high).await {
                info!("i2c.front: tca6408a=online addr=0x{:02X}", TCA6408_ADDR);
                info!("boot.check: name=panel state=ok fault=-");
                boot_snapshot.set_sys(SysCheck::Front, SelfCheckItemState::Ok, BootFaultCode::None);
                front_panel_online = true;
                break;
            }

            warn!(
                "i2c.front: tca6408a=offline addr=0x{:02X}; retry={}/{}",
                TCA6408_ADDR, attempt_no, FRONT_PANEL_BOOT_PROBE_ATTEMPTS
            );
            warn!(
                "front.int: level={} while_panel_pending",
                if front_int_high { "high" } else { "low" }
            );
            if attempt == 0 {
                front_panel_peer_mask =
                    log_front_i2c_peer_matrix(bus, hub_bus, "front-panel-wait", attempt_no).await;
                log_i2c_diag_scan(bus, "front-panel-wait").await;
            }
            flush_boot_self_check(&mut disp, &boot_snapshot, false).await;
            Timer::after(Duration::from_millis(500)).await;
        }
        if !front_panel_online {
            if front_panel_peer_mask == 0 {
                front_panel_peer_mask = log_front_i2c_peer_matrix(
                    bus,
                    hub_bus,
                    "front-panel-final",
                    FRONT_PANEL_BOOT_PROBE_ATTEMPTS,
                )
                .await;
            }
            log_front_panel_failure_classification(i2c_recovery, front_panel_peer_mask);
            warn!(
                "boot.check: name=panel state=warn fault=FrontPanelOffline recovery=blocked_no_reset_pin"
            );
            boot_snapshot.set_sys(
                SysCheck::Front,
                SelfCheckItemState::Warn,
                BootFaultCode::FrontPanelOffline,
            );
        }
    } else {
        boot_snapshot.set_sys(
            SysCheck::Front,
            SelfCheckItemState::Skipped,
            power_boot.fault,
        );
    }
    flush_boot_self_check(&mut disp, &boot_snapshot, false).await;

    let fan_ready = if power_boot.ready {
        if fan::spawn(&spawner, p.LEDC, p.PCNT, p.SENS, p.GPIO1, p.GPIO2, p.GPIO6).is_ok() {
            with_timeout(Duration::from_millis(1500), fan::bootstrap_signal().wait())
                .await
                .is_ok_and(|ready| ready)
        } else {
            false
        }
    } else {
        false
    };
    if power_boot.ready && fan_ready {
        info!("boot.check: name=fan state=ok fault=-");
        boot_snapshot.set_sys(SysCheck::Fan, SelfCheckItemState::Ok, BootFaultCode::None);
    } else if power_boot.ready {
        warn!("boot.check: name=fan state=warn fault=FanUnavailable");
        boot_snapshot.set_sys(
            SysCheck::Fan,
            SelfCheckItemState::Warn,
            BootFaultCode::FanUnavailable,
        );
    } else {
        boot_snapshot.set_sys(SysCheck::Fan, SelfCheckItemState::Skipped, power_boot.fault);
    }
    flush_boot_self_check(&mut disp, &boot_snapshot, false).await;

    let mut gates = GateDecision::new();
    gates.allow_runtime_tasks = power_boot.ready;
    gates.keep_input_switch_open = !power_boot.ready;
    gates.allow_front_panel = front_panel_online;

    if front_panel_online {
        front_panel::spawn(&spawner, bus, front_int_pin).expect("spawn front_panel task");
    }

    if !power_boot.ready {
        warn!("pwr.in: vin_on=false; skip module init; do ack-scan only");
        ack_scan_vin_off().await;
        for (ch, done) in CH_SCAN_DONE.iter().enumerate() {
            boot_snapshot.set_port(ch, SelfCheckItemState::Skipped, power_boot.fault);
            done.store(true, Ordering::Relaxed);
        }
    }

    // After VIN ON, scan each output module for the V3 sensor pair (INA226 + TMP112).
    if power_boot.ready {
        info!("i2c.scan:start vin_on=true topo=direct");
    }
    for ch in 0u8..4u8 {
        if !power_boot.ready {
            continue;
        }

        let mut i2c_scan = I2cDevice::new(bus);
        let (ina_addr, tmp_addr) = module_addr_pair(ch);
        boot_snapshot.set_port(
            ch as usize,
            SelfCheckItemState::Pending,
            BootFaultCode::None,
        );
        Timer::after(Duration::from_millis(10)).await;

        let mut ina_ok = false;
        let mut ina_method = "no";
        let mut ina_tries: u8 = 0;
        for attempt in 0..MODULE_SENSOR_RETRIES {
            let (ok, method) = i2c_ack_probe(&mut i2c_scan, ina_addr).await;
            ina_tries = attempt + 1;
            if ok {
                ina_ok = true;
                ina_method = method;
                break;
            }
            Timer::after(Duration::from_millis(MODULE_SENSOR_RETRY_MS)).await;
        }

        let mut tmp_ok = false;
        let mut tmp_method = "no";
        let mut tmp_tries: u8 = 0;
        for attempt in 0..MODULE_SENSOR_RETRIES {
            let (ok, method) = i2c_ack_probe(&mut i2c_scan, tmp_addr).await;
            tmp_tries = attempt + 1;
            if ok {
                tmp_ok = true;
                tmp_method = method;
                break;
            }
            Timer::after(Duration::from_millis(MODULE_SENSOR_RETRY_MS)).await;
        }

        info!(
            "i2c.scan: ch={} ina226@0x{:02X}={} via={} tries={} tmp112@0x{:02X}={} via={} tries={}",
            ch,
            ina_addr,
            if ina_ok { "online" } else { "offline" },
            ina_method,
            ina_tries,
            tmp_addr,
            if tmp_ok { "online" } else { "offline" },
            tmp_method,
            tmp_tries
        );

        if ina_ok && tmp_ok {
            info!("boot.check: name=port{} state=ok fault=-", ch + 1);
            CH_RDY[ch as usize].store(true, Ordering::Relaxed);
            boot_snapshot.set_port(ch as usize, SelfCheckItemState::Ok, BootFaultCode::None);
        } else {
            let fault = if !ina_ok && !tmp_ok {
                BootFaultCode::PortModuleOffline(ch + 1)
            } else if !ina_ok {
                BootFaultCode::PortInaOffline(ch + 1)
            } else {
                BootFaultCode::PortTempOffline(ch + 1)
            };
            warn!(
                "boot.check: name=port{} state=err fault={}",
                ch + 1,
                fault_label(fault)
            );
            CH_RDY[ch as usize].store(false, Ordering::Relaxed);
            boot_snapshot.set_port(ch as usize, SelfCheckItemState::Err, fault);
        }

        CH_SCAN_DONE[ch as usize].store(true, Ordering::Relaxed);
        flush_boot_self_check(&mut disp, &boot_snapshot, true).await;
    }

    if power_boot.ready && hub_ctrl.is_none() {
        for ch in 0..4usize {
            CH_RDY[ch].store(false, Ordering::Relaxed);
            CH_SCAN_DONE[ch].store(true, Ordering::Relaxed);
            boot_snapshot.set_port(
                ch,
                SelfCheckItemState::Err,
                BootFaultCode::HubSidebandOffline,
            );
        }
        warn!("boot.check: name=hub-sideband state=err fault=HubSidebandOffline");
        flush_boot_self_check(&mut disp, &boot_snapshot, true).await;
    }

    if power_boot.ready {
        let mut pwren_enabled = [false; 4];
        let mut hub_read_failed = false;
        if let Some(ctrl) = hub_ctrl.as_mut() {
            let mut hub_i2c = I2cDevice::new(hub_bus);
            match ctrl.snapshot(&mut hub_i2c).await {
                Ok(snapshot) => pwren_enabled = snapshot.pwren_enabled,
                Err(_) => hub_read_failed = true,
            }
        }
        if hub_read_failed {
            warn!(
                "hub.sideband: read failed during gate apply addr=0x{:02X}; keep outputs closed",
                hub_sideband::TCA6408_ADDR
            );
            hub_ctrl = None;
            for ch in 0..4usize {
                CH_RDY[ch].store(false, Ordering::Relaxed);
                CH_SCAN_DONE[ch].store(true, Ordering::Relaxed);
                boot_snapshot.set_port(
                    ch,
                    SelfCheckItemState::Err,
                    BootFaultCode::HubSidebandOffline,
                );
            }
            warn!("boot.check: name=hub-sideband state=err fault=HubSidebandOffline");
            flush_boot_self_check(&mut disp, &boot_snapshot, true).await;
        }
        let upstream_powered = usb_v1ok.is_high();
        let hub_mode = hub_power_mode_label(hub_ctrl.is_some(), upstream_powered);
        for ch in 0..4u8 {
            let enabled = hub_allows_output(
                hub_ctrl.is_some(),
                upstream_powered,
                pwren_enabled,
                ch as usize,
            );
            set_port_enable(ch, enabled, &mut port_enables);
            gates.allow_port[ch as usize] = enabled;
            info!(
                "boot.gate: port{} mode={} v1ok={} pwren={} ocp=false en={}",
                ch + 1,
                hub_mode,
                if upstream_powered { "high" } else { "low" },
                if pwren_enabled[ch as usize] {
                    "on"
                } else {
                    "off"
                },
                if enabled { "on" } else { "off" }
            );
        }
    }

    boot_snapshot.set_stage(BootStage::GateApply);
    info!("boot.stage: stage=gate-apply");
    boot_snapshot.finalize(gates);
    let show_sticky = boot_snapshot.outcome != BootOutcome::Ok;
    let mut final_gates = boot_snapshot.gates;
    final_gates.show_sticky_self_check = show_sticky;
    boot_snapshot.finalize(final_gates);
    info!(
        "boot.summary: outcome={} first_fault={} runtime={} front_panel={}",
        outcome_label(boot_snapshot.outcome),
        fault_label(boot_snapshot.first_fault),
        if boot_snapshot.gates.allow_runtime_tasks {
            "on"
        } else {
            "off"
        },
        if boot_snapshot.gates.allow_front_panel {
            "on"
        } else {
            "off"
        }
    );
    flush_boot_self_check(&mut disp, &boot_snapshot, false).await;

    if boot_snapshot.outcome == BootOutcome::Fatal {
        loop {
            flush_boot_self_check(&mut disp, &boot_snapshot, false).await;
            Timer::after(Duration::from_millis(700)).await;
            flush_boot_self_check(&mut disp, &boot_snapshot, true).await;
            Timer::after(Duration::from_millis(700)).await;
        }
    }

    if boot_snapshot.gates.show_sticky_self_check {
        for step in 0..4u8 {
            flush_boot_self_check(&mut disp, &boot_snapshot, (step & 1) != 0).await;
            Timer::after(Duration::from_millis(700)).await;
        }
    }

    boot_snapshot.set_stage(BootStage::Runtime);
    info!("boot.stage: stage=runtime");

    // 当前 V3 输出模块不再沿用旧的 SW2303 runtime 遥测任务；
    // dashboard 直接按通道读取模块 INA226，避免旧驱动误报。
    // === UI periodic refresh loop (2 Hz) ===
    front_panel::clear_events();
    let front_events = front_panel::event_receiver();
    let mut selected_port: usize = 0;
    let mut manual_enabled = [true; 4];
    let mut ocp_latched = [false; 4];
    let mut ocp_safe_samples = [0u8; 4];
    let mut ocp_retry_wait = [0u8; 4];
    let mut port_output_active = boot_snapshot.gates.allow_port;
    loop {
        while let Ok(event) = front_events.try_receive() {
            handle_front_key_event(
                event,
                &mut selected_port,
                &mut manual_enabled,
                &mut port_enables,
            );
        }

        // Derive per-port samples
        let mut view: [PortSample; 4] = [
            PortSample {
                connected: false,
                selected: false,
                vbus_mv: 0,
                ich_ma: 0,
                ui_state: UiPortState::Initializing,
                pwren_enabled: false,
                en_enabled: false,
                ocp_latched: false,
            },
            PortSample {
                connected: false,
                selected: false,
                vbus_mv: 0,
                ich_ma: 0,
                ui_state: UiPortState::Initializing,
                pwren_enabled: false,
                en_enabled: false,
                ocp_latched: false,
            },
            PortSample {
                connected: false,
                selected: false,
                vbus_mv: 0,
                ich_ma: 0,
                ui_state: UiPortState::Initializing,
                pwren_enabled: false,
                en_enabled: false,
                ocp_latched: false,
            },
            PortSample {
                connected: false,
                selected: false,
                vbus_mv: 0,
                ich_ma: 0,
                ui_state: UiPortState::Initializing,
                pwren_enabled: false,
                en_enabled: false,
                ocp_latched: false,
            },
        ];
        let target = if PWR_SW_TARGET.load(Ordering::Relaxed) == (PowerSwitchTarget::Open as u8) {
            PowerSwitchTarget::Open
        } else {
            PowerSwitchTarget::Closed
        };
        let mut hub_runtime_failed = false;
        let hub_snapshot = if let Some(ctrl) = hub_ctrl.as_mut() {
            let mut hub_i2c = I2cDevice::new(hub_bus);
            match ctrl.snapshot(&mut hub_i2c).await {
                Ok(snapshot) => Some(snapshot),
                Err(_) => {
                    hub_runtime_failed = true;
                    None
                }
            }
        } else {
            None
        };
        if hub_runtime_failed {
            warn!(
                "hub.sideband: runtime read failed addr=0x{:02X}; close all outputs",
                hub_sideband::TCA6408_ADDR
            );
            hub_ctrl = None;
        }
        let sideband_online = hub_snapshot.is_some();
        let pwren_enabled = hub_snapshot
            .map(|snapshot| snapshot.pwren_enabled)
            .unwrap_or([false; 4]);
        let upstream_powered = usb_v1ok.is_high();
        let hub_mode = hub_power_mode_label(sideband_online, upstream_powered);
        let mut desired_en = [false; 4];
        for ch in 0u8..4u8 {
            let idx = ch as usize;
            view[idx].selected = idx == selected_port;
            view[idx].pwren_enabled = pwren_enabled[idx];
            view[idx].ocp_latched = ocp_latched[idx];
            if !manual_enabled[idx] {
                view[idx].ui_state = UiPortState::Closed;
                continue;
            }
            if target == PowerSwitchTarget::Open {
                view[idx].ui_state = UiPortState::Closed;
                continue;
            }
            if !hub_allows_output(sideband_online, upstream_powered, pwren_enabled, idx) {
                view[idx].ui_state = UiPortState::Closed;
                continue;
            }
            if ocp_latched[idx] && !port_output_active[idx] {
                ocp_retry_wait[idx] = ocp_retry_wait[idx].saturating_add(1);
                view[idx].ui_state = UiPortState::Overcurrent;
                if ocp_retry_wait[idx] >= PORT_OCP_RELEASE_SAFE_SAMPLES {
                    desired_en[idx] = true;
                }
                continue;
            }
            let mut keep_recovery_probe_enabled = false;
            if CH_RDY[idx].load(Ordering::Relaxed) {
                view[idx].connected = true;
                match sample_module_ina226(ch).await {
                    Some((v_mv, i_ma)) => {
                        view[idx].vbus_mv = v_mv;
                        view[idx].ich_ma = i_ma;
                        let ocp_decision = port_overcurrent(v_mv, i_ma);
                        if ocp_decision != PortOcpDecision::None {
                            let was_latched = ocp_latched[idx];
                            ocp_latched[idx] = true;
                            ocp_safe_samples[idx] = 0;
                            ocp_retry_wait[idx] = 0;
                            set_port_enable(ch, false, &mut port_enables);
                            port_output_active[idx] = false;
                            view[idx].en_enabled = false;
                            if !was_latched {
                                warn!(
                                    "hub.sideband: ocp latch port{} reason={} vbus={}mV i={}mA",
                                    ch + 1,
                                    ocp_reason_label(ocp_decision),
                                    v_mv,
                                    i_ma
                                );
                            }
                        } else if ocp_latched[idx] && port_output_active[idx] {
                            keep_recovery_probe_enabled = true;
                            if port_ocp_recovery_safe(v_mv, i_ma) {
                                ocp_safe_samples[idx] = ocp_safe_samples[idx].saturating_add(1);
                                if ocp_safe_samples[idx] >= PORT_OCP_RELEASE_SAFE_SAMPLES {
                                    ocp_latched[idx] = false;
                                    ocp_safe_samples[idx] = 0;
                                    ocp_retry_wait[idx] = 0;
                                    keep_recovery_probe_enabled = false;
                                    info!("hub.sideband: ocp release port{}", ch + 1);
                                }
                            } else {
                                ocp_safe_samples[idx] = 0;
                            }
                        } else if ocp_latched[idx] {
                            ocp_safe_samples[idx] = 0;
                        }
                        view[idx].ui_state = if ocp_latched[idx] {
                            UiPortState::Overcurrent
                        } else {
                            UiPortState::Normal
                        };
                    }
                    None => {
                        view[idx].connected = false;
                        view[idx].ui_state = UiPortState::Disconnected;
                    }
                }
            } else if CH_SCAN_DONE[idx].load(Ordering::Relaxed) {
                view[idx].ui_state = UiPortState::Disconnected;
            } else {
                view[idx].ui_state = UiPortState::Initializing;
            }
            if ocp_latched[idx] {
                view[idx].ui_state = UiPortState::Overcurrent;
            }
            desired_en[idx] = !ocp_latched[idx] || keep_recovery_probe_enabled;
            view[idx].ocp_latched = ocp_latched[idx];
        }
        let mut hub_write_failed = false;
        if let Some(ctrl) = hub_ctrl.as_mut() {
            let mut hub_i2c = I2cDevice::new(hub_bus);
            for ch in 0u8..4u8 {
                if ctrl
                    .set_overcurrent(&mut hub_i2c, ch, ocp_latched[ch as usize])
                    .await
                    .is_err()
                {
                    hub_write_failed = true;
                    break;
                }
            }
        }
        if hub_write_failed {
            warn!(
                "hub.sideband: runtime write failed addr=0x{:02X}; close all outputs",
                hub_sideband::TCA6408_ADDR
            );
            hub_ctrl = None;
            desired_en = [false; 4];
        }
        for ch in 0u8..4u8 {
            set_port_enable(ch, desired_en[ch as usize], &mut port_enables);
            view[ch as usize].en_enabled = desired_en[ch as usize];
            port_output_active[ch as usize] = desired_en[ch as usize];
        }
        info!(
            "port.telemetry: hub_mode={} v1ok={} p1={} {}mV {}mA pwren={} ocp={} en={} p2={} {}mV {}mA pwren={} ocp={} en={} p3={} {}mV {}mA pwren={} ocp={} en={} p4={} {}mV {}mA pwren={} ocp={} en={}",
            hub_mode,
            if upstream_powered { "high" } else { "low" },
            port_state_label(view[0].ui_state),
            view[0].vbus_mv,
            view[0].ich_ma,
            if view[0].pwren_enabled { "on" } else { "off" },
            if view[0].ocp_latched { "on" } else { "off" },
            if view[0].en_enabled { "on" } else { "off" },
            port_state_label(view[1].ui_state),
            view[1].vbus_mv,
            view[1].ich_ma,
            if view[1].pwren_enabled { "on" } else { "off" },
            if view[1].ocp_latched { "on" } else { "off" },
            if view[1].en_enabled { "on" } else { "off" },
            port_state_label(view[2].ui_state),
            view[2].vbus_mv,
            view[2].ich_ma,
            if view[2].pwren_enabled { "on" } else { "off" },
            if view[2].ocp_latched { "on" } else { "off" },
            if view[2].en_enabled { "on" } else { "off" },
            port_state_label(view[3].ui_state),
            view[3].vbus_mv,
            view[3].ich_ma,
            if view[3].pwren_enabled { "on" } else { "off" },
            if view[3].ocp_latched { "on" } else { "off" },
            if view[3].en_enabled { "on" } else { "off" },
        );
        // Draw and flush
        draw_dashboard_frame(&mut disp, &view);
        let _ = disp.flush().await;
        if let Ok(event) = with_timeout(Duration::from_millis(500), front_events.receive()).await {
            handle_front_key_event(
                event,
                &mut selected_port,
                &mut manual_enabled,
                &mut port_enables,
            );
            apply_dashboard_control_state(&mut view, selected_port, manual_enabled, target);
            draw_dashboard_frame(&mut disp, &view);
            let _ = disp.flush().await;
        }
    }
}
