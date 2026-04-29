# 固件健壮化与开机自检历史

## 关键演进

- 固件启动从早期 `panic!` 改为统一 self-check、故障码、LCD 摘要页与门控决策。
- 当前硬件从旧的 I2C mux 结构演进为直连共享 I2C，总线拓扑检查保留 `mux` 槽位以兼容后续硬件。
- 实物验证发现 MCU-only reset 后，前面板 `TCA6408A@0x21` 可能保持离线；当前 V3 没有 MCU 可控 reset 或 power-cycle 路径，因此固件只能执行 I2C bus-clear 与有限探测。
- 为避免产品在当前硬件上卡在 `PANEL PEND`，V3 固件将前面板离线记录为 `Warn/FrontPanelOffline` 并继续 runtime；硬件缺陷已登记为 GitHub issue #18。

## Legacy Source

- `docs/software_design.md`
- `docs/specs/5f74j-firmware-boot-self-check/SPEC.md`
