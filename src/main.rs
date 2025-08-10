// src/main.rs
#![no_std]
#![no_main]

use core::ptr;
use defmt::*;
use embassy_executor::Spawner;
use embedded_alloc::LlffHeap as Heap;
// use embedded_graphics::prelude::WebColors;
use {defmt_rtt as _, panic_probe as _};

mod app;
mod display;
mod hardware;
mod joystick_example;

extern crate alloc;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting ISO USB Hub - Enhanced Power-On Stability Version");

    // Configure STM32 system with improved stability
    let config = hardware::configure_stm32();
    let p = embassy_stm32::init(config);

    // Add initial system stabilization delay
    // This is critical for reliable startup after power-on reset
    info!("System initialization - waiting for power and clocks to stabilize...");
    embassy_time::Timer::after_millis(200).await;
    info!("System stabilization complete");

    // Initialize the allocator BEFORE you use it
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }
    }

    // Initialize all hardware components
    let hardware = hardware::initialize_hardware(p).await;

    // Run the main application
    app::run_application(hardware).await;
}
