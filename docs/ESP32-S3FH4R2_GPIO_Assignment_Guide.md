# ESP32-S3FH4R2 GPIO 引脚分配指南

## 项目概述

本文档为基于 ESP32-S3FH4R2 微控制器的项目提供确定的 GPIO 引脚分配方案。ESP32-S3FH4R2 集成了 4MB Flash 和 2MB PSRAM，本方案经过性能优化，确保最佳的外设性能。

## 芯片规格

- **型号**: ESP32-S3FH4R2
- **Flash**: 4MB (Quad SPI)
- **PSRAM**: 2MB (Quad SPI)
- **工作温度**: -40°C ~ 85°C
- **工作电压**: 3.3V

## 确定的引脚分配方案

### SPI 屏幕 (SPI2/VSPI - 最高性能配置)

使用 IOMUX 直连，获得最佳性能：

- **SCLK**: GPIO12 ⚡ (IOMUX 直连，最高 80MHz)
- **MOSI**: GPIO11 ⚡ (IOMUX 直连，半双工支持)
- **DC**: GPIO10 (数据/命令控制)
- **RES**: GPIO14 (复位信号)
- **CS**: GPIO13 ⚡ (IOMUX 直连片选)
- **BLK**: GPIO15 (PWM 背光控制)

**性能特点**:

- 最高时钟频率: 80MHz (IOMUX) vs 40MHz (GPIO 矩阵)
- 最低延迟和最佳信号完整性
- 支持半双工和全双工模式

### I2C 总线 (I2C0 - 标准配置)

- **SDA**: GPIO8 (默认 I2C 数据线)
- **SCL**: GPIO9 (默认 I2C 时钟线)
- **INT**: GPIO16 (I2C 设备中断引脚)

**性能特点**:

- 支持标准模式 (100kHz) 和快速模式 (400kHz)
- 任何 GPIO 都可用作 I2C，性能相同
- 使用默认引脚确保最佳兼容性

### 蜂鸣器

- **BUZZER**: GPIO4 (PWM 音调控制)

**功能特点**:

- 支持 PWM 输出，可产生不同音调
- 安全的非启动配置引脚

### 系统控制引脚

- **CHIP_PU (EN)**: 复位按钮连接
- **GPIO0**: BOOT 按钮连接 (可读取按钮状态)

### 可用的普通 IO 引脚

以下引脚可用作普通数字 IO：

- **GPIO1, GPIO2, GPIO5, GPIO6, GPIO7**
- **GPIO17, GPIO18, GPIO21**
- **GPIO38, GPIO39, GPIO40, GPIO41, GPIO42**
- **GPIO47, GPIO48**

**总计**: 15 个可用的普通 IO 引脚

## 引脚分配汇总表

| 功能 | 引脚 | 说明 |
|------|------|------|
| **SPI 屏幕** | GPIO10-15 | 6个引脚，IOMUX高性能 |
| **I2C 总线** | GPIO8, GPIO9, GPIO16 | 3个引脚，标准配置 |
| **蜂鸣器** | GPIO4 | PWM 音调控制 |
| **系统控制** | EN, GPIO0 | 复位和启动按钮 |
| **普通 IO** | 15个引脚 | 数字输入输出 |
| **总计使用** | **25个引脚** | 完整功能配置 |

## 性能优化建议

### SPI 性能优化

1. **使用推荐的 IOMUX 引脚** (GPIO11/12/13) 获得最高性能
2. **时钟频率**: 最高可达 80MHz
3. **信号完整性**: IOMUX 直连提供最佳信号质量

### I2C 性能优化

1. **时钟频率**: 推荐使用 400kHz 快速模式
2. **上拉电阻**: 使用适当的上拉电阻 (通常 4.7kΩ)
3. **中断处理**: 利用 GPIO16 进行高效的中断处理

### 普通 IO 优化

1. **电平兼容**: 注意 3.3V 电平兼容性
2. **驱动能力**: 每个 GPIO 最大驱动电流 40mA
3. **上拉下拉**: 根据需要配置内部上拉/下拉电阻

## 开发注意事项

### 启动时序

1. **BOOT 引脚 (GPIO0)**: 启动时低电平进入下载模式
2. **上电时序**: 确保外设在 ESP32-S3 启动后再初始化
3. **复位控制**: 使用 EN 引脚进行系统复位

### 电源管理

1. **工作电压**: 3.3V ± 10%
2. **电流消耗**: 根据使用的外设调整电源设计
3. **去耦电容**: 在电源引脚附近放置适当的去耦电容

### PCB 设计建议

1. **高速信号**: SPI 信号线保持短而直
2. **时钟信号**: 避免时钟线与其他信号线平行走线
3. **地平面**: 提供良好的地平面以减少噪声

## 代码配置示例 (Rust + esp-hal)

### 引脚定义

```rust
use esp_hal::{
    gpio::{Io, Level, Output, Pull},
    spi::{master::Spi, SpiMode},
    i2c::I2c,
    ledc::{Ledc, LowSpeed, channel},
    prelude::*,
};

// 引脚定义常量
pub struct PinConfig;

impl PinConfig {
    // SPI 屏幕引脚
    pub const SPI_SCLK: u8 = 12;
    pub const SPI_MOSI: u8 = 11;
    pub const SPI_CS: u8 = 13;
    pub const SPI_DC: u8 = 10;
    pub const SPI_RES: u8 = 14;
    pub const SPI_BLK: u8 = 15;

    // I2C 总线引脚
    pub const I2C_SDA: u8 = 8;
    pub const I2C_SCL: u8 = 9;
    pub const I2C_INT: u8 = 16;

    // 蜂鸣器引脚
    pub const BUZZER: u8 = 4;

    // 系统控制引脚
    pub const BOOT_BUTTON: u8 = 0;
}
```

