# 硬件 V3 引脚与显示链路实现状态

## 当前状态

- 主项目显示路径使用 MCU 直接控制 `DC/BLK`，通过前面板 `TCA6408A@0x21` P6/P5 控制 `CS/RST`。
- MCU `GPIO13/GPIO14` 不再由固件分配给显示网络，保持默认未占用状态。
- `LCD_BLK` 按低有效处理，输出低电平打开背光。
- `DisplayConfig.orientation` 使用 `Orientation::LandscapeSwapped`。
- `gc9d01` submodule 指向包含 `LandscapeSwapped` 坐标修复的 driver `main`。

## Coverage

- 固件：显示初始化、TCA 控制的 CS/RST、背光极性、屏幕方向。
- 文档：GPIO assignment 中 `SPI_BLK` 极性，以及 `GPIO13/GPIO14` 默认未分配。
- Driver：通过 submodule 指针引用已合并的 `gc9d01-rs` 修复。

## Remaining Gaps

- legacy `docs/plan/j6nvw-hw-v3-pin-assignment/**` 尚未删除，等待主人确认。
- V3 pinmap 中与显示无关的历史条目仍需后续独立清理。

## Related Changes

- `src/main.rs`
- `Cargo.toml`
- `docs/esp32-s3fh4r2_gpio_assignment_guide.md`
- `gc9d01`
