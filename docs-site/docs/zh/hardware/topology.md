---
title: 硬件拓扑
description: 当前 V3 基线的电源、USB Hub、I²C、端口遥测和前面板连接。
---

<!-- markdownlint-disable MD025 -->

# 硬件拓扑

本页总结当前 V3 基线。完整板级事实源是仓库内的 `docs/hardware_connection_overview.md`，站点只做公开导览。

## 当前基线

- 主控：`ESP32-S3`
- USB Hub 控制器：`CH335F`
- 持久化配置：`M24C64@0x50`
- 输入电源保护：`TPS2490`
- 输入电源遥测：`Input INA226@0x44`
- 主板 sideband expander：`Mainboard TCA6408A@0x20`
- 前面板 / LCD expander：`Front-panel TCA6408A@0x21`
- owner-facing 端口：`port1`、`port2`、`port3`、`port4`

`PCA9545A@0x70` 只保留为兼容命名槽位；当前验证板按直连共享 I²C 总线运行，不依赖它完成端口寻址。

## 系统拓扑

```text
DC IN
  └─ VIN_UNSAFE -> shunt -> TPS2490 / input gate -> VIN
                      └─ Input INA226@0x44

ESP32-S3
  ├─ Sensor I2C (GPIO8/GPIO9)
  │   ├─ Front-panel TCA6408A@0x21
  │   ├─ Input INA226@0x44
  │   ├─ Port 3 INA226@0x42 + TMP112@0x4A
  │   └─ Port 4 INA226@0x43 + TMP112@0x4B
  ├─ Hub I2C (GPIO14/GPIO13)
  │   ├─ Mainboard TCA6408A@0x20
  │   ├─ M24C64@0x50
  │   ├─ Port 1 INA226@0x40 + TMP112@0x48
  │   └─ Port 2 INA226@0x41 + TMP112@0x49
  ├─ EN1..EN4 -> Port 1..4 power gate
  ├─ ISOUSB211 V1OK <- upstream isolation status
  ├─ USB D+ / D- <-> native USB
  └─ LCD + front panel + buzzer + fan

CH335F
  ├─ PWREN1#..4# -> Mainboard TCA6408A@0x20 -> MCU
  └─ OVCUR1#..4# <- Mainboard TCA6408A@0x20 <- MCU
```

这张图的重点是边界：ESP32-S3 负责电源门控、遥测、持久化配置和控制面；CH335F 提供 USB hub 数据面以及 `PWREN#` / `OVCUR#` sideband。

## 电源输入

输入路径为：

```text
VIN_UNSAFE -> 5 mΩ shunt -> TPS2490 / input gate -> VIN
                                  └─ Input INA226@0x44
```

关键网络：

- `IN_EN`：MCU 控制输入开关资格。
- `IN_PG`：高电平表示 power-good。
- `VIN_ADC`：MCU ADC 分压采样点。

固件在启动期先判定输入资格。输入不安全时，`IN_CE` 保持关闭，端口初始化与 runtime 均不放行。

输入电源 bring-up 时按这个顺序看：

1. `VIN_UNSAFE` 是否存在。
2. `IN_EN` 是否允许 TPS2490 input gate。
3. `IN_PG` 是否为 power-good。
4. `Input INA226@0x44` 是否能读到 VIN / current / power。
5. `boot.summary` 是否把 VIN 判为 `Ok`、`Warn` 或 `Fatal`。

只有 VIN ready 后，固件才会继续探测前面板、风扇、主板 sideband 和四路输出模块。

## 四路端口门控

四路输出由 MCU 直驱 `EN1..EN4`：

| 端口 | Enable | GPIO | 遥测 |
| --- | --- | --- | --- |
| `port1` | `EN1` | `GPIO17` | `INA226@0x40` + `TMP112@0x48` |
| `port2` | `EN2` | `GPIO18` | `INA226@0x41` + `TMP112@0x49` |
| `port3` | `EN3` | `GPIO39` | `INA226@0x42` + `TMP112@0x4A` |
| `port4` | `EN4` | `GPIO40` | `INA226@0x43` + `TMP112@0x4B` |

`port.power_set` 影响对应 `ENx`。`port.replug` 是受控断电再上电，不承诺真实 per-port data disconnect。

## CH335F sideband

`CH335F` 的 sideband 通过 `Mainboard TCA6408A@0x20` 接入 MCU：

- `P0/P2/P4/P6` 读取低有效 `PWREN1#..4#`。
- `P1/P3/P5/P7` 注入低有效 `OVCUR1#..4#`。
- `ISOUSB211 V1OK` 由 `GPIO21` 读取，用于区分 standalone/no-upstream 与 upstream-managed。

