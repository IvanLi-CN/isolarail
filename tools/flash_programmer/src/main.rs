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
use w25::{W25, Q, Error};
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
async fn initialize_flash_spi(p: embassy_stm32::Peripherals) -> W25<Q, EmbassySpiDevice<'static, CriticalSectionRawMutex, Stm32Spi<'static, mode::Async>, Output<'static>>, DummyPin, DummyPin> {
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

    let flash = match W25::new(spi_device, hold_pin, wp_pin, 16 * 1024 * 1024) { // 128Mbit = 16MB
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

/// Test checkerboard image data (160x40 RGB565)
const TEST_IMAGE: &[u8] = include_bytes!("../../test_data/checkerboard_160x40.bin");

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

    // Program test checkerboard image to entire Flash
    info!("=== Programming Test Checkerboard Image (16MB) ===");
    let test_image_addr = 0x000000; // Start at beginning of Flash
    info!("Programming test image ({} bytes) to fill 16MB Flash starting at address 0x{:06X}",
          TEST_IMAGE.len(), test_image_addr);

    // First, let's try to read what's currently at address 0x000000
    info!("Reading current data at address 0x000000...");
    let mut read_buffer = [0u8; 32];
    match programmer.read_data(test_image_addr, &mut read_buffer).await {
        Ok(()) => {
            info!("Current data at 0x000000: {:?}", &read_buffer[0..16]);
        }
        Err(_e) => {
            error!("Failed to read current data");
        }
    }

    // Erase entire chip for 16MB programming
    info!("Erasing entire Flash chip (this may take several minutes)...");
    match programmer.erase_chip().await {
        Ok(()) => {
            info!("✓ Flash chip erased successfully");
        }
        Err(e) => {
            error!("✗ Failed to erase Flash chip");
            return Err(e);
        }
    }

    // Calculate how many times to repeat the test image to fill 16MB
    const FLASH_SIZE: u32 = 16 * 1024 * 1024; // 16MB
    let image_size = TEST_IMAGE.len() as u32;
    let repeat_count = FLASH_SIZE / image_size;
    let remaining_bytes = FLASH_SIZE % image_size;

    info!("Will write {} complete images + {} remaining bytes", repeat_count, remaining_bytes);

    // Program the test image repeatedly
    let mut current_address = 0u32;
    for i in 0..repeat_count {
        info!("Programming image {} of {} at address 0x{:08X}", i + 1, repeat_count, current_address);

        match programmer.program_data(current_address, TEST_IMAGE).await {
            Ok(()) => {
                current_address += image_size;
                if (i + 1) % 100 == 0 {
                    info!("  Progress: {}/{} images written", i + 1, repeat_count);
                }
            }
            Err(e) => {
                error!("✗ Failed to program image {} at address 0x{:08X}", i + 1, current_address);
                return Err(e);
            }
        }
    }

    // Program remaining partial image if any
    if remaining_bytes > 0 {
        info!("Programming remaining {} bytes at address 0x{:08X}", remaining_bytes, current_address);
        let partial_image = &TEST_IMAGE[..(remaining_bytes as usize)];
        match programmer.program_data(current_address, partial_image).await {
            Ok(()) => {
                info!("✓ Remaining bytes programmed successfully");
            }
            Err(e) => {
                error!("✗ Failed to program remaining bytes");
                return Err(e);
            }
        }
    }

    info!("✓ Test image programmed successfully to entire 16MB Flash");

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
