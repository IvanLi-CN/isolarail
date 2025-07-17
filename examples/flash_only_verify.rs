#![no_std]
#![no_main]

use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice as EmbassySpiDevice;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    spi::{Config as SpiConfig, Spi as Stm32Spi},
    time::Hertz,
};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_alloc::LlffHeap as Heap;
use static_cell::StaticCell;
use w25::{Q, W25};
use {defmt_rtt as _, panic_probe as _};

// Include hardware module for DummyPin
#[path = "../src/hardware.rs"]
mod hardware;
use hardware::DummyPin;

#[global_allocator]
static HEAP: Heap = Heap::empty();

// This is a minimal Flash-only verification program
// It tests Flash reading without using the hardware module

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Initialize the heap with minimal size
    use core::mem::MaybeUninit;
    const HEAP_SIZE: usize = 4096; // 4KB heap
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe {
        let heap_ptr = core::ptr::addr_of_mut!(HEAP_MEM);
        HEAP.init(heap_ptr as *mut u8 as usize, HEAP_SIZE)
    }

    info!("=== Flash-Only Verification Test ===");

    // Initialize STM32 peripherals
    let p = embassy_stm32::init(Default::default());

    // Initialize Flash SPI
    info!("Initializing Flash SPI...");

    let flash_sck_pin = p.PB13; // SPI2_SCK
    let flash_mosi_pin = p.PB15; // SPI2_MOSI
    let flash_miso_pin = p.PA10; // SPI2_MISO
    let flash_cs_pin = Output::new(p.PB12, Level::High, Speed::VeryHigh); // SPI2_NSS

    let mut flash_spi_config = SpiConfig::default();
    flash_spi_config.frequency = Hertz(1_000_000); // 1MHz for Flash communication

    info!(
        "Flash SPI Config - Frequency: {} Hz",
        flash_spi_config.frequency.0
    );

    let flash_spi_bus = Stm32Spi::new(
        p.SPI2,
        flash_sck_pin,
        flash_mosi_pin,
        flash_miso_pin,
        p.DMA1_CH1,
        p.DMA1_CH2,
        flash_spi_config,
    );

    // Create SPI bus mutex
    static FLASH_SPI_BUS_CELL: StaticCell<
        Mutex<CriticalSectionRawMutex, Stm32Spi<'static, embassy_stm32::mode::Async>>,
    > = StaticCell::new();
    let flash_spi_bus_mutex_ref = FLASH_SPI_BUS_CELL.init(Mutex::new(flash_spi_bus));

    // Create SPI device
    let flash_spi_device = EmbassySpiDevice::<
        'static,
        CriticalSectionRawMutex,
        Stm32Spi<'static, embassy_stm32::mode::Async>,
        Output<'static>,
    >::new(flash_spi_bus_mutex_ref, flash_cs_pin);

    // Initialize W25Q128 Flash driver
    let mut flash = W25::<Q, _, _, _>::new(flash_spi_device, DummyPin, DummyPin, 16 * 1024 * 1024)
        .expect("Failed to initialize Flash");

    info!("Flash driver initialized");

    // Test Flash reading
    info!("=== Testing Flash Read ===");
    let mut test_buffer = [0u8; 64];

    match flash.read(0x000000, &mut test_buffer).await {
        Ok(_) => {
            info!("✓ Flash read successful!");
            info!("First 32 bytes: {:?}", &test_buffer[0..32]);
            info!("Next 32 bytes: {:?}", &test_buffer[32..64]);

            // Check for expected checkerboard pattern
            let expected_red = [0x00, 0xF8]; // RGB565 red in little-endian
            let expected_yellow = [0xE0, 0x07]; // RGB565 yellow in little-endian

            let mut red_count = 0;
            let mut yellow_count = 0;

            for chunk in test_buffer.chunks_exact(2) {
                if chunk == expected_red {
                    red_count += 1;
                } else if chunk == expected_yellow {
                    yellow_count += 1;
                }
            }

            info!(
                "Found {} red pixels and {} yellow pixels",
                red_count, yellow_count
            );

            if red_count > 0 && yellow_count > 0 {
                info!("✓ Flash contains expected checkerboard pattern!");
            } else if red_count > 0 || yellow_count > 0 {
                info!("✓ Flash contains some expected pattern data");
            } else {
                info!("Flash data doesn't match expected pattern");
            }
        }
        Err(e) => {
            error!("✗ Flash read failed: {:?}", e);
        }
    }

    // Test reading multiple locations
    info!("=== Testing Multiple Flash Locations ===");

    let test_addresses = [0x000000, 0x003200, 0x100000, 0x800000];

    for (i, &address) in test_addresses.iter().enumerate() {
        let mut location_buffer = [0u8; 16];
        match flash.read(address, &mut location_buffer).await {
            Ok(_) => {
                info!(
                    "✓ Location {} (0x{:06X}): {:?}",
                    i + 1,
                    address,
                    &location_buffer[0..8]
                );

                // Check for pattern
                let mut pattern_count = 0;
                for chunk in location_buffer.chunks_exact(2) {
                    if chunk == [0x00, 0xF8] || chunk == [0xE0, 0x07] {
                        pattern_count += 1;
                    }
                }
                info!("  Pattern pixels found: {}", pattern_count);
            }
            Err(e) => {
                error!(
                    "✗ Location {} (0x{:06X}) read failed: {:?}",
                    i + 1,
                    address,
                    e
                );
            }
        }
    }

    info!("=== Flash Verification Complete ===");
}
