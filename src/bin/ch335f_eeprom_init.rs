//! One-shot CH335F external EEPROM initializer.
//!
//! This binary is a lab tool for probing and programming the external M24C64
//! image on the current shared CH335F/ESP32-S3 EEPROM bus. The Rev2.3 0 ohm
//! parallel topology is not a reliable production handoff path; future hardware
//! should isolate the EEPROM direction with a switch such as CH442E.

#![no_std]
#![no_main]

use defmt::{error, info, warn};
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{AnyPin, DriveMode, Input, InputConfig, Level, Output, OutputConfig},
    i2c::master::{Config as I2cConfig, I2c},
    main,
    time::Rate,
};
use esp_println as _;

esp_bootloader_esp_idf::esp_app_desc!();

defmt::timestamp!("{=u64} ms", {
    esp_hal::time::Instant::now()
        .duration_since_epoch()
        .as_millis()
});

const PIN_HUB_RESET: u8 = 5;
const PIN_HUB_SDA: u8 = 36;
const PIN_HUB_SCL: u8 = 37;

const EEPROM_ADDR: u8 = 0x50;
const EEPROM_ADDR_SCAN_START: u8 = 0x50;
const EEPROM_ADDR_SCAN_END: u8 = 0x57;
const EEPROM_IMAGE_LEN: usize = 0x100;
const EEPROM_PAGE_SIZE: usize = 32;
const EEPROM_WRITE_CYCLE_MS: u32 = 5;
const EEPROM_ACK_POLL_TRIES: usize = 20;
const HUB_RESET_ASSERT_MS: u32 = 3000;
const HUB_RESET_RELOAD_PULSE_MS: u32 = 3000;
const VENDOR_UTF16_LEN_OFFSET: usize = 0x10;
const VENDOR_UTF16_OFFSET: usize = 0x11;
const PRODUCT_DESCRIPTOR_LEN_OFFSET: usize = 0x3F;
const PRODUCT_UTF16_LEN_OFFSET: usize = 0x40;
const PRODUCT_UTF16_OFFSET: usize = 0x41;
const SERIAL_DESCRIPTOR_LEN_OFFSET: usize = 0x6F;
const SERIAL_UTF16_LEN_OFFSET: usize = 0x70;

const WCH_VID: u16 = 0x1A86;
const CH335F_PID: u16 = 0x8094;
const VENDOR: &[u8] = b"Ivan";
const PRODUCT: &[u8] = b"ISO USB Hub";

#[derive(Clone, Copy, Eq, PartialEq)]
enum RunOutcome {
    Unchanged,
    Written,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum InitError {
    NoEeprom,
    FailedAfterEepromSeen,
}

#[main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    info!("ch335f_eeprom_init.start");
    info!(
        "pins: reset=GPIO{} sda=GPIO{} scl=GPIO{} eeprom=0x50",
        PIN_HUB_RESET, PIN_HUB_SDA, PIN_HUB_SCL
    );

    let reset_cfg = OutputConfig::default().with_drive_mode(DriveMode::OpenDrain);
    let mut hub_reset = Output::new(p.GPIO5, Level::Low, reset_cfg);
    info!("hub.reset: asserted-low before EEPROM access");
    delay.delay_millis(HUB_RESET_ASSERT_MS);

    let mut i2c = I2c::new(
        p.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(100)),
    )
    .unwrap()
    .with_sda(p.GPIO36)
    .with_scl(p.GPIO37);
    info!("i2c.ready: bus=I2C0 sda=GPIO36 scl=GPIO37 freq=100kHz");

    let target = make_eeprom_image();
    let outcome = match run_init(&mut i2c, &delay, &target) {
        Ok(outcome) => {
            let _released_pins = release_i2c_pins(i2c);
            delay.delay_millis(5);
            info!("i2c.release: GPIO36/GPIO37 input high-z");
            outcome
        }
        Err(InitError::FailedAfterEepromSeen) => {
            let _released_pins = release_i2c_pins(i2c);
            error!("ch335f_eeprom_init.failed: EEPROM seen but write/verify failed");
            loop {
                delay.delay_millis(1000);
            }
        }
        Err(InitError::NoEeprom) => {
            {
                let _released_pins = release_i2c_pins(i2c);
            }
            warn!("i2c.retry: normal mapping failed; trying swapped SDA/SCL on same pins");

            let mut swapped_i2c = I2c::new(
                p.I2C1,
                I2cConfig::default().with_frequency(Rate::from_khz(100)),
            )
            .unwrap()
            .with_sda(unsafe { AnyPin::steal(PIN_HUB_SCL) })
            .with_scl(unsafe { AnyPin::steal(PIN_HUB_SDA) });
            info!("i2c.ready: bus=I2C1 sda=GPIO37 scl=GPIO36 freq=100kHz");

            match run_init(&mut swapped_i2c, &delay, &target) {
                Ok(outcome) => {
                    let _released_pins = release_i2c_pins(swapped_i2c);
                    delay.delay_millis(5);
                    info!("i2c.release: GPIO36/GPIO37 input high-z");
                    outcome
                }
                Err(InitError::NoEeprom) => {
                    let _released_pins = release_i2c_pins(swapped_i2c);
                    hub_reset.set_high();
                    warn!("hub.reset: released after no EEPROM ACK; no write attempted");
                    report_no_eeprom_forever(&delay);
                }
                Err(InitError::FailedAfterEepromSeen) => {
                    let _released_pins = release_i2c_pins(swapped_i2c);
                    error!("ch335f_eeprom_init.failed: EEPROM seen but write/verify failed");
                    loop {
                        delay.delay_millis(1000);
                    }
                }
            }
        }
    };

    match outcome {
        RunOutcome::Unchanged => info!("eeprom.result: already matched"),
        RunOutcome::Written => info!("eeprom.result: updated and verified"),
    }
    info!("hub.reset: pulse low to reload CH335F descriptors");
    hub_reset.set_low();
    delay.delay_millis(HUB_RESET_RELOAD_PULSE_MS);
    hub_reset.set_high();
    delay.delay_millis(20);
    info!("hub.reset: released after reload pulse");
    info!("ch335f_eeprom_init.done");

    report_success_forever(&delay, outcome);
}

