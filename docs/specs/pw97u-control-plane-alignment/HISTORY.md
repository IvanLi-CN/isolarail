# 四路 USB Hub 控制面对齐历史

## 2026-06-09

- 创建控制面对齐主题规格，确定本项目采用参考项目的控制面架构路线，但保持四路 Hub 硬件语义。
- 明确当前 V3 板上的 `replug` 仅表示受控断电再上电，不承诺真 per-port data disconnect。
- 修正 topic spec 的命名真相：当前项目 owner-facing 命名为 `isohub` / `isohub-devd`，参考项目 `isolapurr` 仅作为架构来源，不再混入本项目软件包与门户命名。
- 补充开发环境启动规范要求：开发者统一通过 `just` 入口使用 companion/web 命令；`isohub-devd` 默认进入 `serve` 原生 IPC 模式，Web 联调仅在显式启动 `bridge-http` 时开放 localhost HTTP bridge。
- 澄清控制面门户语义：当前普通用户默认通过 `isohub` CLI 操作设备；未来若引入 desktop 程序，也必须发现或自启全局单例 `isohub-devd`；手动直启 `devd` 仅用于开发与诊断路径。

## 2026-06-13

- 扩展控制面对齐 spec 的命名真相范围，把软件包名、二进制名、开发者入口 `just`、V3 硬件基线、板级子系统名与 BOM/netlist 别名策略统一冻结。
- 明确 `ESP32-S3`、`CH335F`、`M24C64@0x50`、`Mainboard TCA6408A@0x20`、`Front-panel TCA6408A@0x21`、`Input INA226@0x44` 与 `port1..port4` 的 canonical 命名边界。
- 继续把 reference-only 硬件资料从 current-truth 文档中剥离：`docs/ch335f_tca6408a_appnote.md`、`docs/i2c_gpio_expanders_comparison.md`、`docs/power_management_and_startup_control.md`、`docs/pwm_fan_control_circuit_design.md`、`docs/development_notes.md` 现已被明确标注为参考输入，不再拥有命名裁决权。
- 补充 `Software Package Matrix` 与 `Naming Conformance` 质量门槛，显式冻结 root firmware package、`isohub-companion` workspace/binaries、`web` 包名与禁用旧参考项目/误导性命名的规则。
- 进一步补齐全仓 manifest inventory，把 root `package.json` 的 `isohub-dev-tools`、`gc9d01`、`dashboard_preview`、`icon2raw`、`png2raw` 一并纳入命名归属。
- 增补 manifest coverage audit，明确当前仓内全部 `Cargo.toml` / `package.json` 已被 `pw97u` 覆盖，而 `tools/firmware-catalog/` 当前仅作为 script-only release 编排目录存在。
- 继续补上 reference-only 资料边界：`docs/hardware/**/*.enet.enet` 现已被明确归类为网表 / CAD 导出资产，只能提供 BOM、网络名、料号与历史资产标题证据，不再隐式拥有 owner-facing 命名裁决权。
- 进一步补齐当前板级硬件命名清单，把 `TPS2490`、`ISOUSB211 V1OK`、`PCA9545A@0x70`、`HUB_RESET#`、`VIN_UNSAFE` / `VIN`、`IN_EN` / `IN_PG`、front-panel 5-way switch、fan、buzzer 纳入 canonical naming 边界。
- 进一步把 `INA226` 与 `TMP112` 家族名提升为受控 canonical hardware naming，防止软件设计文档与后续控制面实现对同一遥测器件继续各写各的。
- 把 `TPS82130SILR`、`RT9043GB`、`TCA6408APWR`、`TCA6408ARSVR`、`TCA9535RTWR` 纳入 reference-only 料号策略，明确它们只能停留在历史方案、选型比较或局部电路说明中。
- 继续补齐当前文档和网表里真实使用的板级网络名，把 `USB D+` / `USB D-`、`HUB_SDA` / `HUB_SCL`、`I2C_INT` / `I2C_RESET`、`ISO_OK`、`LCD_CS` / `LCD_RST` / `LCD_RES` / `LCD_BLK`、`BUZZER`、`FAN_PWM` / `FAN_EN` / `FAN_TACH` 纳入 spec 的受控命名和别名策略。
- 在控制面对齐 spec 中补上默认 IPC transport 约束：`isohub-devd serve` 必须走 Unix domain socket / Windows named pipe，localhost HTTP 只允许由显式 `bridge-http` 暴露。
- 把 `gc9d01/examples/**` 的示例 manifest 明确归类为 vendored example package，并补充 `SC8815 + SW2303`、`PCA9545A INT/INTx`、`PSTOP_CTL/PSTOP`、`VIN_ADC`、scoped `RESET#` 等 legacy/scoped 名称的受限使用规则，避免它们漂移进 V3 owner-facing 控制面命名。
- 为主 spec 增补 `Documentation Truth Boundary`，明确 `pw97u` 持有 owner-facing 命名真相，`j6nvw` 持有 V3 pin-level / display / reset 真相；随后将 `docs/hardware_connection_overview.md` 提升为当前 V3 硬件总览，并把 `docs/esp32-s3fh4r2_gpio_assignment_guide.md` 降级为历史 GPIO 参考。
- 完成 Web 活动代码面第一轮命名迁移：`desktopAgent` / `desktopStorage` 替换为 `companionBridge` / `companionStorage`，`Justfile` 中的 companion bridge 单测入口与相关 stories/mock 数据同步改名。
- 继续收口 web 设备页语义：`/devices/:deviceId`、`/hardware`、`/info` 现在分别对应 Dashboard / Hardware / Info，旧 `/overview` 与 `/details` 保留为兼容重定向；同时补上 `just web-test-e2e` 作为正式开发者入口。
- 修正活动设备契约中的 identity 漂移：固件 USB JSONL `wifi.get` 改为 `storage="eeprom"`，hostname / Storybook 示例统一为 `isohub-*`，活动固件 `device.variant` 统一为 `v3`。
- 新增共享 `src/device_identity.rs`，把 `iso-usb-hub` / `isohub-<shortid>` / `v3` / MAC 文本格式化的活动固件真相收口到单一模块，并让 USB JSONL 与 mDNS 复用这套逻辑。
- 新增共享 `src/device_contract.rs`，把活动四路 `info` / `ports` / `wifi` JSON 结果、canonical `port1..port4` 解析与 `state.overcurrent` 字段收口到单一模块，避免后续 HTTP/Wi-Fi 接入再从旧双口 skeleton 复制错误形状。
- 将根仓共享 contract 的 native 测试路径从固件 Xtensa 配置中剥离：把固件专用依赖收口到 Xtensa 目标依赖，限制 `build.ref.rs` 的 linker 参数只作用于嵌入式目标，并把 `just firmware-contract-test` 固定到 native stable toolchain，使 `device_identity` / `device_contract` / `http_api_v1` 的 11 个单元测试可在本机上独立通过。
- 继续把 `src/http_api_v1.rs` 从 helper 集升级为共享 dispatcher：新增 `ApiOutcome` / `ApiPendingAction`，使四路 canonical HTTP contract 现在既能统一返回 `info` / `ports` / `wifi` 只读响应，也能为 `port.power` / `port.replug` 产出可执行的 action plan；相应 native contract 测试增加到 14 项并全部通过。
- 新增共享 `src/runtime_control.rs`，把四路端口 power/replug 动作、Wi‑Fi runtime snapshot 更新与 replug holdoff tick 从 `main.rs` 内联逻辑抽成可复用 helper；USB JSONL 已改为复用这层，native contract 测试扩展到 21 项并通过，为后续 HTTP transport 接入提供同一套动作语义基线。
- companion CLI 现已把 Wi-Fi 写操作限定到 Local USB：`wifi set` / `wifi clear` 会拒绝 `--url` 与 Wi-Fi/LAN saved hardware，保持 `wifi show` 可跨 USB/LAN 只读查询的分层语义。
- companion daemon 现已为普通 Local USB JSONL 请求加入每串口互斥；并发 `status` / `ports` / `wifi` / `monitor` / `diagnostics` 访问不再把 CDC 口抢成随机空响应，而是收敛为稳定串行化或明确 `device busy`。
- `isohub` CLI 的 auto-start 现已补上 endpoint-scoped start gate：多个 CLI 同时发现 daemon 缺席时，不会再各自抢着 spawn `isohub-devd serve`，从而避免对同一 USB 口制造额外的启动竞争。
- `monitor` / `reset` CLI 路径现已在 USB-only selector 下先执行 `devices.scan` 注册，避免固定 IPC 端点场景下直接报 `device not found`。
- `diagnostics export` 现已改为 companion 聚合导出 `status` + `ports` + `wifi` + recent session traces，摆脱对固件侧缺失的 `pd.diagnostics` 方法的硬依赖。
- 将 `src/net.rs` 明确标记为未接入的 legacy dual-port skeleton，防止仓内旧 `port_a` / `port_c` / USB-C route 语义继续被误当成当前四路控制面的实现基线。
- 将 companion 开发路径收紧到显式 USB allowlist：新增 `ISOHUB_USB_PORT` 约束，repo-root `just` 以 `USB_PORT` 暴露它；串口扫描、USB JSONL、flash 与 reset 现在都会拒绝 allowlist 外的串口，避免误操作其他项目硬件。
- 以 `just web-check`、`just web-test-companion-bridge`、`just web-test-unit`、`just web-build`、`just devd-help`、`just isohub --help`、`just tools-test`、`just firmware-check`、`just firmware-contract-test` 作为本轮已验证入口；同时明确 `src/net*` 旧双口 HTTP skeleton 尚未接入主固件，不计入控制面对齐完成证据。
