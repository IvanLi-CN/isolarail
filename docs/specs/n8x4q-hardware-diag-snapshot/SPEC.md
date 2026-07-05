# ISO USB Hub 硬件底层详细信息快照接口

## 背景

调试 ISO USB Hub 时，需要一次性读取底层硬件状态，而不是从多条串口日志里人工拼接。当前固件已有 boot self-check、sideband、前面板、四路输出模块和风扇状态，但缺少统一 schema、host 读取链路和 Web 可视化入口。

## 目标

- 固件输出只读硬件快照 JSON，覆盖输入电源、I2C 拓扑、主板 sideband、前面板、风扇、四路输出模块和 boot self-check。
- host-tools 提供 source-built `iso-usb-hub` CLI 与 `iso-usb-hub-devd`。
- Web app 在 `/devices/:deviceId/debug/hardware` 展示高级硬件调试页。
- 四路输出模块和前面板可离线，离线只体现在对应节点，不导致整包失败。

## 非目标

- 不实现写寄存器、端口控制、强制复位、刷机或任何改变硬件状态的调试动作。
- 不修改 `vbus_ratio` 或 RATIO `0x08 bit0`。
- 不要求真机 HIL 作为本规格完成前置条件。

## Schema

固件日志输出一行：

```text
diag.snapshot: { ... }
```

顶层字段：

- `schema`: `iso-usb-hub.hardware.snapshot.v1`
- `sequence`, `uptime_ms`, `firmware`, `reset_reason`
- `boot`: self-check outcome、first fault、gate decision、sys checks
- `power_input`: `INA226@0x44`、VIN、PG、ready、目标电源状态
- `i2c`: `direct_shared_bus`、`PCA9545A@0x70` skipped、bus recovery 信息
- `sideband`: `TCA6408A@0x20`、寄存器、PWREN/OVCUR
- `front_panel`: `TCA6408A@0x21`、寄存器或离线原因
- `fan`: ready/state
- `ports[]`: `INA226@0x40..0x43`、`TMP112@0x48..0x4B`、probe 方法、tries、telemetry、EN/PWREN/OCP

节点状态统一为：

- `online`: 本次快照可读
- `offline`: 可选器件未 ACK 或未装配
- `skipped`: 前置条件不满足而未读
- `error`: 预期在线但运行期读取失败

## 行为规格

- 固件在 boot gate 完成后输出一次快照；若进入 fatal 自检页，也必须先输出失败快照。
- runtime 低频输出快照，默认约 10 秒一条。
- devd 可从 JSON 文件或包含 `diag.snapshot:` 的 monitor/JSONL 日志中读取最新快照。
- CLI/devd 没有 `--snapshot-file` 或 `ISO_USB_HUB_SNAPSHOT_FILE` 时不得伪造本地硬件；样例数据必须显式使用 `--sample`。
- devd 默认 Unix socket IPC，HTTP bridge 只能显式启动。
- CLI 支持 `devices list`、`device local status`、`device local diag-snapshot [--watch] [--json]`。
- Web 页面默认使用 mock data；带 `?devd=http://127.0.0.1:51210` 时读取 HTTP bridge。

## 验收标准

- 给定四路模块全在线，快照 `ports[].state` 均为 `online`。
- 给定单路 INA226 或 TMP112 离线，该路为 `error`，其它路保持自身状态。
- 给定四路全离线，整包仍返回 JSON，四路均为 `offline`。
- 给定前面板离线，`front_panel.state=offline`，整包仍返回 JSON。
- 给定 sideband 运行期读取失败，`sideband.state=error` 且输出保持关闭。
- CLI `--json` 输出可被 Web 同一 schema 消费。

## 质量门槛

- `cargo +esp check`
- `cargo +esp build --release`
- `HOST_TRIPLE="$(rustc +stable -vV | sed -n 's/^host: //p')"; cargo +stable test --target "$HOST_TRIPLE"` in `tools/iso-usb-hub-host`
- `bun install --frozen-lockfile && bun test && bun run build` in `web/`
- Web 改动需要可控视觉证据。

## Visual Evidence

Desktop `ui_demo`:

![Hardware debug desktop](assets/hardware-debug-desktop-trimmed.png)

Mobile `ui_demo`:

![Hardware debug mobile](assets/hardware-debug-mobile-trimmed.png)

## 里程碑

- [x] 固件 schema 与 JSONL 输出
- [x] host CLI/devd 与基础测试
- [x] Web 高级调试页与 mock schema
- [x] 项目文档同步
- [ ] 真机串口端到端验证
