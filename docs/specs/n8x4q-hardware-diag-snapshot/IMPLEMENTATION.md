# 实现记录

## 固件

- 新增 `src/hardware_snapshot.rs`，使用 `heapless::String` 渲染 `iso-usb-hub.hardware.snapshot.v1`。
- boot gate 完成后生成并缓存一次硬件快照，fatal 自检循环前也能留下快照。
- runtime 以低频刷新快照缓存，复用 dashboard 当轮端口读数与 sideband snapshot。
- 快照新增 MCU/fan/buzzer 运行期状态：ESP32-S3 内部温度、过温告警、风扇使能/实测 RPM/目标 RPM/控制百分比/硬件 PWM duty、蜂鸣器 LEDC ready/播放状态/tone/alarm/频率/duty。
- `ports[].control` 明确暴露每路手动门控、sideband PWREN、模块 EN、OCP latch、ready 与 scan 状态，避免调用方从分散字段自行拼装硬件控制结论。
- 四路输出模块传感器按器件独立采样：INA226 节点返回电压、电流、分流电压和关键寄存器，TMP112 节点返回温度和配置/阈值寄存器；单个传感器读取失败只标记该节点 `error`。
- USB JSONL 方法 `hardware.snapshot` 返回最新缓存，响应只读且不触发写寄存器动作；JSONL response buffer 已扩到 16 KiB，避免 full debug package 被 envelope 截断。
- `front_panel` 与 `hub_sideband` 增加只读寄存器快照，不增加调试写操作。

## Host Tools

- `isohub` CLI 增加 `diag-snapshot`，输出 `iso-usb-hub.hardware.snapshot.v1` 纯快照对象。
- `isohub-devd` IPC 增加 `device.hardware.snapshot`。
- `isohub-devd` HTTP bridge 增加 `/api/v1/devices/:id/diag-snapshot`，返回同一纯快照对象。
- `diagnostics export` 同步包含 `hardware_snapshot` 字段；快照读取失败不会阻断其它诊断字段。

## Web

- 新增 `web/` Vite TypeScript 应用。
- 高级调试入口为 `/devices/:deviceId/debug/hardware`。
- 页面包含设备树、MCU/fan/buzzer 运行状态、四路端口控制矩阵、四路端口状态、寄存器信息、刷新与复制 JSON。
- 四路输出模块表格直接展示 INA226/TMP112 的读数、寄存器摘要和失败原因，不把 `present` 探测布尔值当作完整传感器数据。
- 低价值细节不直接展开在主视图；点击状态卡、端口控制矩阵和传感器按钮会在右侧 JSON explorer 高亮并滚动到对应节点。
- 默认 mock data 覆盖前面板离线、单路传感器缺失、整路模块离线。
- 通过 devd HTTP bridge 读取真机快照时，页面先读取 `/api/v1/bootstrap` bearer token，再请求 `/api/v1/devices/:id/diag-snapshot`。

## 真机验证

- `/dev/cu.usbmodem21224101` 已通过 `just flash` 使用 `isohub` 正常烧录 app 分区。
- reset 后 `status` 返回 `device_id=f1fb44`、`firmware.name=iso-usb-hub`、Wi-Fi connected。
- `isohub --json diag-snapshot --device usb--dev-cu-usbmodem21224101` 返回 `iso-usb-hub.hardware.snapshot.v1` 纯快照对象。
- 真机快照中四个端口均 `online`，四路 INA226/TMP112 均 present，front panel 与 sideband 均 `online`。
- devd HTTP bridge `/api/v1/devices/usb--dev-cu-usbmodem21224101/diag-snapshot` 返回同一纯快照对象。
- `diagnostics export` 已包含 `hardware_snapshot` 字段。
