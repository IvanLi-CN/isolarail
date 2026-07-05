# 固件蜂鸣器音效历史

## 关键演进

- 项目先通过 `tools/buzzer_audio_preview/` 做 Web Audio 试听页，确认每类声音的推荐候选。
- 固件实现阶段决定只固化推荐音，不携带备选音效或运行期配置接口。
- 硬件测试发现软件/RMT 方式不如 PWM 方案稳定后，LEDC 外设改为启动流程统一初始化；fan 使用 Timer0/Channel0，蜂鸣器使用 Timer1/Channel1，避免 CPU 生成波形。
- 告警设计采用安全优先的单一 active alarm reducer，one-shot 音效低于持续告警优先级。

## Legacy Source

- `tools/buzzer_audio_preview/README.md`
- `docs/software_design.md`
