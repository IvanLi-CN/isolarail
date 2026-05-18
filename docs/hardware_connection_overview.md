# 硬件连接概览（ISO USB Hub v2）

> 适用：本项目主板 + 四路 USB 供电子板（SC8815+SW2303）
>
> 版本边界：本文件仅描述 V2 口径（`PSTOP_CTL -> 反相 -> PSTOP`）。
> V3 请以 `docs/plan/j6nvw-hw-v3-pin-assignment/hardware_v3_pin_assignment.md` 为准（`EN1..EN4` 直连高有效）。

本文件汇总当前硬件的关键连接关系，便于固件开发与硬件联调时查阅。内容覆盖电源输入、四路 USB 快充电源模块、用户交互与 I²C 拓扑/地址等。

## 电源输入（VIN_UNSAFE → VIN）

- 电源接口：直流 DC，输入 5–24 V，网络名为 `VIN_UNSAFE`。
- 拓扑链路：`VIN_UNSAFE → 5 mΩ 检流电阻 → NMOS（电源输入开关） → VIN`。
  - 其中 NMOS 作为输入电源开关，由上游控制信号驱动导通/关断。
- 采样与保护：
  - 5 mΩ 检流电阻同时提供给 INA226 与 TPS2490 进行电流/功率检测与保护判定。
  - INA226（I²C 0x44）用于检测输入功率与输入电压。
  - TPS2490 作为热插拔/过流保护与开关控制，过流限制约 10 A。
- 控制信号：
  - `IN_EN`：来自 MCU 的使能信号，高电平闭合 NMOS（导通电源）。

## 四路 USB 快充电源模块（SC8815 + SW2303）

- 结构：四路 USB‑C 口供电，每路作为独立电源子板焊接到主板。
- I²C 隔离/复用：由于 SC8815 与 SW2303 的 I²C 地址固定，主板使用 PCA9545A 进行通道切换。
  - PCA9545A I²C 地址为 0x70（A0/A1/A2 缺省为 0，地址范围 0x70–0x77）[1]。
  - 通道映射：
    - PCA9545A 通道 0 → USB 电源模块 1
    - PCA9545A 通道 1 → USB 电源模块 2
    - PCA9545A 通道 2 → USB 电源模块 3
    - PCA9545A 通道 3 → USB 电源模块 4
- 每路电源子板与主板连接：
  - `SDA/SCL/INT` → 连接到 PCA9545A 对应通道的 `SDx/SCx/INTx`（PCA9545A 提供四路下行中断输入并在上行以 `INT` 汇总）[1]。
  - `PSTOP_CTL` → 连接 MCU（控制信号，经板上反相后生成模块侧 `PSTOP`）。
  - `PSTOP` → 模块侧停止/启动控制信号（由 `PSTOP_CTL` 反相产生），低电平使能 SC8815 功率级；不切断主电源。
  - `DM/DP` → 连接至 CH335F（USB HUB 控制器的数据线侧）。

## 用户交互

- 五向数字开关：通过 TCA6408A 接入 MCU。
  - 前面板 TCA6408A I²C 地址：0x21（ADDR 接 3V3）。
  - 前面板 TCA6408A `RESET#`：来自主板 `RESET#`，由 MCU GPIO35 释放。
  - 引脚映射：中键 → P0；右/下/左/上 → P1/P2/P3/P4；LCD `RES/CS` → P5/P6。
- 显示屏：160×50 像素、GC9D01 驱动的 LCD TFT 彩屏；`DC/MOSI/SCLK/BLK` 由 MCU 直接驱动，`RES/CS` 由前面板 TCA6408A 控制。
- 蜂鸣器：直接连接 MCU（用于提示音与反馈）。

## I²C 总线与地址一览

上行（MCU 主控）I²C 总线设备：

- `0x21`：TCA6408A（前面板五向开关）。
- `0x20`：TCA6408A（主板 U43，HUB `PWREN#/OVCUR#` 扩展）。
- `0x44`：INA226（电压/电流/功率监测，位于输入电源侧）。
- `0x70`：PCA9545A（4 通道 I²C 开关/复用，含下行通道中断汇总）。

