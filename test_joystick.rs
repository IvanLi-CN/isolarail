// test_joystick.rs
//! 五向摇杆测试程序
//!
//! 这个程序用于测试五向摇杆的GPIO配置和功能
//! 运行此程序可以验证每个方向的按键是否正常工作

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Input, Pull};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

/// 简化的五向摇杆结构
struct TestJoystick {
    up: Input<'static>,
    down: Input<'static>,
    left: Input<'static>,
    right: Input<'static>,
    center: Input<'static>,
}

impl TestJoystick {
    fn new(p: embassy_stm32::Peripherals) -> Self {
        Self {
            up: Input::new(p.PA1, Pull::Up),     // UP button on PA1
            down: Input::new(p.PA3, Pull::Up),   // DOWN button on PA3
            left: Input::new(p.PA2, Pull::Up),   // LEFT button on PA2
            right: Input::new(p.PA5, Pull::Up),  // RIGHT button on PA5
            center: Input::new(p.PA6, Pull::Up), // CENTER button on PA6
        }
    }

    fn check_buttons(&self) -> (bool, bool, bool, bool, bool) {
        (
            self.up.is_low(), // 按下时为低电平
            self.down.is_low(),
            self.left.is_low(),
            self.right.is_low(),
            self.center.is_low(),
        )
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // 初始化STM32
    let config = embassy_stm32::Config::default();
    let p = embassy_stm32::init(config);

    info!("五向摇杆测试程序启动");
    info!("GPIO配置:");
    info!("  UP    -> PA1");
    info!("  DOWN  -> PA3");
    info!("  LEFT  -> PA2");
    info!("  RIGHT -> PA5");
    info!("  CENTER-> PA6");
    info!("按下任意方向键进行测试...");

    // 初始化摇杆
    let joystick = TestJoystick::new(p);

    let mut last_state = (false, false, false, false, false);

    loop {
        let current_state = joystick.check_buttons();

        // 只在状态改变时打印信息
        if current_state != last_state {
            let (up, down, left, right, center) = current_state;

            if up && !last_state.0 {
                info!("✓ UP 按键按下 (PA1)");
            }
            if down && !last_state.1 {
                info!("✓ DOWN 按键按下 (PA3)");
            }
            if left && !last_state.2 {
                info!("✓ LEFT 按键按下 (PA2)");
            }
            if right && !last_state.3 {
                info!("✓ RIGHT 按键按下 (PA5)");
            }
            if center && !last_state.4 {
                info!("✓ CENTER 按键按下 (PA6)");
            }

            // 检查按键释放
            if !up && last_state.0 {
                info!("  UP 按键释放");
            }
            if !down && last_state.1 {
                info!("  DOWN 按键释放");
            }
            if !left && last_state.2 {
                info!("  LEFT 按键释放");
            }
            if !right && last_state.3 {
                info!("  RIGHT 按键释放");
            }
            if !center && last_state.4 {
                info!("  CENTER 按键释放");
            }

            last_state = current_state;
        }

        // 每10ms检查一次
        Timer::after(Duration::from_millis(10)).await;
    }
}
