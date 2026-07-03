# CLI/devd Flash Migration (#q9d7h)

## 状态

- Status: 部分完成（3/4）
- Created: 2026-07-03
- Last: 2026-07-03

## 背景 / 问题陈述

本仓固件烧录仍以裸 `espflash`、`cargo run` 和 `mcu-agentd` 配置为主。参考 `isolapurr-usb-hub` 的 Local USB 模式后，默认开发路径应迁移到项目内 CLI + devd：daemon 负责本地串口与烧录操作，CLI 提供可重复的人类入口，并在普通烧录前做设备身份校验。

## 目标 / 非目标

### Goals

- 复用并收紧现有 `isohub-devd` 本地 IPC daemon 与 `isohub` CLI。
- 建立端口枚举、端口选择、身份确认、app `.bin` 生成、普通烧录、首次 full flash、reset 与 monitor 的闭环。
- 固件通过既有 USB Serial/JTAG JSONL `info` 方法返回稳定身份与固件信息。
- 将默认开发入口迁移到 `just` + `isohub`，并把 `mcu-agentd` 降为 legacy/emergency。

### Non-goals

- 不新增 Wi-Fi、Web Serial、Web UI、desktop GUI 或 localhost HTTP bridge 能力。
- 不新增端口电源、过流、风扇、诊断导出等设备控制命令。
- 不发布 release installer 或跨平台 host-tools 包。
- 不修改 CI target 到 ESP32-S3。

## 范围

### In scope

- 既有固件 USB JSONL `info` 合同与 companion 身份校验路径。
- 既有 `tools/isohub-companion` host-tools crate，包含 `isohub` / `isohub-devd`。
- `Justfile`、`tools/isohub-runner`、`.cargo/config.toml`、README/INSTALL/AGENTS 文档入口。

### Out of scope

- 通过 CLI/devd 控制运行期端口状态。
- 用户级安装器和 GitHub Release asset 产出。

## 需求

### MUST

- CLI/devd 默认使用本地 IPC；不得把 localhost HTTP 作为默认 devd transport。
- CLI 不得自动选择串口，即使只有一个候选端口。
- `.esp32-port` 缺失或身份未确认时，普通烧录、reset、monitor 必须停止并提示 `just select-port` 或 `PORT=/dev/cu.xxx just identify`。
- 普通烧录必须先读取 JSONL `info` 并匹配缓存中的 `device_id` 或 `mac`。
- 首次 full flash 允许在 `identity=unconfirmed` 或显式 unconfirmed port 下执行，但必须展示端口、ELF 与 app image 证据，并要求 typed confirmation。
- 普通 app 烧录只写 app image 到 `0x10000`。
- 固件 JSONL 响应必须包含 `device.device_id`、`device.mac`、`device.firmware.name`、`device.firmware.version`、`device.uptime_ms`。

### SHOULD

- `cargo run --release` 通过 `tools/isohub-runner` 复用同一 Local USB 安全边界。
- `mcu-agentd` 只通过 legacy/emergency passthrough 暴露，不作为 README 默认路径。

## 验收标准

- Given 无 `.esp32-port`，When 运行 `just flash`、`just reset` 或 `just monitor`，Then 命令失败并提示选择/确认端口。
- Given 固件运行且端口已确认，When `PORT=/dev/cu.xxx just identify`，Then `.esp32-port` 写入端口、`device_id` 与 `mac`。
- Given `.esp32-port` 身份与设备 `info` 匹配，When `just flash-monitor`，Then 生成 app `.bin`，写入 `0x10000`，reset 后进入 monitor。
- Given 端口身份未确认，When 首次 `just flash`，Then 只有 typed confirmation 后才执行 full ELF flash，并在重启后尝试回填身份。

## 非功能性验收 / 质量门槛

- `cargo +esp check`
- `cargo +esp build --release`
- `cargo test` inside `tools/isohub-companion`
- `just firmware-bin`

## 文档更新

- `README.md`
- `INSTALL.md`
- `AGENTS.md`
- `docs/specs/README.md`

## 实现里程碑

- [x] M1: 固件 USB JSONL `info` 合同确认
- [x] M2: `isohub` / `isohub-devd` host tools 复用
- [x] M3: Justfile、runner、legacy passthrough 与文档同步
- [ ] M4: 真机 `identify` / `flash-monitor` 验证

## 风险 / 开放问题 / 假设

- 真机硬件是否连接可用未检查；无硬件时只能验证构建与安全失败路径。
