// tools/flash_bitmap.rs
// Standalone tool to program bitmap to Flash memory

#![no_std]
#![no_main]

use core::ptr;
use defmt::*;
use embassy_executor::Spawner;
use embedded_alloc::LlffHeap as Heap;
use {defmt_rtt as _, panic_probe as _};

// Include the hardware module from the main crate
#[path = "../src/hardware.rs"]
mod hardware;

extern crate alloc;
use alloc::vec::Vec;

#[global_allocator]
static HEAP: Heap = Heap::empty();

/// Create a test startup bitmap (160x40 RGB565)
fn create_startup_bitmap() -> Vec<u8> {
    let mut bitmap = Vec::with_capacity(12800); // 160 * 40 * 2 bytes

    // Create a simple test pattern to debug coordinate system
    // Top half: red, bottom half: blue
    for y in 0..40 {
        for _x in 0..160 {
            let rgb565 = if y < 20 {
                // Top half: red
                0xF800 // Pure red in RGB565
            } else {
                // Bottom half: blue
                0x001F // Pure blue in RGB565
            };

            bitmap.push((rgb565 >> 8) as u8);
            bitmap.push(rgb565 as u8);
        }
    }

    bitmap
}

/// Get the startup bitmap data
fn get_startup_bitmap() -> Vec<u8> {
    create_startup_bitmap()
}

/// Bitmap header structure (must match the one in src/app.rs)
#[repr(C)]
#[derive(Debug)]
struct BitmapHeader {
    signature: u32, // "GTMB" signature (0x424D5447)
    width: u32,     // Image width in pixels
    height: u32,    // Image height in pixels
    format: u32,    // Pixel format (1 = RGB565)
    data_size: u32, // Size of image data in bytes
    checksum: u32,  // Simple checksum of image data
}

impl BitmapHeader {
    const SIZE: usize = 24; // 6 * 4 bytes
    const SIGNATURE: u32 = 0x424D5447; // "GTMB"

    fn new(width: u32, height: u32, data_size: u32, checksum: u32) -> Self {
        Self {
            signature: Self::SIGNATURE,
            width,
            height,
            format: 1, // RGB565
            data_size,
            checksum,
        }
    }

    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0..4].copy_from_slice(&self.signature.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.width.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.height.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.format.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.data_size.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.checksum.to_le_bytes());
        bytes
    }
}

/// Calculate simple checksum for bitmap data
fn calculate_checksum(data: &[u8]) -> u32 {
    let mut checksum = 0u32;
    for chunk in data.chunks(4) {
        let mut bytes = [0u8; 4];
        for (i, &b) in chunk.iter().enumerate() {
            if i < 4 {
                bytes[i] = b;
            }
        }
        checksum = checksum.wrapping_add(u32::from_le_bytes(bytes));
    }
    checksum
}

/// Program startup bitmap to Flash memory
async fn program_startup_bitmap(hardware: &mut hardware::HardwareConfig<'_>) {
    info!("Programming startup bitmap to Flash...");

    // Get the bitmap data
    let startup_bitmap_data = get_startup_bitmap();
    info!("Bitmap size: {} bytes", startup_bitmap_data.len());

    // Calculate checksum
    let checksum = calculate_checksum(&startup_bitmap_data);
    info!("Calculated checksum: 0x{:08X}", checksum);

    // Create bitmap header
    let header = BitmapHeader::new(160, 40, startup_bitmap_data.len() as u32, checksum);
    let header_bytes = header.to_bytes();

    info!(
        "Bitmap header: {}x{}, format: {}, size: {} bytes, checksum: 0x{:08X}",
        header.width, header.height, header.format, header.data_size, header.checksum
    );

    // Calculate data start address (page-aligned after header)
    const PAGE_SIZE: u32 = 256;
    const SECTOR_SIZE: u32 = 4096; // 4KB per sector
    let header_end = BitmapHeader::SIZE as u32;
    let data_start_address = header_end.div_ceil(PAGE_SIZE) * PAGE_SIZE; // Round up to next page boundary

    // Calculate how many sectors we need to erase
    let total_size = data_start_address + startup_bitmap_data.len() as u32;
    let sectors_needed = total_size.div_ceil(SECTOR_SIZE); // Round up

    info!(
        "Total data size: {} bytes, sectors needed: {}",
        total_size, sectors_needed
    );

    // Erase all required sectors (w25q32jv expects sector index, not address)
    for sector_index in 0..sectors_needed {
        info!(
            "Erasing sector {} (index {})...",
            sector_index, sector_index
        );
        if let Err(e) = hardware.flash.erase_sector(sector_index).await {
            error!("Failed to erase sector {}: {:?}", sector_index, e);
            return;
        }
    }

    // Write the bitmap header first
    info!("Writing bitmap header to Flash...");
    if let Err(e) = hardware.flash.write(0x000000, &header_bytes).await {
        error!("Failed to write bitmap header: {:?}", e);
        return;
    }

    // Write the bitmap data after the header, ensuring page alignment
    info!("Writing bitmap data to Flash...");

    // Use the same data_start_address calculated earlier

    info!(
        "Header ends at 0x{:06X}, bitmap data starts at page-aligned address 0x{:06X}",
        header_end, data_start_address
    );

    // Write bitmap data in page-sized chunks
    let mut offset = data_start_address;
    for chunk in startup_bitmap_data.chunks(PAGE_SIZE as usize) {
        if let Err(e) = hardware.flash.write(offset, chunk).await {
            error!(
                "Failed to write bitmap data chunk at offset 0x{:06X}: {:?}",
                offset, e
            );
            return;
        }
        offset += chunk.len() as u32;
    }
    info!(
        "Successfully wrote {} bytes of bitmap data starting at page-aligned address 0x{:06X}",
        startup_bitmap_data.len(),
        data_start_address
    );

    // Verify the write by reading back the header and some data
    info!("Verifying Flash write...");
    let mut verify_header = [0u8; BitmapHeader::SIZE];
    if let Err(e) = hardware.flash.read(0x000000, &mut verify_header).await {
        error!("Failed to read back header: {:?}", e);
        return;
    }

    let mut verify_data = [0u8; 64]; // Read first 64 bytes of bitmap data
    if let Err(e) = hardware
        .flash
        .read(data_start_address, &mut verify_data)
        .await
    {
        error!("Failed to read back data: {:?}", e);
        return;
    }

    info!("Expected header bytes: {:?}", &header_bytes[..16]);
    info!("Read back header bytes: {:?}", &verify_header[..16]);
    info!(
        "Expected first 16 data bytes: {:?}",
        &startup_bitmap_data[..16]
    );
    info!("Read back first 16 data bytes: {:?}", &verify_data[..16]);

    if verify_header == header_bytes && verify_data == startup_bitmap_data[..64] {
        info!("✓ Startup bitmap programmed successfully!");
    } else {
        error!("✗ Flash verification failed!");
        if verify_header != header_bytes {
            error!("Header verification failed!");
        }
        if verify_data != startup_bitmap_data[..64] {
            error!("Data verification failed!");
        }
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting Flash Bitmap Programming Tool");

    // Configure STM32 system
    let config = hardware::configure_stm32();
    let p = embassy_stm32::init(config);

    // Initialize the allocator BEFORE you use it
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 16384;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }
    }

    // Initialize all hardware components
    let mut hardware = hardware::initialize_hardware(p).await;

    // Program the bitmap to Flash
    program_startup_bitmap(&mut hardware).await;

    info!("Flash programming completed. You can now run the main application.");
}
