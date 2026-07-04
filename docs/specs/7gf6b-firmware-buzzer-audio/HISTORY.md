# 固件蜂鸣器音效历史

## 关键演进

- 项目先通过 `tools/buzzer_audio_preview/` 做 Web Audio 试听页，确认每类声音的推荐候选。
- 固件实现阶段决定只固化推荐音，不携带备选音效或运行期配置接口。
- Fan 已独占 LEDC 外设，因此蜂鸣器使用 GPIO7 软件方波，避免新增外设争用。
- 告警设计采用安全优先的单一 active alarm reducer，one-shot 音效低于持续告警优先级。

## Legacy Source

- `tools/buzzer_audio_preview/README.md`
- `docs/software_design.md`
