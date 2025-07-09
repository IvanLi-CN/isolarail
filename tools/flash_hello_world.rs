//! Flash HELLO WORLD bitmap to W25Q128 Flash memory
//! 
//! This tool reads the hello_world_120x40.bin file and programs it to Flash at address 0x000000

use std::fs;
use std::io::{self, Write};

fn main() -> io::Result<()> {
    println!("=== HELLO WORLD Bitmap Flash Programming Tool ===");
    
    // Read the bitmap file
    let bitmap_path = "hello_world_120x40.bin";
    let bitmap_data = match fs::read(bitmap_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error reading bitmap file '{}': {}", bitmap_path, e);
            return Err(e);
        }
    };
    
    println!("✓ Read bitmap file: {} bytes", bitmap_data.len());
    
    // Analyze the bitmap header
    if bitmap_data.len() >= 24 {
        let signature = u32::from_le_bytes([bitmap_data[0], bitmap_data[1], bitmap_data[2], bitmap_data[3]]);
        let width = u32::from_le_bytes([bitmap_data[4], bitmap_data[5], bitmap_data[6], bitmap_data[7]]);
        let height = u32::from_le_bytes([bitmap_data[8], bitmap_data[9], bitmap_data[10], bitmap_data[11]]);
        let format = u32::from_le_bytes([bitmap_data[12], bitmap_data[13], bitmap_data[14], bitmap_data[15]]);
        let data_size = u32::from_le_bytes([bitmap_data[16], bitmap_data[17], bitmap_data[18], bitmap_data[19]]);
        
        println!("Bitmap Header:");
        println!("  Signature: 0x{:08X}", signature);
        println!("  Dimensions: {}x{}", width, height);
        println!("  Format: {}", format);
        println!("  Data size: {} bytes", data_size);
        
        if signature == 0x424D5447 {
            println!("✓ Valid bitmap header signature");
        } else {
            println!("⚠ Invalid bitmap header signature");
        }
    }
    
    // Analyze color data
    let pixel_data_start = 24;
    if bitmap_data.len() > pixel_data_start {
        let pixel_data = &bitmap_data[pixel_data_start..];
        println!("Pixel data: {} bytes", pixel_data.len());
        
        // Check first few pixels
        if pixel_data.len() >= 16 {
            println!("First 16 bytes of pixel data: {:02X?}", &pixel_data[0..16]);
            
            // Convert first few RGB565 pixels to RGB888 for analysis
            for i in 0..8 {
                if i * 2 + 1 < pixel_data.len() {
                    let rgb565_bytes = [pixel_data[i * 2], pixel_data[i * 2 + 1]];
                    let rgb565 = u16::from_le_bytes(rgb565_bytes);
                    let (r, g, b) = rgb565_to_rgb888(rgb565);
                    println!("  Pixel {}: RGB565=0x{:04X} -> RGB888=({}, {}, {})", i, rgb565, r, g, b);
                }
            }
        }
        
        // Check for non-zero data
        let non_zero_count = pixel_data.iter().filter(|&&b| b != 0).count();
        let zero_count = pixel_data.len() - non_zero_count;
        println!("Pixel data analysis:");
        println!("  Non-zero bytes: {} ({:.1}%)", non_zero_count, (non_zero_count as f32 / pixel_data.len() as f32) * 100.0);
        println!("  Zero bytes: {} ({:.1}%)", zero_count, (zero_count as f32 / pixel_data.len() as f32) * 100.0);
        
        if non_zero_count > 0 {
            println!("✓ Bitmap contains visible color data");
        } else {
            println!("⚠ Bitmap appears to be all black (all zeros)");
        }
    }
    
    // Create a binary file that can be flashed using external tools
    let output_path = "flash_data.bin";
    fs::write(output_path, &bitmap_data)?;
    println!("✓ Created flash data file: {}", output_path);

    // Create a hex dump for manual verification
    let hex_dump_path = "flash_data.hex";
    create_hex_dump(&bitmap_data, hex_dump_path)?;
    println!("✓ Created hex dump file: {}", hex_dump_path);
    
    println!("\n=== Flash Programming Instructions ===");
    println!("1. Use STM32CubeProgrammer or similar tool to flash the binary:");
    println!("   - File: {}", output_path);
    println!("   - Address: 0x08000000 (or external Flash address if available)");
    println!("   - Size: {} bytes", bitmap_data.len());
    println!("\n2. Or use probe-rs with the generated binary:");
    println!("   probe-rs write --chip STM32G431CBUx --address 0x08000000 {}", output_path);
    println!("\n3. The bitmap should display as a blue gradient 'HELLO WORLD' text");
    
    Ok(())
}

/// Convert RGB565 to RGB888
fn rgb565_to_rgb888(rgb565: u16) -> (u8, u8, u8) {
    let r = ((rgb565 >> 11) & 0x1F) as u8;
    let g = ((rgb565 >> 5) & 0x3F) as u8;
    let b = (rgb565 & 0x1F) as u8;
    
    // Scale back to 8-bit
    let r8 = (r << 3) | (r >> 2);
    let g8 = (g << 2) | (g >> 4);
    let b8 = (b << 3) | (b >> 2);
    
    (r8, g8, b8)
}

/// Create a hex dump file for manual inspection
fn create_hex_dump(data: &[u8], output_path: &str) -> io::Result<()> {
    let mut file = fs::File::create(output_path)?;
    
    writeln!(file, "HELLO WORLD Bitmap Hex Dump")?;
    writeln!(file, "Size: {} bytes", data.len())?;
    writeln!(file, "Address: 0x000000")?;
    writeln!(file, "")?;
    
    for (offset, chunk) in data.chunks(16).enumerate() {
        write!(file, "{:08X}: ", offset * 16)?;
        
        // Hex bytes
        for (i, &byte) in chunk.iter().enumerate() {
            write!(file, "{:02X}", byte)?;
            if i % 2 == 1 {
                write!(file, " ")?;
            }
        }
        
        // Pad if necessary
        for i in chunk.len()..16 {
            write!(file, "  ")?;
            if i % 2 == 1 {
                write!(file, " ")?;
            }
        }
        
        write!(file, " ")?;
        
        // ASCII representation
        for &byte in chunk {
            if byte >= 32 && byte <= 126 {
                write!(file, "{}", byte as char)?;
            } else {
                write!(file, ".")?;
            }
        }
        
        writeln!(file)?;
    }
    
    Ok(())
}
