# 固件健壮化与开机自检实现状态

## 当前状态

- Boot self-check 状态模型、阶段日志、LCD 自检页和 runtime 门控已经落地。
- 当前直连 I2C 板型保留 `mux` 槽位，但不再依赖 `PCA9545A`。
- 输入电源不安全仍是 fatal；单路端口传感器异常按 degraded 记录。
- 当前 V3 硬件下，前面板 `TCA6408A@0x21` 离线会在有限恢复失败后降级继续运行，仅禁用前面板按键任务。
- 前面板离线时，启动日志会输出 I2C bus-clear 电平、`0x21` 分阶段探测、peer device 在线矩阵和故障分类，便于区分 MCU-only reset 后的 TCA 未复位、总线卡住和共享供电/I2C 异常。

## Coverage

- 固件：`src/main.rs` boot self-check flow、gate decision、front-panel degraded path。
- 固件：`src/boot_diag.rs` 自检状态、故障码与快照模型。
- 固件：`src/power_in.rs` 输入电源启动探测。
- 文档：`docs/software_design.md` 与本 spec 记录当前启动口径。

## Remaining Gaps

- 当前 V3 前面板 TCA 无 MCU 可控 `RESET#` 或 VCCP power-cycle，固件无法在 MCU-only reset 后硬复位该器件。
- 真机 MCU reset 复现中，SDA/SCL 恢复后均为 high，主板 TCA `0x20` 与输入 INA226 `0x44` 在线，前面板 TCA `0x21` 连续 NACK；当前诊断分类为 `front_tca_only_offline`。
- 未来硬件修订提供前面板 TCA reset/power-cycle 后，应移除 `blocked_no_reset_pin` 降级路径，并重新要求前面板在线后进入 runtime。

## Related Changes

- `src/main.rs`
- `src/front_panel.rs`
- `src/boot_diag.rs`
- `src/power_in.rs`
- `docs/software_design.md`
