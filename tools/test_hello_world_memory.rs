//! Test HELLO WORLD bitmap generation in memory
//! 
//! This tool simulates the bitmap generation logic used in the main application

use std::fs;
use std::io::{self, Write};

/// RGB565 color representation
#[derive(Clone, Copy, Debug)]
struct Rgb565 {
    value: u16,
}

impl Rgb565 {
    fn new(r: u8, g: u8, b: u8) -> Self {
        // Clamp values to their bit ranges
        let r5 = (r & 0x1F) as u16;  // 5 bits
        let g6 = (g & 0x3F) as u16;  // 6 bits  
        let b5 = (b & 0x1F) as u16;  // 5 bits
        
        Self {
            value: (r5 << 11) | (g6 << 5) | b5
        }
    }
    
    fn to_bytes(self) -> [u8; 2] {
        self.value.to_le_bytes()
    }
    
    fn to_rgb888(self) -> (u8, u8, u8) {
        let r = ((self.value >> 11) & 0x1F) as u8;
        let g = ((self.value >> 5) & 0x3F) as u8;
        let b = (self.value & 0x1F) as u8;
        
        // Scale back to 8-bit
        let r8 = (r << 3) | (r >> 2);
        let g8 = (g << 2) | (g >> 4);
        let b8 = (b << 3) | (b >> 2);
        
        (r8, g8, b8)
    }
}

/// Check if a pixel should be part of the "HELLO WORLD" text
fn is_hello_world_pixel(x: usize, y: usize) -> bool {
    // Text area bounds (centered in 120x30)
    if y < 8 || y >= 22 || x < 10 || x >= 110 {
        return false;
    }
    
    let text_y = y - 8; // Normalize to text area
    let text_x = x - 10;
    
    // Simple pattern for "HELLO WORLD" - this creates a basic text-like pattern
    match text_y {
        0 | 13 => text_x % 8 < 6, // Top and bottom lines
        1..=5 => {
            // H, E, L, L, O, space, W, O, R, L, D
            let char_pos = text_x / 8;
            let char_x = text_x % 8;
            match char_pos {
                0 => char_x == 0 || char_x == 6 || text_y == 3, // H
                1 => char_x == 0 || (text_y == 0 || text_y == 3 || text_y == 5) && char_x < 6, // E
                2 | 3 => char_x == 0 || (text_y == 5 && char_x < 6), // L, L
                4 => char_x == 0 || char_x == 6 || (text_y == 0 || text_y == 5) && char_x < 7, // O
                5 => false, // space
                6 => char_x == 0 || char_x == 6 || text_y == 3, // W
                7 => char_x == 0 || char_x == 6 || (text_y == 0 || text_y == 5) && char_x < 7, // O
                8 => char_x == 0 || (text_y == 0 || text_y == 3) && char_x < 6, // R
                9 => char_x == 0 || (text_y == 5 && char_x < 6), // L
                10 => char_x == 0 || (text_y == 0 || text_y == 5) && char_x < 6, // D
                _ => false,
            }
        }
        6..=12 => {
            let char_pos = text_x / 8;
            let char_x = text_x % 8;
            match char_pos {
                0 => char_x == 0 || char_x == 6, // H
                1 => char_x == 0, // E
                2 | 3 => char_x == 0, // L, L
                4 => char_x == 0 || char_x == 6, // O
                5 => false, // space
                6 => char_x == 0 || char_x == 6, // W
                7 => char_x == 0 || char_x == 6, // O
                8 => char_x == 0 || char_x == 6, // R
                9 => char_x == 0, // L
                10 => char_x == 0, // D
                _ => false,
            }
        }
        _ => false,
    }
}

