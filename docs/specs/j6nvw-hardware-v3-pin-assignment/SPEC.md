# 硬件 V3 引脚与显示链路对齐（#j6nvw）

## 状态

- Status: 部分完成（2/3）
- Created: 2026-02-06
- Last: 2026-04-27

## 背景 / 问题陈述

- V3 硬件把显示屏 SPI 数据/时钟与 `DC/BLK` 接到 MCU GPIO；显示 `CS/RES` 优先由前面板 `TCA6408A` 控制，前面板 TCA 在整机冷上电中离线时可由原 MCU `GPIO13/GPIO14` 兜底。
- `BLK` 网络连接到前面板 P 沟道背光门极，固件若按高有效驱动会导致背光关闭。
- 160x50 屏幕实装方向需要使用 driver 的 `LandscapeSwapped` 坐标变换；旧 driver 中该方向与 `Landscape` 映射相同，主项目必须引用已修复版本。

## 目标 / 非目标

### Goals

- 固件显示 GPIO、背光极性和屏幕方向与 V3 网表及实物行为一致。
- 主项目引用已合并的 `gc9d01-rs` driver 修复版本。
- 保留 V3 pin assignment 的 canonical spec 入口，后续实现与文档以本目录为准。

### Non-goals

- 不改变前面板连接器定义。
- 不把显示 CS/RES 默认分配给 MCU GPIO；仅允许在显示控制初始化前确认前面板 TCA 不 ACK 且本次为整机冷上电时启用原 GPIO fallback。
- 不在本规格内重做 dashboard 视觉设计。

## 范围

### In scope

- `src/main.rs` 的显示初始化配置。
- `gc9d01` submodule 指针。
- V3 GPIO 文档中 `SPI_BLK` 极性描述。

### Out of scope

- USB 输出模块电源策略重构。
- CH335F sideband 行为。
- 前面板按键任务功能扩展。

## 需求

### MUST

- `LCD_DC/LCD_MOSI/LCD_SCLK/LCD_BLK` 使用 V3 网表定义的 MCU GPIO。
- `LCD_RST/RES` 优先由前面板 `TCA6408A@0x21` 的 P5 输出控制，`LCD_CS` 优先由 P6 输出控制。
- MCU 侧 `GPIO13/GPIO14` 必须保持默认未驱动状态；仅当前面板 `TCA6408A@0x21` 在显示控制初始化前不 ACK 且 reset reason 明确为 `ChipPowerOn` 时，固件可启用 `GPIO13/GPIO14` 分别兜底驱动 `LCD_CS` 与 `LCD_RST/RES`；若任一 TCA 显示控制初始化尝试进入 partial failure，GPIO13/GPIO14 必须保持高阻。
- `LCD_BLK` 必须按低有效背光使能处理。
- 主板 `RESET#` 使用 `GPIO35` 主动输出确定电平：低电平复位，高电平释放。
- 前面板 `TCA6408A@0x21` 的 `RESET#` 固定上拉，不由 MCU GPIO 控制；MCU-only reset 不会复位前面板 TCA。
- display config 必须使用 `Orientation::LandscapeSwapped`，并依赖包含该映射修复的 `gc9d01-rs` 版本。
- 当前 V3 硬件无法在固件内硬复位前面板 `TCA6408A@0x21`；若 bus-clear 后只有 `0x21` 不 ACK，固件必须标记 `Warn/FrontPanelOffline` 并继续 runtime，同时禁用前面板输入任务；显示控制只在初始化前 `0x21` 不 ACK且本次为整机冷上电时改用 MCU `GPIO13/GPIO14` fallback，MCU-only reset 与 TCA partial init failure 均不启用 fallback 以避免共享网冲突。
- 未来硬件修订引出前面板 TCA `RESET#` 或 VCCP 控制后，应撤销当前 V3 降级路径，改为硬复位恢复并要求 `0x21` 在线。

### SHOULD

- 固件日志应明确当前显示控制路径为 front-panel TCA CS/RES 或 MCU GPIO13/GPIO14 fallback。
- 项目文档应标出 `SPI_BLK` 的低有效语义。

## 功能与行为规格

