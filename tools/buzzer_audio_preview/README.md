# Buzzer Audio Preview

静态蜂鸣器音效试听页，用于先挑选提示音，再进入固件与文档设计。

## Open

直接在浏览器打开：

```bash
tools/buzzer_audio_preview/index.html
```

页面使用 Web Audio API 合成方波，点击播放按钮后浏览器会解锁音频输出。

## Generate MIDI and WAV

可选生成每个候选音的 `score.json`、`.mid` 与 `.wav`：

```bash
python3 tools/buzzer_audio_preview/generate_assets.py
```

输出目录：

```text
tools/buzzer_audio_preview/out/
```

该目录是本地试听产物，不提交到仓库。
