# 硬件 V3 调整与引脚分配文档（#j6nvw）

## 状态

- Status: 待设计
- Created: 2026-02-06
- Last: 2026-02-06

## 背景 / 问题陈述

- 现状：仓库现有硬件文档以 v2 为主（例如 `docs/hardware_connection_overview.md`），固件侧也有一套基于当前板子的 GPIO/I2C 约定（例如 `docs/esp32-s3fh4r2_gpio_assignment_guide.md`、`src/main.rs`）。
- 问题：硬件即将进入 V3，若缺少“单一可信的引脚/地址/信号方向清单”，固件实现与联调阶段容易出现：引脚打架、极性误解、I2C 地址冲突/误配、文档与代码不一致等问题。
- 目标：在进入实现阶段前，补齐并冻结 V3 的关键硬件调整口径，同时产出一份完整的引脚分配文档，作为实现与联调的依据。

## 目标 / 非目标

### Goals

- 明确并记录硬件 V3 的“调整清单”（信号/器件/地址/极性/接口等），并形成可持续维护的文档结构。
- 产出并维护 `./hardware_v3_pin_assignment.md`：覆盖 MCU GPIO、I2C 拓扑与地址、关键控制信号的方向/极性/上拉等。
- 在进入实现阶段前，确保“文档与固件约定”一致（至少在 pin/addr 级别无冲突），避免边实现边修口径。

### Non-goals

- 不在本计划阶段修改实现代码/Runner/CI 工作流（CI 仍可能是 legacy STM32 配置；未经授权不调整）。
- 不在本计划阶段做大规模重构或新增依赖。

## 范围（Scope）

### In scope

- V3 调整清单（逐项记录：变更原因、影响范围、涉及信号/器件/地址、需要同步的固件点）。
- 引脚分配文档：`./hardware_v3_pin_assignment.md`（计划阶段的单一汇总入口）。
- 相关文档的最小同步（仅当有明确且已确认的 V3 变更可落地时）。

### Out of scope

- 实现阶段的固件改动（GPIO 重映射、驱动适配、功能联调）——需在计划冻结后进入实现阶段处理。
- 任何需要硬件/原理图尚未确认的“拍脑袋结论”。

## V3 调整清单（Inventory，待补齐）

说明：

- 本节用于随讨论逐项补齐；在计划进入 `待实现` 前，需要将本表冻结（不再包含 `TBD`）。
- 如果某项变更会影响 pin/addr/极性，请同步落到 `./hardware_v3_pin_assignment.md`，避免“只写在这里但没进入汇总表”。

| # | 模块/信号 | V2 现状（baseline） | V3 调整 | 影响（firmware/docs） | 确认来源（schematic/pcb/measure） | 状态 |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | PSTOP_CTL[1..4] / PSTOP* 网络 | 通过 `PSTOP_CTLx`（MCU）→ 反相 → 模块侧 `PSTOP`（低有效）控制 4 路子板功率级 | V3 不再使用 PSTOP* 网络；撤销 MCU 侧相关引脚分配（GPIO17/18/39/40 释放） | docs: `./hardware_v3_pin_assignment.md`；firmware: 后续实现阶段移除/替换对应控制逻辑（若有新方案需再分配） | Owner decision（本对话确认） | 已确认 |
| 2 | V3 其它硬件改动总览 | - | TBD | docs: PLAN + pinmap | TBD | 待补齐 |

## 需求（Requirements）

### MUST

- 在计划进入 `待实现` 前，V3 的引脚分配文档中不得存在 `TBD`/空白项。
- 引脚分配文档必须包含（至少）：
  - MCU GPIO → 功能信号映射（方向/极性/上拉等关键电气约束）
  - I2C 上行设备地址（含可配置/需确认项的标注）
  - 通过 PCA9545A 下行的 4 路通道中设备地址（用于固件扫描/初始化）
  - 关键控制信号（如 `IN_EN`、`IN_PG`、`HUB_RESET#`，以及 V3 将采用的“4 路子板启停/使能”控制信号）的“逻辑极性”说明
- 文档中明确：哪些信息来自“V3 确认资料”（原理图/PCB/实测），哪些仍需确认（在 `待设计` 阶段允许存在，但进入 `待实现` 前必须清零）。

