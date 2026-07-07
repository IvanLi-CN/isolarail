# 四路 USB Hub 控制面对齐实现状态

## 当前状态

- 控制面对齐已在当前 `HEAD` 收口到本地 `PR-ready`：固件 Wi-Fi runtime、Local USB / LAN profile coalescing、Web canonical route、Dashboard / Settings / Info / Add device 视觉证据、Storybook 与 current-truth 文档已重新按同一轮验证与 spec 证据对齐。
- 新建规格并冻结 `isolarail` 命名空间、`port1..port4` 端口模型、当前 V3 硬件基线命名与 `replug=power-cycle` 语义。
- 规格已补齐仓内一方软件包清单：root firmware `isolarail`、repo JS tooling `isolarail-dev-tools`、frontend package `web`、companion package/binaries、`gc9d01` 与 `tools/` 下的本地预览/资源转换 crate。
- 规格已补上 manifest coverage audit：当前仓内全部 `Cargo.toml` / `package.json` 都已被 `Software Package Matrix` 明确覆盖，`tools/firmware-catalog/` 被记录为 script-only 目录而非独立 package。
- 规格已明确 `gc9d01/examples/**` 属于 vendored display example manifest 范围：允许保留上游示例包名，但必须与 `isolarail` 产品命名、release 资产和 owner-facing 文案隔离。
- 规格已补齐当前控制面会直接引用的硬件命名清单：`ESP32-S3`、`CH335F`、`M24C64@0x50`、`TPS2490`、`ISOUSB211 V1OK`、`PCA9545A@0x70`、`Mainboard TCA6408A@0x20`、`Front-panel TCA6408A@0x21`、`HUB_RESET#`、`EN1..EN4`、`PWREN#` / `OVCUR#`、输入/端口遥测块、LCD/front panel/fan/buzzer。
- 规格已把 `INA226` 与 `TMP112` 家族名正式纳入 `Hardware Naming Matrix`，用于约束 `docs/software_design.md`、`docs/hardware_connection_overview.md` 与后续 companion/web/diagnostics 中的统一写法。
- 规格已补齐板级网络别名与 legacy/scoped 硬件术语边界：`USB D+` / `USB D-`、`HUB_SDA` / `HUB_SCL`、`I2C_INT` / `I2C_RESET`、`ISO_OK`、`LCD_CS` / `LCD_RST` / `LCD_RES` / `LCD_BLK`、`BUZZER`、`FAN_PWM` / `FAN_EN` / `FAN_TACH`、`SC8815 + SW2303`、`PCA9545A INT/INTx`、`PSTOP_CTL1..4` / `PSTOP1..4`、`VIN_ADC`、`Mainboard RESET#`、`Front-panel TCA6408A RESET#` 的受限使用范围。
- 规格已补齐文档真相边界：`pw97u-control-plane-alignment/SPEC.md` 负责 owner-facing 命名，`j6nvw-hardware-v3-pin-assignment/SPEC.md` 负责 V3 pin-level / display / reset 事实，`docs/hardware_connection_overview.md` 已提升为当前 V3 硬件总览，而 `docs/esp32-s3fh4r2_gpio_assignment_guide.md` 已降级为历史 GPIO 参考。
- 规格现已把 `docs/ch335f_tca6408a_appnote.md`、`docs/i2c_gpio_expanders_comparison.md`、`docs/power_management_and_startup_control.md`、`docs/pwm_fan_control_circuit_design.md`、`docs/development_notes.md` 明确归类为 reference-only 硬件资料，防止旧器件名或候选器件覆盖当前 V3 控制面真相。
- 规格已补上硬件命名覆盖边界与 reference-only 料号策略：`TPS82130SILR`、`RT9043GB`、`TCA6408APWR`、`TCA6408ARSVR`、`TCA9535RTWR` 等名称现在被明确限制在历史方案、选型比较或局部电路说明语境中。
- 规格已明确 daemon 默认 IPC 机制：`isolarail-devd serve` 在 Unix 使用 Unix domain socket，在 Windows 使用 named pipe；`isolarail-devd web` 只作为显式 Web companion 存在。
- 项目入口文档已完成第一轮 current-truth 收敛：`README.md`、`INSTALL.md`、`docs/hardware_connection_overview.md` 与 `docs/esp32-s3fh4r2_gpio_assignment_guide.md` 已对齐 `isolarail` / `isolarail-devd` / `port1..port4` / V3 口径，并移除把旧 `SC8815 + SW2303`、`PSTOP*`、`GPIO38` 当作当前实现事实的写法。
- 后续实现需要以该命名矩阵为唯一真相源。
- 设备端、本地 companion tools 与 web app 的活动代码主路径已经大体对齐当前 spec；剩余缺口主要落在最终交付证据、文档 current-truth 与 legacy 清理，而不是“是否已经存在对应实现模块”。
- Web 活动代码面已完成第一轮命名迁移：`desktopAgent` / `desktopStorage` 已替换为 `companionBridge` / `companionStorage`，`Justfile` 的 companion bridge 单测入口已同步改名。
- 当前活动固件与 web 示例数据已统一设备 identity 相关口径：`firmware.name="isolarail"`、hostname `isolarail-<shortid>`、`device.variant="v3"`。
- 固件已新增共享 `device_identity` 模块，把 `firmware.name`、`device.variant`、`isolarail-<shortid>` hostname/FQDN 与 MAC 格式化逻辑收口到单一事实来源，供 USB JSONL 与 mDNS 复用。
- 固件已新增共享 `device_contract` 模块，把活动四路 `info` / `ports` / `wifi` 响应收口到同一事实来源；USB JSONL 现已复用这套渲染逻辑，canonical `port1..port4` 解析、四路端口列表形状、单端口形状与 `state.overcurrent` 字段不再分散在多个实现里各自拼接。
- 固件已把 `http_api_v1` 提升为共享四路 HTTP dispatcher：除了 canonical route parse 与 payload render 之外，现在还能产出只读响应或 `PortPower` / `PortReplug` action plan，后续 transport 层可直接复用这套 `ApiOutcome` / `ApiPendingAction` 契约，而不必再从旧双口 `src/net/http.rs` 拼接语义。
- 固件已新增共享 `runtime_control` 层，把四路 `port.power_set` / `port.replug` / Wi‑Fi runtime snapshot 更新与 replug holdoff tick 收口为可复用 helper；当前 USB JSONL 已复用这套动作语义，后续 HTTP transport 只需搬运 `ApiPendingAction`，不再重复实现一套端口控制状态机。
- 活动 USB JSONL `wifi.get` 已进一步对齐 companion/web 的实际消费形状：现在会返回 EEPROM `storage="eeprom"`、`address="0x50"`、已保存的 `ssid`、`psk_configured` 与运行态 `state/ipv4/is_static`，不再让 Wi‑Fi 读取路径停留在 `ssid/address` 永远缺失的半残状态。
- Web 开发环境已改为通过 Vite 同源代理访问显式配置的本地 `isolarail-devd web` origins；前端不再扫描 localhost 端口，配置应优先放 mDNS URL，再放 IP/localhost fallback。
- web 设备页路由现已按 spec 收口到 `Dashboard / Settings / Info`：`/devices/:deviceId` 为 dashboard，`/devices/:deviceId/settings` 为 settings，`/devices/:deviceId/info` 为 info。非法 profile-suffixed 路径（例如把 internal `--usb` storage id 放进 URL）不做兼容重定向；Web storage 必须对外返回 canonical hardware id。
- Web runtime 在 `wifi.get` / `wifi.set` 回读到 connected IPv4 时会刷新设备列表并补齐 LAN 通道；页面 URL 始终使用 canonical hardware id。
- Web Dashboard 端口卡片已把电源状态和电源操作组合为同一个控制面：待命时用成熟图标库 `lucide-react` 表示电源/重插，power/replug 操作期间切换为 spinning 图标，且 pending 状态保持到 API 回显匹配目标状态。
- companion 实现应以本项目命名 `isolarail-devd` / `isolarail` 为目标；当前 owner-facing 门户是 `isolarail` CLI，`devd` 只承担后台单例；参考项目 `isolapurr` 仅用于架构对齐，不进入本项目 owner-facing 命名。
- companion workspace 需要与仓根 Xtensa 固件配置隔离，避免本地 CLI / daemon 构建继承固件 target/toolchain。
- companion Local USB 路径现已支持显式单端口约束：repo-root `just` 会把 `USB_PORT` 透传为 `ISOLARAIL_USB_PORT`，`list_serial_ports()` 与实际串口打开 / flash 路径都会拒绝 allowlist 之外的设备，避免开发期误碰其他硬件。
- companion CLI 现已补上 Wi-Fi 写操作门禁：`wifi set` / `wifi clear` 仅接受 `--device` 或 USB-backed `--hardware`，`--url` 与 Wi-Fi/LAN saved hardware 在 CLI 中保持只读，避免绕过 spec 的 USB-capable 写策略。
- companion `wifi.set` / `wifi.clear` 现在会等待设备回读确认；`wifi.set` 只有读回期望 SSID 才返回保存成功，`wifi.clear` 只有读回 unconfigured 才返回清除成功。
- companion storage 现在以 firmware identity 的 `device_id` 作为 Web-visible hardware id，把 HTTP/LAN profile 与 Local USB profile 合并成同一个 canonical 设备；internal `--usb` 只保留在 storage/transport 层，不进入 URL。
- companion 在 USB Wi-Fi 状态读到 `state=connected` 与 IPv4 后，会自动写入或刷新同一 hardware id 的 HTTP profile；清除 Wi-Fi 时只删除对应 HTTP profile，保留 Local USB profile。
- companion daemon 现已为普通 Local USB JSONL 请求增加每串口互斥，避免 `status` / `ports` / `wifi` / `monitor` / `diagnostics export` 并发争抢同一 CDC 设备时出现空 IPC 响应或底层串口 `busy` 噪声。
- `isolarail` CLI 的 devd auto-start 现已增加 endpoint-scoped start gate：多个 CLI 进程同时发现 daemon 不存在时，只允许一个进程真正启动 `isolarail-devd serve`，其余进程等待同一 IPC endpoint 就绪，避免并发自启把同一 USB 口拖入额外的 `device busy` 竞争。
- HIL 复测发现默认 IPC socket 处于拒绝连接或刚关闭空响应时，CLI 仍可能直接失败；现已把 `connect IPC`、`Connection refused` 与 `IPC daemon closed the connection without a response` 统一归入 transient IPC 错误并走 auto-start / wait retry 路径，指定 USB 设备上的并发 `status` / `ports` / `wifi-show` / `monitor` 已复测通过。
- `diagnostics export` 现已改为 companion 聚合导出：返回当前 `status`、`ports`、`wifi`、设备 transport 元数据与近期 serial session traces，不再依赖固件侧尚未实现的 `pd.diagnostics` 专用方法。
- 根仓的共享 contract 测试路径已完成第一轮隔离：`heapless` 保留在通用依赖中，`esp-hal` / `embassy` / `gc9d01` 等固件专用依赖收口到 Xtensa 目标依赖，`build.ref.rs` 仅在嵌入式目标下注入 linker 错误处理参数，`just firmware-contract-test` 现在通过 native stable toolchain 运行共享 `device_identity` / `device_contract` / `http_api_v1` 单元测试。
- 固件已接入活动 `network_runtime`：设备启动时从 `M24C64@0x50` 加载凭据，`main.rs` 会启动 `network_runtime::spawn(...)`，循环中持续 `publish_snapshot(...)`，并在 `wifi.set` / `wifi.clear` 后触发 `request_wifi_runtime_apply()`；HTTP/mDNS 只在 Wi-Fi runtime 拿到有效 IPv4 后作为 LAN 通道出现。
- 固件 Wi-Fi EEPROM 访问已按 V3 硬件路由走 `hub_bus`，并在启动时配置 `ROM_WC=GPIO37-low`、`ROM_ROUTE=GPIO38-high`，避免把 Wi-Fi profile 写到错误 I2C 控制器路径。
- USB JSONL frame parser 已改为只在 JSON frame 内累计输入，忽略 defmt/串口噪声，避免二进制日志污染 companion 请求响应。

