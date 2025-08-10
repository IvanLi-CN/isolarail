#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    wdg::IndependentWatchdog,
};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

/// Watchdog functionality test tool
/// This tool tests the watchdog functionality and demonstrates its behavior
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("=== Watchdog Functionality Test ===");

    // Initialize system
    let config = embassy_stm32::Config::default();
    let p = embassy_stm32::init(config);
    info!("✓ STM32 system initialized");

    // Initialize a status LED (if available)
    let mut status_led = Output::new(p.PC13, Level::Low, Speed::Low);

    // Test 1: Normal watchdog operation
    info!("Test 1: Normal watchdog operation with regular feeding");
    let mut watchdog = IndependentWatchdog::new(p.IWDG, 5_000_000); // 5 second timeout in microseconds
    watchdog.unleash();
    info!("✓ Watchdog initialized with 5 second timeout");

    // Feed watchdog regularly for 10 seconds
    info!("Feeding watchdog every 1 second for 10 seconds...");
    for i in 1..=10 {
        status_led.set_high();
        Timer::after_millis(100).await;
        status_led.set_low();
        Timer::after_millis(900).await;

        watchdog.pet();
        info!("Fed watchdog at {}s - system should remain stable", i);
    }
    info!("✓ Test 1 completed - watchdog fed regularly, no reset occurred");

    // Test 2: Demonstrate watchdog reset (WARNING: This will reset the system!)
    info!("Test 2: Watchdog reset demonstration");
    info!("WARNING: System will reset in ~5 seconds due to watchdog timeout");
    info!("This demonstrates that the watchdog is working correctly");

    // Blink LED rapidly to show we're still running
    let mut blink_count = 0;
    loop {
        status_led.set_high();
        Timer::after_millis(100).await;
        status_led.set_low();
        Timer::after_millis(100).await;

        blink_count += 1;
        if blink_count % 5 == 0 {
            info!(
                "Still running... watchdog will reset system soon ({})",
                blink_count / 5
            );
        }

        // Deliberately NOT feeding the watchdog - system should reset in ~5 seconds
        // watchdog.pet(); // This line is commented out to trigger reset
    }
}