PCA9545A 下行各通道（与对应 USB 电源子板）包含：

- SC8815（快充协议/电源管理）与 SW2303（相关控制），二者 I²C 地址固定，因而通过 PCA9545A 通道隔离以避免地址冲突。

当前验证板的四路输出模块监测地址如下（输入 INA226 独占 `0x44`，不与输出模块复用）：

- Port 1：`SDA0/SCL0`，`INA226(0x40)` + `TMP112(0x48)`
- Port 2：`SDA0/SCL0`，`INA226(0x41)` + `TMP112(0x49)`
- Port 3：`SDA1/SCL1`，`INA226(0x42)` + `TMP112(0x4A)`
- Port 4：`SDA1/SCL1`，`INA226(0x43)` + `TMP112(0x4B)`

## 主要信号与方向说明

- `IN_EN`（MCU → TPS2490/NMOS）：高电平使能输入电源通道（闭合 NMOS）。
- `INT`（PCA9545A → MCU）：开漏低有效；为四个下行 `INTx` 的逻辑与汇总信号。
- `PSTOP_CTL[1..4]`（MCU → 板级反相 → `PSTOP[1..4]` → 各 USB 电源模块）：控制各路子板的启动/停止状态（逻辑功能），不负责主电源输入；模块侧 `PSTOP` 为低电平使能；主电源由 `IN_EN` 控制 VIN→VIN（见上）。
- `DM/DP`（各 USB 电源模块 ↔ CH335F）：USB D‑/D+ 数据线连接。

## 简化连接示意

```text
DC IN(5–24V)
  │  VIN_UNSAFE
  ├─→ [5 mΩ Shunt] ─→ [NMOS Switch] ─→ VIN ─→ (系统电源)
  │          │                 │
  │          ├─→ INA226 (0x44) │  IN_EN(来自MCU，高电平闭合NMOS)
  │          └─→ TPS2490  ─────┘

MCU I²C ──────────────┬───────────────────┬───────────────┬───────────────┐
                       │                   │               │               │
                TCA6408A(0x21)      TCA6408A(0x20)    INA226          PCA9545A(0x70)
               (前面板五向开关)   (主板 U43: PWREN/OVCUR) (0x44)          │  │  │  │
                                                                     CH0 CH1 CH2 CH3
                                                                      │   │   │   │
                                                                 USB PWR1 ...   USB PWR4
                                                                  (SC8815+SW2303 子板)
                                                                     │        │
                                                                INTx→PCA9545A→INT→MCU
                                                                      └─────── PSTOP_CTLx（MCU 控制，经板上反相→PSTOPx）
```

## PSTOP 控制极性与真值表（仅 V2）

说明：硬件更新后，所有 USB 电源模块原先的 `CE` 改为 `PSTOP`（低有效）；MCU 不再直接驱动 `PSTOP`，而是输出 `PSTOP_CTL`，经板上反相后得到模块侧 `PSTOP`。业务逻辑不变，仅 MCU 引脚极性取反。

| `PSTOP_CTL`（MCU） | `PSTOP`（到 SC8815） | SC8815 功率级 |
| --- | --- | --- |
| 0（低） | 1（高） | 禁用（OFF） |
| 1（高） | 0（低） | 使能（ON） |

## 参考资料

1) PCA9545A Datasheet（4‑Channel I²C Switch with Interrupts，I²C 地址 0x70–0x77，含 INT 汇总）：[TI PCA9545A Datasheet](https://www.ti.com/lit/ds/symlink/tca9545a.pdf)

2) INA226 Datasheet（默认地址 0x40；支持 16 个地址）：[TI INA226 Datasheet](https://www.ti.com/lit/ds/symlink/ina226.pdf)

输入侧 INA226 地址为 `0x44`（`A1=GND`、`A0=3V3`）。若后续硬件版本调整了 I²C 地址引脚（如 PCA9545A A0/A1/A2 或 INA226 A0/A1），请同步更新本表与固件默认配置。
