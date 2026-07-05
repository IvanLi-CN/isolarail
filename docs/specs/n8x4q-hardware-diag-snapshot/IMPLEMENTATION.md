# 实现记录

## 固件

- 新增 `src/hardware_snapshot.rs`，使用 `heapless::String` 渲染 `iso-usb-hub.hardware.snapshot.v1`。
- boot gate 完成后输出一次 `diag.snapshot:`，fatal 自检循环前也能留下快照。
- runtime 以低频输出快照，复用 dashboard 当轮端口读数与 sideband snapshot。
- `front_panel` 与 `hub_sideband` 增加只读寄存器快照，不增加调试写操作。

## Host Tools

- 新增 `tools/iso-usb-hub-host`。
- `iso-usb-hub` CLI 支持设备列表、状态、快照和 watch。
- `iso-usb-hub-devd` 支持 Unix socket IPC 和显式 HTTP bridge。
- host 解析器支持原始 JSON 和带 `diag.snapshot:` 前缀的串口 monitor 日志。
- host tools 要求显式快照源；只有 `--sample` 会读取仓库内样例数据。

## Web

- 新增 `web/` Vite TypeScript 应用。
- 高级调试入口为 `/devices/:deviceId/debug/hardware`。
- 页面包含设备树、四路端口状态、寄存器信息、刷新与复制 JSON。
- 默认 mock data 覆盖前面板离线、单路传感器缺失、整路模块离线。
