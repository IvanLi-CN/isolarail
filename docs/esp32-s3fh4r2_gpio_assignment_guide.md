# ESP32-S3FH4R2 GPIO 引脚分配指南

## 项目概述

本文档为基于 ESP32-S3FH4R2 微控制器的项目提供确定的 GPIO 引脚分配方案。ESP32-S3FH4R2 集成了 4MB Flash 和 2MB PSRAM，本方案经过性能优化，确保最佳的外设性能。

## 芯片规格

- **型号**: ESP32-S3FH4R2
- **Flash**: 4MB (Quad SPI)
- **PSRAM**: 2MB (Quad SPI)
- **工作温度**: -40°C ~ 85°C
- **工作电压**: 3.3V

## 引脚分配方案

### 专用功能引脚 (10个)

#### SPI 屏幕接口 (6个引脚) - 高性能IOMUX配置

- **GPIO10**: DC (数据/命令控制)
- **GPIO11**: MOSI (主设备输出，从设备输入) ⚡ IOMUX直连
- **GPIO12**: SCLK (SPI时钟) ⚡ IOMUX直连，最高80MHz
- **GPIO13**: CS (片选信号) ⚡ IOMUX直连
- **GPIO14**: RES (复位信号)
- **GPIO15**: BLK (PWM背光控制)

**性能特点**: 最高80MHz时钟频率，最低延迟，支持半双工和全双工模式

#### I2C 总线接口 (3个引脚) - 标准配置

- **GPIO8**: SDA (I2C数据线)
- **GPIO9**: SCL (I2C时钟线)
- **GPIO16**: INT (I2C设备中断引脚)

**性能特点**: 支持标准模式(100kHz)和快速模式(400kHz)，最佳兼容性

#### 蜂鸣器 (1个引脚)

- **GPIO4**: BUZZER (PWM音调控制)

**功能特点**: 支持PWM输出，安全的非启动配置引脚

#### PWM 风扇控制 (3个引脚) - 完整风扇控制方案

- **GPIO1**: FAN_PWM (PWM调压控制，25kHz)
- **GPIO2**: FAN_EN (风扇启停控制)
- **GPIO6**: FAN_TACH (风扇测速输入，PCNT)

**功能特点**: 基于RT9043GB LDO的PWM调压控制，支持5V 0.7A风扇，调速范围2V-5V，集成转速反馈

#### USB HUB控制接口 (6个引脚) - 专用控制信号

- **GPIO7**: I2C_EN (I2C总线上拉电源使能)
- **GPIO5**: HUB_RESET_N (USB HUB芯片重置，低电平有效)
- **GPIO17**: CE1 (USB下行端口1 VBUS使能，低电平有效)
- **GPIO18**: CE2 (USB下行端口2 VBUS使能，低电平有效)
- **GPIO39**: CE3 (USB下行端口3 VBUS使能，低电平有效，物理引脚44/MTCK)
- **GPIO40**: CE4 (USB下行端口4 VBUS使能，低电平有效，物理引脚45/MTDO)

**功能特点**: 专用于USB HUB芯片(CH335F)控制，支持独立端口电源管理

### 系统控制引脚 (2个)

- **CHIP_PU (EN)**: 复位按钮连接
- **GPIO0**: BOOT按钮连接 (可读取按钮状态)

### 预留的普通IO引脚 (6个)

以下引脚保留作为普通数字IO使用：

- **GPIO21**
- **GPIO38, GPIO41, GPIO42**
- **GPIO47, GPIO48**

### 引脚使用统计

| 功能类别 | 使用引脚数量 | 引脚编号 |
|----------|-------------|----------|
| **SPI屏幕** | 6个 | GPIO10, GPIO11, GPIO12, GPIO13, GPIO14, GPIO15 |
| **I2C总线** | 3个 | GPIO8, GPIO9, GPIO16 |
| **蜂鸣器** | 1个 | GPIO4 |
| **PWM风扇控制** | 3个 | GPIO1, GPIO2, GPIO6 |
| **USB HUB控制** | 6个 | GPIO5, GPIO7, GPIO17, GPIO18, GPIO39, GPIO40 |
| **系统控制** | 2个 | EN, GPIO0 |
| **预留IO** | 6个 | GPIO21,38,41,42,47,48 |
| **不可用** | 7个 | GPIO26,27,28,29,30,31,32 (Flash/PSRAM) |
| **不推荐** | 3个 | GPIO3,45,46 (Strapping引脚) |
| **总计** | **37个引脚** | 完整GPIO映射 |

