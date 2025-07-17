//! Flash New Bitmap Tool
//!
//! This tool programs the new 160x40 bitmap to Flash memory

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

// Embed the bitmap data at compile time
static BITMAP_DATA: &[u8] = include_bytes!("../startup_bitmap.bin");

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

    info!("=== Flash New 160x40 Bitmap Tool ===");

    // Initialize hardware
    let config = hardware::configure_stm32();
    let p = embassy_stm32::init(config);
    let mut hardware = hardware::initialize_hardware(p).await;

    info!("Embedded bitmap data size: {} bytes", BITMAP_DATA.len());

    // Create bitmap with header
    let bitmap_with_header = create_bitmap_with_header();

    info!(
        "Programming {} bytes to Flash at address 0x000000",
        bitmap_with_header.len()
    );

    // Erase the first 4 sectors (16KB total) to be safe
    for sector in 0..4 {
        info!("Erasing sector {}...", sector);
        if let Err(e) = hardware.flash.erase_sector(sector).await {
            error!("Failed to erase sector {}: {:?}", sector, e);
            return;
        }
    }

    // Write bitmap data
    info!("Writing bitmap data...");
    if let Err(e) = hardware.flash.write(0x000000, &bitmap_with_header).await {
        error!("Failed to write bitmap: {:?}", e);
        return;
    }

    // Verify the write
    info!("Verifying write...");
    let mut read_buffer = alloc::vec![0u8; bitmap_with_header.len()];
    if let Err(e) = hardware.flash.read(0x000000, &mut read_buffer).await {
        error!("Failed to read back data: {:?}", e);
        return;
    }

    if read_buffer == bitmap_with_header {
        info!("✓ New bitmap programming successful!");
        info!("✓ 160x40 bitmap is now stored in Flash at address 0x000000");
    } else {
        error!("✗ Verification failed - data mismatch");
    }

    info!("Flash programming complete");
}

fn create_bitmap_with_header() -> alloc::vec::Vec<u8> {
    extern crate alloc;
    use alloc::vec::Vec;

    // Bitmap specifications
    let width = 160u32;
    let height = 40u32;
    let data_size = BITMAP_DATA.len() as u32;

    let mut bitmap = Vec::new();

    // Header (24 bytes) - same format as expected by our app
    bitmap.extend_from_slice(&0x424D5447u32.to_le_bytes()); // Signature "GTMB"
    bitmap.extend_from_slice(&width.to_le_bytes()); // Width
    bitmap.extend_from_slice(&height.to_le_bytes()); // Height
    bitmap.extend_from_slice(&1u32.to_le_bytes()); // Format (RGB565)
    bitmap.extend_from_slice(&data_size.to_le_bytes()); // Data size
    bitmap.extend_from_slice(&0x12345678u32.to_le_bytes()); // Checksum (placeholder)

    // Add the actual bitmap data
    bitmap.extend_from_slice(BITMAP_DATA);

    info!(
        "Created bitmap with header: {}x{} pixels, {} bytes total",
        width,
        height,
        bitmap.len()
    );
    bitmap
}
