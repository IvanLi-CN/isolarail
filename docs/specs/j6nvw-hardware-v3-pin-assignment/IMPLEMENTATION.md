# 硬件 V3 引脚与显示链路实现状态

## 当前状态

- 主项目显示路径使用 MCU 直接控制 `DC/CS/RST/BLK`。
- `LCD_BLK` 按低有效处理，输出低电平打开背光。
- `DisplayConfig.orientation` 使用 `Orientation::LandscapeSwapped`。
- `gc9d01` submodule 指向包含 `LandscapeSwapped` 坐标修复的 driver `main`。

## Coverage

- 固件：显示初始化、背光极性、屏幕方向。
- 文档：GPIO assignment 中 `SPI_BLK` 极性。
- Driver：通过 submodule 指针引用已合并的 `gc9d01-rs` 修复。

## Remaining Gaps

- legacy `docs/plan/j6nvw-hw-v3-pin-assignment/**` 尚未删除，等待主人确认。
- V3 pinmap 中与显示无关的历史条目仍需后续独立清理。

## Related Changes

- `src/main.rs`
- `Cargo.toml`
- `docs/esp32-s3fh4r2_gpio_assignment_guide.md`
- `gc9d01`
