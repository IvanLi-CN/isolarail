#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embedded_alloc::LlffHeap as Heap;
use {defmt_rtt as _, panic_probe as _};

#[path = "../src/hardware.rs"]
mod hardware;

#[global_allocator]
static HEAP: Heap = Heap::empty();

// 烧录启动屏和棋盘图案到Flash

// 包含正确的组合图像数据
static COMBINED_IMAGES_DATA: &[u8] = include_bytes!("../combined_images_correct.bin");

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // 初始化堆内存
    use core::mem::MaybeUninit;
    const HEAP_SIZE: usize = 4096;
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe {
        let heap_ptr = core::ptr::addr_of_mut!(HEAP_MEM);
        HEAP.init(heap_ptr as *mut u8 as usize, HEAP_SIZE)
    }

    info!("=== Flash组合图像烧录程序 ===");
    info!("组合图像数据大小: {} 字节", COMBINED_IMAGES_DATA.len());

    // 初始化硬件
    let p = embassy_stm32::init(Default::default());
    let mut hardware = hardware::initialize_hardware(p).await;
    info!("硬件初始化完成");

    // Flash参数
    const CHUNK_SIZE: usize = 256; // 每次写入256字节
    const STARTUP_ADDRESS: u32 = 0x000000; // 启动屏地址
    const CHECKERBOARD_ADDRESS: u32 = 0x003200; // 棋盘图案地址 (12800字节偏移)

    info!("开始烧录组合图像到Flash...");
    info!("启动屏地址: 0x{:06X}", STARTUP_ADDRESS);
    info!("棋盘图案地址: 0x{:06X}", CHECKERBOARD_ADDRESS);

    // 擦除Flash芯片 (W25Q32JV只支持整片擦除)
    info!("擦除Flash芯片...");

    match hardware.flash.erase_chip_async().await {
        Ok(_) => {
            info!("✓ Flash芯片擦除成功");
        }
        Err(e) => {
            error!("✗ Flash芯片擦除失败: {:?}", e);
            return;
        }
    }

    // 分块写入数据
    let total_chunks = COMBINED_IMAGES_DATA.len().div_ceil(CHUNK_SIZE);
    info!("开始写入数据，共{}个块...", total_chunks);

    for (chunk_index, chunk) in COMBINED_IMAGES_DATA.chunks(CHUNK_SIZE).enumerate() {
        let address = STARTUP_ADDRESS + (chunk_index * CHUNK_SIZE) as u32;

        match hardware.flash.write_async(address, chunk).await {
            Ok(_) => {
                if chunk_index % 10 == 0 {
                    let progress = ((chunk_index + 1) * 100) / total_chunks;
                    info!(
                        "写入进度: {}% (块 {}/{})",
                        progress,
                        chunk_index + 1,
                        total_chunks
                    );
                }
            }
            Err(e) => {
                error!("✗ 写入失败，地址0x{:06X}: {:?}", address, e);
                return;
            }
        }
    }

    info!("✓ 所有数据写入完成");

    // 验证写入的数据
    info!("验证写入的数据...");

    // 验证启动屏数据
    let mut verify_buffer = [0u8; 32];
    match hardware
        .flash
        .read_async(STARTUP_ADDRESS, &mut verify_buffer)
        .await
    {
        Ok(_) => {
            info!("✓ 启动屏数据验证: {:?}", &verify_buffer[0..8]);
        }
        Err(e) => {
            error!("✗ 启动屏数据验证失败: {:?}", e);
        }
    }

    // 验证棋盘图案数据
    match hardware
        .flash
        .read_async(CHECKERBOARD_ADDRESS, &mut verify_buffer)
        .await
    {
        Ok(_) => {
            info!("✓ 棋盘图案数据验证: {:?}", &verify_buffer[0..8]);
        }
        Err(e) => {
            error!("✗ 棋盘图案数据验证失败: {:?}", e);
        }
    }

    info!("=== 烧录完成 ===");
    info!("启动屏地址: 0x{:06X} (12800字节)", STARTUP_ADDRESS);
    info!("棋盘图案地址: 0x{:06X} (12800字节)", CHECKERBOARD_ADDRESS);
    info!("总共烧录: {} 字节", COMBINED_IMAGES_DATA.len());

    // 保持程序运行
    loop {
        embassy_time::Timer::after_millis(1000).await;
        info!("烧录程序完成，可以断开连接");
    }
}
