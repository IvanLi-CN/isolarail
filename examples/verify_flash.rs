//! Verify Flash Bitmap Tool
//!
//! This tool verifies if the bitmap was correctly written to Flash

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

    info!("=== Flash Bitmap Verification Tool ===");

    // Initialize hardware
    let config = hardware::configure_stm32();
    let p = embassy_stm32::init(config);
    let mut hardware = hardware::initialize_hardware(p).await;

    // Read bitmap header (24 bytes)
    let mut header_buffer = [0u8; 24];
    if let Err(e) = hardware
        .flash
        .read_async(0x000000, &mut header_buffer)
        .await
    {
        error!("Failed to read bitmap header: {:?}", e);
        return;
    }

    // Parse header
    let signature = u32::from_le_bytes([
        header_buffer[0],
        header_buffer[1],
        header_buffer[2],
        header_buffer[3],
    ]);
    let width = u32::from_le_bytes([
        header_buffer[4],
        header_buffer[5],
        header_buffer[6],
        header_buffer[7],
    ]);
    let height = u32::from_le_bytes([
        header_buffer[8],
        header_buffer[9],
        header_buffer[10],
        header_buffer[11],
    ]);
    let format = u32::from_le_bytes([
        header_buffer[12],
        header_buffer[13],
        header_buffer[14],
        header_buffer[15],
    ]);
    let data_size = u32::from_le_bytes([
        header_buffer[16],
        header_buffer[17],
        header_buffer[18],
        header_buffer[19],
    ]);
    let checksum = u32::from_le_bytes([
        header_buffer[20],
        header_buffer[21],
        header_buffer[22],
        header_buffer[23],
    ]);

    info!("=== Bitmap Header ===");
    info!("Signature: 0x{:08X} (expected: 0x424D5447)", signature);
    info!("Width: {} pixels", width);
    info!("Height: {} pixels", height);
    info!("Format: {} (1=RGB565)", format);
    info!("Data size: {} bytes", data_size);
    info!("Checksum: 0x{:08X}", checksum);

    // Verify signature
    if signature == 0x424D5447 {
        info!("✓ Signature is correct");
    } else {
        error!("✗ Invalid signature");
        return;
    }

    // Verify dimensions
    if width == 160 && height == 40 {
        info!("✓ Dimensions are correct (160x40)");
    } else {
        error!("✗ Unexpected dimensions: {}x{}", width, height);
    }

    // Verify format
    if format == 1 {
        info!("✓ Format is RGB565");
    } else {
        error!("✗ Unexpected format: {}", format);
    }

    // Verify data size
    let expected_size = width * height * 2; // RGB565 = 2 bytes per pixel
    if data_size == expected_size {
        info!("✓ Data size is correct ({} bytes)", data_size);
    } else {
        error!(
            "✗ Data size mismatch: expected {}, got {}",
            expected_size, data_size
        );
    }

    // Read first few pixels to verify data
    let mut pixel_buffer = [0u8; 32]; // Read first 16 pixels (32 bytes)
    if let Err(e) = hardware.flash.read_async(24, &mut pixel_buffer).await {
        error!("Failed to read pixel data: {:?}", e);
        return;
    }

    info!("=== First 16 Pixels ===");
    for i in 0..16 {
        let offset = i * 2;
        let rgb565 = u16::from_le_bytes([pixel_buffer[offset], pixel_buffer[offset + 1]]);
        let r = (rgb565 >> 11) & 0x1F;
        let g = (rgb565 >> 5) & 0x3F;
        let b = rgb565 & 0x1F;
        info!(
            "Pixel {}: RGB565=0x{:04X} R={} G={} B={}",
            i, rgb565, r, g, b
        );
    }

    info!("=== Verification Complete ===");
}