`V1OK=low` 时，产品按独立输出能力运行，不因 `PWREN#` 为高而关闭端口。`V1OK=high` 时，对应 `PWREN#` 为低的端口才允许输出。

sideband 的方向和低有效语义如下：

| 信号 | TCA6408A 位 | 方向 | 语义 |
| --- | --- | --- | --- |
| `PWREN1#` | P0 | 输入 | 低表示 CH335F 允许 `port1` |
| `OVCUR1#` | P1 | 注入 | 输出低表示向 CH335F 报过流 |
| `PWREN2#` | P2 | 输入 | 低表示 CH335F 允许 `port2` |
| `OVCUR2#` | P3 | 注入 | 输出低表示向 CH335F 报过流 |
| `PWREN3#` | P4 | 输入 | 低表示 CH335F 允许 `port3` |
| `OVCUR3#` | P5 | 注入 | 输出低表示向 CH335F 报过流 |
| `PWREN4#` | P6 | 输入 | 低表示 CH335F 允许 `port4` |
| `OVCUR4#` | P7 | 注入 | 输出低表示向 CH335F 报过流 |

初始化默认释放所有 `OVCUR#`：输出寄存器写 `0xFF`、极性寄存器写 `0x00`、方向寄存器写 `0xFF`，让相关位保持输入高阻。

## 两条 I²C 总线

### Sensor / front-panel I²C

- `I2C_SDA = GPIO8`
- `I2C_SCL = GPIO9`
- 挂载：`Input INA226@0x44`、`Front-panel TCA6408A@0x21`、`port3/port4` 遥测器件。

### Hub-sideband / output I²C

- `HUB_SDA = GPIO14`
- `HUB_SCL = GPIO13`
- 挂载：`Mainboard TCA6408A@0x20`、`M24C64@0x50`、`port1/port2` 遥测器件。

## 前面板与显示

前面板 expander 为 `Front-panel TCA6408A@0x21`：

- `P0..P4`：五向按键，依次为 Center、Right、Down、Left、Up。
- `P5 = LCD_RES`
- `P6 = LCD_CS`

LCD 直连 MCU：

- `LCD_DC = GPIO10`
- `LCD_MOSI = GPIO11`
- `LCD_SCLK = GPIO12`
- `LCD_BLK = GPIO15`

显示模块是 `160x50 LCD`，驱动 IC 为 `GC9D01`，背光 `LCD_BLK` 当前为低有效。

V3 显示链路有两个容易踩坑的地方：

- `LCD_BLK` 低有效。按高有效驱动会导致背光关闭。
- 显示方向使用 `Orientation::LandscapeSwapped`。旧 `Landscape` 映射会让 160x50 实装方向不对。

当前 V3 硬件不能由 MCU 单独硬复位 `Front-panel TCA6408A@0x21`。如果 bus-clear 后只有 `0x21` 不 ACK，固件按 `Warn/FrontPanelOffline` 降级继续运行，只禁用前面板输入。

## 维护动作与硬件能力

| 控制面动作 | 硬件动作 | 不承诺 |
| --- | --- | --- |
| `port.power_set` | 拉高或拉低对应 `ENx` | 不改变 USB 数据拓扑 |
| `port.replug` | 对对应 `ENx` 做受控断电再上电 | 不保证真 per-port data disconnect |
| `hub.reset` | 整机级维护复位 | 不替代单端口 replug |
| `wifi.set` / `wifi.clear` | 写 `M24C64@0x50` | 不允许 LAN-only 写入 |

## Bring-up 检查清单

| 阶段 | 证据 |
| --- | --- |
| 输入电源 | `IN_PG`、`Input INA226@0x44`、`boot.check: name=vin` |
| I²C 拓扑 | 当前直连共享总线，`mux=Skipped` |
| 主板 sideband | `hub.sideband: tca6408a=online addr=0x20` |
| 端口遥测 | 每路 `INA226 + TMP112` 地址对可达 |
| 前面板 | `TCA6408A@0x21` 在线，或降级为 `Warn/FrontPanelOffline` |
| 显示 | 背光点亮，方向为 `LandscapeSwapped` |

## 历史边界

下列术语只允许出现在历史或迁移语境，不代表当前控制面硬件：

- `SC8815 + SW2303`
- `PSTOP_CTL1..4`
- `PSTOP1..4`
- 历史 `USB-C route` / 双口产品抽象

## 参考

- `docs/hardware_connection_overview.md`
- `docs/ch335f_tca6408a_appnote.md`
- `docs/specs/pw97u-control-plane-alignment/SPEC.md`
- `docs/specs/j6nvw-hardware-v3-pin-assignment/SPEC.md`
