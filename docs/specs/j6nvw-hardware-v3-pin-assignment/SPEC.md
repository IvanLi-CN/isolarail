# 硬件 V3 引脚与显示链路对齐（#j6nvw）

## 状态

- Status: 部分完成（2/3）
- Created: 2026-02-06
- Last: 2026-04-27

## 背景 / 问题陈述

- V3 硬件把显示屏 SPI 与控制脚接到 MCU GPIO，前面板 `TCA6408A` 只承载五向按键输入。
- `BLK` 网络连接到前面板 P 沟道背光门极，固件若按高有效驱动会导致背光关闭。
- 160x50 屏幕实装方向需要使用 driver 的 `LandscapeSwapped` 坐标变换；旧 driver 中该方向与 `Landscape` 映射相同，主项目必须引用已修复版本。

## 目标 / 非目标

### Goals

- 固件显示 GPIO、背光极性和屏幕方向与 V3 网表及实物行为一致。
- 主项目引用已合并的 `gc9d01-rs` driver 修复版本。
- 保留 V3 pin assignment 的 canonical spec 入口，后续实现与文档以本目录为准。

### Non-goals

- 不改变前面板连接器定义。
- 不把显示 CS/RES 迁回 `TCA6408A` 控制。
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

- `LCD_DC/LCD_MOSI/LCD_SCLK/LCD_CS/LCD_RST/LCD_BLK` 使用 V3 网表定义的 MCU GPIO。
- `LCD_BLK` 必须按低有效背光使能处理。
- 主板 `RESET#` 使用 `GPIO35` 主动输出确定电平：低电平复位，高电平释放。
- 前面板 `TCA6408A@0x21` 的 `RESET#` 固定上拉，不由 MCU GPIO 控制。
- display config 必须使用 `Orientation::LandscapeSwapped`，并依赖包含该映射修复的 `gc9d01-rs` 版本。
- 前面板 `TCA6408A@0x21` 离线必须阻塞启动自检，直到按键输入扩展器可达。

### SHOULD

- 固件日志应明确当前显示控制路径为 MCU CS/RST。
- 项目文档应标出 `SPI_BLK` 的低有效语义。

## 功能与行为规格

- 启动时固件先初始化 SPI2 与显示 GPIO，然后初始化 GC9D01 160x50 panel。
- `BLK` 输出低电平后背光打开。
- framebuffer 以 160x50 逻辑尺寸渲染，driver 负责 `LandscapeSwapped` 到物理坐标的转换。
- `TCA6408A@0x21` 是进入 dashboard 的必需启动条件；离线时 LCD 停留在系统自检页。

## 验收标准

- Given V3 前面板与显示屏焊接正确，When 固件启动，Then 背光点亮且 LCD 初始化日志显示 `mcu cs/rst, landscape-swapped`。
- Given dashboard 正常刷新，When 观察屏幕方向，Then 画面相对旧 `Landscape` 配置旋转 180 度。
- Given 前面板 `TCA6408A@0x21` 在线，When 固件完成自检，Then 日志报告前面板 online 并继续 dashboard。
- Given 前面板 `TCA6408A@0x21` 离线，When 固件启动自检，Then LCD 停留在 `PANEL PEND` 并持续重试，不进入 dashboard。

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
