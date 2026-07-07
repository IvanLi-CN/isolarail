---
title: 规格索引
description: 文档站关联的 canonical specs、项目文档与实现状态。
---

<!-- markdownlint-disable MD025 -->

# 规格索引

文档站负责导览和发布，不替代仓库内的 canonical specs。需要判断“现在到底以什么为准”时，优先回到 `docs/specs/**`、`docs/software_design.md` 和 `docs/hardware_connection_overview.md`。

## 当前文档站规格

| ID | 标题 | 状态 | 作用 |
| --- | --- | --- | --- |
| `r6wfr` | 文档 Web 站点 | 首版实现中 | 定义 Rspress/Bun 双语站点、发布 workflow、视觉证据和验收门禁 |

对应目录：

- `docs/specs/r6wfr-docs-site/SPEC.md`
- `docs/specs/r6wfr-docs-site/IMPLEMENTATION.md`
- `docs/specs/r6wfr-docs-site/HISTORY.md`

本规格只定义站点本身：Rspress 结构、双语路由、GitHub Pages workflow、视觉证据和文档边界。它不替代硬件、固件或控制面规格。

## 产品与控制面规格

| ID | 标题 | 状态 | 站点关联 |
| --- | --- | --- | --- |
| `pw97u` | 四路 USB Hub 控制面对齐 | 已完成 / 部分追踪 | 固件身份、CLI/daemon 命名、USB JSONL、HTTP、Web app、port model |
| `q9d7h` | CLI/devd 烧录迁移 | 部分完成 | `isolarail`、`isolarail-devd`、身份校验、烧录与 monitor 路径 |
| `h8c4s` | CH335F sideband 电源控制 | 部分完成 | `PWREN#`、`OVCUR#`、runtime 门控与 OCP latch |
| `7gf6b` | 固件蜂鸣器音效 | 已完成 | GPIO7 LEDC PWM、音效优先级、告警循环和站内音效预览 |

这些规格解释控制面为什么叫 `isolarail` / `isolarail-devd`，以及为什么 owner-facing 端口固定为 `port1..port4`。

阅读建议：

1. 先读 `pw97u`，理解 owner-facing 名称、端口模型和接口边界。
2. 再读 `q9d7h`，理解为什么烧录、reset、monitor 必须走 `isolarail` / `isolarail-devd`。
3. 最后读 `h8c4s`，把 `PWREN#`、`OVCUR#`、OCP latch 和 dashboard 状态连起来。
4. 修改提示音时读 `7gf6b`，并使用[蜂鸣器音效预览](../firmware/buzzer-audio-preview)
   页面检查时序。

## 硬件与固件规格

| ID | 标题 | 状态 | 站点关联 |
| --- | --- | --- | --- |
| `5f74j` | 固件健壮化与开机自检 | 已完成 | boot self-check、degraded/fatal、LCD 自检页 |
| `j6nvw` | 硬件 V3 引脚与显示链路对齐 | 部分完成 | GPIO、显示链路、背光极性、front panel 降级边界 |

阅读建议：

1. 先读 `docs/hardware_connection_overview.md`，建立 V3 板级拓扑。
2. 再读 `j6nvw`，确认 GPIO、显示、背光和前面板 reset 边界。
3. 再读 `5f74j`，确认 boot self-check 为什么会 degraded 而不是 panic。
4. 如果要调端口电源，再回到 `h8c4s` 看运行期 sideband 门控。

## Current-truth 文档

- `docs/hardware_connection_overview.md`：当前 V3 硬件总览。
- `docs/software_design.md`：当前固件运行时行为、启动自检和门控语义。
- `docs/dashboard_spec.md`：160x50 dashboard 像素布局与状态。
- `README.md`：开发入口和当前命令集合。
- `PRODUCT.md`：站点产品定位。
- `DESIGN.md`：站点视觉和内容方向。

## 事实优先级

| 问题 | 优先事实源 |
| --- | --- |
| 当前产品和命名叫什么 | `docs/specs/pw97u-control-plane-alignment/SPEC.md` |
| V3 硬件到底怎么连 | `docs/hardware_connection_overview.md` |
| GPIO / 显示 / 前面板 reset | `docs/specs/j6nvw-hardware-v3-pin-assignment/SPEC.md` |
| 固件启动和运行期门控 | `docs/software_design.md` |
| 蜂鸣器提示音时序和优先级 | `docs/specs/7gf6b-firmware-buzzer-audio/SPEC.md` |
| dashboard 像素布局 | `docs/dashboard_spec.md` |
| 本地开发命令 | `README.md` |
| 文档站自身发布 | `docs/specs/r6wfr-docs-site/SPEC.md` |

站点页面是公开导览层，允许压缩、重排和解释，但不能越过这些事实源改写行为。

## 站点内容边界

站点页面可以压缩和重写上述内容，但不得改变：

- 固件身份：`isolarail`
- CLI：`isolarail`
- daemon：`isolarail-devd`
- 端口模型：`port1..port4`
- 当前 V3 硬件基线
- `port.replug` = 受控断电再上电
- `PCA9545A@0x70` 当前只作为兼容命名槽位

若站点内容与 specs 或 current-truth 文档冲突，以 specs/current-truth 文档为准，并应修正站点。
