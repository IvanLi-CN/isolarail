# 硬件 V3 调整与固件适配计划（#xhn4c）

## 状态

- Status: 部分完成（2/3）
- Created: 2026-02-25
- Last: 2026-03-14

## 背景 / 问题陈述

- 现有文档与固件实现基于“硬件 V2”假设（见 `docs/hardware_connection_overview.md`、`docs/software_design.md`）。
- 硬件即将演进到 V3，若不先冻结差异与适配边界，固件改动容易出现反复与回归。
- 本规格用于先锁定 V3 调整计划，再进入实现。

## 目标 / 非目标

### Goals

- 建立 V2 -> V3 的硬件差异清单（引脚、I2C 地址、电源链路、外设连接、极性与阈值）。
- 明确固件需要调整的模块、顺序与验收标准。
- 输出可执行的实施里程碑与验证清单，供后续实现直接落地。

### Non-goals

- 不在本规格阶段改动运行时代码与硬件参数寄存器。
- 不改动 CI 目标平台与发布流程。
- 不重构与 V3 无关的功能模块。

## 范围（Scope）

### In scope

- 盘点并确认 V3 硬件变化点（至少覆盖输入电源链路、PCA9545A 下行通道、TCA6408A 前面板、显示与蜂鸣器、保护/监测链路）。
- 形成固件改动映射（受影响文件、配置常量、初始化顺序、日志口径）。
- 定义最小回归验证集合（编译、静态检查、上电序列、关键日志断言）。
- 明确需要同步更新的文档项与验收证据。

### Out of scope

- PCB 设计文件本身（原理图/布局）的编辑。
- 量产参数最终定版（若与 V3 适配无直接耦合）。
- 新功能需求（如 UI 全新交互、协议新增特性）

## 需求（Requirements）

### MUST

- 输出“V2 -> V3 硬件差异表”，字段至少包含：信号名、V2、V3、影响模块、风险等级。
- 输出“固件适配任务矩阵”，字段至少包含：任务、影响文件、前置条件、验收方式。
- 明确并保留项目硬约束：不得修改 `vbus_ratio`（RATIO 0x08 bit0）。
- 在进入实现前冻结验收标准（核心路径 + 关键异常路径）。

### SHOULD

- 按模块切片组织实现顺序（电源输入 -> I2C 拓扑 -> 模块初始化 -> 前面板/显示）。
- 给出“可回滚策略”（每个切片可独立回退）。
- 对可能存在版本差异的地址/引脚提供“探测 + 日志证据”策略。

### COULD

- 给出 V3 bring-up 现场调试清单（串口关键词、故障定位顺序）。
- 给出兼容期策略（允许 V2/V3 共存时的识别分支）。

## 功能与行为规格（Functional/Behavior Spec）

### Core flows

- 收到 V3 硬件变更输入（原理图/BOM/连线说明）后，整理成结构化差异表。
- 基于差异表生成固件改动计划与验收项，形成可执行里程碑。
- 基于已冻结的 V3 差异，完成固件引脚与文档更新，并在进入台架验证前保留可审阅证据。

### Edge cases / errors

- 若 V3 信息不完整（缺少引脚或地址），保持 `待设计` 并阻断实现。
- 若某项变化与当前约束冲突（例如触及禁写寄存器策略），必须先升级为决策问题。
- 若发现 V2/V3 同时需支持，必须显式标注兼容策略与失效边界。

## 接口契约（Interfaces & Contracts）

None

## 验收标准（Acceptance Criteria）

- Given 已提供 V3 变更信息，When 完成计划文档，Then 存在可审阅的差异表与任务矩阵，且覆盖所有受影响子系统。
- Given 准备进入实现，When 审阅本规格，Then 可直接得到范围、里程碑、验证方法与风险项，无需二次猜测。
- Given 存在未确认信息，When 状态检查，Then `Status` 维持 `待设计` 且开放问题列表非空。

## 实现前置条件（Definition of Ready / Preconditions）

