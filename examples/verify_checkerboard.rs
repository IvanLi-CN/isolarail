//! Verify Checkerboard Flash Tool
//!
//! This tool verifies the checkerboard pattern was correctly written to Flash
//! and displays it on the screen

#![no_std]
#![no_main]

extern crate alloc;

use defmt::*;
use embassy_executor::Spawner;
use embedded_alloc::LlffHeap as Heap;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{RgbColor, WebColors};
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

    info!("=== Checkerboard Flash Verification Tool ===");

    // Initialize hardware with default config to avoid SPI frequency issues
    let p = embassy_stm32::init(Default::default());
    let mut hardware = hardware::initialize_hardware(p).await;

    info!("Hardware initialized successfully");

    // 专门验证Flash图像读取和屏幕渲染
    info!("=== Flash图像读取和屏幕渲染验证 ===");

    // 图像参数
    const BITMAP_WIDTH: u16 = 160;
    const BITMAP_HEIGHT: u16 = 40;
    const BYTES_PER_ROW: usize = 320; // 160像素 * 2字节/像素 = 320字节/行

    info!("图像尺寸: {}x{} 像素", BITMAP_WIDTH, BITMAP_HEIGHT);
    info!("每行字节数: {}", BYTES_PER_ROW);

    // 从Flash读取完整的棋盘图像并渲染到屏幕
    info!("=== 开始从Flash读取棋盘图像并渲染到屏幕 ===");

    // 逐行读取和渲染图像
    for y in 0..BITMAP_HEIGHT {
        let row_address = y as u32 * BYTES_PER_ROW as u32;
        let mut row_buffer = [0u8; BYTES_PER_ROW];

        // 从Flash读取一行数据
        match hardware.flash.read(row_address, &mut row_buffer).await {
            Ok(_) => {
                // 验证读取成功
                if y < 3 {
                    info!("✓ 第{}行读取成功，地址0x{:06X}", y, row_address);
                    info!(
                        "  前8字节: {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                        row_buffer[0],
                        row_buffer[1],
                        row_buffer[2],
                        row_buffer[3],
                        row_buffer[4],
                        row_buffer[5],
                        row_buffer[6],
                        row_buffer[7]
                    );
                }

                // 将原始字节转换为RGB565像素
                let mut pixel_row = [Rgb565::BLACK; BITMAP_WIDTH as usize];
                for (pixel_index, pixel_bytes) in row_buffer.chunks_exact(2).enumerate() {
                    if pixel_index < pixel_row.len() {
                        // Flash中的RGB565数据是小端格式
                        let pixel_value = (pixel_bytes[0] as u16) | ((pixel_bytes[1] as u16) << 8);
                        pixel_row[pixel_index] = Rgb565::new(
                            ((pixel_value >> 11) & 0x1F) as u8, // 红色 (5位)
                            ((pixel_value >> 5) & 0x3F) as u8,  // 绿色 (6位)
                            (pixel_value & 0x1F) as u8,         // 蓝色 (5位)
                        );
                    }
                }

                // 将这一行写入显示缓冲区
                hardware
                    .display
                    .write_area(0, y, BITMAP_WIDTH, 1, &pixel_row);

                // 每10行报告一次进度
                if y % 10 == 0 {
                    info!("已处理第{}行，渲染到屏幕", y);
                }
            }
            Err(e) => {
                error!("✗ 第{}行读取失败: {:?}", y, e);

                // 错误时显示红色行作为错误指示
                let error_row = [Rgb565::CSS_RED; BITMAP_WIDTH as usize];
                hardware
                    .display
                    .write_area(0, y, BITMAP_WIDTH, 1, &error_row);
            }
        }
    }

    // 刷新显示器以显示完整图像
    match hardware.display.flush().await {
        Ok(_) => {
            info!("✓ 显示器刷新成功 - 棋盘图像应该已显示在屏幕上");
        }
        Err(e) => {
            error!("✗ 显示器刷新失败: {:?}", e);
        }
    }

    info!("=== 图像读取和渲染验证完成 ===");
    info!("请检查显示屏是否显示了10x10的彩色棋盘图案");

    // 保持程序运行并显示状态
    let mut counter = 0;
    loop {
        embassy_time::Timer::after_millis(10000).await;
        counter += 1;
        info!("验证程序运行中... {} (屏幕应显示棋盘图案)", counter);
    }
}

