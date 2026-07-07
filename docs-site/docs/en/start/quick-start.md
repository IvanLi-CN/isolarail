---
title: Quick Start
description: The shortest repeatable path from toolchain setup to docs preview and board bring-up.
---

<!-- markdownlint-disable MD025 -->

# Quick Start

This page describes the shortest repeatable path for the current V3 baseline. The default development entrypoint is `just`; `bun` is used for Web/docs tooling, and `cargo +esp` is used for ESP32-S3 firmware.

## Prepare the environment

Firmware builds require the Espressif Rust toolchain:

```bash
cargo install espup
espup install
source ~/export-esp.sh
cargo install espflash
```

The local companion, Web app, and docs site also need:

```bash
cargo install just
bun install --frozen-lockfile
```

`espflash` is used behind the `isolarail-devd` path. It is not the normal direct flashing entrypoint for this project.

## Entrypoint rules

Run normal development commands from the repository root through `just`.

- Firmware checks consistently use the `+esp` toolchain and `xtensa-esp32s3-none-elf` target.
- Companion tools build from the correct `tools/isolarail-companion/` context instead of inheriting the root Xtensa target.
- Flash, reset, and monitor paths go through `isolarail` / `isolarail-devd` identity checks before touching hardware.

Avoid these as routine paths:

- raw `espflash flash --monitor`
- hand-written `cargo --manifest-path tools/isolarail-companion/Cargo.toml ...` from the repo root
- state-changing commands without an explicit `SELECTOR`

## Build firmware

Use the repository entrypoints first:

```bash
just firmware-check
just firmware-build
just firmware-bin
```

The underlying ESP32-S3 gates are:

```bash
cargo +esp check --target xtensa-esp32s3-none-elf
cargo +esp build --release --target xtensa-esp32s3-none-elf
```

## Attach one board

Do not auto-select a serial port. List candidates, then bind one physical port to an observed device identity:

```bash
just ports
PORT=/dev/cu.usbmodem1101 just identify
```

Normal flashing and monitoring use the project safety path:

```bash
just flash-monitor
```

Use `just flash-first-time` only for new hardware or download-mode recovery. That path requires typed confirmation; do not bypass identity checks with raw `espflash flash --monitor`.

## Minimal one-board HIL sequence

When only one board is on the bench, pin the physical serial path first:

```bash
USB_PORT=/dev/cu.usbmodem21234101 just discover
USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' just status
USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' just device-ports
USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' just wifi-show
USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' TAIL=12 just device-monitor
```

`USB_PORT` is forwarded to `ISOLARAIL_USB_PORT`, so scan, JSONL, flash, reset, and monitor paths reject other serial ports.

## Use the local control plane

`isolarail` is the CLI portal. `isolarail-devd` is the local daemon. The default daemon mode is native IPC:

```bash
just tools-build
just tools-test
just isolarail --help
just devd-help
```

Common read-only checks:

```bash
just discover
just devices
just hardware-available
SELECTOR='--device <device-id>' just status
SELECTOR='--device <device-id>' just device-ports
```

Run state-changing operations sequentially against one board:

```bash
SELECTOR='--device <device-id>' PORT=port1 ENABLED=true just port-power
SELECTOR='--device <device-id>' PORT=port1 just port-replug
SELECTOR='--device <device-id>' just device-reset
```

After a state change, leave a short settle window before reading status:

```bash
SELECTOR='--device <device-id>' PORT=port1 ENABLED=false just port-power
sleep 1
SELECTOR='--device <device-id>' just device-ports
```

`device-reset` temporarily drops the USB session. Treat it as a standalone command, wait for re-enumeration, then run `discover` or `status` again.

## Wi-Fi write boundary

Wi-Fi credentials are stored in `M24C64@0x50`. Writes and clears must use a USB-backed path:

```bash
SELECTOR='--device <device-id>' SSID='Lab WiFi' PSK='secret' just wifi-set
SELECTOR='--device <device-id>' just wifi-clear
```

`--url` and Wi-Fi/LAN saved hardware stay read-only for Wi-Fi writes. LAN visibility is not a credential-write authority.

## Build the docs site

The docs site lives in `docs-site/`. Local builds default to a root path:

```bash
bun run docs:build
DOCS_PORT=50885 bun run docs:preview
```

For subpath deployment or GitHub Pages project paths, override `DOCS_BASE`:

```bash
DOCS_BASE=/preview/ bun run docs:build
```

Handwritten links do not depend on a fixed `/isolarail/` path.

## Local quality gates

| Area | Command | Purpose |
| --- | --- | --- |
| Firmware | `just firmware-check` | Fast ESP32-S3 firmware check |
| Firmware | `cargo +esp build --release --target xtensa-esp32s3-none-elf` | Release-shape firmware build |
| Companion | `just tools-build` | Build `isolarail` and `isolarail-devd` |
| Companion | `just tools-test` | Local companion tests |
| Web | `just web-check` / `just web-build` | Frontend checks and build |
| Docs site | `bun run docs:build` | Rspress static build |

If a change only touches the docs site, docs build and Markdown lint are enough. Firmware or companion changes need their own gates.

## Read next

- [Hardware Topology](../hardware/topology): current V3 board truth.
- [Firmware Runtime](../firmware/boot-runtime): boot self-check, gating, and runtime tasks.
- [Control Plane](../control-plane/interfaces): CLI, daemon, USB JSONL, and Web companion boundaries.
- [Front Panel](../dashboard/front-panel): the 160x50 dashboard layout and states.