- 主人确认 V3 硬件变更输入源（至少一份可信清单）。
- V2 -> V3 差异表已补齐并通过一次评审。
- 验收标准覆盖启动路径、I2C 设备发现、通道配对异常、关键日志。
- 文档同步范围已确认（README 与设计文档）。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Unit tests: 当前仓库未建立稳定单测基线，本规格阶段不新增。
- Integration tests: 以台架上电与串口日志核验为主。
- E2E tests (if applicable): 不适用。

### Quality checks

- Lint / formatting: `cargo +esp fmt --all -- --check`、`cargo +esp clippy --all-targets -- -D warnings`。
- Build: `cargo check` 与 `cargo build --release`。

## 文档更新（Docs to Update）

- `docs/hardware_connection_overview.md`：补充 V3 连接与地址/极性差异。
- `docs/software_design.md`：同步初始化流程、阈值与日志口径变化。
- `README.md`：更新硬件版本说明与文档索引。

## 计划资产（Plan assets）

- Directory: `docs/specs/xhn4c-hardware-v3-adjustments/assets/`

## 资产晋升（Asset promotion）

None

## 实现里程碑（Milestones / Delivery checklist）

- [x] M1: 冻结 V2 -> V3 硬件差异表并完成主人确认。
- [x] M2: 完成受影响模块的固件改动与本地质量检查（fmt/clippy/check/build）。
- [ ] M3: 完成台架验证与文档同步，达到本地 PR-ready。

## 当前实现与证据（Implementation snapshot）

- 已落地的 V3 适配：
  - `GPIO35` 作为 I2C 复位脚，并按共享 `RESET#` 语义使用开漏释放；
  - `GPIO17/18/39/40` 切换为 `EN1..EN4` 高有效输出模块控制；
  - `GPIO33/34` 分配给 `UCM_DIN/UCM_DCE`，用于 CH442E USB 通道路由；
  - 前面板五向开关维持 `TCA6408A@0x21`，并补充主板 `TCA6408A@0x20`（`PWREN#/OVCUR#`）文档与网表证据。
- 已同步的文档：
  - `docs/esp32-s3fh4r2_gpio_assignment_guide.md`
  - `docs/hardware_connection_overview.md`
  - `docs/power_management_and_startup_control.md`
  - `docs/software_design.md`
  - `docs/plan/j6nvw-hw-v3-pin-assignment/hardware_v3_pin_assignment.md`
- 本地验证证据：
  - `source ~/export-esp.sh && cargo +esp check`
  - `source ~/export-esp.sh && cargo +esp build --release`
- 尚未完成：
  - 台架上电验证；
  - 串口日志对照 `front_panel`/I2C 设备发现结果的现场证据。

## 方案概述（Approach, high-level）

- 先文档后实现：优先冻结规格与验收口径，避免边改边猜。
- 以“最小可回滚切片”推进实现，降低硬件联调风险。
- 每个切片都要求对应验证证据，最终汇总为 PR-ready 摘要。

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：V3 变更未一次性给全，可能导致计划反复。
- 风险：现场台架供电与实验条件差异影响验证稳定性。
- 需要决策的问题：
  - V3 是否要求与 V2 并行兼容？
  - V3 是否引入新的 I2C 地址或 GPIO 复用策略？
  - V3 是否调整输入阈值/保护策略（与 4.5V 开发阈值策略关系）？
- 假设（需主人确认）：
  - 假设当前仍以 ESP32-S3 为主控与 Embassy 异步框架。
  - 假设暂不改动 CI/workflow，仅聚焦固件与文档适配。

## 变更记录（Change log）

- 2026-02-25: 新建 V3 硬件调整计划规格。
- 2026-03-14: 同步 V3 网表证据与固件引脚映射，完成本地 `cargo +esp check` / `cargo +esp build --release`，状态更新为 `部分完成（2/3）`。
- 2026-03-14: 根据 PR 阶段 review-loop 修正文档保留引脚与 `I2C_RESET` 开漏释放语义。

## 参考（References）

- `docs/hardware_connection_overview.md`
- `docs/plan/j6nvw-hw-v3-pin-assignment/hardware_v3_pin_assignment.md`
- `docs/software_design.md`
- `AGENTS.md`
