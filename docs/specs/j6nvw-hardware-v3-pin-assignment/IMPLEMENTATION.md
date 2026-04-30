# 硬件 V3 引脚与显示链路实现状态

## 当前状态

- 主项目显示路径使用 MCU 直接控制 `DC/BLK`，优先通过前面板 `TCA6408A@0x21` P6/P5 控制 `CS/RST`。
- MCU `GPIO13/GPIO14` 启动时默认不驱动；若显示控制初始化前确认前面板 `TCA6408A@0x21` 不 ACK 且 reset reason 明确为 `ChipPowerOn`，则启用原 GPIO13/GPIO14 fallback 驱动 `CS/RST`；若任一初始化尝试进入 partial failure，则保持 GPIO13/GPIO14 高阻。
- 早期显示路径为 `Unavailable` 且后续前面板探测恢复在线时，固件重配 TCA P5/P6 并重试 LCD 初始化；早期已启用 MCU fallback 时不热切换路径。
- TCA 路径下 `CS` 在 LCD 初始化前由 TCA P6 拉低并保持为屏幕使能闸门，SPI transaction 本身不再经 I2C 翻转 CS；MCU fallback 路径下由 GPIO13 执行低有效 CS。
- `LCD_BLK` 按低有效处理，输出低电平打开背光。
- `DisplayConfig.orientation` 使用 `Orientation::LandscapeSwapped`。
- `gc9d01` submodule 指向包含 `LandscapeSwapped` 坐标修复的 driver `main`。

## Coverage

- 固件：显示初始化、TCA 优先的 CS/RST、MCU fallback、背光极性、屏幕方向。
- 文档：GPIO assignment 中 `SPI_BLK` 极性，以及 `GPIO13/GPIO14` fallback 条件。
- Driver：通过 submodule 指针引用已合并的 `gc9d01-rs` 修复。

## Remaining Gaps

- legacy `docs/plan/j6nvw-hw-v3-pin-assignment/**` 尚未删除，等待主人确认。
- V3 pinmap 中与显示无关的历史条目仍需后续独立清理。

## Related Changes

- `src/main.rs`
- `Cargo.toml`
- `docs/esp32-s3fh4r2_gpio_assignment_guide.md`
- `gc9d01`