## 当前验证证据

### 本轮重新复验证据（2026-06-29）

- `just firmware-check`
- `just firmware-contract-test`
- `just tools-test`
- `just web-check`
- `just web-build`
- `just web-test-unit`
- `just devd-help`
- `just isolarail --help`
- `git diff --check`
- `cargo +esp check --release`
- `just web-storybook`（修复默认 mock-only 启动，不再要求 `ISOLARAIL_DEVD_ORIGINS`）
- HIL on `/dev/cu.usbmodem21234101`:
  - `USB_PORT=/dev/cu.usbmodem21234101 just discover`
  - `USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' just status`
  - `USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' just ports`
  - `USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' just wifi-show`
  - `USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' TAIL=12 just monitor`
- `USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' PORT=port1 ENABLED=false just port-power`
- `USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' PORT=port1 ENABLED=true just port-power`
- `USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' PORT=port1 just port-replug`
- `USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' just reset`
- `USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' just diagnostics-export`
- Storybook visual evidence refresh on current `HEAD`:
  - `Dialogs/AddDeviceDialog/LongList`
  - `Dialogs/AddDeviceDialog/WebSerialConnectionLog`
  - `Panels/DeviceDashboardPanel/Default`
  - `Panels/DeviceDashboardPanel/HeaderBadgeWrapRegression`
  - `Panels/DeviceInfoPanel/SettingsMaintenance`
  - `Panels/DeviceInfoPanel/InfoSummary`
  - `Panels/DeviceInfoPanel/WebSerialActivity`
  - `Panels/DeviceInfoPanel/NarrowWebSerial`