- 启动时固件先尝试初始化 I2C 与前面板 `TCA6408A@0x21` 的显示控制输出；若初始化前探测不到 `0x21` 且 reset reason 明确为 `ChipPowerOn`，则启用 MCU `GPIO13/GPIO14` fallback，再初始化 SPI2 与 GC9D01 160x50 panel；若任一尝试发生 partial failure，则不启用 MCU fallback。若早期显示路径为 `Unavailable` 且后续前面板探测恢复在线，固件重配 TCA P5/P6 并重试 LCD 初始化；若早期已启用 MCU fallback，本次启动保持该路径。
- 前面板 TCA 的 `CS` 作为慢速屏幕使能闸门使用：固件在 LCD 初始化前拉低并保持，不得在每个 SPI 事务内通过 I2C 翻转。
- MCU fallback 路径下，`GPIO13` 作为低有效 `CS`，`GPIO14` 作为低有效 `RES`。
- `BLK` 输出低电平后背光打开。
- framebuffer 以 160x50 逻辑尺寸渲染，driver 负责 `LandscapeSwapped` 到物理坐标的转换。
- 当前 V3 硬件下，`TCA6408A@0x21` 在线时同时影响显示 CS/RES 与前面板按键任务；离线时前面板按键不可用，`ChipPowerOn` 启动中显示改用 MCU fallback，固件仍记录 `Warn/FrontPanelOffline` 并继续 runtime。

## 验收标准

- Given V3 前面板与显示屏焊接正确，When 固件启动，Then 背光点亮且 LCD 初始化日志显示 `tca cs/res, landscape-swapped` 或 `mcu cs/res fallback, landscape-swapped`。
- Given dashboard 正常刷新，When 观察屏幕方向，Then 画面相对旧 `Landscape` 配置旋转 180 度。
- Given 前面板 `TCA6408A@0x21` 在线，When 固件完成自检，Then 日志报告前面板 online 并继续 dashboard。
- Given 前面板 `TCA6408A@0x21` 在显示控制初始化前不 ACK，When 当前 V3 硬件整机冷上电启动自检，Then 日志报告 `Warn/FrontPanelOffline`，不启动按键任务，显示使用 MCU fallback 并继续 runtime。

## 实现前置条件

- V3 网表已导入仓库：`docs/hardware/mainboard_netlist.enet.enet` 与 `docs/hardware/front_panel_netlist.enet.enet`。
- `gc9d01-rs` 的 `LandscapeSwapped` 坐标映射修复已合并到 driver `main`。

## 非功能性验收 / 质量门槛

### Testing

- `cargo +esp check`
- `cargo +esp build --release`

### UI / Visual Evidence

- 使用 dashboard preview 生成 160x50 正常态与混合状态预览。
- PR 阶段展示至少一张稳定快照图。

## Visual Evidence

正常态预览：

![Dashboard 160x50 normal](../../assets/dashboard_wireframe_160x50_color_bold.svg)

混合状态预览：

![Dashboard 160x50 states](../../assets/dashboard_wireframe_160x50_states_color_bold.svg)

## 文档更新

- `docs/esp32-s3fh4r2_gpio_assignment_guide.md`
- `docs/specs/j6nvw-hardware-v3-pin-assignment/IMPLEMENTATION.md`
- `docs/specs/j6nvw-hardware-v3-pin-assignment/HISTORY.md`

## 实现里程碑

- [x] M1: 建立 V3 pin assignment canonical spec，并保留 legacy plan 删除确认边界
- [x] M2: 显示背光、方向和 driver submodule 指针对齐
- [ ] M3: 完成其余 V3 pinmap 历史文档清理与 legacy `docs/plan/**` 删除

## 风险 / 开放问题 / 假设

- 风险：若后续硬件修改 `BLK` 驱动拓扑，需要同步调整固件极性和文档。
- 开放问题：legacy `docs/plan/j6nvw-hw-v3-pin-assignment/**` 删除需要主人确认。
- 假设：V3 显示连接器定义正确，固件只需对齐 GPIO、极性、方向和 driver 版本。

## 参考

- `docs/hardware/mainboard_netlist.enet.enet`
- `docs/hardware/front_panel_netlist.enet.enet`
- `docs/plan/j6nvw-hw-v3-pin-assignment/PLAN.md`
- `docs/plan/j6nvw-hw-v3-pin-assignment/hardware_v3_pin_assignment.md`
