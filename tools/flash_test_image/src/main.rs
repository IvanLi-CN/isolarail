//! Flash Test Image Tool
//!
//! This tool flashes a 16MB test image to the external W25Q128 Flash memory

#![no_std]
#![no_main]

extern crate alloc;

use defmt::*;
use embassy_executor::Spawner;
use embedded_alloc::Heap;
use {defmt_rtt as _, panic_probe as _};

use embassy_stm32::{
    bind_interrupts,
    gpio::{Level, Output, Speed},
    peripherals,
    spi::{Config as SpiConfig, Spi},
    time::Hertz,
    Config,
};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice as EmbassySpiDevice;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use static_cell::StaticCell;
use w25q32jv::W25q32jv;

// Dummy pin implementation for Flash WP and HOLD pins
#[derive(Debug, Clone, Copy)]
pub struct DummyPin;

#[derive(Debug)]
pub struct DummyError;

impl defmt::Format for DummyError {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "DummyError");
    }
}

impl embedded_hal::digital::Error for DummyError {
    fn kind(&self) -> embedded_hal::digital::ErrorKind {
        embedded_hal::digital::ErrorKind::Other
    }
}

impl embedded_hal::digital::ErrorType for DummyPin {
    type Error = DummyError;
}

impl embedded_hal::digital::OutputPin for DummyPin {
    fn set_low(&mut self) -> Result<(), Self::Error> { Ok(()) }
    fn set_high(&mut self) -> Result<(), Self::Error> { Ok(()) }
}

impl embedded_hal::digital::ErrorType for DummyPin {
    type Error = DummyError;
}

#[global_allocator]
static HEAP: Heap = Heap::empty();

// Initialize heap
fn init_heap() {
    const HEAP_SIZE: usize = 32768; // 32KB heap for larger operations
    static mut HEAP_MEM: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    unsafe {
        HEAP.init(
            core::ptr::addr_of_mut!(HEAP_MEM) as *mut u8 as usize,
            HEAP_SIZE,
        )
    }
}

// Test image data - 160x40 RGB565 checkerboard pattern
const TEST_IMAGE: &[u8] = include_bytes!("../../test_data/checkerboard_160x40.bin");

