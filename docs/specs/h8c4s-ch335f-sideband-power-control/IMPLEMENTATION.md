# CH335F Sideband 电源控制实现状态

## 当前状态

- 主板 `TCA6408A@0x20` sideband helper 已实现，负责读取 `PWREN#` 与控制 `OVCUR#`。
- 启动与 runtime 输出门控已经接入 `VIN ready`、sideband online、`GPIO21/V1OK`、`PWREN#`、前面板手动开关与 OCP latch。
- `V1OK=low` 时进入 standalone 模式，允许未连接上游电脑时保持产品独立输出。
- `V1OK=high` 时进入 upstream-managed 模式，按 CH335F `PWREN#` 关闭或开启对应输出。
- OCP 由 INA226 电压/电流估算，命中阈值立即 latch、关闭 `ENx` 并拉低 `OVCUR#`；释放要求连续带电安全采样。

## Coverage

- 固件：`src/hub_sideband.rs` TCA6408A register helper。
- 固件：`src/main.rs` sideband 初始化、输出门控、OCP latch/release 与 telemetry。
- 文档：`docs/software_design.md` 与本 spec 描述 standalone/upstream-managed 行为。
- 真机：已验证 standalone 无上游主机仍可输出。
- 真机：已验证上游侧 `2-1.2.2` port 2/3/4 可分别驱动固件 `p1/p3/p4` 的 `pwren/en` 状态。

## Remaining Gaps

- 上游侧 port 1 当前承载 MCU USB JTAG/serial 调试口，未在串口监视期间关断测试。
- `PWREN1#` / `PWREN2#` 硬件连接错误已登记为 GitHub issue #18，后续硬件修版应修复端口映射。
- 物理 OCP 触发未用高电流或低压负载夹具强制验证；当前 PR 仅完成固件阈值实现、构建验证与可观测日志路径。

## Related Changes

- `src/hub_sideband.rs`
- `src/main.rs`
- `docs/software_design.md`
- `docs/specs/h8c4s-ch335f-sideband-power-control/SPEC.md`