### SHOULD

- 明确 V3 相对 V2 的差异列表（新增/删除/修改项），并指向对应的文档/实现影响点。
- 对可能踩坑的引脚（Strapping/JTAG/USB 专用等）给出警示与约束。

### COULD

- 追加一份机器可读的 pinmap（例如 CSV/JSON），便于后续生成代码或做一致性校验（仅在主人需要时）。

## 接口契约（Interfaces & Contracts）

None（计划阶段为文档与口径冻结；不新增对外接口契约）。

## 验收标准（Acceptance Criteria）

- Given 已确认的硬件 V3 原理图/连线/地址绑法，
  When 工程师查阅 `./hardware_v3_pin_assignment.md`，
  Then 可以在不反复翻原理图的情况下，明确所有固件需要使用的 GPIO/I2C 地址/信号方向与极性，且文档中不存在 `TBD`。

- Given 本计划 `Status` 进入 `待实现`，
  When 进入实现阶段，
  Then 固件实现不再需要“猜测/回填”硬件口径，且文档可作为 code review 的对照依据。

## 实现前置条件（Definition of Ready / Preconditions）

- 已拿到并确认 V3 的关键资料（至少包含：MCU 相关 nets、I2C 地址绑法、关键控制信号极性）。
- `./hardware_v3_pin_assignment.md` 已补齐且无 `TBD`。
- V3 调整清单已由主人确认（哪些要做、哪些不做、哪些延后）。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Unit tests: N/A（本计划阶段不涉及代码实现）
- Integration tests: N/A
- E2E tests: N/A

### Quality checks

- Markdown lint: `markdownlint-cli2`（按仓库既有 lefthook 约定）

## 文档更新（Docs to Update）

- `docs/plan/j6nvw-hw-v3-pin-assignment/hardware_v3_pin_assignment.md`: 新增并持续维护（V3 引脚/地址单一汇总入口）
- `docs/hardware_connection_overview.md`: 如需保留 v2 作为历史参考，建议后续新增 v3 版本并在实现阶段同步索引（待确认）
- `docs/esp32-s3fh4r2_gpio_assignment_guide.md`: 若 V3 变更涉及 GPIO 映射，需同步修正并避免出现自相矛盾的描述（待确认）

## 计划资产（Plan assets）

- None

## 资产晋升（Asset promotion）

None

## 实现里程碑（Milestones）

（实现阶段交付物，待主人明确“V3 需要落地到固件的范围”后补齐。）

- [ ] M1: 固件侧按 V3 pinmap 更新 GPIO/I2C 配置与初始化流程（范围待定）
- [ ] M2: 在 V3 实物上完成一次 `cargo build --release` + 基础联调日志验证（若硬件可用）
- [ ] M3: 同步/补齐相关硬件与软件设计文档（仅涉及本计划范围）

## 方案概述（Approach, high-level）

- 以 `./hardware_v3_pin_assignment.md` 为“单一真相入口”，其它文档按需引用/拆分，但避免多处重复维护同一份 pinmap。
- V3 调整清单按“可落实项”逐条记录：变更 → 影响 → 需要同步的固件点 → 验证方式。
- 在进入实现前，先做一次“文档 ↔ 代码常量”一致性自检（至少检查 GPIO 号与 I2C 地址是否冲突）。

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：V3 硬件口径未冻结导致 pinmap 频繁变动，进而影响固件并行开发节奏。
- 需要决策的问题：
  - V3 相对 V2 的具体调整项清单（请主人逐项给我，我会填入并维护）
  - 是否需要为 V3 新增独立的硬件连接概览文档（v2 保留不动）
- 假设（需主人确认）：
  - V3 仍以 ESP32-S3FH4R2 为主控，且基本功能模块（PCA9545A/TCA6408A/INA226/四路子板）仍保留（如有变更请指出）

## 变更记录（Change log）

- 2026-02-06: 初始化计划（创建计划目录与 V3 pinmap 文档入口）

## 参考（References）

- `docs/hardware_connection_overview.md`（当前为 v2）
- `docs/esp32-s3fh4r2_gpio_assignment_guide.md`（当前 GPIO 约定）
- `src/main.rs`、`src/power_in.rs`（当前固件常量与 INA226 地址约定）
