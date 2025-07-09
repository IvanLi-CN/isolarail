use std::fs::File;
use std::io::Read;

// Convert RGB888 to RGB565
fn rgb888_to_rgb565(r: u8, g: u8, b: u8) -> u16 {
    let r5 = (r >> 3) as u16;
    let g6 = (g >> 2) as u16;
    let b5 = (b >> 3) as u16;
    (r5 << 11) | (g6 << 5) | b5
}

// Convert RGB565 back to RGB888 (for verification)
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

fn main() -> std::io::Result<()> {
    // Test our color conversion functions
    println!("Testing color conversions:");
    
    // Test background color
    let bg_rgb565 = rgb888_to_rgb565(0, 0, 32);
    let (bg_r, bg_g, bg_b) = rgb565_to_rgb888(bg_rgb565);
    println!("Background: RGB(0,0,32) -> RGB565(0x{:04X}) -> RGB({},{},{})", 
             bg_rgb565, bg_r, bg_g, bg_b);
    
    // Test gradient colors
    for x in [0, 30, 60, 90, 119] {
        let ratio = x as f32 / 120.0;
        let r = (ratio * 64.0) as u8;
        let g = (128.0 + ratio * 127.0) as u8;
        let b = (255.0 - ratio * 64.0) as u8;
        
        let rgb565 = rgb888_to_rgb565(r, g, b);
        let (r_back, g_back, b_back) = rgb565_to_rgb888(rgb565);
        
        println!("Gradient x={}: RGB({},{},{}) -> RGB565(0x{:04X}) -> RGB({},{},{})", 
                 x, r, g, b, rgb565, r_back, g_back, b_back);
    }
    
    // Read and analyze the bitmap file
    println!("\nAnalyzing bitmap file:");
    let mut file = File::open("hello_world_120x30.bin")?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    
    println!("File size: {} bytes", data.len());
    
    // Skip header (24 bytes) and analyze first few pixels
    if data.len() >= 24 + 16 {
        println!("First 8 pixels after header:");
        for i in (24..24+16).step_by(2) {
            let rgb565_bytes = [data[i], data[i + 1]];
            let rgb565_value = u16::from_le_bytes(rgb565_bytes);
            let (r, g, b) = rgb565_to_rgb888(rgb565_value);
            
            println!("  Pixel {}: bytes=[{:02X},{:02X}] -> RGB565=0x{:04X} -> RGB({},{},{})", 
                     (i-24)/2, data[i], data[i + 1], rgb565_value, r, g, b);
        }
    }
    
    // Count non-zero pixels
    let pixel_data = &data[24..];
    let non_zero_count = pixel_data.iter().filter(|&&b| b != 0).count();
    println!("Non-zero bytes: {}/{}", non_zero_count, pixel_data.len());
    
    // Check for specific patterns
    let mut unique_colors = std::collections::HashSet::new();
    for i in (0..pixel_data.len()).step_by(2) {
        if i + 1 < pixel_data.len() {
            let rgb565_bytes = [pixel_data[i], pixel_data[i + 1]];
            let rgb565_value = u16::from_le_bytes(rgb565_bytes);
            unique_colors.insert(rgb565_value);
        }
    }
    
    println!("Unique colors found: {}", unique_colors.len());
    let mut colors: Vec<_> = unique_colors.into_iter().collect();
    colors.sort();
    
    println!("First 10 unique colors:");
    for (i, &color) in colors.iter().take(10).enumerate() {
        let (r, g, b) = rgb565_to_rgb888(color);
        println!("  {}: 0x{:04X} -> RGB({},{},{})", i, color, r, g, b);
    }
    
    Ok(())
}