### 引脚快速查找表 (按PIN序号排序)

| GPIO编号 | PIN序号 | 功能 | 说明 | 特性 |
|----------|---------|------|------|------|
| **EN** | 4 | 复位按钮 | 系统复位控制 | 硬件复位信号 |
| **GPIO0** | 5 | BOOT按钮 | 启动控制，可读取按钮状态 | ⚠️ Strapping引脚，谨慎使用 |
| **GPIO1** | 6 | FAN_PWM | PWM风扇调压控制 | 25kHz PWM输出，RT9043GB控制 |
| **GPIO2** | 7 | FAN_EN | 风扇启停控制 | 数字输出，风扇使能信号 |
| **GPIO3** | 8 | 不推荐 | JTAG信号源控制 | ⚠️ Strapping引脚，影响调试 |
| **GPIO4** | 9 | 蜂鸣器 | PWM音调控制 | 安全的非启动配置引脚 |
| **GPIO5** | 10 | HUB_RESET_N | USB HUB芯片重置控制 | 低电平有效重置 |
| **GPIO6** | 11 | FAN_TACH | 风扇测速输入 | PCNT脉冲计数，转速反馈 |
| **GPIO7** | 12 | I2C_EN | I2C总线上拉电源使能 | 高电平有效使能 |
| **GPIO8** | 13 | I2C_SDA | I2C数据线 | 默认I2C引脚，最佳兼容性 |
| **GPIO9** | 14 | I2C_SCL | I2C时钟线 | 默认I2C引脚，最佳兼容性 |
| **GPIO10** | 15 | SPI_DC | 数据/命令控制 | SPI屏幕控制信号 |
| **GPIO11** | 16 | SPI_MOSI | 主设备输出，从设备输入 | ⚡ IOMUX直连，高性能 |
| **GPIO12** | 17 | SPI_SCLK | SPI时钟 | ⚡ IOMUX直连，最高80MHz |
| **GPIO13** | 18 | SPI_CS | 片选信号 | ⚡ IOMUX直连，最低延迟 |
| **GPIO14** | 19 | SPI_RES | 复位信号 | SPI屏幕复位控制 |
| **GPIO15** | 21 | SPI_BLK | PWM背光控制 | SPI屏幕背光调节 |
| **GPIO16** | 22 | I2C_INT | I2C设备中断引脚 | 中断处理 |
| **GPIO17** | 23 | CE1 | USB下行端口1 VBUS使能 | 低电平有效使能 |
| **GPIO18** | 24 | CE2 | USB下行端口2 VBUS使能 | 低电平有效使能 |
| **GPIO21** | 27 | 预留IO | 普通数字输入输出 | 通用IO |
| **GPIO26** | 28 | 不可用 | Flash/PSRAM CLK | 🚫 Flash专用，禁止使用 |
| **GPIO27** | 30 | 不可用 | Flash/PSRAM CS0 | 🚫 Flash专用，禁止使用 |
| **GPIO28** | 31 | 不可用 | Flash/PSRAM DATA0 | 🚫 Flash专用，禁止使用 |
| **GPIO29** | 32 | 不可用 | Flash/PSRAM DATA1 | 🚫 Flash专用，禁止使用 |
| **GPIO30** | 33 | 不可用 | Flash/PSRAM DATA2 | 🚫 Flash专用，禁止使用 |
| **GPIO31** | 34 | 不可用 | Flash/PSRAM DATA3 | 🚫 Flash专用，禁止使用 |
| **GPIO32** | 35 | 不可用 | Flash/PSRAM DATA4 | 🚫 PSRAM专用，禁止使用 |
| **GPIO48** | 36 | 预留IO | 普通数字输入输出 | 通用IO |
| **GPIO47** | 37 | 预留IO | 普通数字输入输出 | 通用IO |
| **GPIO38** | 43 | 预留IO | 普通数字输入输出 | 通用IO |
| **GPIO39** | 44 | CE3 | USB下行端口3 VBUS使能 | 低电平有效使能 (MTCK) |
| **GPIO40** | 45 | CE4 | USB下行端口4 VBUS使能 | 低电平有效使能 (MTDO) |
| **GPIO41** | 47 | 预留IO | 普通数字输入输出 | 通用IO |
| **GPIO42** | 48 | 预留IO | 普通数字输入输出 | 通用IO |
| **GPIO45** | 51 | 不推荐 | VDD_SPI电压选择 | ⚠️ Strapping引脚，影响Flash供电 |
| **GPIO46** | 52 | 不推荐 | ROM消息打印控制 | ⚠️ Strapping引脚，影响启动日志 |

