# 硬件 V3 引脚与显示链路实现状态

## 当前状态

- 主项目显示路径使用 MCU 直接控制 `DC/CS/RST/BLK`。
- `LCD_BLK` 按低有效处理，输出低电平打开背光。
- `DisplayConfig.orientation` 使用 `Orientation::LandscapeSwapped`。
- `gc9d01` submodule 指向包含 `LandscapeSwapped` 坐标修复的 driver `main`。
- `docs/hardware_connection_overview.md` 已重写为当前 V3 硬件总览，跨文档硬件入口不再停留在 V2 `SC8815 + SW2303` / `PSTOP*` 语义。
- `docs/esp32-s3fh4r2_gpio_assignment_guide.md` 已降级为历史参考，明确不再充当当前 V3 pin-level 真相源。

## Coverage

- 固件：显示初始化、背光极性、屏幕方向。
- 文档：GPIO assignment 中 `SPI_BLK` 极性。
- Driver：通过 submodule 指针引用已合并的 `gc9d01-rs` 修复。

## Remaining Gaps

- legacy `docs/plan/j6nvw-hw-v3-pin-assignment/**` 尚未删除，等待主人确认。
- 仍需继续把更广泛的 pin-level current truth 从历史计划文档中彻底 promotion 到 spec / 项目文档。

## Related Changes

- `src/main.rs`
- `Cargo.toml`
- `docs/esp32-s3fh4r2_gpio_assignment_guide.md`
- `gc9d01`
