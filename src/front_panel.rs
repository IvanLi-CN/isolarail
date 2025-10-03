use defmt::{info, warn};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::{task, SpawnError, Spawner};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use embedded_hal::digital::InputPin as _;
use embedded_hal_async::i2c::I2c;
use esp_hal::gpio::Input;

use crate::{I2cBus, TCA6408_ADDR};

pub fn spawn(
    spawner: &Spawner,
    bus: &'static Mutex<CriticalSectionRawMutex, I2cBus>,
    int_pin: Input<'static>,
) -> Result<(), SpawnError> {
    spawner.spawn(task(bus, int_pin))
}

#[task]
async fn task(bus: &'static Mutex<CriticalSectionRawMutex, I2cBus>, mut int_pin: Input<'static>) {
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

    // Track last INT level to detect falling edges even if we poll.
    let mut last_int_high = int_pin.is_high();

    loop {
        let now_high = int_pin.is_high();
        // Detect falling edge on INT (high -> low)
        if last_int_high && !now_high {
            // On INT assertion, read inputs to clear the interrupt latch, then compare.
            match tca_read_inputs(&mut i2c).await {
                Ok(now) => {
                    let prev = last_inputs;
                    last_inputs = now;
                    // Interested only in P0..P4 falling edges: 1 -> 0
                    let mask_5 = 0x1F;
                    let falling = (prev & mask_5) & !(now & mask_5);
                    if falling != 0 {
                        info!(
                            "front.key: fall mask=0x{:02X} prev=0x{:02X} now=0x{:02X}",
                            falling, prev, now
                        );
                        for bit in 0..=4u8 {
                            if (falling & (1u8 << bit)) != 0 {
                                info!("front.key: fall=P{}", bit);
                            }
                        }
                    } else {
                        // Change occurred but no falling edges on P0..P4
                        info!("front.key: change prev=0x{:02X} now=0x{:02X}", prev, now);
                    }
                }
                Err(_) => {
                    warn!("front.gpio: tca6408a read fail on INT addr=0x21");
                }
            }
        }
        last_int_high = now_high;

        // Debounce/polling interval
        Timer::after(Duration::from_millis(2)).await;
    }
}

async fn tca_read_inputs<I2C: I2c>(i2c: &mut I2C) -> Result<u8, I2C::Error> {
    // TCA6408A Input Port register address = 0x00
    let mut b = [0u8; 1];
    i2c.write_read(TCA6408_ADDR, &[0x00], &mut b).await?;
    Ok(b[0])
}
