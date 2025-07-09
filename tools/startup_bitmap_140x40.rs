//! Startup Bitmap Generator (140×40)
//! 
//! Generates a 140×40 pixel startup bitmap
//! Output format: RGB565 (16-bit per pixel)

use std::fs::File;
use std::io::Write;

const WIDTH: usize = 140;
const HEIGHT: usize = 40;
const BYTES_PER_PIXEL: usize = 2; // RGB565

// Simple 5x7 font for each character
const FONT_5X7: [[u8; 7]; 26] = [
    // A
    [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
    // B
    [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
    // C
    [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
    // D
    [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
    // E
    [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
    // F
    [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
    // G
    [0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
    // H
    [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
    // I
    [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
    // J
    [0b00111, 0b00001, 0b00001, 0b00001, 0b10001, 0b10001, 0b01110],
    // K
    [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
    // L
    [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
    // M
    [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
    // N
    [0b10001, 0b11001, 0b10101, 0b10101, 0b10011, 0b10001, 0b10001],
    // O
    [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
    // P
    [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
    // Q
    [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
    // R
    [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
    // S
    [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
    // T
    [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
    // U
    [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
    // V
    [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
    // W
    [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
    // X
    [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
    // Y
    [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
    // Z
    [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
];

// Convert RGB888 to RGB565
fn rgb888_to_rgb565(r: u8, g: u8, b: u8) -> u16 {
    let r5 = (r >> 3) as u16;
    let g6 = (g >> 2) as u16;
    let b5 = (b >> 3) as u16;
    (r5 << 11) | (g6 << 5) | b5
}

// Generate gradient color based on position
fn get_gradient_color(x: usize, y: usize) -> u16 {
    // Create a blue to purple gradient
    let ratio_x = x as f32 / WIDTH as f32;
    let ratio_y = y as f32 / HEIGHT as f32;
    
    let r = (ratio_x * 128.0 + ratio_y * 64.0) as u8;
    let g = (64.0 + ratio_y * 64.0) as u8;
    let b = (200.0 + ratio_x * 55.0) as u8;
    
    rgb888_to_rgb565(r, g, b)
}

// Get character index (A=0, B=1, etc.)
fn char_to_index(c: char) -> Option<usize> {
    if c.is_ascii_alphabetic() {
        Some((c.to_ascii_uppercase() as u8 - b'A') as usize)
    } else {
        None
    }
}

fn main() -> std::io::Result<()> {
    let mut bitmap = vec![0u8; WIDTH * HEIGHT * BYTES_PER_PIXEL];
    
    // Text to render
    let text = "ISO USB HUB";
    let char_width = 6; // 5 pixels + 1 space
    let char_height = 7;
    
    // Calculate starting position to center the text
    let text_width = text.len() * char_width - 1; // -1 because last char doesn't need space
    let start_x = (WIDTH - text_width) / 2;
    let start_y = (HEIGHT - char_height) / 2;
    
    // Background color (dark blue gradient)
    let bg_color = rgb888_to_rgb565(20, 40, 80);
    
    // Fill background with gradient
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let pixel_idx = (y * WIDTH + x) * BYTES_PER_PIXEL;
            let color = if y < 5 || y >= HEIGHT - 5 || x < 5 || x >= WIDTH - 5 {
                // Border effect
                rgb888_to_rgb565(60, 120, 200)
            } else {
                get_gradient_color(x, y)
            };
            let color_bytes = color.to_le_bytes();
            bitmap[pixel_idx] = color_bytes[0];
            bitmap[pixel_idx + 1] = color_bytes[1];
        }
    }
    
    // Render text
    for (char_idx, ch) in text.chars().enumerate() {
        if ch == ' ' {
            continue; // Skip spaces
        }
        
        if let Some(font_idx) = char_to_index(ch) {
            let char_x = start_x + char_idx * char_width;
            let char_y = start_y;
            
            // Render character
            for row in 0..7 {
                let font_row = FONT_5X7[font_idx][row];
                for col in 0..5 {
                    if (font_row >> (4 - col)) & 1 != 0 {
                        let pixel_x = char_x + col;
                        let pixel_y = char_y + row;
                        
                        if pixel_x < WIDTH && pixel_y < HEIGHT {
                            let pixel_idx = (pixel_y * WIDTH + pixel_x) * BYTES_PER_PIXEL;
                            // White text
                            let color = rgb888_to_rgb565(255, 255, 255);
                            let color_bytes = color.to_le_bytes();
                            bitmap[pixel_idx] = color_bytes[0];
                            bitmap[pixel_idx + 1] = color_bytes[1];
                        }
                    }
                }
            }
        }
    }
    
    // Write bitmap header (compatible with our bitmap format)
    let mut file = File::create("startup_140x40.bin")?;
    
    // Bitmap header structure
    let signature = 0x424D5447u32; // "GTMB" signature
    let width = WIDTH as u32;
    let height = HEIGHT as u32;
    let format = 1u32; // RGB565 format
    let data_size = (WIDTH * HEIGHT * BYTES_PER_PIXEL) as u32;
    
    // Calculate simple checksum
    let mut checksum = 0u32;
    for chunk in bitmap.chunks(4) {
        let mut bytes = [0u8; 4];
        for (i, &b) in chunk.iter().enumerate() {
            if i < 4 {
                bytes[i] = b;
            }
        }
        checksum = checksum.wrapping_add(u32::from_le_bytes(bytes));
    }
    
    // Write header
    file.write_all(&signature.to_le_bytes())?;
    file.write_all(&width.to_le_bytes())?;
    file.write_all(&height.to_le_bytes())?;
    file.write_all(&format.to_le_bytes())?;
    file.write_all(&data_size.to_le_bytes())?;
    file.write_all(&checksum.to_le_bytes())?;
    
    // Write bitmap data
    file.write_all(&bitmap)?;
    
    println!("Generated startup_140x40.bin");
    println!("Size: {} bytes", 24 + bitmap.len()); // 24 bytes header + data
    println!("Dimensions: {}×{}", WIDTH, HEIGHT);
    println!("Format: RGB565");
    println!("Checksum: 0x{:08X}", checksum);
    
    Ok(())
}
