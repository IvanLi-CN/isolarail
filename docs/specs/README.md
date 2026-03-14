# 规格（Spec）总览

本目录用于管理工作项的规格：记录范围、验收标准与任务清单，作为交付依据；实现与验证以对应 `SPEC.md` 为准。

> Legacy compatibility: historical repos may still contain `docs/plan/**/PLAN.md`. New entries must be created under `docs/specs/**/SPEC.md`.

## 快速新增一个规格

1. 生成一个新的规格 `ID`（推荐 5 个字符的 nanoId 风格）。
2. 新建目录：`docs/specs/<id>-<title>/`（`<title>` 建议使用 kebab-case）。
3. 在该目录下创建 `SPEC.md`。
4. 在下方 Index 表新增一行；若需要记录推进情况，请放到对应 PR、issue 或其他追踪载体中。

## Index

|ID|Title|Spec|Notes|
|---:|---|---|---|
|xhn4c|硬件 V3 调整与固件适配计划|`xhn4c-hardware-v3-adjustments/SPEC.md`|聚焦 V3 硬件差异与固件适配边界|
