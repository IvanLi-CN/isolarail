# CH335F EEPROM 初始化固件（#m7gtw）

## 状态

- Status: 阻塞（0 Ω 并联 EEPROM 拓扑不可作为可控初始化方案）
- Created: 2026-04-27
- Last: 2026-04-27

## 背景 / 问题陈述

- V3 主板上的 CH335F 通过 `LED3/SCL`、`LED4/SDA` 复用脚连接外部 M24C64 EEPROM。
- ESP32-S3 通过 0 Ω 电阻同样连接到 `HUB_SCL/HUB_SDA`，Rev2.3 网表对应 `GPIO37/GPIO36`，并可用 `HUB_RESET#` 让 CH335F 保持复位。
- 原计划是用独立固件在 CH335F 复位期间写入 EEPROM 镜像，随后释放 I2C 总线并启动 CH335F，以完成 hub 描述符定制验证。
- 实机验证后，该 0 Ω 并联拓扑不能作为稳定方案；下一版硬件计划使用 CH442E 切换 EEPROM 连接方向，让 ESP32-S3 与 CH335F 不再同时挂在 EEPROM 总线上。

## 目标 / 非目标

### Goals

- 提供独立 ESP32-S3 bin，用于初始化 CH335F 外部 EEPROM。
- 按 CH334/CH335 官方 V2.4 EEPROM 布局写入 VID/PID、配置、Product String 和 Serial Number String。
- 写入前先读比对，只有 EEPROM 内容不一致才执行页写；写后必须读回校验。
- 写入完成后释放 `GPIO36/GPIO37` 为高阻输入，再释放 `HUB_RESET#`。

### Non-goals

- 不把 EEPROM 初始化逻辑并入常规运行固件默认路径。
- 不改 CH335F VID/PID；验证阶段保持 `0x1A86:0x8094`。
- 不提供主机侧图形配置工具或量产批处理系统。

## 范围

### In scope

- `src/bin/ch335f_eeprom_init.rs`
- README 中的烧录和验证说明
- CH335F EEPROM 初始化流程相关规格与实现状态

### Out of scope

- 常规 dashboard/电源管理运行固件
- USB 下行端口供电策略
- CH335F 内置信息存储器的厂商批量定制流程

## 需求

### MUST

- `HUB_RESET#` 使用 `GPIO5`，低有效；reset-only 时序不得再作为下一版 EEPROM 初始化的唯一隔离策略。
- EEPROM I2C 使用 `GPIO36=HUB_SDA`、`GPIO37=HUB_SCL`，7-bit 地址 `0x50`。
- M24C64 使用 16-bit word address，按 32-byte page 写入，写周期通过 ACK polling 确认。
- EEPROM 镜像 `00h..0Ah` 必须符合 CH334/CH335 V2.4 表 3-5-1/3-5-2：`CHKSUM = VID_H + VID_L + PID_L + PID_H + 1`，`SIG=0x5A`。
- Product String 写为 `ISO USB Hub`，UTF-16LE 编码，VID/PID 保持 `0x1A86:0x8094`。
- 为兼容旧版 CH334/CH335 EEPROM 布局，镜像可写入 `Vendor=Ivan` 字段；但当前硬件仍必须以 host USB 枚举结果为最终判据。

### SHOULD

- 日志应覆盖 reset asserted、read、compare、write/skip、verify、I2C release、reset release。
- 若 EEPROM 写入或读回校验失败，应保持 CH335F 复位，避免启动到未知配置。
- 若扫描阶段完全未发现 EEPROM ACK 且未尝试写入，应释放 I2C 与 `HUB_RESET#`，让 Hub 回到默认枚举以便继续调试。
- 验证脚本或说明应以 macOS `ioreg -p IOUSB -l -w0` 的目标 Hub 枚举字段为准。

## 功能与行为规格

