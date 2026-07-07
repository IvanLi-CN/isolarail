---
title: 快速开始
description: 从工具链、构建、文档站预览到硬件 bring-up 的最短路径。
---

<!-- markdownlint-disable MD025 -->

# 快速开始

本页给出当前 V3 基线的最短可重复路径。项目的默认开发入口是 `just`；`bun` 只用于 Web 与文档站工具链，`cargo +esp` 用于 ESP32-S3 固件。

## 环境准备

固件构建需要 Espressif Rust 工具链：

```bash
cargo install espup
espup install
source ~/export-esp.sh
cargo install espflash
```

本地 companion、Web 和文档站还需要：

```bash
cargo install just
bun install --frozen-lockfile
```

`espflash` 是 `isolarail-devd` 后端路径的一部分，不是本项目的日常烧录入口。

## 项目入口规则

日常开发只从仓库根目录走 `just`。这样做不是为了包装命令，而是为了固定三件事：

- ESP32-S3 固件始终走 `+esp` toolchain 和 `xtensa-esp32s3-none-elf` target。
- 本机 companion 工具在 `tools/isolarail-companion/` 的正确上下文中构建，避免根目录 Xtensa target 泄漏。
- 烧录、reset、monitor 都先经过 `isolarail` / `isolarail-devd` 的身份检查，不会误操作无关板子。

不要把下面这些当作常规入口：

- 裸 `espflash flash --monitor`
- 从仓库根目录手写 `cargo --manifest-path tools/isolarail-companion/Cargo.toml ...`
- 绕过 `SELECTOR` 的状态变更命令

## 构建固件

先用仓库统一入口检查固件：

```bash
just firmware-check
just firmware-build
just firmware-bin
```

底层等价门禁仍以 ESP32-S3 target 为准：

```bash
cargo +esp check --target xtensa-esp32s3-none-elf
cargo +esp build --release --target xtensa-esp32s3-none-elf
```

## 连接一块板

首次连接不要自动选择串口。先列出候选，再把一个物理端口和设备身份绑定：

```bash
just ports
PORT=/dev/cu.usbmodem1101 just identify
```

普通烧录和监视走项目内安全路径：

```bash
just flash-monitor
```

全新硬件或下载模式下使用 `just flash-first-time`。该路径会要求 typed confirmation；不要用裸 `espflash flash --monitor` 绕过身份校验。

## 单板 HIL 最小序列

如果桌上只有一块板，先把物理串口固定到 `USB_PORT`，再跑一条可复现序列：

```bash
USB_PORT=/dev/cu.usbmodem21234101 just discover
USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' just status
USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' just device-ports
USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' just wifi-show
USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' TAIL=12 just device-monitor
```

`USB_PORT` 会转发为 `ISOLARAIL_USB_PORT`，限制 scan、JSONL、flash、reset、monitor 只碰这一条串口路径。多块开发板同时接入时，这一步很重要。

## 使用本机控制面

`isolarail` 是命令行入口，`isolarail-devd` 是本机 daemon。默认 daemon 模式是 native IPC：

```bash
just tools-build
just tools-test
just isolarail --help
just devd-help
```

常用只读检查：

```bash
just discover
just devices
just hardware-available
SELECTOR='--device <device-id>' just status
SELECTOR='--device <device-id>' just device-ports
```

会改变设备状态的动作要按单块板串行执行，例如：

```bash
SELECTOR='--device <device-id>' PORT=port1 ENABLED=true just port-power
SELECTOR='--device <device-id>' PORT=port1 just port-replug
SELECTOR='--device <device-id>' just device-reset
```

状态变更后给硬件留一个 settle window，再读回状态：

```bash
SELECTOR='--device <device-id>' PORT=port1 ENABLED=false just port-power
sleep 1
SELECTOR='--device <device-id>' just device-ports
```

`device-reset` 会让 USB session 短暂消失。把它当成独立动作执行，等待重新枚举后再跑 `discover` 或 `status`。

## Wi-Fi 写入边界

Wi-Fi 凭据保存在主板 `M24C64@0x50`。写入和清除必须走 USB-backed 路径：

```bash
SELECTOR='--device <device-id>' SSID='Lab WiFi' PSK='secret' just wifi-set
SELECTOR='--device <device-id>' just wifi-clear
```

`--url` 和 Wi-Fi/LAN saved hardware 只能读 Wi-Fi 状态，不能写凭据。这个限制避免 LAN 上的未配对 HTTP 面成为配置写入口。

## 构建文档站

文档站位于 `docs-site/`。本地构建默认使用根路径：

```bash
bun run docs:build
DOCS_PORT=50885 bun run docs:preview
```

如果部署到子路径或 GitHub Pages project path，构建时覆盖 `DOCS_BASE`：

```bash
DOCS_BASE=/preview/ bun run docs:build
```

站内手写链接不依赖固定 `/isolarail/` 路径。

## 本地质量门禁

一次完整开发收口至少覆盖这些门禁：

| 范围 | 命令 | 目的 |
| --- | --- | --- |
| 固件 | `just firmware-check` | 快速检查 ESP32-S3 固件 |
| 固件 | `cargo +esp build --release --target xtensa-esp32s3-none-elf` | release 形态构建 |
| companion | `just tools-build` | 构建 `isolarail` 与 `isolarail-devd` |
| companion | `just tools-test` | 本机工具测试 |
| Web | `just web-check` / `just web-build` | 类型与前端构建 |
| 文档站 | `bun run docs:build` | Rspress 静态构建 |

如果当前任务只改文档站，可以只跑 `bun run docs:build` 和 Markdown lint；如果改到固件或 companion，不能只靠文档站构建收口。

## 常见卡点

| 现象 | 优先检查 |
| --- | --- |
| 找不到设备 | `just ports`、`USB_PORT` 是否指向当前板 |
| 命令返回 `device busy` | 是否对同一串口并发执行了 status/ports/wifi/power |
| 烧录目标不确定 | 先跑 `PORT=... just identify`，不要裸烧 |
| Wi-Fi 写入被拒绝 | selector 是否是 `--device` 或 USB-backed `--hardware` |
| reset 后读不到状态 | 等待 USB 重新枚举，再重新 `discover` |

## 下一步阅读

- [硬件拓扑](../hardware/topology)：确认 V3 板级 current truth。
- [固件运行](../firmware/boot-runtime)：理解启动自检、门控和运行期任务。
- [控制面](../control-plane/interfaces)：理解 CLI、daemon、USB JSONL、Web companion 的边界。
- [前面板显示](../dashboard/front-panel)：查看 160x50 dashboard 布局和状态。
