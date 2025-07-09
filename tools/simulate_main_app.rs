//! Simulate the main application startup sequence
//! 
//! This tool simulates what happens when the main application tries to read from Flash
//! and falls back to generating the HELLO WORLD bitmap in memory

use std::fs;
use std::convert::TryInto;

/// Bitmap header structure (matches the one in app.rs)
#[derive(Debug)]
struct BitmapHeader {
    signature: u32,
    width: u32,
    height: u32,
    format: u32,
    data_size: u32,
    checksum: u32,
}

impl BitmapHeader {
    const SIZE: usize = 24;
    
    fn from_bytes(bytes: &[u8; Self::SIZE]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        
        Some(Self {
            signature: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            width: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            height: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            format: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            data_size: u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            checksum: u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
        })
    }
}

/// Simulate Flash read operation
fn simulate_flash_read(_address: usize, buffer: &mut [u8]) -> Result<(), &'static str> {
    // Simulate reading from Flash - in real hardware this would be empty/invalid
    // For simulation, we'll fill with zeros to simulate empty Flash
    buffer.fill(0);
    Ok(())
}

/// Simulate the startup splash display logic
fn simulate_display_startup_splash() -> Result<(), &'static str> {
    println!("=== Simulating display_startup_splash() ===");
    println!("Loading startup splash screen from Flash...");
    
    // Read bitmap header from Flash
    let mut header_bytes = [0u8; BitmapHeader::SIZE];
    if let Err(e) = simulate_flash_read(0x000000, &mut header_bytes) {
        println!("Failed to read bitmap header from Flash: {:?}", e);
        return Ok(()); // Continue without splash screen
    }
    
    let header = match BitmapHeader::from_bytes(&header_bytes) {
        Some(h) => h,
        None => {
            println!("Invalid bitmap header in Flash, generating HELLO WORLD bitmap in memory");
            return simulate_display_hello_world_bitmap();
        }
    };
    
    println!("Bitmap header: {}x{}, format: {}, size: {} bytes",
             header.width, header.height, header.format, header.data_size);
    
    // Validate bitmap format and size
    if header.signature != 0x424D5447 {
        println!("Invalid bitmap signature: 0x{:08X}", header.signature);
        return simulate_display_hello_world_bitmap();
    }
    
    if header.format != 1 {
        println!("Unsupported bitmap format: {}", header.format);
        return simulate_display_hello_world_bitmap();
    }
    
    if header.width != 120 || header.height != 30 {
        println!("Invalid bitmap dimensions: {}x{}", header.width, header.height);
        return simulate_display_hello_world_bitmap();
    }
    
    println!("Valid bitmap found in Flash, would display it now");
    println!("Startup splash screen displayed successfully");
    Ok(())
}

/// Simulate the HELLO WORLD bitmap generation
fn simulate_display_hello_world_bitmap() -> Result<(), &'static str> {
    println!("=== Simulating display_hello_world_bitmap() ===");
    println!("Generating HELLO WORLD bitmap in memory...");
    
    const WIDTH: usize = 120;
    const HEIGHT: usize = 30;
    const TOTAL_PIXELS: usize = WIDTH * HEIGHT;
    
    // Simulate memory allocation
    println!("Allocating memory for {} pixels", TOTAL_PIXELS);
    
    // Simulate bitmap generation
    let mut text_pixels = 0;
    let mut bg_pixels = 0;
    
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if is_hello_world_pixel(x, y) {
                text_pixels += 1;
            } else {
                bg_pixels += 1;
            }
        }
    }
    
    println!("Generated {}x{} bitmap with {} pixels", WIDTH, HEIGHT, TOTAL_PIXELS);
    println!("  Text pixels: {}", text_pixels);
    println!("  Background pixels: {}", bg_pixels);
    
    // Calculate display position (center the 120x30 bitmap on 240x240 display)
    let start_x = (240 - WIDTH as u16) / 2;
    let start_y = (240 - HEIGHT as u16) / 2;
    
    println!("Display position: ({}, {}) to ({}, {})", 
             start_x, start_y, 
             start_x + WIDTH as u16 - 1, 
             start_y + HEIGHT as u16 - 1);
    
    println!("HELLO WORLD bitmap displayed successfully");
    Ok(())
}

/// Check if a pixel should be part of the "HELLO WORLD" text (simplified version)
fn is_hello_world_pixel(x: usize, y: usize) -> bool {
    // Text area bounds (centered in 120x30)
    if y < 8 || y >= 22 || x < 10 || x >= 110 {
        return false;
    }
    
    let text_y = y - 8;
    let text_x = x - 10;
    
    // Simple pattern for "HELLO WORLD"
    match text_y {
        0 | 13 => text_x % 8 < 6, // Top and bottom lines
        1..=5 | 6..=12 => {
            let char_pos = text_x / 8;
            let char_x = text_x % 8;
            match char_pos {
                0..=4 | 6..=10 => char_x == 0 || char_x == 6 || (text_y == 3 && char_x < 6), // Letters
                5 => false, // space
                _ => false,
            }
        }
        _ => false,
    }
}

/// Simulate the main application startup
fn simulate_main_startup() {
    println!("=== Simulating Main Application Startup ===");
    println!("Hardware initialization...");
    println!("Display initialization...");
    println!("Flash initialization...");
    println!("Clearing display to black...");
    println!();
    
    // Display startup splash screen from Flash for 30 seconds
    println!("Displaying startup splash screen...");
    if let Err(e) = simulate_display_startup_splash() {
        println!("Failed to display startup splash: {:?}", e);
        // Fallback to test pattern if splash fails
        println!("Fallback to test pattern...");
    }
    
    println!();
    println!("Showing startup splash for 30 seconds...");
    println!("(In real hardware, this would display for 30 seconds)");
    println!();
    
    println!("Clearing display after splash screen...");
    println!("Testing buzzer...");
    println!("Buzzer test complete.");
    println!();
    
    println!("Starting main dashboard...");
    println!("=== Main Application Running ===");
}

fn main() {
    println!("🚀 ISO USB Hub Startup Simulation");
    println!("==================================");
    println!();
    
    // Check if we have a valid bitmap file from our previous tests
    if let Ok(data) = fs::read("hello_world_memory_test.bin") {
        println!("Found test bitmap file: {} bytes", data.len());
        if data.len() >= 24 {
            let header_bytes: [u8; 24] = data[0..24].try_into().unwrap();
            if let Some(header) = BitmapHeader::from_bytes(&header_bytes) {
                println!("Test bitmap header: {}x{}, format: {}, signature: 0x{:08X}",
                         header.width, header.height, header.format, header.signature);
            }
        }
        println!();
    }
    
    simulate_main_startup();
    
    println!();
    println!("✅ Simulation completed successfully!");
    println!("📝 Expected behavior:");
    println!("   1. Flash read fails (empty Flash)");
    println!("   2. Falls back to memory-generated HELLO WORLD bitmap");
    println!("   3. Displays blue background with gradient text");
    println!("   4. Shows for 30 seconds, then continues to main app");
}
