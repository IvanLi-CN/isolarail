# 固件蜂鸣器音效（#7gf6b）

## 状态

- Status: 已完成
- Created: 2026-07-05
- Last: 2026-07-05

## 背景 / 问题陈述

- V3 硬件已经预留无源蜂鸣器网络 `BUZZER`，GPIO 分配为 `GPIO7`。
- 固件此前没有声音反馈，按键、通道电源变化、保护告警与阈值提示只能依赖 LCD 或串口日志。
- 音效已通过 `tools/buzzer_audio_preview/` 静态预览页筛选，固件需要固化推荐方案并在运行期事件中播放。

## 目标 / 非目标

### Goals

- 使用 `GPIO7` 软件方波播放预览页推荐音效，静音时保持低电平。
- 在开机、有效按键操作、操作拒绝、通道上电/断电、持续告警、间隔告警与通道阈值提示时播放对应音效。
- 用纯逻辑状态机覆盖告警优先级、输入过功率 hysteresis、端口插拔/3A/5A hysteresis 与按键接受/拒绝判定。
- 串口日志输出 `buzzer:*` 摘要，便于非破坏性验证。

### Non-goals

- 不新增用户可配置音效接口。
- 不修改 Web/companion/API 协议。
- 不占用 fan 已使用的 LEDC 外设。
- 不修改 `vbus_ratio` 或 RATIO `0x08 bit0`。
- 不执行真机高温、高功率或短路破坏性验证。

## 范围（Scope）

### In scope

- `src/buzzer.rs`：GPIO7 软件方波播放任务、音效常量、命令队列与告警循环。
- `src/audio_logic.rs`：可在 host 测试的告警与阈值判定逻辑。
- `src/main.rs`：boot、front panel、USB JSONL、network action、port runtime 与告警状态接入。
- `src/fan.rs`：暴露 80C set / 75C clear 的过温告警状态。
- `src/power_in.rs`：暴露输入电源最新状态，用于输入过功率判断。
- `docs/software_design.md` 与 `docs/specs/README.md`。

### Out of scope

- Web/companion/API 的声音配置。
- LEDC 蜂鸣器驱动。
- 硬件接线或电气参数调整。

## 需求（Requirements）

### MUST

- 蜂鸣器必须使用 `GPIO7` 软件方波；播放完成后必须拉低静音。
- 开机音必须在 boot self-check 非 Fatal、进入 Runtime 后播放。
- Left/Right 按键成功移动选中通道时必须播放操作提示音。
- Center 成功切换通道电源时必须播放通道上电音或通道断电音。
- Center 尝试启用通道但因 OCP、全局关断、sideband 门控或端口未就绪被拒绝时必须播放操作拒绝音。
- Up/Down 没有定义动作时不得播放声音。
- 告警优先级必须为 `channel_short` > `over_temp` > `input_over_power` > `channel_over_5a` > one-shot。
- `channel_short` 必须复用低 VBUS 软件 OCP 条件。
- `channel_over_5a` 必须在任一 active 通道的新鲜遥测 `current_ma >= 5000` 或 high-current OCP latch 时以长间隔循环。
- 过温告警必须使用 80C 触发、75C 清除。
- 输入过功率告警必须使用 100W 触发、90W 清除。
- 通道提示音必须使用插入 `vbus_mv >= 3300`、拔出 `< 3000`、3A set/clear `3000/2800mA`、5A set/clear `5000/4800mA` 的 hysteresis。
- 保护关断同 tick 内必须抑制普通拔出/断电提示，只保留告警。

### SHOULD

- 告警循环期间 one-shot 音效可以被丢弃，并记录 `buzzer.play` 摘要。
- USB JSONL 与 network action 触发端口电源变化时应播放通道上电/断电音。

### COULD

- 后续可在控制面增加静音或音量策略，但本规格不定义该接口。

## 功能与行为规格（Functional/Behavior Spec）

### 音效来源

- 固件音符表来自 `tools/buzzer_audio_preview/README.md` 所描述的预览页推荐候选。
- 固件运行时只内置 `{freq_hz, ms}` 与 rest 常量，不读取 HTML、JSON、MIDI 或 WAV 产物。

### Playback

- `buzzer` 任务拥有 GPIO7 输出。
- `buzzer::play(Tone)` 排队 one-shot 音效。
- `buzzer::set_alarm(Option<AlarmTone>)` 设置当前循环告警。
- 无告警时，任务按队列顺序播放 one-shot。
- 有告警时，任务按优先级选出的 `AlarmTone` 循环播放；告警切换或清除通过命令队列即时生效。
- 每次播放与告警状态变化输出 `buzzer.play:*`、`buzzer.alarm:*` 或 `buzzer.alarm.play:*` 日志摘要。

### Runtime events

- Boot：`BootOutcome != Fatal` 且 stage 切换为 `Runtime` 后播放 `boot`。
- Front panel：
  - Left/Right：播放 `operation_ok`；
  - Center accepted enable：播放 `channel_power_on`；
  - Center accepted disable：播放 `channel_power_off`；
  - Center rejected：播放 `operation_denied`。
- USB JSONL / network：
  - `PortPowerSet(enabled=true)`：播放 `channel_power_on`；
  - `PortPowerSet(enabled=false)` 或 `PortReplug`：播放 `channel_power_off`。