## 引脚特性说明

### 高性能引脚 (⚡ IOMUX直连)

- **GPIO11**: MOSI - 支持半双工和全双工模式
- **GPIO12**: SCLK - 最高80MHz时钟频率
- **GPIO13**: CS - 片选信号，最低延迟

### 标准功能引脚

- **GPIO8/GPIO9**: I2C默认引脚，最佳兼容性
- **GPIO4**: 安全的非启动配置引脚，适合PWM
- **GPIO1/GPIO2/GPIO6**: PWM风扇控制专用引脚，完整风扇控制方案（调速+启停+测速）
- **GPIO0**: 启动控制引脚，低电平进入下载模式

### 通用IO引脚特性

- **工作电压**: 3.3V
- **最大驱动电流**: 40mA/引脚
- **支持功能**: 数字输入/输出、PWM、ADC(部分引脚)
- **内置功能**: 可配置上拉/下拉电阻

## 不推荐使用的GPIO引脚

### Flash和PSRAM专用引脚 (不可用)

以下引脚被Flash和PSRAM占用，**不能用于用户应用**：

- **GPIO26**: SPI Flash/PSRAM CLK
- **GPIO27**: SPI Flash/PSRAM CS0
- **GPIO28**: SPI Flash/PSRAM DATA0
- **GPIO29**: SPI Flash/PSRAM DATA1
- **GPIO30**: SPI Flash/PSRAM DATA2
- **GPIO31**: SPI Flash/PSRAM DATA3
- **GPIO32**: SPI Flash/PSRAM DATA4 (仅PSRAM)

### Strapping引脚 (谨慎使用)

以下引脚在启动时用于配置系统参数，使用时需要特别注意：

- **GPIO0**: BOOT按钮 - 启动时低电平进入下载模式
- **GPIO3**: JTAG信号源控制 - 影响调试功能
- **GPIO45**: VDD_SPI电压选择 - 影响Flash供电
- **GPIO46**: ROM消息打印控制 - 影响启动日志

### 使用建议

1. **绝对避免**: GPIO26-32 (Flash/PSRAM专用)
2. **谨慎使用**: GPIO0, GPIO3, GPIO45, GPIO46 (Strapping引脚)
3. **推荐使用**: 本文档中列出的预留IO引脚

## 性能优化建议

### SPI 性能优化

1. **使用推荐的 IOMUX 引脚** (GPIO11/12/13) 获得最高性能
2. **时钟频率**: 最高可达 80MHz
3. **信号完整性**: IOMUX 直连提供最佳信号质量

### I2C 性能优化

1. **时钟频率**: 推荐使用 400kHz 快速模式
2. **上拉电阻**: 使用适当的上拉电阻 (通常 4.7kΩ)
3. **中断处理**: 利用 GPIO16 进行高效的中断处理
4. **电源控制**: 通过 GPIO7 (I2C_EN) 控制I2C总线上拉电源

### PWM 风扇控制优化

1. **PWM 频率**: GPIO1 输出25kHz PWM，避开音频范围
2. **滤波设计**: 使用两级RC滤波器(2.2kΩ + 68nF)获得平滑DC电压
3. **电压范围**: 基于RT9043GB LDO实现2V-5V调速范围
4. **启停控制**: GPIO2 提供独立的风扇启停控制
5. **转速反馈**: GPIO6 通过PCNT外设实现高精度转速测量
6. **闭环控制**: 三引脚控制确保快速响应、精确控制和实时监控

### USB HUB 控制优化

1. **重置时序**: GPIO5 (HUB_RESET_N) 低电平重置，确保足够的重置脉宽
2. **端口管理**: CE1-CE4 (GPIO17/18/39/40) 独立控制各USB端口VBUS
3. **电源序列**: 建议先使能HUB芯片，再依次使能各端口
4. **故障保护**: 低电平有效设计提供更好的故障安全特性

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
