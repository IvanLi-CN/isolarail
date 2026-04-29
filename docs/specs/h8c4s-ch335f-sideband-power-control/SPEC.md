# CH335F Sideband Power Control (#h8c4s)

## 状态

- Status: 部分完成（2/3）
- Created: 2026-04-28
- Last: 2026-04-29

## 背景 / 问题陈述

- 主板 `TCA6408A@0x20` 连接 CH335F 的四路 `PWREN#` 与 `OVCUR#` sideband。
- 固件当前只探测该 TCA 是否在线，没有把 CH335F 端口使能状态纳入四路输出 `EN` 门控，也没有向 CH335F 注入过流状态。
- USB-C 真实过流状态暂不可直接读取，运行期需先用输出模块 INA226 的电压/电流估算保护状态。

## 目标 / 非目标

### Goals

- 初始化主板 `TCA6408A@0x20`，读取 CH335F `PWREN1#..4#` 并转换为每路端口使能布尔状态。
- 用 `OVCUR1#..4#` 向 CH335F 注入过流：释放为输入高阻，告警为输出低。
- 四路 MCU `EN1..EN4` 由 `VIN ready`、主板 sideband 在线、`GPIO21/V1OK` 模式、`PWREN#` 与 `ocp_latched` 决定。
- 运行期按 INA226 读数估算过流并锁存，连续安全周期后释放。

### Non-goals

- 不修改 CH335F EEPROM、USB 拓扑或 CI target。
- 不实现 USB-C 协议层真实过流读取。
- 不修改 `vbus_ratio` 或 RATIO `0x08 bit0`。

## 范围

### In scope

- `src/main.rs` 启动门控、运行期采样与日志。
- 新增主板 TCA sideband helper。
- `docs/software_design.md` 同步运行期契约。

### Out of scope

- Host 端 CH335F 控制程序产品化。
- 输出模块硬件限流参数重标定。

## 需求

### MUST

- P0/P2/P4/P6 必须保持输入，用于读取低有效 `PWREN1#..4#`。
- P1/P3/P5/P7 必须按“输出低=OVCUR asserted，输入高阻=OVCUR released”处理。
- `TCA6408A@0x20` 离线时四路 `EN` 必须保持关闭。
- `GPIO21/V1OK` 为低时必须进入 standalone 模式，允许产品在未连接上游电脑时独立输出。
- 过流判定默认：`vbus < 3.0 V && current > 0.1 A`，或 `current > 5.3 A`。
- 命中过流必须立即关闭对应 `ENx` 并拉低对应 `OVCUR#`。

### SHOULD

- 释放过流需连续 4 个运行期采样周期都安全，避免抖动。
- 日志包含每路 `pwren/en/ocp` 状态，便于串口验证。

## 功能与行为规格

- 启动时在 VIN ready 后初始化 `TCA6408A@0x20`：输出寄存器先写 `0xFF`，极性寄存器写 `0x00`，方向寄存器写 `0xFF`，默认全部高阻释放。
- 若主板 TCA 在线且 `V1OK=low`，启动门控进入 standalone 模式，端口输出不因 `PWREN#` 为高而关闭。
- 若主板 TCA 在线且 `V1OK=high`，启动门控进入 host-managed 模式，仅对 CH335F 已使能且未过流的端口拉高 `ENx`。
- 若主板 TCA 离线，四路端口保持关闭，boot self-check 记录 degraded 端口故障。
- 运行期每 500 ms 复用现有 INA226 采样；安全读数更新 dashboard，过流读数进入 `Overcurrent` UI 状态。
- 过流 latch 清除条件为恢复探测期间输出带电时连续 4 个安全采样周期；读取失败或输出关闭后的 0V/0mA 不清除 latch。

## 验收标准

- Given 固件启动且主板 TCA 在线，When 初始化完成，Then 日志出现 `hub.sideband: tca6408a=online addr=0x20` 与寄存器初始化成功。
- Given `V1OK=low` 且主板 TCA 在线，When 固件启动或 runtime 刷新，Then 日志中 `hub_mode=standalone`，四路输出不因 `pwren=off` 关闭。
- Given `V1OK=high` 且 CH335F 禁用某一路端口，When 固件读取 `PWREN#`，Then 对应 `ENx` 为低且日志中该路 `pwren=off en=off`。
- Given 某路 INA226 读数满足过流阈值，When runtime 刷新，Then 对应 `OVCUR#` 被拉低、`ENx` 为低、UI 状态为 `cc`。
- Given 过流后进入恢复探测且连续 4 个带电周期读数安全，When runtime 刷新，Then 对应 `OVCUR#` 释放，若 `PWREN#` 仍 enabled 则 `ENx` 保持高。

## 实现前置条件

- V3 网表已确认 `TCA6408A@0x20` 的 P0/P2/P4/P6 为 `PWREN#`，P1/P3/P5/P7 为 `OVCUR#`。
- 输出模块 INA226 地址表已在固件中固定为 `0x40..0x43`。

## 非功能性验收 / 质量门槛

### Testing

- `cargo +esp check`
- `cargo +esp build --release`
- 真机 `mcu-agentd flash` 与 `mcu-agentd monitor` 验证可用时执行。

### UI / Visual Evidence

- LCD dashboard 仅复用既有 `Overcurrent` 状态，不改变视觉布局。

## 文档更新

- `docs/software_design.md`
- `docs/specs/README.md`

## 实现里程碑

- [x] M1: TCA6408A sideband helper 与启动初始化
- [x] M2: PWREN/OVCUR/EN runtime 门控与过流 latch
- [ ] M3: 文档同步、构建与真机验证

## 风险 / 开放问题 / 假设

- 风险：若 CH335F host 控制程序不可发现，只能完成固件与串口侧验证，不能自动切换电脑端端口。
- 当前硬件缺陷：`PWREN1#` / `PWREN2#` 连接错误已登记为 GitHub issue #18。
- 当前验证缺口：物理 OCP 未用高电流或低压负载夹具强制触发。
- 假设：IP6557 标称 `28V5A`，固件软件硬阈值 `5.3A` 作为保护余量。

## 参考

- `docs/ch335f_tca6408a_appnote.md`
- `docs/hardware/mainboard_netlist.enet.enet`
- `docs/software_design.md`
