# 硬件 V3 引脚与显示链路历史

## 关键演进

- V3 显示控制口径改为 MCU 直接控制 `DC/BLK`，前面板 `TCA6408A@0x21` 的 P6/P5 控制 `CS/RST`，MCU `GPIO13/GPIO14` 保持默认未分配。
- 实物验证发现背光需要低电平使能；网表显示 `BLK` 驱动 P 沟道门极，与低有效固件行为一致。
- 屏幕实装方向需要 180 度旋转；driver 侧 `LandscapeSwapped` 映射修复后，主项目切换到该方向。

## Legacy Source

- `docs/plan/j6nvw-hw-v3-pin-assignment/PLAN.md`
- `docs/plan/j6nvw-hw-v3-pin-assignment/hardware_v3_pin_assignment.md`

legacy 源文档暂时保留；删除需主人确认。