### SPI 配置

```rust
use esp_hal::spi::master::{Config as SpiConfig, Spi};

pub fn init_spi(peripherals: &mut esp_hal::Peripherals, io: &Io) -> Spi<'static, esp_hal::Blocking> {
    let sclk = io.pins.gpio12;
    let mosi = io.pins.gpio11;
    let cs = io.pins.gpio13;

    let spi_config = SpiConfig {
        frequency: 80.MHz(),
        mode: SpiMode::Mode0,
        ..Default::default()
    };

    Spi::new(peripherals.SPI2, sclk, mosi, cs, spi_config)
}

// SPI 屏幕控制引脚
pub fn init_display_pins(io: &Io) -> (Output, Output, Output) {
    let dc = Output::new(io.pins.gpio10, Level::Low);
    let res = Output::new(io.pins.gpio14, Level::High);
    let blk = Output::new(io.pins.gpio15, Level::High);

    (dc, res, blk)
}
```

### I2C 配置

```rust
use esp_hal::i2c::{Config as I2cConfig, I2c};

pub fn init_i2c(peripherals: &mut esp_hal::Peripherals, io: &Io) -> I2c<'static, esp_hal::Blocking> {
    let sda = io.pins.gpio8;
    let scl = io.pins.gpio9;

    let i2c_config = I2cConfig {
        frequency: 400.kHz(),
        timeout: Some(10),
        ..Default::default()
    };

    I2c::new(peripherals.I2C0, sda, scl, i2c_config)
}

// I2C 中断引脚
pub fn init_i2c_interrupt(io: &Io) -> esp_hal::gpio::Input {
    esp_hal::gpio::Input::new(io.pins.gpio16, Pull::Up)
}
```

### 蜂鸣器配置

```rust
use esp_hal::ledc::{Ledc, LowSpeed, channel::config::Config as ChannelConfig};

pub fn init_buzzer(peripherals: &mut esp_hal::Peripherals, io: &Io) -> channel::Channel<LowSpeed, 0> {
    let mut ledc = Ledc::new(peripherals.LEDC);
    ledc.set_global_slow_clock(esp_hal::ledc::LSGlobalClkSource::APBClk);

    let mut lstimer0 = ledc.timer::<LowSpeed>(esp_hal::ledc::timer::Number::Timer0);
    lstimer0.configure(esp_hal::ledc::timer::config::Config {
        duty: esp_hal::ledc::timer::config::Duty::Duty8Bit,
        clock_source: esp_hal::ledc::timer::LSClockSource::APBClk,
        frequency: 2.kHz(),
    }).unwrap();

    let mut channel0 = ledc.channel(esp_hal::ledc::channel::Number::Channel0, io.pins.gpio4);
    channel0.configure(ChannelConfig {
        timer: &lstimer0,
        duty_pct: 0,
        pin_config: esp_hal::ledc::channel::config::PinConfig::PushPull,
    }).unwrap();

    channel0
}

// 播放音调函数
pub async fn play_tone(
    channel: &mut channel::Channel<LowSpeed, 0>,
    frequency: u32,
    duration_ms: u64
) {
    // 设置频率和占空比
    channel.set_duty(50).unwrap(); // 50% 占空比

    embassy_time::Timer::after(embassy_time::Duration::from_millis(duration_ms)).await;

    // 停止声音
    channel.set_duty(0).unwrap();
}
```

### 按钮控制

```rust
// BOOT 按钮读取
pub fn init_boot_button(io: &Io) -> esp_hal::gpio::Input {
    esp_hal::gpio::Input::new(io.pins.gpio0, Pull::Up)
}

// 检查 BOOT 按钮状态
pub fn is_boot_pressed(button: &esp_hal::gpio::Input) -> bool {
    button.is_low()
}
```

### 完整使用示例

```rust
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    gpio::Io,
    timer::timg::TimerGroup,
    prelude::*,
};

#[embassy_executor::task]
async fn peripheral_task() {
    // 这里可以添加外设控制逻辑
    loop {
        esp_println::println!("外设任务运行中...");
        Timer::after(Duration::from_millis(2000)).await;
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let mut peripherals = esp_hal::init(esp_hal::Config::default());
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    esp_println::println!("ESP32-S3FH4R2 项目启动!");

    // 初始化定时器
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    // 初始化外设
    let _spi = init_spi(&mut peripherals, &io);
    let _i2c = init_i2c(&mut peripherals, &io);
    let _buzzer = init_buzzer(&mut peripherals, &io);
    let _boot_button = init_boot_button(&io);

    esp_println::println!("所有外设初始化完成!");

    // 启动外设任务
    spawner.spawn(peripheral_task()).ok();

    // 主循环
    loop {
        esp_println::println!("主任务心跳");
        Timer::after(Duration::from_millis(5000)).await;
    }
}
```

## Cargo.toml 依赖配置

确保在 `Cargo.toml` 中添加必要的依赖：

```toml
[dependencies]
embassy-executor = { version = "0.7.0", features = ["task-arena-size-20480"] }
embassy-time = "0.4.0"
esp-backtrace = { version = "0.15.0", features = ["esp32s3", "panic-handler", "println"] }
esp-hal = { version = "0.23.0", features = ["esp32s3"] }
esp-hal-embassy = { version = "0.6.0", features = ["esp32s3"] }
esp-println = { version = "0.13.0", features = ["esp32s3"] }
```
