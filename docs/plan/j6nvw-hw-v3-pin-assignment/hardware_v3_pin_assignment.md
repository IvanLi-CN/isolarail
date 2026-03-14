# 硬件引脚分配（ISO USB Hub V3）

> 说明：本文件在“计划阶段”暂存于 `docs/plan/` 下；在进入实现阶段并冻结口径后，再决定是否需要晋升到 `docs/` 下的稳定路径。
>
> 目的：为固件实现与联调提供“单一可信”的引脚/地址/极性清单。进入实现阶段前，本文件不得存在 `TBD`。

## 适用范围

- Hardware: ISO USB Hub 主板 V3（含 4 路 USB 供电子板）
- MCU: ESP32-S3R2（以 V3 主板网表 U39 标注为准）
- Firmware: 本仓库（`iso-usb-hub`）

## 变更状态（V3 相对当前实现）

本文件用于 V3。若某项信息尚未被 V3 原理图/实测确认，请显式标记为 `TBD`；在计划进入“待实现”前必须清零。

## MCU GPIO 分配（汇总）

说明：下表以当前仓库文档/代码为基线整理，若 V3 有改动，请以 V3 硬件确认结果覆盖。

| 功能域 | 信号（Signal） | GPIO | 方向 | 逻辑/极性 | 备注 |
| --- | --- | ---: | --- | --- | --- |
| I2C | I2C_SDA | 8 | MCU →/← I2C | - | 上行 I2C SDA |
| I2C | I2C_SCL | 9 | MCU →/← I2C | - | 上行 I2C SCL |
| I2C | I2C_INT | 16 | I2C → MCU | active-low (OD) | PCA9545A INT 汇总（开漏，外部上拉） |
| I2C | I2C_RESET | 35 | MCU → I2C | active-low | 复位 I2C 外设（网表 `RESET#`，建议 OD 驱动；低电平复位） |
| Power in | IN_EN | 41 | MCU → Power path | active-high | TPS2490/NMOS 使能（高=导通） |
| Power in | IN_PG | 42 | Power path → MCU | active-high (OD) | TPS2490 PG（开漏，外部上拉） |
| USB power module | EN1 | 17 | MCU → OUT1 | active-high | OUT1 `EN` 直连控制（高=使能） |
| USB power module | EN2 | 18 | MCU → OUT2 | active-high | OUT2 `EN` 直连控制（高=使能） |
| USB power module | EN3 | 39 | MCU → OUT3 | active-high | OUT3 `EN` 直连控制（高=使能，MTCK 复用脚） |
| USB power module | EN4 | 40 | MCU → OUT4 | active-high | OUT4 `EN` 直连控制（高=使能，MTDO 复用脚） |
| USB hub | HUB_RESET# | 5 | MCU → HUB | active-low | CH335F Reset# |
| USB hub | HUB_SCL | 46 | MCU ↔ HUB | digital I/O | CH335F `LED3/SCL` 复用控制信号（Strapping 引脚，硬件已分配） |
| USB hub | HUB_SDA | 45 | MCU ↔ HUB | digital I/O | CH335F `LED4/SDA` 复用控制信号（Strapping 引脚，硬件已分配） |
| UI | BUZZER | 7 | MCU → buzzer | PWM | 无源蜂鸣器（PWM 输出） |
| UI | FAN_PWM | 1 | MCU → fan ctrl | PWM | 风扇调压 PWM（参考 `docs/pwm_fan_control_circuit_design.md`） |
| UI | FAN_EN | 2 | MCU → fan | active-high | 风扇使能 |
| UI | FAN_TACH | 6 | fan → MCU | pulse in | 风扇测速输入（PCNT） |
| UI | VIN_ADC | 4 | VIN sense → MCU | ADC | 输入电压采样（分压网络） |
| UI | ISO_OK | 21 | ISO chip → MCU | digital in | ISOUSB211DPR 隔离 OK（需外部上拉） |
| Display (SPI) | LCD_DC | 10 | MCU → LCD | - | 数据/命令选择 |
| Display (SPI) | LCD_MOSI | 11 | MCU → LCD | - | SPI MOSI（IOMUX 建议） |
| Display (SPI) | LCD_SCLK | 12 | MCU → LCD | - | SPI SCLK（IOMUX 建议） |
| Display (SPI) | LCD_CS | 13 | MCU → LCD | active-low | SPI CS |
| Display (SPI) | LCD_RST | 14 | MCU → LCD | active-low | LCD Reset |
| Display (SPI) | LCD_BLK | 15 | MCU → LCD | PWM | 背光（如为 PWM 调光） |
| System | EN | - | - | - | CHIP_PU / EN（复位） |
| System | GPIO0 | 0 | button → MCU | TBD | BOOT 按键（Strapping，引脚用途需谨慎） |

