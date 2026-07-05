# 固件蜂鸣器音效实现状态

## 当前状态

- 固件已新增 `audio_logic` 纯逻辑模块，覆盖告警优先级、输入过功率 hysteresis、端口插拔/3A/5A hysteresis 与 Center 按键接受/拒绝音效选择。
- 固件已新增 `buzzer` 任务，使用 LEDC Timer1/Channel1 在 GPIO7 输出预览页推荐音效，空闲与播放结束后保持低电平。
- 音符播放段使用 LEDC 硬件 PWM 输出，不使用阻塞半周期 busy-wait 或 async GPIO 翻转；静音 rest 与告警间隔使用 async timer 让出执行权。
- LEDC 外设由启动流程统一初始化，fan 使用 Timer0/Channel0，蜂鸣器使用 Timer1/Channel1。
- 告警清除后若队列中已有 one-shot 音效，播放器会暂存并播放第一条 one-shot，避免在告警状态切换 drain queue 时吞掉操作提示音。
- Runtime 已接入 boot 音、front panel 操作音、USB JSONL/network 端口动作音、通道提示音，以及持续/间隔告警循环。
- USB JSONL/network replug 动作只复位普通通道提示状态，不清除 OCP 告警原因；只有明确断电清除 latch 的动作才同步清除安全告警原因。
- Fan 任务暴露 80C set / 75C clear 的过温告警状态。
- Power input 任务暴露最新输入状态，用于 100W set / 90W clear 的输入过功率告警。

## Coverage

- 固件：`src/audio_logic.rs`
- 固件：`src/buzzer.rs`
- 固件：`src/main.rs`
- 固件：`src/fan.rs`
- 固件：`src/power_in.rs`
- 文档：`docs/software_design.md` 与本 spec 目录

## Remaining Gaps

- 未做真机高温、高功率或短路破坏性验证。
- `cargo test` 在默认 ESP target 下仍会尝试构建 no-std test harness；host 逻辑测试使用 `cargo +stable test --lib --target aarch64-apple-darwin`。

## Related Changes

- `tools/buzzer_audio_preview/` 保留为音效试听与来源说明。
- `docs/specs/README.md` 已登记本规格。
- `docs/solutions/firmware/esp32-s3-ledc-passive-buzzer.md` 记录 ESP32-S3 无源蜂鸣器驱动复用方案。
