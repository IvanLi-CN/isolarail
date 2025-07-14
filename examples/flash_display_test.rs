#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embedded_alloc::LlffHeap as Heap;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{RgbColor, WebColors};
use {defmt_rtt as _, panic_probe as _};

#[path = "../src/hardware.rs"]
mod hardware;

#[global_allocator]
static HEAP: Heap = Heap::empty();

// 专门验证Flash图像读取和屏幕渲染的程序

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // 初始化堆内存 - 使用更小的堆
    use core::mem::MaybeUninit;
    const HEAP_SIZE: usize = 2048; // 2KB堆
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe {
        let heap_ptr = core::ptr::addr_of_mut!(HEAP_MEM);
        HEAP.init(heap_ptr as *mut u8 as usize, HEAP_SIZE)
    }

    info!("=== Flash图像读取和屏幕渲染验证程序 ===");

    // 初始化硬件
    let p = embassy_stm32::init(Default::default());
    let mut hardware = hardware::initialize_hardware(p).await;
    info!("硬件初始化完成");

    // 图像参数
    const BITMAP_WIDTH: u16 = 160;
    const BITMAP_HEIGHT: u16 = 40;
    const BYTES_PER_ROW: usize = 320; // 160像素 * 2字节/像素
    const STARTUP_ADDRESS: u32 = 0x000000; // 启动屏地址
    const CHECKERBOARD_ADDRESS: u32 = 0x003200; // 棋盘图案地址

    info!("开始交替显示启动屏和棋盘图案");
    info!("图像尺寸: {}x{} 像素", BITMAP_WIDTH, BITMAP_HEIGHT);
    info!("启动屏地址: 0x{:06X}", STARTUP_ADDRESS);
    info!("棋盘图案地址: 0x{:06X}", CHECKERBOARD_ADDRESS);

    let mut show_startup = true;
    let mut cycle_count = 0;

    loop {
        cycle_count += 1;
        let current_address = if show_startup {
            STARTUP_ADDRESS
        } else {
            CHECKERBOARD_ADDRESS
        };
        let image_name = if show_startup {
            "启动屏"
        } else {
            "棋盘图案"
        };

        info!("=== 周期 {} - 显示{} ===", cycle_count, image_name);

        // 逐行读取Flash并渲染到屏幕
        for y in 0..BITMAP_HEIGHT {
            let row_address = current_address + (y as u32 * BYTES_PER_ROW as u32);
            let mut row_buffer = [0u8; BYTES_PER_ROW];

            // 从Flash读取一行数据
            match hardware
                .flash
                .read_async(row_address, &mut row_buffer)
                .await
            {
                Ok(_) => {
                    // 转换为RGB565像素
                    let mut pixel_row = [Rgb565::BLACK; BITMAP_WIDTH as usize];
                    for (pixel_index, pixel_bytes) in row_buffer.chunks_exact(2).enumerate() {
                        if pixel_index < pixel_row.len() {
                            // RGB565小端格式
                            let pixel_value =
                                (pixel_bytes[0] as u16) | ((pixel_bytes[1] as u16) << 8);
                            pixel_row[pixel_index] = Rgb565::new(
                                ((pixel_value >> 11) & 0x1F) as u8, // 红色
                                ((pixel_value >> 5) & 0x3F) as u8,  // 绿色
                                (pixel_value & 0x1F) as u8,         // 蓝色
                            );
                        }
                    }

                    // 写入显示缓冲区
                    hardware
                        .display
                        .write_area(0, y, BITMAP_WIDTH, 1, &pixel_row);
                }
                Err(e) => {
                    error!("{}第{}行读取失败: {:?}", image_name, y, e);

                    // 错误时显示红色行
                    let error_row = [Rgb565::CSS_RED; BITMAP_WIDTH as usize];
                    hardware
                        .display
                        .write_area(0, y, BITMAP_WIDTH, 1, &error_row);
                }
            }
        }

        // 刷新显示器
        match hardware.display.flush().await {
            Ok(_) => {
                info!("✓ {}显示成功！", image_name);
            }
            Err(e) => {
                error!("✗ {}显示失败: {:?}", image_name, e);
            }
        }

        // 等待5秒
        info!("等待5秒后切换到下一个图像...");
        embassy_time::Timer::after_millis(5000).await;

        // 切换图像
        show_startup = !show_startup;
    }
}
