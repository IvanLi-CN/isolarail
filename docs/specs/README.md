# 规格（Spec）总览

本目录用于管理工作项的**规格与追踪**：记录范围、验收标准、任务清单与状态，作为交付依据；实现与验证应以对应 `SPEC.md` 为准。

当前仓库的软件与硬件命名真相源由 `pw97u-control-plane-alignment/SPEC.md` 统一定义：

- 固件 package / identity：`isolarail`
- repo JS tooling package：`isolarail-dev-tools`
- CLI：`isolarail`
- daemon：`isolarail-devd`
- companion workspace：`tools/isolarail-companion/`
- internal support crates / tools：`gc9d01`、`dashboard_preview`、`icon2raw`、`png2raw`
- vendored example packages：`gc9d01/examples/**` 下的示例包名仅用于驱动示例，不得进入本项目 owner-facing 命名
- 开发者入口：`just`
- owner-facing 端口模型：`port1..port4`
- 当前硬件基线：`V3`
- 主控与关键子系统名：`ESP32-S3`、`CH335F`、`M24C64@0x50`、`TPS2490`、`ISOUSB211 V1OK`、`PCA9545A@0x70`、`Mainboard TCA6408A@0x20`、`Front-panel TCA6408A@0x21`
- legacy/scoped hardware names：`SC8815 + SW2303`、`PSTOP_CTL/PSTOP`、`VIN_ADC`、`PCA9545A INT/INTx`、scoped `RESET#` 只允许在硬件拓扑或迁移语境下出现
- 参考硬件资料中的 `TPS82130SILR`、`RT9043GB`、`TCA6408APWR`、`TCA6408ARSVR`、`TCA9535RTWR` 等料号仅允许作为 reference-only 历史输入，不得覆盖当前 V3 控制面命名

后续 spec、README、实现与 UI 文案不得偏离这套命名；BOM/netlist 精确料号只能作为电气资料别名，不能替代控制面 canonical 名称。

> Legacy compatibility: historical repos may still contain `docs/plan/**/PLAN.md`. New entries must be created under `docs/specs/**/SPEC.md`.

## 快速新增一个规格

1. 生成一个新的规格 `ID`（推荐 5 个字符的 nanoId 风格，降低并行建规格时的冲突概率）。
2. 新建目录：`docs/specs/<id>-<title>/`（`<title>` 用简短 slug，建议 kebab-case）。
3. 在该目录下创建 `SPEC.md`（模板见下方“SPEC.md 写法（简要）”）。
4. 在下方 Index 表新增一行，并把 `Status` 设为 `待设计` 或 `待实现`（取决于是否已冻结验收标准），并填入 `Last`（通常为当天）。

## 目录与命名规则

- 每个规格一个目录：`docs/specs/<id>-<title>/`
- `<id>`：推荐 5 个字符的 nanoId 风格，一经分配不要变更。
  - 推荐字符集（小写 + 避免易混淆字符）：`23456789abcdefghjkmnpqrstuvwxyz`
  - 正则：`[23456789abcdefghjkmnpqrstuvwxyz]{5}`
  - 兼容：若仓库历史已使用四位数字 `0001`-`9999`，允许继续共存。
- `<title>`：短标题 slug（建议 kebab-case，避免空格与特殊字符）；目录名尽量稳定。
- 人类可读标题写在 Index 的 `Title` 列；标题变更优先改 `Title`，不强制改目录名。

## 状态（Status）说明

仅允许使用以下状态值：

- `待设计`：范围/约束/验收标准尚未冻结，仍在补齐信息与决策。
- `待实现`：规格已冻结，可开工；实现与测试验证应以该规格为准。
- `跳过`：计划已冻结或部分完成，但**当前明确不应自动开工**（例如需要特定时机/外部条件/等待依赖）；自动挑选“下一个规格”时应跳过它。需要实现时再把状态改回 `待实现`（或由主人显式点名实现该规格）。
- `部分完成（x/y）`：实现进行中；`y` 为该规格里定义的“实现里程碑”数，`x` 为已完成“实现里程碑”数（见该规格 `SPEC.md` 的 Milestones；不要把计划阶段产出算进里程碑）。
- `已完成`：该规格已完成（实现已落地或将随某个 PR 落地）；如需关联 PR 号，写在 Index 的 `Notes`（例如 `PR #123`）。
- `作废`：不再推进（取消/价值不足/外部条件变化）。
- `重新设计（#<id>）`：该规格被另一个规格取代；`#<id>` 指向新的规格编号。

## `Last` 字段约定（推进时间）

