# 固件健壮化与开机自检（#5f74j）

## 状态

- Status: 部分完成（3/4）
- Created: 2026-03-14
- Last: 2026-03-14

## 背景 / 问题陈述

- 当前固件在关键启动探测失败时会直接 `panic!`，例如 `PCA9545A` 不在线时会在启动早期崩溃，导致后续诊断信息丢失。
- 项目需要像 `mains-aegis` 一样，把启动失败转化为可见、可定位、可门控的自检结果，而不是“黑屏 + 无日志”。
- 不做本改造的代价是：真机联调时缺少稳定的故障定位界面，串口监视窗口稍晚接入就会丢掉启动瞬间证据。

## 目标 / 非目标

### Goals

- 将启动过程重构为 `Early Bring-up -> Self-Check -> Gate Apply -> Runtime` 四阶段。
- 引入统一的 boot self-check 状态模型、故障码、门控决策和摘要页口径。
- 缺模块时优先降级继续运行；只有输入电源不安全、无法保证端口关闭、或明确保护粘滞时才进入 fatal 停留态。
- 在 160x50 LCD 上显示最小自检摘要页，并在 degraded/fatal 场景保留可复查的结果。

### Non-goals

- 不实现 `mains-aegis` 的多页面自检 UI、音频 cue、BMS 交互或复杂激活流程。
- 不改硬件地址、原理图、host 侧 `mcu-agentd` monitor 产品化体验。

## 范围（Scope）

### In scope

- `src/main.rs` 启动状态机、boot self-check UI、自检日志前缀、门控策略。
- `src/power_in.rs` 启动电源探测改成“上报结果而不是 panic”。
- `src/boot_diag.rs` 新增统一状态模型。
- `docs/software_design.md` 同步新的开机自检与降级口径。

### Out of scope

- 新增额外板级硬件 kill 信号支持。
- 新的远程诊断工具或 PC 端 UI。

## 需求（Requirements）

### MUST

- 启动期关键检查必须输出 `boot.stage:*`、`boot.check:*`、`boot.summary:*` 日志。
- `PCA9545A` 缺失时不得再直接 `panic!`；应记录 `MuxOffline` 并禁用所有下游端口初始化。
- 输入电源链路异常时必须保持 `IN_EN` 关闭，并停留在 fatal 自检页。
- 前面板缺失时不得阻断 dashboard 与其余链路运行。
- 单路 `SC8815/SW2303` 异常时只能关闭该路功率级，不连坐其它路。

### SHOULD

- 自检页在启动中显示进度，并在 degraded 场景展示失败摘要后再进入 dashboard。
- 自检页的系统面和端口面使用同一套状态/故障码口径。

### COULD

- 后续将 boot self-check 快照扩展为 runtime overlay 或更丰富的维护页。

## 功能与行为规格（Functional/Behavior Spec）

### Core flows

- 固件初始化时先建立显示与基础 GPIO，再进入 `Self-Check` 阶段。
- `Self-Check` 固定顺序为：MUX -> VIN/INA226 -> Front Panel -> Fan -> 4 路端口。
- 通过 `GateDecision` 决定是否放行 runtime task、front panel 和各端口。
- `BootOutcome=Fatal` 时常驻自检页；`BootOutcome=Degraded` 时展示摘要后进入 dashboard；`BootOutcome=Ok` 时直接切换 dashboard。

### Edge cases / errors

- MUX 离线：标记系统项 `Err/MuxOffline`，四路端口全部 `Skipped`，PSTOP 保持关闭。
- VIN 不可用或 PG 不良：标记系统项 `Fatal`，不进入端口初始化，不放行 runtime。
- Front panel 离线：标记 `Warn/FrontPanelOffline`，只禁用 panel 功能。
- 单路端口 `VBUS ready` 超时、SC/SW 缺失或配对异常：标记对应端口 `Err`，并关闭该路 PSTOP。
- 明确的端口保护粘滞：标记该路 `Fatal`，整机停留在 fatal 自检页。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `boot_diag` 状态模型 | internal | internal | New | None (this SPEC) | firmware | boot flow / LCD boot page | 新增内部状态与门控口径 |
| `power_in::bootstrap_signal()` | internal | internal | New | None (this SPEC) | firmware | boot flow | 提供启动期电源探测结果 |

