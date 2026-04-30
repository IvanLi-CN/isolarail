use defmt::{info, warn};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::{task, SpawnError, Spawner};
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver};
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker, Timer};
// not using InputPin trait directly; rely on esp-hal Input::is_high()
use embedded_hal_async::i2c::I2c;
use esp_hal::gpio::Input;

use crate::{I2cBus, TCA6408_ADDR};

const KEY_DEBOUNCE_MS: u64 = 25;
const FALLBACK_SCAN_MS: u64 = 500;
const TCA_READ_RETRY_DELAY_MS: u64 = 2;
const REG_INPUT: u8 = 0x00;
const REG_OUTPUT: u8 = 0x01;
const REG_POLARITY: u8 = 0x02;
const REG_CONFIG: u8 = 0x03;
const DISPLAY_RST_BIT: u8 = 1 << 5;
const DISPLAY_CS_BIT: u8 = 1 << 6;
const DISPLAY_OUTPUT_IDLE: u8 = 0xFF;
const DISPLAY_CONFIG: u8 = 0b1001_1111; // P0..P4/P7 inputs, P5 RES + P6 CS outputs.

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum KeyEvent {
    Left,
    Right,
    Center,
}

static KEY_EVENTS: Channel<CriticalSectionRawMutex, KeyEvent, 8> = Channel::new();
static INT_TRIGGER: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub fn event_receiver() -> Receiver<'static, CriticalSectionRawMutex, KeyEvent, 8> {
    KEY_EVENTS.receiver()
}

pub fn clear_events() {
    KEY_EVENTS.clear();
}

/// Probe TCA6408A presence by reading its input register once.
/// Returns true when device ACKs and read succeeds.
pub async fn is_present(bus: &'static Mutex<CriticalSectionRawMutex, I2cBus>) -> bool {
    let mut i2c = I2cDevice::new(bus);
    let mut b = [0u8; 1];
    embedded_hal_async::i2c::I2c::write_read(&mut i2c, TCA6408_ADDR, &[REG_INPUT], &mut b)
        .await
        .is_ok()
}

pub async fn init_display_control(bus: &'static Mutex<CriticalSectionRawMutex, I2cBus>) -> bool {
    let mut i2c = I2cDevice::new(bus);
    if tca_write_output(&mut i2c, DISPLAY_OUTPUT_IDLE)
        .await
        .is_err()
    {
        return false;
    }
    if i2c
        .write(TCA6408_ADDR, &[REG_POLARITY, 0x00])
        .await
        .is_err()
    {
        return false;
    }
    i2c.write(TCA6408_ADDR, &[REG_CONFIG, DISPLAY_CONFIG])
        .await
        .is_ok()
}

pub async fn pulse_display_reset(bus: &'static Mutex<CriticalSectionRawMutex, I2cBus>) -> bool {
    let mut i2c = I2cDevice::new(bus);
    if tca_write_output(&mut i2c, DISPLAY_OUTPUT_IDLE & !DISPLAY_RST_BIT)
        .await
        .is_err()
    {
        return false;
    }
    Timer::after(Duration::from_millis(10)).await;
    if tca_write_output(&mut i2c, DISPLAY_OUTPUT_IDLE)
        .await
        .is_err()
    {
        return false;
    }
    Timer::after(Duration::from_millis(120)).await;
    true
}

pub async fn set_display_cs(
    bus: &'static Mutex<CriticalSectionRawMutex, I2cBus>,
    asserted: bool,
) -> bool {
    let mut i2c = I2cDevice::new(bus);
    let output = if asserted {
        DISPLAY_OUTPUT_IDLE & !DISPLAY_CS_BIT
    } else {
        DISPLAY_OUTPUT_IDLE
    };
    tca_write_output(&mut i2c, output).await.is_ok()
}

pub fn spawn(
    spawner: &Spawner,
    bus: &'static Mutex<CriticalSectionRawMutex, I2cBus>,
    int_pin: Input<'static>,
) -> Result<(), SpawnError> {
    spawner.spawn(int_task(int_pin))?;
    spawner.spawn(task(bus))
}

#[task]
async fn int_task(mut int_pin: Input<'static>) {
    info!("front.gpio: int=edge-wait");
    loop {
        if int_pin.is_low() {
            INT_TRIGGER.signal(());
            int_pin.wait_for_high().await;
            Timer::after(Duration::from_millis(KEY_DEBOUNCE_MS)).await;
        }
        int_pin.wait_for_falling_edge().await;
    }
}

