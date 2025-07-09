#![no_std]
#![no_main]

mod programmer;


use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::spi::{Config as SpiConfig, Spi as Stm32Spi};
use embassy_stm32::time::Hertz;
use embassy_stm32::{bind_interrupts, mode};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice as EmbassySpiDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
// use embedded_alloc::Heap;
use static_cell::StaticCell;
use w25q32jv::{W25q32jv, Error};
use programmer::FlashProgrammer;
use {defmt_rtt as _, panic_probe as _};

// #[global_allocator]
// static HEAP: Heap = Heap::empty();

bind_interrupts!(struct Irqs {
    // SPI2 => embassy_stm32::spi::InterruptHandler<peripherals::SPI2>;
});

/// Configure STM32 system
fn configure_stm32() -> embassy_stm32::Config {
    let mut config = embassy_stm32::Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hsi48 = Some(Hsi48Config {
            sync_from_usb: true,
        });
        config.rcc.pll = Some(Pll {
            source: PllSource::HSI,
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL85,
            divp: None,
            divq: None,
            // Main system clock at 170 MHz
            divr: Some(PllRDiv::DIV2),
        });
        config.rcc.mux.adc12sel = mux::Adcsel::SYS;
        config.rcc.sys = Sysclk::PLL1_R;
        config.rcc.mux.clk48sel = mux::Clk48sel::HSI48;
    }
    config
}

use crate::programmer::DummyPin;

/// Initialize SPI2 for W25Q128 Flash communication
async fn initialize_flash_spi(p: embassy_stm32::Peripherals) -> W25q32jv<EmbassySpiDevice<'static, CriticalSectionRawMutex, Stm32Spi<'static, mode::Async>, Output<'static>>, DummyPin, DummyPin> {
    info!("Initializing SPI2 for W25Q128 Flash...");

    // SPI2 pins for W25Q128 Flash
    let sck_pin = p.PB13;   // SPI2_SCK
    let mosi_pin = p.PB15;  // SPI2_MOSI
    let miso_pin = p.PA10;  // SPI2_MISO
    let cs_pin_output = Output::new(p.PB12, Level::High, Speed::VeryHigh); // SPI2_NSS

    // Use dummy pins for WP and HOLD (they're not connected or not needed for basic operation)
    let wp_pin = DummyPin;
    let hold_pin = DummyPin;

    let mut spi_config = SpiConfig::default();
    spi_config.frequency = Hertz(1_000_000); // 1MHz for Flash communication (reduced for debugging)

    // W25Q128 requires SPI Mode 0 (CPOL=0, CPHA=0)
    // Default SPI config should be Mode 0, which is what we need
    info!("SPI Config - Frequency: {} Hz", spi_config.frequency.0);

    let spi_bus = Stm32Spi::new(
        p.SPI2,
        sck_pin,
        mosi_pin,
        miso_pin,
        p.DMA1_CH4, // TX DMA
        p.DMA1_CH5, // RX DMA
        spi_config,
    );

    static SPI_BUS_CELL: StaticCell<
        Mutex<CriticalSectionRawMutex, Stm32Spi<'static, mode::Async>>,
    > = StaticCell::new();
    let spi_bus_mutex_ref = SPI_BUS_CELL.init(Mutex::new(spi_bus));

    let spi_device = EmbassySpiDevice::<
        'static,
        CriticalSectionRawMutex,
        Stm32Spi<'static, mode::Async>,
        Output<'static>,
    >::new(spi_bus_mutex_ref, cs_pin_output);

    let flash = match W25q32jv::new(spi_device, hold_pin, wp_pin) {
        Ok(flash) => {
            info!("W25Q128 Flash initialized successfully!");
            flash
        }
        Err(e) => {
            error!("Failed to initialize W25Q128 Flash: {:?}", e);
            core::panic!("Flash initialization failed");
        }
    };

    flash
}

/// Startup bitmap data (140x40 RGB565 with header)
const STARTUP_BITMAP: &[u8] = include_bytes!("../../screenshot_140x40.bin");

