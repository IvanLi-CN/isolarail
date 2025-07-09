//! PNG to 140×40 Bitmap Converter (Real Implementation)
//! 
//! Converts a PNG image to 140×40 RGB565 bitmap format using the image crate

use std::fs::File;
use std::io::Write;
use image::{ImageReader, DynamicImage, imageops::FilterType};

const WIDTH: u32 = 140;
const HEIGHT: u32 = 40;
const BYTES_PER_PIXEL: usize = 2; // RGB565

// Convert RGB888 to RGB565
fn rgb888_to_rgb565(r: u8, g: u8, b: u8) -> u16 {
    let r5 = (r >> 3) as u16;
    let g6 = (g >> 2) as u16;
    let b5 = (b >> 3) as u16;
    (r5 << 11) | (g6 << 5) | b5
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Converting PNG to 140×40 bitmap...");
    
    // Load PNG image
    let img = ImageReader::open("screenshot-De8lylrp.png")?
        .decode()?;
    
    println!("Original image: {}×{}", img.width(), img.height());
    
    // Resize to 140×40 using high-quality filtering
    let resized = img.resize_exact(WIDTH, HEIGHT, FilterType::Lanczos3);
    println!("Resized to: {}×{}", resized.width(), resized.height());
    
    // Convert to RGB8
    let rgb_img = resized.to_rgb8();
    
    // Convert to RGB565 bitmap
    let mut bitmap = vec![0u8; (WIDTH * HEIGHT) as usize * BYTES_PER_PIXEL];
    
    for (y, row) in rgb_img.rows().enumerate() {
        for (x, pixel) in row.enumerate() {
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];
            
            let rgb565 = rgb888_to_rgb565(r, g, b);
            let color_bytes = rgb565.to_le_bytes();
            
            let bitmap_idx = (y * WIDTH as usize + x) * BYTES_PER_PIXEL;
            bitmap[bitmap_idx] = color_bytes[0];
            bitmap[bitmap_idx + 1] = color_bytes[1];
        }
    }
    
    // Write bitmap header (compatible with our bitmap format)
    let mut file = File::create("screenshot_140x40.bin")?;
    
    // Bitmap header structure
    let signature = 0x424D5447u32; // "GTMB" signature
    let width = WIDTH;
    let height = HEIGHT;
    let format = 1u32; // RGB565 format
    let data_size = (WIDTH * HEIGHT * BYTES_PER_PIXEL as u32);
    
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
    
    println!("Generated screenshot_140x40.bin");
    println!("Size: {} bytes", 24 + bitmap.len()); // 24 bytes header + data
    println!("Dimensions: {}×{}", WIDTH, HEIGHT);
    println!("Format: RGB565");
    println!("Checksum: 0x{:08X}", checksum);
    
    Ok(())
}
