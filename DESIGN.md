# Design

## Overview

`iso-usb-hub` 使用克制的 product UI 视觉系统，为四路 USB Hub 提供仪器化控制面。界面应在桌面工位、串口联调和局域网配网场景下保持清晰、稳定、紧凑，优先服务状态判断与动作执行。

## Color

采用 restrained 策略，使用轻微带冷色倾向的中性色表面，加一个低饱和主强调色表示当前选择、主操作和活动通道。语义色只用于 `success`、`warning`、`error`、`busy`、`disabled`、`usb-only` 这类明确状态。

建议 token 方向：

- `--bg`: 带轻微蓝灰偏色的近白底。
- `--panel`: 一级工作面板色。
- `--panel-2`: 次级分组、日志、内嵌列表区。
- `--border`: 低对比结构线。
- `--primary`: 冷静的蓝靛或钢青色，用于激活态与主要 CTA。
- `--success`, `--warning`, `--error`: 仅用于状态与告警，不用于装饰。

新颜色优先使用 OKLCH，不使用纯黑纯白。

## Typography

优先使用系统 UI sans，数值、设备 ID、MAC、hostnames、串口路径、日志和固件版本使用 monospace。标题保持任务导向，不做 hero 级排版；正文与表单说明控制在易扫读密度，数值列尽量使用等宽数字避免跳动。

## Layout

整体为 app shell：左侧设备列表或导航，右侧主工作面。主工作面优先级如下：

1. 顶部连接与身份状态带。
2. 主列放四路端口总览与当前设备运行状态。
3. 次列或下部放 Wi-Fi、固件更新、诊断与日志。
4. 窄屏时按动作顺序纵向堆叠，保证关键按钮与状态先出现。

卡片只用于明确的重复硬件单元，如端口卡、设备卡、阶段面板。禁止嵌套厚重卡片。

## Components

- 状态 badge：统一文本标签与图标体系，覆盖 `Wi-Fi/LAN`、`Web Serial`、`Local USB`、`offline`、`busy`、`usb-only`、`ocp`。
- 连接方式切换：使用分段控件或紧凑 tab，而不是夸张大按钮组。
- 端口面板：每个端口固定展示身份、供电状态、数据状态、遥测、动作区与异常态。
- 表单：标签始终可见，错误贴近字段，禁用态仍可读。
- 日志与进度面板：使用 monospace，空态和失败态要明确。

## Motion

交互动效维持在 150–220 ms 的 ease-out，主要用于连接状态切换、面板展开、进度更新与错误反馈。不做入场编舞，不动画布局属性。

## Content

文案使用直接、操作型语句，例如 `Connect via Web Serial`、`Save Wi-Fi`、`Power off port 3`、`Restart hub`。错误信息必须同时说明失败点和下一步可做的动作。owner-facing 名称统一使用 `port1..port4` / `Port 1..4`，不用参考项目里的 `USB-A` / `USB-C` 双口语义。