/// Demonstrate Flash programming operations
async fn demo_flash_operations<SPI>(flash: W25q32jv<SPI, DummyPin, DummyPin>) -> Result<(), Error<SPI::Error, crate::programmer::DummyError>>
where
    SPI: embedded_hal_async::spi::SpiDevice,
    SPI::Error: core::fmt::Debug,
{
    let mut programmer = FlashProgrammer::new(flash);

    // Get and display device information
    info!("Reading device information...");
    let device_info = programmer.get_device_info().await?;
    device_info.print_info();

    // Program startup bitmap to startup bitmap area
    info!("=== Programming Startup Bitmap (140x40) ===");
    let startup_bitmap_addr = 0x000000; // Startup bitmap area starts at 0x000000
    info!("Programming startup bitmap ({} bytes) to address 0x{:06X}",
          STARTUP_BITMAP.len(), startup_bitmap_addr);

    // First, let's try to read what's currently at address 0x000000
    info!("Reading current data at address 0x000000...");
    let mut read_buffer = [0u8; 32];
    match programmer.read_data(startup_bitmap_addr, &mut read_buffer).await {
        Ok(()) => {
            info!("Current data at 0x000000: {:?}", &read_buffer[0..16]);
        }
        Err(_e) => {
            error!("Failed to read current data");
        }
    }

    // Note: w25q32jv crate handles write protection internally
    info!("Proceeding with Flash programming (w25q32jv handles protection internally)...");

    // First, let's try a 16-byte test at address 0x0000
    info!("=== Testing 16-Byte Write at Address 0x0000 ===");
    let test_address = 0x0000u32; // Start at the very beginning
    let test_pattern: [u8; 16] = [0xAA, 0x55, 0xCC, 0x33, 0xAA, 0x55, 0xCC, 0x33,
                                  0xAA, 0x55, 0xCC, 0x33, 0xAA, 0x55, 0xCC, 0x33];
    info!("Programming 16 bytes: {:?} at address 0x{:06X}", test_pattern, test_address);

    match programmer.program_and_verify(test_address, &test_pattern).await {
        Ok(()) => {
            info!("✓ Test pattern programmed and verified successfully");
            info!("Now trying full bitmap...");

            // Now try the full bitmap
            match programmer.program_and_verify(startup_bitmap_addr, STARTUP_BITMAP).await {
                Ok(()) => {
                    info!("✓ Startup bitmap (140x40) programmed and verified successfully");
                }
                Err(_e) => {
                    error!("Full bitmap programming failed");
                }
            }
        }
        Err(_e) => {
            error!("Test pattern programming failed");

            // Let's read back what was actually written
            info!("Reading back what was actually written...");
            let mut read_buffer = [0u8; 32];
            match programmer.read_data(test_address, &mut read_buffer).await {
                Ok(()) => {
                    info!("Actual data written: {:?}", &read_buffer[0..16]);
                    info!("Expected data:      {:?}", &test_pattern);
                }
                Err(_e) => {
                    error!("Failed to read back data");
                }
            }
        }
    }

    // Example 1: Program test data (commented out for debugging)
    // let test_data = include_bytes!("../test_data.bin");
    // let program_address = 0x100000; // Start at 1MB offset

    // info!("=== Programming Test Data ===");
    // programmer.program_and_verify(program_address, test_data).await?;

    // Example 2: Read back some data and dump it (commented out for debugging)
    // info!("=== Reading Back Data ===");
    // programmer.dump_flash(program_address, 256).await?;

    // Example 3: Program a pattern (commented out for debugging)
    // let pattern_data: [u8; 1024] = core::array::from_fn(|i| (i % 256) as u8);
    // let pattern_address = 0x200000; // Start at 2MB offset

    // info!("=== Programming Pattern Data ===");
    // programmer.program_and_verify(pattern_address, &pattern_data).await?;

    // Example 4: Erase a specific sector (commented out for debugging)
    // info!("=== Erasing Sector ===");
    // let sector_to_erase = 0x300000 / w25q128::constants::SECTOR_SIZE; // Sector at 3MB
    // programmer.erase_sector(sector_to_erase).await?;

    info!("Flash programmer finished. System will halt.");
    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting W25Q128 Flash Programmer");

    // Initialize the allocator (disabled for now)
    // {
    //     use core::mem::MaybeUninit;
    //     const HEAP_SIZE: usize = 8192;
    //     static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    //     unsafe { HEAP.init(ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }
    // }

    // Configure STM32 system
    let config = configure_stm32();
    let p = embassy_stm32::init(config);

    // Initialize Flash
    let flash = initialize_flash_spi(p).await;

    // Run Flash programming demonstration
    match demo_flash_operations(flash).await {
        Ok(_) => {
            info!("Flash programming demonstration completed successfully!");
        }
        Err(e) => {
            error!("Flash programming demonstration failed: {:?}", e);
        }
    }

    info!("Flash programmer finished. System will halt.");
    
    // Halt the system
    loop {
        cortex_m::asm::wfi();
    }
}