### 契约文档（按 Kind 拆分）

None

## 验收标准（Acceptance Criteria）

- Given 板子正常上电，When 固件启动，Then LCD 先显示自检页，串口输出 `boot.stage:*` 与 `boot.check:*`，最终输出 `boot.summary: outcome=OK|DEG` 并进入 dashboard。
- Given `PCA9545A` 缺失，When 固件启动，Then 不发生启动 `panic!`，LCD 与日志都显示 `MuxOffline`，四路端口保持关闭。
- Given 前面板 `TCA6408A` 缺失，When 固件启动，Then front panel 功能被禁用，但 dashboard 仍可运行。
- Given 单路 SC/SW 异常或 `VBUS ready` 超时，When 固件启动，Then 仅该路被门控关闭，其余路按结果继续运行。
- Given 输入电源资格失败或 PG 不良，When 固件启动，Then `IN_EN` 保持关闭且 LCD 常驻 fatal 自检页。

## 实现前置条件（Definition of Ready / Preconditions）

- 目标、非目标、范围与 fatal/degraded 判据已冻结。
- 关键内部接口与自检口径已在本 SPEC 确定。
- 验收标准覆盖正常启动、MUX 离线、VIN 异常、front panel 离线、单路异常几种核心场景。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Unit tests: None
- Integration tests: None
- E2E tests (if applicable): 真机串口日志与 LCD 行为验证至少 1 轮

### UI / Storybook (if applicable)

- Stories to add/update: None
- Visual regression baseline changes (if any): None

### Quality checks

- `cargo check`
- `cargo build --release`

## 文档更新（Docs to Update）

- `docs/software_design.md`: 同步 boot self-check、门控、fatal/degraded 口径。
- `docs/specs/README.md`: 增加本规格索引。

## 计划资产（Plan assets）

None

## Visual Evidence (PR)

- 真机串口日志截图或 LCD 自检页照片在 PR 阶段补入 `./assets/`。

## 资产晋升（Asset promotion）

None

## 实现里程碑（Milestones / Delivery checklist）

- [x] M1: 落地 boot self-check 状态模型、阶段日志与门控框架
- [x] M2: 启动流程改为降级而非 panic，并补齐 VIN/MUX/front panel/port 判定
- [x] M3: 落地 LCD boot self-check 摘要页与 degraded/fatal 切换行为
- [ ] M4: 更新软件设计文档并完成构建/真机验证证据

## 方案概述（Approach, high-level）

- 参考 `mains-aegis/docs/boot-self-test-flow.md` 的“默认只读探测 + 非紧急不乱改输出 + 紧急才阻断”方法，但只保留本仓库所需的最小实现。
- 用统一 `BootSelfCheckSnapshot` 驱动日志、自检页和启动后门控，避免每个模块各说各话。
- 用最小的两页轮转自检 UI 承载系统项和端口项，不新增复杂交互。

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：现有监视工具可能仍会错过启动瞬间日志，因此需要依赖固件自身 LCD 摘要页兜底。
- 需要决策的问题：None
- 假设（需主人确认）：当前可观测的 fatal 条件以输入电源不安全和端口保护粘滞为主。

## 变更记录（Change log）

- 2026-03-14: 初版规格，冻结 boot self-check 与分级降级方案。
- 2026-03-14: 启动状态机、boot self-check 页、输入电源 bootstrap 结果上报与门控框架已落地，等待真机证据补齐。

## 参考（References）

- `docs/software_design.md`
- `/Users/ivan/Projects/Ivan/mains-aegis/docs/boot-self-test-flow.md`