#[task]
async fn task(bus: &'static Mutex<CriticalSectionRawMutex, I2cBus>) {
    // Establish baseline: read input register once.
    let mut i2c = I2cDevice::new(bus);
    let mut last_inputs: u8 = match tca_read_inputs(&mut i2c).await {
        Ok(v) => v,
        Err(_) => {
            warn!("front.gpio: tca6408a read fail at start addr=0x21; assume 0xFF");
            0xFF
        }
    };
    info!("front.gpio: tca6408a baseline=0x{:02X}", last_inputs);

    let mut pressed = [false; 5];
    for bit in 0..=4u8 {
        pressed[bit as usize] = (last_inputs & (1u8 << bit)) == 0;
    }
    let mut fallback = Ticker::every(Duration::from_millis(FALLBACK_SCAN_MS));

    loop {
        match select(INT_TRIGGER.wait(), fallback.next()).await {
            Either::First(()) => {
                handle_read_and_log(&mut i2c, &mut last_inputs, &mut pressed, true).await;
            }
            Either::Second(()) => {
                handle_read_and_log(&mut i2c, &mut last_inputs, &mut pressed, false).await;
            }
        }
    }
}

async fn tca_read_inputs<I2C: I2c>(i2c: &mut I2C) -> Result<u8, I2C::Error> {
    // TCA6408A Input Port register address = 0x00
    let mut b = [0u8; 1];
    i2c.write_read(TCA6408_ADDR, &[REG_INPUT], &mut b).await?;
    Ok(b[0])
}

async fn tca_write_output<I2C: I2c>(i2c: &mut I2C, output: u8) -> Result<(), I2C::Error> {
    i2c.write(TCA6408_ADDR, &[REG_OUTPUT, output]).await
}

async fn tca_read_inputs_retry<I2C: I2c>(i2c: &mut I2C, attempts: usize) -> Result<u8, I2C::Error> {
    for attempt in 0..attempts {
        match tca_read_inputs(i2c).await {
            Ok(v) => return Ok(v),
            Err(e) if attempt + 1 == attempts => return Err(e),
            Err(_) => Timer::after(Duration::from_millis(TCA_READ_RETRY_DELAY_MS)).await,
        }
    }
    unreachable!()
}

async fn handle_read_and_log<I2C: I2c>(
    i2c: &mut I2C,
    last_inputs: &mut u8,
    pressed: &mut [bool; 5],
    debounce: bool,
) {
    let first = match tca_read_inputs_retry(i2c, 3).await {
        Ok(v) => v,
        Err(_) => {
            warn!("front.gpio: tca6408a read fail addr=0x21");
            return;
        }
    };

    let now = if debounce {
        Timer::after(Duration::from_millis(KEY_DEBOUNCE_MS)).await;
        match tca_read_inputs_retry(i2c, 2).await {
            Ok(v) => v,
            Err(_) => {
                warn!("front.gpio: tca6408a debounce read fail addr=0x21; use edge sample");
                first
            }
        }
    } else {
        first
    };

    let prev = *last_inputs;
    *last_inputs = now;
    let mask_5 = 0x1F; // P0..P4
    let falling = (prev & mask_5) & !(now & mask_5); // 1->0
    let rising = !(prev & mask_5) & (now & mask_5); // 0->1
    if falling != 0 || rising != 0 {
        info!(
            "front.key: change prev=0x{:02X} now=0x{:02X} fall=0x{:02X} rise=0x{:02X}",
            prev, now, falling, rising
        );
    }

    for bit in 0..=4u8 {
        let m = 1u8 << bit;
        let idx = bit as usize;
        let is_low = (now & m) == 0;
        if is_low && !pressed[idx] {
            pressed[idx] = true;
            info!("front.key: fall={}", dir_name(bit));
            if let Some(event) = key_event(bit) {
                if KEY_EVENTS.try_send(event).is_err() {
                    warn!("front.key: event queue full");
                }
            }
        }
        if !is_low && pressed[idx] {
            pressed[idx] = false;
            info!("front.key: rise={}", dir_name(bit));
        }
    }
}

#[inline]
fn key_event(bit: u8) -> Option<KeyEvent> {
    match bit {
        0 => Some(KeyEvent::Center),
        1 => Some(KeyEvent::Right),
        3 => Some(KeyEvent::Left),
        _ => None,
    }
}

#[inline]
fn dir_name(bit: u8) -> &'static str {
    // V3 front-panel netlist mapping:
    // P0=center, P1=right, P2=down, P3=left, P4=up
    match bit {
        0 => "center",
        1 => "right",
        2 => "down",
        3 => "left",
        4 => "up",
        _ => "p?",
    }
}