- `Last` 表示该规格**上一次“推进进度/口径”**的日期，用于快速发现长期未推进的规格。
- 仅在以下情况更新 `Last`（不要因为改措辞/排版就更新）：
  - `Status` 变化（例如 `待设计` -> `待实现`，或 `部分完成（x/y）` -> `已完成`）
  - `Notes` 中写入/更新 PR 号（例如 `PR #123`）
  - `SPEC.md` 的里程碑勾选变化
  - 范围/验收标准冻结或发生实质变更

## SPEC.md 写法（简要）

每个规格的 `SPEC.md` 至少应包含：

- 背景/问题陈述（为什么要做）
- 目标 / 非目标（做什么、不做什么）
- 范围（in/out）
- 需求列表（MUST/SHOULD/COULD）
- 功能与行为规格（Functional/Behavior Spec：核心流程/关键边界/错误反馈）
- 验收标准（Given/When/Then + 边界/异常）
- 实现前置条件（Definition of Ready / Preconditions；未满足则保持 `待设计`）
- 非功能性验收/质量门槛（测试策略、质量检查、Storybook/视觉回归等按仓库已有约定）
- 文档更新（需要同步更新的项目设计文档/架构说明/README/ADR）
- 实现里程碑（Milestones，用于驱动 `部分完成（x/y）`；只写实现交付物，不要包含计划阶段产出）
- 风险与开放问题（需要决策的点）
- 假设（需主人确认）

## Index（固定表格）

<!-- markdownlint-disable MD060 -->
|    ID | Title      | Status    | Spec                                     | Last       | Notes  |
|------:|------------|-----------|------------------------------------------|------------|--------|
| k3p8m | 示例：新增工作项规格 | 待设计       | `k3p8m-example-spec/SPEC.md`             | YYYY-MM-DD | -      |
| pw97u | 四路 USB Hub 控制面对齐 | 已完成 | `pw97u-control-plane-alignment/SPEC.md` | 2026-06-29 | 当前 HEAD 已完成本地 `PR-ready` 收口：Wi-Fi/LAN、USB CDC、companion、web、current-truth 文档与视觉证据已统一对齐 |
| 5f74j | 固件健壮化与开机自检 | 已完成 | `5f74j-firmware-boot-self-check/SPEC.md` | 2026-03-17 | 当前板型为直连 I2C；保留 mux 槽位以兼容后续 PCA9545A |
| j6nvw | 硬件 V3 引脚与显示链路对齐 | 部分完成（2/3） | `j6nvw-hardware-v3-pin-assignment/SPEC.md` | 2026-04-27 | legacy `docs/plan/j6nvw-hw-v3-pin-assignment/**` 删除待确认 |
| h8c4s | CH335F sideband 电源控制 | 部分完成（2/3） | `h8c4s-ch335f-sideband-power-control/SPEC.md` | 2026-06-13 | 固件与上游侧 p1/p3/p4 控制已验证；命名已对齐到 `isolarail` / 上游侧语义；PWREN1#/2# 硬件缺陷见 issue #18 |
| 7gf6b | 固件蜂鸣器音效 | 已完成 | `7gf6b-firmware-buzzer-audio/SPEC.md` | 2026-07-05 | PR #25 |
| e5nyr | 发布失败 Telegram 告警接入 | 已完成 | `e5nyr-release-failure-telegram-alerts/SPEC.md` | 2026-07-07 | 覆盖范围扩展到 `Docs Pages` 部署失败；GitHub Pages settings 仍需 owner 侧启用 Actions source |
| q9d7h | CLI/devd 烧录迁移 | 部分完成（3/4） | `q9d7h-cli-devd-flash-migration/SPEC.md` | 2026-07-04 | source workflow 已完成，旧烧录入口已退役；真机 flash-monitor 验证待硬件确认 |
| r6wfr | 文档 Web 站点 | 待实现 | `r6wfr-docs-site/SPEC.md` | 2026-07-05 | Rspress/Bun 双语 docs-site，GitHub Pages 发布与视觉证据 |
| n8x4q | 硬件底层详细信息快照接口 | 部分完成（4/5） | `n8x4q-hardware-diag-snapshot/SPEC.md` | 2026-07-05 | 真机串口端到端验证待硬件接入 |
| b8r3n | 品牌视觉资产 | 已完成 | `b8r3n-brand-visual-assets/SPEC.md` | 2026-07-08 | PR #32；Logo、App Icon、海报、GitHub Social preview 主图与变体、HTML 海报素材 |
<!-- markdownlint-enable MD060 -->