### 本轮真机修复结论（2026-06-29）

- companion CLI 的 human output 曾把成功 envelope 直接压成单个 `ok`，导致真机 `status` / `ports` / `wifi-show` 对开发者几乎不可读；现已改为在存在 `result` 时递归展示真实 payload，并补上回归测试。
- 当前 V3 板上的 Local USB 控制路径要求顺序使用：同一串口的并发命令会被 companion 互斥保护收敛为 `device busy`，这属于预期门禁，不再视为控制面缺陷。
- `port.power_set` 在真机上存在状态回显收敛窗口；顺序 HIL 证明 `accepted=true` 后等待数秒再读 `ports`，可以观测到 `power_enabled` 与 `telemetry.status` 正确切换。
- `reset` 在真机上应作为独占操作单独执行：命令返回 `{"accepted":true}` 后 USB 会短暂断开并重新枚举，后续 `discover` / `status` 需要等待板子重新上线。
- `DeviceDashboardPanel` 顶部 status badges 曾在紧凑桌面宽度下与右侧 Build/Last ok/Notes 摘要发生重叠；现已改为可换行 badge 流式布局，并补上 `HeaderBadgeWrapRegression` Storybook 回归场景。
- `just web-storybook` 曾被 Vite dev proxy 的 `ISOLARAIL_DEVD_ORIGINS` 强制要求阻断，导致 mock-only 视觉取证面在无 companion 环境下无法启动；现已在 Storybook 入口默认关闭 dev proxy 并重新验证通过。

