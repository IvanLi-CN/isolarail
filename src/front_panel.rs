use defmt::{info, warn};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::{task, SpawnError, Spawner};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver};
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
// not using InputPin trait directly; rely on esp-hal Input::is_high()
use embedded_hal_async::i2c::I2c;
use esp_hal::gpio::Input;

use crate::{I2cBus, TCA6408_ADDR};

const KEY_DEBOUNCE_TICKS: u8 = 8; // 8 * 5 ms polling cadence = ~40 ms

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum KeyEvent {
    Left,
    Right,
    Center,
}

static KEY_EVENTS: Channel<CriticalSectionRawMutex, KeyEvent, 8> = Channel::new();

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
    embedded_hal_async::i2c::I2c::write_read(&mut i2c, TCA6408_ADDR, &[0x00], &mut b)
        .await
        .is_ok()
}

pub fn spawn(
    spawner: &Spawner,
    bus: &'static Mutex<CriticalSectionRawMutex, I2cBus>,
    int_pin: Input<'static>,
) -> Result<(), SpawnError> {
    spawner.spawn(task(bus, int_pin))
}

#[task]
async fn task(bus: &'static Mutex<CriticalSectionRawMutex, I2cBus>, int_pin: Input<'static>) {
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

    // Track last INT level（若该引脚未接线，后续轮询亦可识别按键变化）。
    let mut last_int_high = int_pin.is_high();
    let mut debounce_ticks = [0u8; 5];

    loop {
        let now_high = int_pin.is_high();
        let mut handled = false;

        // 快速路径：检测到 INT 变化时打印电平，并在下降沿优先读取
        if last_int_high && !now_high {
            handled = true;
            handle_read_and_log(&mut i2c, &mut last_inputs, &mut debounce_ticks).await;
            info!("front.gpio: int=low");
        } else if !last_int_high && now_high {
            info!("front.gpio: int=high");
        }
        last_int_high = now_high;

        // 保障路径：定期轮询，避免 INT 未接线时漏报
        if !handled {
            handle_read_and_log(&mut i2c, &mut last_inputs, &mut debounce_ticks).await;
        }

        // 轻微去抖/限流
        Timer::after(Duration::from_millis(5)).await;
    }
}

async fn tca_read_inputs<I2C: I2c>(i2c: &mut I2C) -> Result<u8, I2C::Error> {
    // TCA6408A Input Port register address = 0x00
    let mut b = [0u8; 1];
    i2c.write_read(TCA6408_ADDR, &[0x00], &mut b).await?;
    Ok(b[0])
}

async fn handle_read_and_log<I2C: I2c>(
    i2c: &mut I2C,
    last_inputs: &mut u8,
    debounce_ticks: &mut [u8; 5],
) {
    for ticks in debounce_ticks.iter_mut() {
        *ticks = ticks.saturating_sub(1);
    }
    match tca_read_inputs(i2c).await {
        Ok(now) => {
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
                for bit in 0..=4u8 {
                    let m = 1u8 << bit;
                    if (falling & m) != 0 {
                        info!("front.key: fall={}", dir_name(bit));
                        let idx = bit as usize;
                        if let Some(event) = key_event(bit).filter(|_| debounce_ticks[idx] == 0) {
                            debounce_ticks[idx] = KEY_DEBOUNCE_TICKS;
                            if KEY_EVENTS.try_send(event).is_err() {
                                warn!("front.key: event queue full");
                            }
                        }
                    }
                    if (rising & m) != 0 {
                        info!("front.key: rise={}", dir_name(bit));
                    }
                }
            }
        }
        Err(_) => {
            warn!("front.gpio: tca6408a read fail addr=0x21");
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
