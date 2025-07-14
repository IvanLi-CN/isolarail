use std::fs::File;
use std::io::Write;

// 生成启动屏图像数据

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 生成启动屏图像 ===");

    const WIDTH: usize = 160;
    const HEIGHT: usize = 40;
    const TOTAL_PIXELS: usize = WIDTH * HEIGHT;

    // 创建启动屏图像数据 (简单的渐变图案)
    let mut startup_image = Vec::with_capacity(TOTAL_PIXELS * 2);

    println!("生成160x40启动屏图像...");

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            // 创建简单的渐变启动屏
            let color = if y < 10 {
                // 顶部区域 - 蓝色渐变
                let intensity = (x * 31) / WIDTH;
                rgb565_to_bytes(0, 0, intensity as u8)
            } else if y < 20 {
                // 中上区域 - 绿色渐变
                let intensity = (x * 63) / WIDTH;
                rgb565_to_bytes(0, intensity as u8, 0)
            } else if y < 30 {
                // 中下区域 - 红色渐变
                let intensity = (x * 31) / WIDTH;
                rgb565_to_bytes(intensity as u8, 0, 0)
            } else {
                // 底部区域 - 白色渐变
                let intensity_r = (x * 31) / WIDTH;
                let intensity_g = (x * 63) / WIDTH;
                let intensity_b = (x * 31) / WIDTH;
                rgb565_to_bytes(intensity_r as u8, intensity_g as u8, intensity_b as u8)
            };

            startup_image.extend_from_slice(&color);
        }
    }

    // 保存启动屏图像
    let mut file = File::create("startup_screen.bin")?;
    file.write_all(&startup_image)?;

    println!("✓ 启动屏图像已生成: startup_screen.bin");
    println!("  尺寸: {}x{} 像素", WIDTH, HEIGHT);
    println!("  大小: {} 字节", startup_image.len());

    Ok(())
}

// 将RGB值转换为RGB565格式的小端字节
fn rgb565_to_bytes(r: u8, g: u8, b: u8) -> [u8; 2] {
    // 将8位RGB转换为RGB565
    let r5 = (r >> 3) & 0x1F; // 5位红色
    let g6 = (g >> 2) & 0x3F; // 6位绿色  
    let b5 = (b >> 3) & 0x1F; // 5位蓝色

    // 组合成16位RGB565值
    let rgb565 = ((r5 as u16) << 11) | ((g6 as u16) << 5) | (b5 as u16);

    // 转换为小端字节序
    [
        (rgb565 & 0xFF) as u8,        // 低字节
        ((rgb565 >> 8) & 0xFF) as u8, // 高字节
    ]
}