### 历史已记录证据（需后续按收口阶段择机重放）

- `just web-test-companion-bridge`
- `just web-test-unit`
- `just web-build`
- `just web-test-e2e`
- `cargo +esp build --release`
- `source ~/export-esp.sh && cargo check`
- `source ~/export-esp.sh && cargo build --release`
- `source ~/export-esp.sh && cargo +esp fmt --all -- --check`
- `source ~/export-esp.sh && cargo +esp clippy --bin isolarail -- -D warnings`
- `cargo check --target aarch64-apple-darwin`（`tools/isolarail-companion`）
- `cargo test --target aarch64-apple-darwin web_storage -- --nocapture`（`tools/isolarail-companion`）
- `cargo test --target aarch64-apple-darwin delete_http_profile_keeps_usb_profile -- --nocapture`（`tools/isolarail-companion`）
- `cargo test --target aarch64-apple-darwin matches_wifi_set_verification_shape -- --nocapture`（`tools/isolarail-companion`）
- `npm run build`（`web`）
- `bun test ./src`（`web`）
- `npm run build-storybook`（`web`）
- `git diff --check`
- Chrome / Web runtime smoke：`/devices/f1fb44/info` 使用 canonical URL，页面链接不再包含 `--usb`，USB-only profile 不再进入 `Device not found`。
- `USB_PORT=/dev/cu.usbmodem2123101 SELECTOR='--device usb--dev-cu-usbmodem2123101' just status`
- `USB_PORT=/dev/cu.usbmodem2123101 SELECTOR='--device usb--dev-cu-usbmodem2123101' just ports`
- `USB_PORT=/dev/cu.usbmodem2123101 SELECTOR='--device usb--dev-cu-usbmodem2123101' just wifi-show`
- `USB_PORT=/dev/cu.usbmodem2123101 SELECTOR='--device usb--dev-cu-usbmodem2123101' TAIL=12 just monitor`
- `USB_PORT=/dev/cu.usbmodem2123101 SELECTOR='--device usb--dev-cu-usbmodem2123101' just diagnostics-export`
- `USB_PORT=/dev/cu.usbmodem2123101 SCAN=1 just discover`
- `USB_PORT=/dev/cu.usbmodem2123101 SCAN=1 just hardware-available`
- `USB_PORT=/dev/cu.usbmodem2123101 just isolarail --json status --device usb--dev-cu-usbmodem2123101`
- `USB_PORT=/dev/cu.usbmodem2123101 just isolarail --json ports --device usb--dev-cu-usbmodem2123101`
- `USB_PORT=/dev/cu.usbmodem2123101 just isolarail --json wifi show --device usb--dev-cu-usbmodem2123101`
- `USB_PORT=/dev/cu.usbmodem2123101 SELECTOR='--device usb--dev-cu-usbmodem2123101' PORT=port1 ENABLED=false just port-power`
- `USB_PORT=/dev/cu.usbmodem2123101 SELECTOR='--device usb--dev-cu-usbmodem2123101' PORT=port1 ENABLED=true just port-power`
- `USB_PORT=/dev/cu.usbmodem2123101 SELECTOR='--device usb--dev-cu-usbmodem2123101' PORT=port1 just port-replug`
- 并发 HIL：同一 `USB_PORT=/dev/cu.usbmodem2123101` 下并行 `status` / `ports` / `wifi-show` / `monitor`，四个进程退出码均为 0。

## 待完成

- 远端收口项：完成当前 HEAD 的提交、推送、PR 建立、GitHub checks/review 收敛与 merge。
- 维护项：`src/net*` 中旧双口 skeleton 仍在仓内作为 migration reference；若继续清理 legacy 文件，应单独做删除/迁移任务并保持 `device_contract` / `http_api_v1` / `runtime_control` 为唯一契约来源。