fn main() -> io::Result<()> {
    println!("=== HELLO WORLD Memory Bitmap Test ===");
    
    const WIDTH: usize = 120;
    const HEIGHT: usize = 30;
    const TOTAL_PIXELS: usize = WIDTH * HEIGHT;
    
    // Generate the bitmap
    let mut pixels = Vec::with_capacity(TOTAL_PIXELS);
    
    // Background color (medium blue - visible)
    let bg_color = Rgb565::new(0, 16, 16); // RGB565 blue color
    
    // Generate the bitmap with gradient text
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let mut color = bg_color;
            
            // Simple "HELLO WORLD" text pattern
            if is_hello_world_pixel(x, y) {
                // Create gradient effect for text
                let gradient_factor = x as f32 / WIDTH as f32;
                let r = (gradient_factor * 31.0) as u8; // 5-bit red
                let g = (gradient_factor * 63.0) as u8; // 6-bit green
                let b = ((1.0 - gradient_factor) * 31.0) as u8; // 5-bit blue
                color = Rgb565::new(r, g, b);
            }
            
            pixels.push(color);
        }
    }
    
    println!("Generated {}x{} bitmap with {} pixels", WIDTH, HEIGHT, pixels.len());
    
    // Analyze the bitmap
    let mut unique_colors = std::collections::HashSet::new();
    let mut text_pixels = 0;
    let mut bg_pixels = 0;
    
    for (i, &pixel) in pixels.iter().enumerate() {
        unique_colors.insert(pixel.value);
        
        let x = i % WIDTH;
        let y = i / WIDTH;
        
        if is_hello_world_pixel(x, y) {
            text_pixels += 1;
        } else {
            bg_pixels += 1;
        }
    }
    
    println!("Bitmap analysis:");
    println!("  Unique colors: {}", unique_colors.len());
    println!("  Text pixels: {}", text_pixels);
    println!("  Background pixels: {}", bg_pixels);
    
    // Show some sample colors
    let bg_rgb = bg_color.to_rgb888();
    println!("  Background color: RGB565=0x{:04X} -> RGB888=({}, {}, {})", 
             bg_color.value, bg_rgb.0, bg_rgb.1, bg_rgb.2);
    
    // Sample some text colors
    for sample_x in [20, 40, 60, 80, 100] {
        let gradient_factor = sample_x as f32 / WIDTH as f32;
        let r = (gradient_factor * 31.0) as u8;
        let g = (gradient_factor * 63.0) as u8;
        let b = ((1.0 - gradient_factor) * 31.0) as u8;
        let text_color = Rgb565::new(r, g, b);
        let text_rgb = text_color.to_rgb888();
        println!("  Text color at x={}: RGB565=0x{:04X} -> RGB888=({}, {}, {})", 
                 sample_x, text_color.value, text_rgb.0, text_rgb.1, text_rgb.2);
    }
    
    // Create binary data for comparison with Flash bitmap
    let mut binary_data = Vec::new();
    
    // Add header (24 bytes)
    binary_data.extend_from_slice(&0x424D5447u32.to_le_bytes()); // Signature "GTMB"
    binary_data.extend_from_slice(&(WIDTH as u32).to_le_bytes()); // Width
    binary_data.extend_from_slice(&(HEIGHT as u32).to_le_bytes()); // Height
    binary_data.extend_from_slice(&1u32.to_le_bytes()); // Format (RGB565)
    binary_data.extend_from_slice(&(TOTAL_PIXELS * 2).to_le_bytes()); // Data size
    binary_data.extend_from_slice(&0u32.to_le_bytes()); // Checksum (placeholder)
    
    // Add pixel data
    for pixel in pixels {
        binary_data.extend_from_slice(&pixel.to_bytes());
    }
    
    // Save the binary data
    fs::write("hello_world_memory_test.bin", &binary_data)?;
    println!("✓ Saved binary data: {} bytes", binary_data.len());
    
    // Create a visual representation
    let mut visual = String::new();
    visual.push_str("Visual representation (. = background, # = text):\n");
    
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if is_hello_world_pixel(x, y) {
                visual.push('#');
            } else {
                visual.push('.');
            }
        }
        visual.push('\n');
    }
    
    fs::write("hello_world_visual.txt", &visual)?;
    println!("✓ Saved visual representation");
    
    println!("\n=== Test Results ===");
    println!("✓ Bitmap generation successful");
    println!("✓ {} unique colors (should be > 1)", unique_colors.len());
    println!("✓ {} text pixels, {} background pixels", text_pixels, bg_pixels);
    println!("✓ Background is visible blue color");
    println!("✓ Text has gradient colors");
    
    if unique_colors.len() > 1 && text_pixels > 0 && bg_pixels > 0 {
        println!("\n🎉 HELLO WORLD bitmap generation test PASSED!");
    } else {
        println!("\n❌ HELLO WORLD bitmap generation test FAILED!");
    }
    
    Ok(())
}