fn run_init<I2C>(
    i2c: &mut I2C,
    delay: &Delay,
    target: &[u8; EEPROM_IMAGE_LEN],
) -> Result<RunOutcome, InitError>
where
    I2C: embedded_hal::i2c::I2c,
{
    if !scan_eeprom_window(i2c) {
        return Err(InitError::NoEeprom);
    }

    let mut current = [0u8; EEPROM_IMAGE_LEN];
    eeprom_read(i2c, 0, &mut current).map_err(|_| InitError::NoEeprom)?;
    info!("eeprom.read: addr=0x50 len=256 ok");

    if current == *target {
        info!("eeprom.compare: match; write skipped");
        return Ok(RunOutcome::Unchanged);
    }

    warn!("eeprom.compare: mismatch; writing target image");
    for offset in (0..EEPROM_IMAGE_LEN).step_by(EEPROM_PAGE_SIZE) {
        let end = offset + EEPROM_PAGE_SIZE;
        if current[offset..end] == target[offset..end] {
            continue;
        }
        eeprom_page_write(i2c, offset as u16, &target[offset..end])
            .map_err(|_| InitError::FailedAfterEepromSeen)?;
        eeprom_ack_poll(i2c, delay, offset as u16).map_err(|_| InitError::FailedAfterEepromSeen)?;
        info!("eeprom.write: page_offset=0x{:02x} len=32 ok", offset);
    }

    let mut verify = [0u8; EEPROM_IMAGE_LEN];
    eeprom_read(i2c, 0, &mut verify).map_err(|_| InitError::FailedAfterEepromSeen)?;
    if verify != *target {
        error!("eeprom.verify: readback mismatch");
        return Err(InitError::FailedAfterEepromSeen);
    }

    info!("eeprom.verify: readback match");
    Ok(RunOutcome::Written)
}

fn scan_eeprom_window<I2C>(i2c: &mut I2C) -> bool
where
    I2C: embedded_hal::i2c::I2c,
{
    let mut found_expected = false;
    for addr in EEPROM_ADDR_SCAN_START..=EEPROM_ADDR_SCAN_END {
        let mut sample = [0u8; 1];
        if embedded_hal::i2c::I2c::write_read(i2c, addr, &[0x00, 0x00], &mut sample).is_ok() {
            info!("i2c.scan: addr=0x{:02x} sample0=0x{:02x}", addr, sample[0]);
            if addr == EEPROM_ADDR {
                found_expected = true;
            }
        }
    }

    if !found_expected {
        error!("i2c.scan: expected eeprom 0x50 not detected");
    }

    found_expected
}