bind_interrupts!(struct Irqs {
    SPI2 => embassy_stm32::spi::InterruptHandler<peripherals::SPI2>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    init_heap();

    info!("=== Flash Test Image Programming Tool ===");
    info!("Test image size: {} bytes", TEST_IMAGE.len());

    // Initialize STM32
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hsi = true;
        config.rcc.pll = Some(Pll {
            source: PllSource::HSI,
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL85,
            divp: None,
            divq: None,
            divr: Some(PllRDiv::DIV2),
        });
        config.rcc.sys = Sysclk::PLL1_R;
    }

    let p = embassy_stm32::init(config);

    // Initialize Flash SPI
    let flash = initialize_flash(p).await;

    info!("Hardware initialized successfully");

    // Flash programming parameters
    const FLASH_SIZE: u32 = 16 * 1024 * 1024; // 16MB
    const SECTOR_SIZE: u32 = 4096; // 4KB sectors
    const CHUNK_SIZE: usize = 256; // 256 bytes per write operation
    const IMAGE_SIZE: u32 = TEST_IMAGE.len() as u32;

    info!("Flash parameters:");
    info!("  Total Flash size: {} MB", FLASH_SIZE / (1024 * 1024));
    info!("  Sector size: {} bytes", SECTOR_SIZE);
    info!("  Write chunk size: {} bytes", CHUNK_SIZE);
    info!("  Test image size: {} bytes", IMAGE_SIZE);

    // Calculate how many complete images fit in 16MB
    let images_count = FLASH_SIZE / IMAGE_SIZE;
    let remaining_bytes = FLASH_SIZE % IMAGE_SIZE;

    info!("Will write {} complete images + {} remaining bytes", images_count, remaining_bytes);

    // Erase entire chip first
    info!("Erasing entire Flash chip (this may take several minutes)...");
    match flash.erase_chip_async().await {
        Ok(_) => info!("✓ Flash chip erased successfully"),
        Err(e) => {
            error!("✗ Failed to erase Flash chip: {:?}", e);
            return;
        }
    }

    // Program the Flash with repeated test images
    info!("Programming Flash with test images...");
    let mut current_address = 0u32;
    let mut images_written = 0u32;

    // Write complete images
    for image_num in 0..images_count {
        info!("Writing image {} of {} at address 0x{:08X}", image_num + 1, images_count, current_address);

        // Write image in chunks
        let mut image_offset = 0usize;
        while image_offset < TEST_IMAGE.len() {
            let chunk_size = core::cmp::min(CHUNK_SIZE, TEST_IMAGE.len() - image_offset);
            let chunk = &TEST_IMAGE[image_offset..image_offset + chunk_size];

            match flash.write_async(current_address, chunk).await {
                Ok(_) => {
                    current_address += chunk_size as u32;
                    image_offset += chunk_size;
                }
                Err(e) => {
                    error!("✗ Failed to write chunk at address 0x{:08X}: {:?}", current_address, e);
                    return;
                }
            }
        }

        images_written += 1;

        // Progress update every 100 images
        if (image_num + 1) % 100 == 0 {
            info!("  Progress: {}/{} images written", image_num + 1, images_count);
        }
    }

    // Write remaining partial image if any
    if remaining_bytes > 0 {
        info!("Writing remaining {} bytes at address 0x{:08X}", remaining_bytes, current_address);

        let mut bytes_written = 0u32;
        while bytes_written < remaining_bytes {
            let chunk_size = core::cmp::min(CHUNK_SIZE as u32, remaining_bytes - bytes_written);
            let image_offset = (bytes_written % IMAGE_SIZE) as usize;
            let chunk_end = core::cmp::min(image_offset + chunk_size as usize, TEST_IMAGE.len());
            let chunk = &TEST_IMAGE[image_offset..chunk_end];

            match flash.write_async(current_address, chunk).await {
                Ok(_) => {
                    current_address += chunk.len() as u32;
                    bytes_written += chunk.len() as u32;
                }
                Err(e) => {
                    error!("✗ Failed to write remaining chunk at address 0x{:08X}: {:?}", current_address, e);
                    return;
                }
            }
        }
    }

    info!("✓ Flash programming completed!");
    info!("  Total images written: {}", images_written);
    info!("  Total bytes written: {} MB", current_address / (1024 * 1024));

    // Verify first image
    info!("Verifying first image...");
    let mut verify_buffer = [0u8; 256];
    let mut verification_passed = true;

    for chunk_start in (0..IMAGE_SIZE).step_by(verify_buffer.len()) {
        let chunk_size = core::cmp::min(verify_buffer.len() as u32, IMAGE_SIZE - chunk_start) as usize;
        let verify_slice = &mut verify_buffer[..chunk_size];

        match flash.read_async(chunk_start, verify_slice).await {
            Ok(_) => {
                let expected = &TEST_IMAGE[chunk_start as usize..chunk_start as usize + chunk_size];
                if verify_slice != expected {
                    error!("✗ Verification failed at address 0x{:08X}", chunk_start);
                    verification_passed = false;
                    break;
                }
            }
            Err(e) => {
                error!("✗ Failed to read for verification at address 0x{:08X}: {:?}", chunk_start, e);
                verification_passed = false;
                break;
            }
        }
    }

    if verification_passed {
        info!("✓ First image verification passed!");
    } else {
        error!("✗ First image verification failed!");
    }

    info!("=== Flash Programming Complete ===");
    info!("The Flash now contains a 16MB test pattern with 160x40 RGB565 checkerboard images");
    info!("Each image is {} bytes and repeats throughout the Flash", IMAGE_SIZE);
}

async fn initialize_flash(p: embassy_stm32::Peripherals) -> W25q32jv<EmbassySpiDevice<'static, CriticalSectionRawMutex, Spi<'static, peripherals::SPI2, embassy_stm32::dma::NoDma, embassy_stm32::dma::NoDma>, Output<'static>>, DummyPin, DummyPin> {
    info!("Initializing SPI2 for W25Q128 Flash...");

    // SPI2 pins for W25Q128 Flash
    let sck_pin = p.PB13;   // SPI2_SCK
    let mosi_pin = p.PB15;  // SPI2_MOSI
    let miso_pin = p.PA10;  // SPI2_MISO
    let cs_pin = Output::new(p.PB12, Level::High, Speed::VeryHigh); // SPI2_NSS

    let mut spi_config = SpiConfig::default();
    spi_config.frequency = Hertz(1_000_000); // 1MHz for Flash communication

    info!("Flash SPI Config - Frequency: {} Hz", spi_config.frequency.0);

    let spi_bus = Spi::new(
        p.SPI2,
        sck_pin,
        mosi_pin,
        miso_pin,
        embassy_stm32::dma::NoDma,
        embassy_stm32::dma::NoDma,
        spi_config,
    );

    static SPI_BUS_CELL: StaticCell<
        Mutex<CriticalSectionRawMutex, Spi<'static, peripherals::SPI2, embassy_stm32::dma::NoDma, embassy_stm32::dma::NoDma>>,
    > = StaticCell::new();
    let spi_bus_mutex_ref = SPI_BUS_CELL.init(Mutex::new(spi_bus));

    let spi_device = EmbassySpiDevice::new(spi_bus_mutex_ref, cs_pin);

    let flash = W25q32jv::new(spi_device, DummyPin, DummyPin)
        .expect("Failed to initialize W25Q128 Flash");

    info!("W25Q128 Flash initialized successfully!");
    flash
}
