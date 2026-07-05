---
title: 前面板显示
description: 160x50 dashboard 的四列布局、状态、输入映射和预览资产。
---

<!-- markdownlint-disable MD025 -->

# 前面板显示

ISO USB Hub 的本机状态面是一个 160x50 像素 dashboard，对齐四个物理端口。完整布局规范在 `docs/dashboard_spec.md`。

## 布局

- 逻辑分辨率：`160x50 px`
- 四列等宽：每列 `40 px`
- 每列对应 `port1..port4`
- 生产布局不显示列标题，优先给数值和状态图标留空间

Dashboard 是硬件状态面，不是营销屏。它必须优先回答：

- 哪一路被选中？
- 哪一路有电压、电流和功率？
- 哪一路断开、关闭、初始化或过流？
- 当前用户按下中键会影响哪一路？

行预算：

| 行 | 内容 |
| --- | --- |
| y≈2 | 电压 `V` |
| y≈17 | 电流 `A` / `mA` |
| y≈32 | 功率 `W` / `mW` |
| y≈47..49 | 功率条 |

## 字符预算

每列可用宽度约 `36 px`，值行按 7 px advance 预算约 5 个字符：

- 电压：`5.12V`、`20.0V`、`9.00V`
- 电流：`0.98A`、`2.50A`、`650mA`
- 功率：`4.9W`、`22.5W`、`750mW`
- 未知：`--`

数值需要按列宽截断或降级，不能挤压相邻列。

格式化规则：

| 数值 | 规则 | 示例 |
| --- | --- | --- |
| 电压 `< 10 V` | 2 位小数 | `5.12V` |
| 电压 `>= 10 V` | 1 位小数 | `20.0V` |
| 电流 `>= 1 A` | A，2 位小数 | `2.50A` |
| 电流 `< 1 A` | mA，无小数 | `650mA` |
| 功率 `>= 1 W` | W，1 位小数 | `13.0W` |
| 功率 `< 1 W` | mW，无小数 | `750mW` |

如果格式化后仍超宽，优先降低精度，再显示 `--`；不要让字符越过相邻列。

## 状态

Dashboard 使用紧凑状态，而不是长文本：

- Normal：显示 V/I/W 数值和功率条。
- Disconnected：显示图标和 `DISC`。
- Over-current：显示 `CC` 图标，不显示功率数值或功率条。
- Closed：显示插头断开图标和 `OFF`。
- Initializing：三行均显示 `--`。

选中列用细 cyan inset 矩形表示，不能遮挡数值或图标。

状态优先级：

1. `Over-current`
2. `Closed`
3. `Disconnected`
4. `Initializing`
5. `Normal`

保护关断发生时，dashboard 应显示过流状态，而不是普通断电或拔出状态。

## 输入映射

前面板五向按键映射：

- 左 / 右：在四列间循环移动选择。
- 中键短按：手动断开或恢复选中端口输出。
- 中键长按：保留给未来快捷菜单。
- 上 / 下：保留给未来显示模式或详情页。

手动断开状态优先于遥测显示；被断开的列显示 `OFF`。

前面板任务只发布按键事件，不直接操作 `EN1..EN4`。真正的输出决策在 runtime reducer 中完成，仍要经过 VIN、CH335F sideband、OCP latch 和 owner 手动状态。

## 刷新

- 周期刷新：`2 Hz`，即每 500 ms。
- 电流和功率可用 1-2 秒滑动平均减少闪烁。
- 状态变化或超过 10% 的数值变化应立即刷新。

刷新来源：

- 周期遥测：端口电压、电流、功率、温度。
- runtime 状态：OCP latch、sideband fault、manual closed。
- 前面板事件：选中列变化、中键切换。
- boot handoff：从自检页切到 dashboard 时的第一帧。

前面板事件要立即重绘，不能等下一个 500 ms 周期。

## 颜色与资产约束

- 背景：白色。
- 默认文本 / 边框：黑色。
- 电压：深黄。
- 电流：红色。
- 功率和功率条：绿色。
- 选中列：cyan inset。

预览 SVG 是像素级规范资产：每个像素由一个 1x1 rect 表达，用来检查 160x50 逻辑布局是否仍然成立。

## 运行期数据口径

Dashboard 消费的 `V/I/W` 优先来自每路输出模块 `INA226`。功率使用 `W = V x I` 计算。初始化、读取失败或缺失传感器不能伪装成 0；应显示 `--` 或对应状态。

## 预览资产

正常态：

![Dashboard 160x50 normal](../../assets/dashboard_wireframe_160x50_color_bold.svg)

混合状态：

![Dashboard 160x50 states](../../assets/dashboard_wireframe_160x50_states_color_bold.svg)

这些 SVG 是每像素 1x1 矩形生成的像素级预览，用作 dashboard 规范资产。

## 参考

- `docs/dashboard_spec.md`
- `docs/assets/dashboard_wireframe_160x50_color_bold.svg`
- `docs/assets/dashboard_wireframe_160x50_states_color_bold.svg`