fn make_eeprom_image() -> [u8; EEPROM_IMAGE_LEN] {
    let mut image = [0xFFu8; EEPROM_IMAGE_LEN];
    let vid_l = WCH_VID as u8;
    let vid_h = (WCH_VID >> 8) as u8;
    let pid_l = CH335F_PID as u8;
    let pid_h = (CH335F_PID >> 8) as u8;

    image[0x00] = vid_l;
    image[0x01] = vid_h;
    image[0x02] = pid_l;
    image[0x03] = pid_h;
    image[0x04] = vid_h
        .wrapping_add(vid_l)
        .wrapping_add(pid_l)
        .wrapping_add(pid_h)
        .wrapping_add(1);
    image[0x05] = 0xFF;
    image[0x06] = 0x00;
    image[0x07] = 0x04;
    image[0x08] = 0x32;
    image[0x09] = 0x5A;
    image[0x0A] = 0x57;
    image[0x0B..=0x0F].fill(0xFF);

    let vendor_utf16_len = VENDOR.len() * 2;
    assert!(vendor_utf16_len <= 0x2F);
    image[VENDOR_UTF16_LEN_OFFSET] = vendor_utf16_len as u8;
    for (idx, ch) in VENDOR.iter().copied().enumerate() {
        image[VENDOR_UTF16_OFFSET + idx * 2] = ch;
        image[VENDOR_UTF16_OFFSET + idx * 2 + 1] = 0x00;
    }

    let product_utf16_len = PRODUCT.len() * 2;
    assert!(product_utf16_len <= 0x1E);
    // CH334/CH335 V2.4 table 3-5-1 uses 0x3F for "Prod Len+2",
    // 0x40 for "Prod Len", and starts Product String at 0x41.
    image[PRODUCT_DESCRIPTOR_LEN_OFFSET] = (product_utf16_len + 2) as u8;
    image[PRODUCT_UTF16_LEN_OFFSET] = product_utf16_len as u8;
    for (idx, ch) in PRODUCT.iter().copied().enumerate() {
        image[PRODUCT_UTF16_OFFSET + idx * 2] = ch;
        image[PRODUCT_UTF16_OFFSET + idx * 2 + 1] = 0x00;
    }

    image[SERIAL_DESCRIPTOR_LEN_OFFSET] = 0x02;
    image[SERIAL_UTF16_LEN_OFFSET] = 0x00;

    image
}

fn eeprom_read<I2C>(i2c: &mut I2C, address: u16, buf: &mut [u8]) -> Result<(), ()>
where
    I2C: embedded_hal::i2c::I2c,
{
    let mut offset = 0usize;
    while offset < buf.len() {
        let chunk_len = core::cmp::min(32, buf.len() - offset);
        let addr = address + offset as u16;
        let addr_bytes = [(addr >> 8) as u8, addr as u8];
        embedded_hal::i2c::I2c::write_read(
            i2c,
            EEPROM_ADDR,
            &addr_bytes,
            &mut buf[offset..offset + chunk_len],
        )
        .map_err(|_| {
            error!("eeprom.read: failed at offset=0x{:02x}", offset);
        })?;
        offset += chunk_len;
    }
    Ok(())
}

fn eeprom_page_write<I2C>(i2c: &mut I2C, address: u16, data: &[u8]) -> Result<(), ()>
where
    I2C: embedded_hal::i2c::I2c,
{
    let mut packet = [0u8; EEPROM_PAGE_SIZE + 2];
    packet[0] = (address >> 8) as u8;
    packet[1] = address as u8;
    packet[2..2 + data.len()].copy_from_slice(data);
    embedded_hal::i2c::I2c::write(i2c, EEPROM_ADDR, &packet[..2 + data.len()]).map_err(|_| {
        error!("eeprom.write: failed at offset=0x{:02x}", address);
    })
}

fn eeprom_ack_poll<I2C>(i2c: &mut I2C, delay: &Delay, address: u16) -> Result<(), ()>
where
    I2C: embedded_hal::i2c::I2c,
{
    let addr_bytes = [(address >> 8) as u8, address as u8];
    for _ in 0..EEPROM_ACK_POLL_TRIES {
        delay.delay_millis(EEPROM_WRITE_CYCLE_MS);
        if embedded_hal::i2c::I2c::write(i2c, EEPROM_ADDR, &addr_bytes).is_ok() {
            return Ok(());
        }
    }
    error!(
        "eeprom.write: ack polling timed out at offset=0x{:02x}",
        address
    );
    Err(())
}

struct ReleasedI2cPins {
    _sda: Input<'static>,
    _scl: Input<'static>,
}

fn release_i2c_pins(i2c: I2c<'_, esp_hal::Blocking>) -> ReleasedI2cPins {
    drop(i2c);

    ReleasedI2cPins {
        _sda: Input::new(
            unsafe { AnyPin::steal(PIN_HUB_SDA) },
            InputConfig::default(),
        ),
        _scl: Input::new(
            unsafe { AnyPin::steal(PIN_HUB_SCL) },
            InputConfig::default(),
        ),
    }
}

fn report_success_forever(delay: &Delay, outcome: RunOutcome) -> ! {
    loop {
        delay.delay_millis(2000);
        match outcome {
            RunOutcome::Unchanged => info!("ch335f_eeprom_init.status: done unchanged"),
            RunOutcome::Written => info!("ch335f_eeprom_init.status: done written verified"),
        }
    }
}

fn report_no_eeprom_forever(delay: &Delay) -> ! {
    loop {
        delay.delay_millis(2000);
        warn!("ch335f_eeprom_init.status: no EEPROM ACK; no write attempted");
    }
}
