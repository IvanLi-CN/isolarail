# 规格（Spec）总览

本目录用于管理工作项的规格与追踪：记录范围、验收标准、任务清单与状态，作为交付依据；实现与验证以对应 `SPEC.md` 为准。

> Legacy compatibility: historical repos may still contain `docs/plan/**/PLAN.md`. New entries must be created under `docs/specs/**/SPEC.md`.

## 快速新增一个规格

1. 生成一个新的规格 `ID`（推荐 5 个字符的 nanoId 风格）。
2. 新建目录：`docs/specs/<id>-<title>/`（`<title>` 建议使用 kebab-case）。
3. 在该目录下创建 `SPEC.md`。
4. 在下方 Index 表新增一行，并更新 `Status` 与 `Last`。

## 状态（Status）说明

仅允许使用以下状态值：

- `待设计`
- `待实现`
- `跳过`
- `部分完成（x/y）`
- `已完成`
- `作废`
- `重新设计（#<id>）`

## Index

|ID|Title|Status|Spec|Last|Notes|
|---:|---|---|---|---|---|
|xhn4c|硬件 V3 调整与固件适配计划|待设计|`xhn4c-hardware-v3-adjustments/SPEC.md`|2026-02-25|新建|