备注：

- USB D+/D-：ESP32-S3 的 USB PHY 引脚为固定映射，但当前仓库文档对 GPIO19/GPIO20 的 D+/D- 标注存在自相矛盾处；V3 请以芯片资料与板级连线最终确认，并在此处补齐。
- HUB 侧带信号：V3 原理图使用 `HUB_SDA/HUB_SCL = GPIO45/GPIO46`（CH335F `LED4/SDA` 与 `LED3/SCL` 复用路径）。
- 当前实现已将 `I2C_RESET` 切换到 `GPIO35`；后续联调与回归验证都应以本表为准，不要回退到 `GPIO38`。

## V3 关键占用 GPIO（与 V2 差异）

V3 网表中，原先 `PSTOP_CTLx` 讨论涉及的 4 根 MCU 引脚并未释放，而是用于 4 路输出模块 `ENx` 直连使能：

| GPIO | V3 信号 | 说明 |
| ---: | --- | --- |
| 17 | EN1 | OUT1 `EN`（高电平使能） |
| 18 | EN2 | OUT2 `EN`（高电平使能） |
| 39 | EN3 | OUT3 `EN`（高电平使能） |
| 40 | EN4 | OUT4 `EN`（高电平使能） |

## I2C 拓扑与地址（V3）

### 上行 I2C（MCU 主控侧）

| 器件 | 作用 | I2C 地址 | 备注 |
| --- | --- | ---: | --- |
| TCA6408A（前面板 U43） | 五向开关输入 | 0x21 | 五向：中=P0，右=P1，下=P2，左=P3，上=P4（ADDR 接 3V3） |
| TCA6408A（主板 U43） | HUB `PWREN#/OVCUR#` 扩展 | 0x20 | P0/P2/P4/P6=`PWREN1..4#`，P1/P3/P5/P7=`OVCUR1..4#`（ADDR 接地） |
| INA226 | 输入功率/电压/电流检测 | 0x40 | 当前网表中 A1/A0 均接地；`TBD`：是否保留历史固件 `0x44` 兼容逻辑 |
| PCA9545A | 4 通道 I2C 复用/隔离 | 0x70 | 下行通道隔离 4 路子板（SC8815/SW2303 地址固定） |

### PCA9545A 下行（每个通道）

| 器件 | 作用 | I2C 地址 | 备注 |
| --- | --- | ---: | --- |
| SC8815 | USB 电源协议/电源管理 | 0x74 | `TBD`：地址来自 `sc8815-rs` 默认值，需以 V3 硬件实测确认 |
| SW2303 | USB-C/PD 相关控制 | 0x3C | `TBD`：地址来自 `sw2303-rs` 默认值，需以 V3 硬件实测确认 |

## 待确认（V3 必须补齐）

- `TBD`：USB D+/D-（GPIO19/GPIO20）在本项目中的实际连接与命名
- `TBD`：INA226 最终地址（0x40/0x44 或其它），以及是否需要在固件中做“扫描 + 锁定”策略
- `TBD`：若 V3 改动了 I2C 地址脚（PCA9545A/INA226/TCA6408A），需同步更新本表与固件默认配置