- Channel hints：
  - 插入：`vbus_mv >= 3300`；
  - 拔出：`vbus_mv < 3000`；
  - 3A：首次达到 `current_ma >= 3000`，低于 `2800` 后复位；
  - 5A：首次达到 `current_ma >= 5000`，低于 `4800` 后复位。

### Alarm mapping

- `channel_short`：现有 `vbus < 3000mV && current > 100mA` OCP latch。
- `channel_over_5a`：现有 high-current OCP latch，或运行期任意 active 通道的新鲜遥测 `current_ma >= 5000`。
- `over_temp`：fan 温度 EMA 达到 `80C` 置位，低于等于 `75C` 清除。
- `input_over_power`：输入 VIN 与电流的绝对值乘积达到 `100W` 置位，低于 `90W` 清除。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `buzzer::play` | internal | firmware | New | None (this SPEC) | firmware | boot/front/runtime/API action bridge | one-shot tone queue |
| `buzzer::set_alarm` | internal | firmware | New | None (this SPEC) | firmware | runtime alarm reducer | looping alarm command |
| `audio_logic` | internal | library tests | New | None (this SPEC) | firmware | runtime / unit tests | pure threshold and priority logic |
| `fan::over_temp_alarm_active` | internal | firmware | New | None (this SPEC) | firmware | runtime alarm reducer | 80C set / 75C clear |
| `power_in::latest_status` | internal | firmware | New | None (this SPEC) | firmware | runtime alarm reducer | latest input status snapshot |

### 契约文档（按 Kind 拆分）

None

## 验收标准（Acceptance Criteria）

- Given 固件非 Fatal 启动，When `boot.stage: stage=runtime` 输出，Then `buzzer.play: tone=boot` 被排队。
- Given 前面板 Left/Right 触发，When 选中通道发生移动，Then 播放 `operation_ok`。
- Given 前面板 Center 启用通道且 gate 允许，When 手动输出切换成功，Then 播放 `channel_power_on`。
- Given 前面板 Center 禁用通道，When 手动输出切换成功，Then 播放 `channel_power_off`。
- Given 前面板 Center 尝试启用通道但被 OCP、global off、sideband gate 或 port not ready 拒绝，When 状态不变，Then 播放 `operation_denied` 并记录拒绝原因。
- Given 多个告警同时存在，When runtime reducer 计算当前告警，Then 按 `channel_short` > `over_temp` > `input_over_power` > `channel_over_5a` 选择唯一循环告警。
- Given 保护关断在同一 tick 发生，When 端口电压跌落，Then 普通拔出提示被抑制。
- Given 端口电流跨越 3A 或 5A 阈值，When 未低于对应 clear 阈值，Then 每个阈值只提示一次。

## 实现前置条件（Definition of Ready / Preconditions）

- 预览页推荐音效已经确认，不再保留固件侧备选音效。
- GPIO7 被确认为蜂鸣器控制网络，且没有其它固件模块占用。
- fan LEDC 资源保持独占，蜂鸣器不得改用 LEDC。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Unit tests: `cargo +stable test --lib --target aarch64-apple-darwin`
- Firmware checks: `cargo check`
- Firmware release build: `cargo build --release`

### UI / Storybook (if applicable)

- Stories to add/update: None
- Visual regression baseline changes (if any): None
- Visual evidence: Not applicable; this is a firmware acoustic behavior change.

### Quality checks

- `bunx markdownlint-cli2 README.md docs/**/*.md tools/buzzer_audio_preview/README.md`

## 文档更新（Docs to Update）

- `docs/software_design.md`: 同步蜂鸣器 runtime 行为、阈值和日志口径。
- `docs/specs/README.md`: 增加本规格索引。
- `tools/buzzer_audio_preview/README.md`: 保留为预览工具说明。

## 计划资产（Plan assets）

- `tools/buzzer_audio_preview/index.html`
- `tools/buzzer_audio_preview/README.md`

## Visual Evidence

Not applicable; this spec implements firmware acoustic behavior. Validation is by unit tests, firmware build, and serial log observability.

## 资产晋升（Asset promotion）

None

## 实现里程碑（Milestones / Delivery checklist）

- [x] M1: 固化推荐音效表与 GPIO7 软件方波播放器
- [x] M2: 接入 boot、front panel、USB JSONL 与 network 端口动作音
- [x] M3: 接入持续/间隔告警 reducer 与通道提示 hysteresis
- [x] M4: 补齐纯逻辑单元测试与文档 current truth
- [ ] M5: 完成本地构建、PR CI 与 review proof

## 方案概述（Approach, high-level）

- 用 `audio_logic` 承载可测试的阈值和优先级，避免把可验证规则埋进硬件主循环。
- 用独立 `buzzer` 任务拥有 GPIO7，主循环只发送播放/告警命令。
- fan 和 power input 只暴露最小状态查询接口，不改变既有控制策略。
- 主循环继续保持 `port.telemetry` 与 OCP 日志语义，新增声音只作为并行反馈。

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：软件方波会占用 embassy timer 调度，音效长度保持短促以降低 runtime 干扰。
- 风险：告警持续时 one-shot 可能被丢弃；这是为了保证安全告警优先。
- 需要决策的问题：None
- 假设（已确认）：只采用推荐音效；GPIO7 空闲；输入过功率阈值为 100W/90W；过温阈值为 80C/75C；Up/Down 保持无定义静默。

## 参考（References）

- `tools/buzzer_audio_preview/README.md`
- `docs/software_design.md`
- `docs/specs/j6nvw-hardware-v3-pin-assignment/SPEC.md`