#[allow(dead_code)]
async fn verify_checkerboard_image(
    hardware: &mut hardware::HardwareConfig<'_>,
    start_address: u32,
) -> bool {
    const BITMAP_WIDTH: u16 = 160;
    const BITMAP_HEIGHT: u16 = 40;
    const CHUNK_SIZE: usize = 320; // 160 pixels * 2 bytes per pixel

    let mut chunk_buffer = [0u8; CHUNK_SIZE];
    let mut verification_passed = true;

    // Expected checkerboard colors (RGB565)
    let colors = [
        0xF800, // Red
        0xFFE0, // Yellow
        0x07E0, // Green
        0x07FF, // Cyan
        0x001F, // Blue
        0xF81F, // Magenta
        0xFFFF, // White
        0x7BEF, // Gray
    ];

    for y in 0..BITMAP_HEIGHT {
        let chunk_address = start_address + (y as u32 * CHUNK_SIZE as u32);

        // Read chunk from Flash
        match hardware.flash.read(chunk_address, &mut chunk_buffer).await {
            Ok(_) => {
                // Verify checkerboard pattern
                for x in 0..(BITMAP_WIDTH as usize) {
                    let pixel_offset = x * 2;
                    if pixel_offset + 1 < chunk_buffer.len() {
                        // Read RGB565 pixel (little-endian)
                        let pixel_value = (chunk_buffer[pixel_offset] as u16)
                            | ((chunk_buffer[pixel_offset + 1] as u16) << 8);

                        // Calculate expected color based on checkerboard pattern
                        let square_x = x / 10; // 10x10 squares
                        let square_y = (y as usize) / 10;
                        let expected_color_index = (square_x + square_y) % colors.len();
                        let expected_color = colors[expected_color_index];

                        if pixel_value != expected_color {
                            error!(
                                "Pixel mismatch at ({}, {}): expected 0x{:04X}, got 0x{:04X}",
                                x, y, expected_color, pixel_value
                            );
                            return false; // Early exit on first error
                        }
                    }
                }

                if (y + 1) % 10 == 0 {
                    info!("  Verified rows 0-{}", y);
                }
            }
            Err(_) => {
                error!("Failed to read chunk at address 0x{:08X}", chunk_address);
                verification_passed = false;
                break;
            }
        }
    }

    verification_passed
}

#[allow(dead_code)]
async fn display_flash_image(hardware: &mut hardware::HardwareConfig<'_>, start_address: u32) {
    const BITMAP_WIDTH: u16 = 160;
    const BITMAP_HEIGHT: u16 = 40;
    const CHUNK_SIZE: usize = 320; // 160 pixels * 2 bytes per pixel

    let mut chunk_buffer = [0u8; CHUNK_SIZE];

    info!(
        "Reading and displaying image from Flash address 0x{:08X}",
        start_address
    );

    for y in 0..BITMAP_HEIGHT {
        let chunk_address = start_address + (y as u32 * CHUNK_SIZE as u32);

        // Read chunk from Flash
        match hardware.flash.read(chunk_address, &mut chunk_buffer).await {
            Ok(_) => {
                // Convert raw bytes to RGB565 pixels
                let mut rgb565_chunk = [Rgb565::BLACK; 160]; // Max pixels per row

                for (i, chunk_bytes) in chunk_buffer.chunks_exact(2).enumerate() {
                    if i < rgb565_chunk.len() {
                        // RGB565 data is stored as little-endian in Flash
                        let pixel_value = (chunk_bytes[0] as u16) | ((chunk_bytes[1] as u16) << 8);
                        rgb565_chunk[i] = Rgb565::new(
                            ((pixel_value >> 11) & 0x1F) as u8, // Red (5 bits)
                            ((pixel_value >> 5) & 0x3F) as u8,  // Green (6 bits)
                            (pixel_value & 0x1F) as u8,         // Blue (5 bits)
                        );
                    }
                }

                // Write row to display
                hardware
                    .display
                    .write_area(0, y, BITMAP_WIDTH, 1, &rgb565_chunk);

                if (y + 1) % 10 == 0 {
                    info!("  Displayed rows 0-{}", y);
                }
            }
            Err(_) => {
                error!(
                    "Failed to read chunk for display at address 0x{:08X}",
                    chunk_address
                );

                // Fill with error pattern (red)
                let error_chunk = [Rgb565::CSS_RED; 160];
                hardware
                    .display
                    .write_area(0, y, BITMAP_WIDTH, 1, &error_chunk);
            }
        }

        // Small delay to avoid overwhelming the system
        embassy_time::Timer::after_millis(5).await;
    }

    // Flush display
    let _ = hardware.display.flush().await;

    info!("✓ Image displayed on screen");
}
