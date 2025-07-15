#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embedded_alloc::LlffHeap as Heap;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::WebColors;
use {defmt_rtt as _, panic_probe as _};

#[path = "../src/hardware.rs"]
mod hardware;

#[global_allocator]
static HEAP: Heap = Heap::empty();

// This is a simple Flash verification program that only reads from Flash
// and displays basic patterns on screen to verify functionality

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Initialize the heap with smaller size
    use core::mem::MaybeUninit;
    const HEAP_SIZE: usize = 8192; // 8KB heap
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe {
        let heap_ptr = core::ptr::addr_of_mut!(HEAP_MEM);
        HEAP.init(heap_ptr as *mut u8 as usize, HEAP_SIZE)
    }

    info!("=== Simple Flash Verification Test ===");

    // Initialize hardware
    let p = embassy_stm32::init(Default::default());
    let mut hardware = hardware::initialize_hardware(p).await;
    info!("Hardware initialized successfully");

    // Test Flash reading
    info!("=== Testing Flash Read ===");
    let mut test_buffer = [0u8; 64];

    match hardware.flash.read_async(0x000000, &mut test_buffer).await {
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
            } else {
                info!("Flash data doesn't match expected pattern");
            }
        }
        Err(e) => {
            error!("✗ Flash read failed: {:?}", e);
        }
    }

    // Test display with simple patterns
    info!("=== Testing Display ===");

    // Create simple test patterns
    let colors = [
        Rgb565::CSS_RED,
        Rgb565::CSS_YELLOW,
        Rgb565::CSS_GREEN,
        Rgb565::CSS_CYAN,
        Rgb565::CSS_BLUE,
        Rgb565::CSS_PURPLE,
        Rgb565::CSS_WHITE,
        Rgb565::CSS_MAGENTA,
    ];

    // Display color stripes (each 5 rows high)
    for (color_index, &color) in colors.iter().enumerate() {
        let start_row = color_index * 5;
        let end_row = (start_row + 5).min(40);

        for row in start_row..end_row {
            let color_row = [color; 160];
            hardware
                .display
                .write_area(0, row as u16, 160, 1, &color_row);
        }

        info!(
            "Drew color stripe {} at rows {}-{}",
            color_index,
            start_row,
            end_row - 1
        );
    }

    // Flush display
    match hardware.display.flush().await {
        Ok(_) => {
            info!("✓ Display flush successful - color stripes should be visible");
        }
        Err(e) => {
            error!("✗ Display flush failed: {:?}", e);
        }
    }

    info!("=== Test Complete ===");
    info!(
        "Check the display for 8 color stripes (red, yellow, green, cyan, blue, purple, white, magenta)"
    );

    // Keep running and show periodic status
    let mut counter = 0;
    loop {
        embassy_time::Timer::after_millis(5000).await;
        counter += 1;
        info!(
            "Test running... {} (display should show color stripes)",
            counter
        );

        // Every 10 iterations, try reading Flash again
        if counter % 10 == 0 {
            let mut verify_buffer = [0u8; 16];
            match hardware
                .flash
                .read_async(0x000000, &mut verify_buffer)
                .await
            {
                Ok(_) => {
                    info!("✓ Flash still readable: {:?}", &verify_buffer[0..8]);
                }
                Err(e) => {
                    error!("✗ Flash read error: {:?}", e);
                }
            }
        }
    }
}
