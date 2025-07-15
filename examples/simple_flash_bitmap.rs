//! Simple Bitmap Flash Programming Tool
//!
//! This tool programs a bitmap file directly to Flash memory at address 0x000000

#![no_std]
#![no_main]

extern crate alloc;

use defmt::*;
use embassy_executor::Spawner;
use embedded_alloc::LlffHeap as Heap;
use {defmt_rtt as _, panic_probe as _};

// Include hardware module
#[path = "../src/hardware.rs"]
mod hardware;

#[global_allocator]
static HEAP: Heap = Heap::empty();

// Initialize heap
fn init_heap() {
    const HEAP_SIZE: usize = 16384; // 16KB heap
    static mut HEAP_MEM: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    unsafe {
        HEAP.init(
            core::ptr::addr_of_mut!(HEAP_MEM) as *mut u8 as usize,
            HEAP_SIZE,
        )
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    init_heap();

    info!("=== Simple Bitmap Flash Programming Tool ===");

    // Initialize hardware
    let p = embassy_stm32::init(Default::default());
    let mut hardware = hardware::initialize_hardware(p).await;

    // Read bitmap file data (this would be embedded in the binary)
    // For now, we'll create a simple test pattern
    let bitmap_data = create_test_bitmap();

    info!(
        "Programming {} bytes to Flash at address 0x000000",
        bitmap_data.len()
    );

    // Erase the first sector (4KB)
    info!("Erasing sector 0...");
    if let Err(e) = hardware.flash.erase_sector_async(0).await {
        error!("Failed to erase sector: {:?}", e);
        return;
    }

    // Write bitmap data
    info!("Writing bitmap data...");
    if let Err(e) = hardware.flash.write_async(0x000000, &bitmap_data).await {
        error!("Failed to write bitmap: {:?}", e);
        return;
    }

    // Verify the write
    info!("Verifying write...");
    let mut read_buffer = alloc::vec![0u8; bitmap_data.len()];
    if let Err(e) = hardware.flash.read_async(0x000000, &mut read_buffer).await {
        error!("Failed to read back data: {:?}", e);
        return;
    }

    if read_buffer == bitmap_data {
        info!("✓ Bitmap programming successful!");
    } else {
        error!("✗ Verification failed - data mismatch");
    }

    info!("Flash programming complete");
}

fn create_test_bitmap() -> alloc::vec::Vec<u8> {
    extern crate alloc;
    use alloc::vec::Vec;

    // Create a simple 120x30 RGB565 bitmap with header
    let width = 120u32;
    let height = 30u32;
    let pixel_count = (width * height) as usize;
    let data_size = pixel_count * 2; // RGB565 = 2 bytes per pixel

    let mut bitmap = Vec::new();

    // Header (24 bytes)
    bitmap.extend_from_slice(&0x424D5447u32.to_le_bytes()); // Signature "GTMB"
    bitmap.extend_from_slice(&width.to_le_bytes()); // Width
    bitmap.extend_from_slice(&height.to_le_bytes()); // Height
    bitmap.extend_from_slice(&1u32.to_le_bytes()); // Format (RGB565)
    bitmap.extend_from_slice(&(data_size as u32).to_le_bytes()); // Data size
    bitmap.extend_from_slice(&0x12345678u32.to_le_bytes()); // Checksum (placeholder)

    // Pixel data - create a simple gradient pattern
    for _y in 0..height {
        for x in 0..width {
            // Create a blue to cyan gradient
            let ratio = x as f32 / width as f32;
            let r = 0u8;
            let g = (64.0 + ratio * 191.0) as u8;
            let b = (128.0 + ratio * 127.0) as u8;

            // Convert to RGB565
            let r5 = (r >> 3) as u16;
            let g6 = (g >> 2) as u16;
            let b5 = (b >> 3) as u16;
            let rgb565 = (r5 << 11) | (g6 << 5) | b5;

            // Store in little-endian format
            let color_bytes = rgb565.to_le_bytes();
            bitmap.push(color_bytes[0]);
            bitmap.push(color_bytes[1]);
        }
    }

    info!(
        "Created test bitmap: {}x{} pixels, {} bytes total",
        width,
        height,
        bitmap.len()
    );
    bitmap
}
