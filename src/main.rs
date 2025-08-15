//! ESP32-S3 Hello World
//!
//! This is a simple hello world example for ESP32-S3 using esp-hal and embassy.
//! It demonstrates basic async functionality with periodic logging.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::timer::timg::TimerGroup;

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn hello_task() {
    let mut counter = 0u32;
    loop {
        esp_println::println!("Hello World from ESP32-S3! Counter: {}", counter);
        counter = counter.wrapping_add(1);
        Timer::after(Duration::from_millis(1000)).await;
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_println::println!("ESP32-S3 Hello World Starting!");

    // Initialize the embassy time driver
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    esp_println::println!("Main task started, spawning hello task...");

    // Spawn the hello task
    spawner.spawn(hello_task()).ok();

    // Main loop - just keep the main task alive
    loop {
        esp_println::println!("Main task heartbeat");
        Timer::after(Duration::from_millis(5000)).await;
    }
}
