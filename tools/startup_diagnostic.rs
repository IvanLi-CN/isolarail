#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

/// Simple startup diagnostic tool to test basic MCU functionality
/// This tool helps diagnose power-on issues by testing core components
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("=== ISO USB Hub Startup Diagnostic Tool ===");
    info!("This tool tests basic MCU functionality after power-on");

    // Test 1: Basic system initialization
    info!("Test 1: Basic system initialization");
    let config = embassy_stm32::Config::default();
    let p = embassy_stm32::init(config);
    info!("✓ STM32 system initialized successfully");

    // Test 2: GPIO functionality
    info!("Test 2: GPIO functionality test");
    let mut led_pin = Output::new(p.PC13, Level::Low, Speed::Low); // Assuming PC13 is available

    for i in 1..=5 {
        info!("GPIO test blink {}/5", i);
        led_pin.set_high();
        Timer::after_millis(200).await;
        led_pin.set_low();
        Timer::after_millis(200).await;
    }
    info!("✓ GPIO functionality test completed");

    // Test 3: Timer functionality
    info!("Test 3: Timer functionality test");
    let start_time = embassy_time::Instant::now();
    Timer::after_millis(1000).await;
    let elapsed = embassy_time::Instant::now() - start_time;
    info!(
        "Timer test: Expected 1000ms, Actual {}ms",
        elapsed.as_millis()
    );
    info!("✓ Timer functionality test completed");

    // Test 4: Memory allocation test
    info!("Test 4: Memory allocation test");
    {
        // Test stack allocation
        let test_array: [u8; 256] = [0x55; 256];
        let checksum: u32 = test_array.iter().map(|&x| x as u32).sum();
        info!(
            "Stack allocation test: checksum = {} (expected 13824)",
            checksum
        );

        if checksum == 13824 {
            info!("✓ Memory allocation test passed");
        } else {
            error!("✗ Memory allocation test failed");
        }
    }

    // Test 5: Clock stability test
    info!("Test 5: Clock stability test (10 second duration)");
    let mut tick_count = 0u32;
    let test_start = embassy_time::Instant::now();

    while (embassy_time::Instant::now() - test_start).as_millis() < 10000 {
        Timer::after_millis(100).await;
        tick_count += 1;
        if tick_count % 10 == 0 {
            info!("Clock stability: {}s elapsed", tick_count / 10);
        }
    }

    let actual_duration = embassy_time::Instant::now() - test_start;
    info!(
        "Clock stability test: Expected 10000ms, Actual {}ms",
        actual_duration.as_millis()
    );

    if actual_duration.as_millis() >= 9900 && actual_duration.as_millis() <= 10100 {
        info!("✓ Clock stability test passed");
    } else {
        error!("✗ Clock stability test failed - clock drift detected");
    }

    // Final report
    info!("=== Diagnostic Summary ===");
    info!("All basic functionality tests completed");
    info!("If this diagnostic runs successfully, the MCU hardware is working");
    info!("If the main application still fails, the issue is likely in:");
    info!("  1. I2C communication with external devices");
    info!("  2. Power supply to external components");
    info!("  3. Complex initialization sequences");
    info!("  4. Timing-sensitive operations");

    // Keep running to allow observation
    info!("Diagnostic complete. System will continue running for observation...");
    loop {
        led_pin.set_high();
        Timer::after_millis(1000).await;
        led_pin.set_low();
        Timer::after_millis(1000).await;
        info!("System heartbeat - diagnostic tool still running");
    }
}