- 当前固件可作为调试/验证工具保留，但 0 Ω 并联拓扑不再作为推荐生产路径。
- 初始化 I2C0 到 `GPIO36/GPIO37`，频率 100 kHz，先扫描 `0x50..0x57`，再读取 EEPROM 前 256 bytes。
- 若正常映射未发现 EEPROM ACK，允许只在同一对物理引脚上尝试一次 `GPIO37=SDA/GPIO36=SCL` 诊断；仍未发现 ACK 时不得写入。
- 生成目标镜像并与 EEPROM 当前内容比对；若完全一致，跳过写入。
- 若不同，按页写入差异页，完成后读回完整镜像校验。
- 释放流程必须先 drop I2C 外设，再把 `GPIO36/GPIO37` 配成无内部上下拉的输入，最后释放 `HUB_RESET#`。

## 验收标准

- Given 固件烧录到目标 ESP32-S3，When 通过当前目标串口运行，Then 日志显示 EEPROM `0x50` 读取成功。
- Given EEPROM 内容不同，When 初始化固件运行，Then 日志显示差异页写入、ACK polling 成功、readback match。
- Given EEPROM 内容已匹配，When 初始化固件再次运行，Then 日志显示 compare match 并跳过写入。
- Given CH335F reset 被释放或 Hub 重新枚举，When macOS 读取目标 Hub，Then 若 Product String 仍为默认 `USB HUB`，应判定当前 0 Ω 并联拓扑未完成可验证定制流程，而不是继续扩大 reset-only 软件绕法。

## 实现前置条件

- EEPROM `WC` 已被硬件拉到 GND。
- 目标 ESP32-S3 串口已确认为 `/dev/cu.usbmodem212301`。
- 当前目标 Hub baseline 是 `USB HUB`，VID/PID 为 `0x1A86:0x8094`。
- CH334/CH335 V2.4 数据手册作为 EEPROM 布局依据。

## 非功能性验收 / 质量门槛

### Testing

- `cargo +esp check`
- `cargo +esp build --release`
- `make run BIN=ch335f_eeprom_init PORT=/dev/cu.usbmodem212301 ESPFLASH_ARGS='--after hard-reset'`
- `ioreg -p IOUSB -l -w0` 枚举差分

## 文档更新

- `README.md`
- `docs/specs/m7gtw-ch335f-eeprom-initializer/IMPLEMENTATION.md`
- `docs/specs/m7gtw-ch335f-eeprom-initializer/HISTORY.md`

## 实现里程碑

- [x] M1: 独立 EEPROM 初始化固件与文档落地
- [x] M2: 在正确目标上确认 EEPROM `0x50` 可读
- [ ] M3: EEPROM 写入后确认 Hub Product String 枚举变化
- [ ] M4: 下一版硬件加入 CH442E EEPROM 方向切换

## 风险 / 开放问题 / 假设

- 风险：不同 CH334/CH335 文档版本对 Vendor String 字段定义不同，Host 侧 Vendor 名称不一定通过本流程变为 `Ivan`。
- 风险：若 CH335F 在 reset 期间仍驱动 `LED3/SCL` 或 `LED4/SDA`，需用示波器确认并调整硬件隔离策略。
- 风险：错误串口烧录会污染验证结论；烧录前必须确认 selector 指向 `/dev/cu.usbmodem212301`。
- 风险：V3 网表显示 `SDA_ROM/SCL_ROM` 通过 0 Ω 串到 `HUB_LED4/HUB_LED3`，其中 `HUB_LED4` 还连接 LED 支路；实机无 ACK 时需确认 R113/R114/R115/R116、EEPROM 供电、I2C 上拉与 LED 支路负载。
- 阻塞：ESP 可在部分状态下读写并校验 EEPROM，但 CH335F 仍枚举默认 `USB HUB`；0 Ω 并联方案无法作为可控初始化路径。
- 下一版方向：使用 CH442E 切换 EEPROM 连接方向，确保编程模式只连接 ESP32-S3，运行模式只连接 CH335F。
- 假设：M24C64 页大小为 32 bytes，`E0/E1/E2` 接 GND，对应地址 `0x50`。

## 参考

- CH334/CH335 数据手册 V2.4，表 3-5-1 / 表 3-5-2
- `docs/hardware/mainboard_netlist.enet.enet`
- `docs/plan/j6nvw-hw-v3-pin-assignment/hardware_v3_pin_assignment.md`
- `docs/solutions/hardware/ch335f-eeprom-bus-isolation.md`
