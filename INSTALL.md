# Development setup

## Firmware toolchain

Install the ESP Rust toolchain and backend tools:

```bash
cargo install espup
espup install
source ~/export-esp.sh
cargo install espflash
cargo install just
```

`espflash` is required because `isolarail-devd` invokes it internally. Firmware flashing still goes through `just flash-monitor` or `PORT=/dev/cu.xxx just flash-first-time`, not through direct `espflash` commands.

Verify the setup:

```bash
rustup toolchain list
just firmware-check
just tools-build
```

## Web workspace

Use the repo-root `just` commands for web development and checks.

```bash
just web-install
just web-check
```

## Companion workspace

The local companion workspace is isolated under `tools/isolarail-companion/` so it does not inherit the firmware Xtensa target defaults.

Verify it with:

```bash
just tools-build
just tools-test
just isolarail --help
just devd-help
```

Notes:

- Prefer the repo-root `just` commands for all normal companion development.
- If you need raw Cargo, `cd tools/isolarail-companion` first and run Cargo there.
- Avoid `cargo --manifest-path tools/isolarail-companion/Cargo.toml ...` from the repo root; the firmware Xtensa target default can leak into that invocation.
- To constrain companion discovery and Local USB operations to one serial device, set `USB_PORT=/dev/cu.usbmodem...` on the `just` command. The workspace forwards it as `ISOLARAIL_USB_PORT`.
- `just wifi-set` and `just wifi-clear` only support `--device` or USB-backed `--hardware`; Wi-Fi/LAN `--url` selectors remain read-only.
- `just diagnostics-export` emits a companion-side diagnostics bundle derived from the current Local USB `status`, `ports`, `wifi`, and recent serial session activity.

## Firmware flashing

Use the Local USB CLI/devd path for normal firmware work:

```bash
just ports
PORT=/dev/cu.xxx just identify
just firmware-bin
just flash-monitor
```

First-time or download-mode flashing is explicit:

```bash
PORT=/dev/cu.xxx just flash-first-time
```

## Local browser development

When the browser needs Local USB or companion-backed storage, start the explicit Web companion separately:

```bash
BIND=127.0.0.1:51200 ALLOW_DEV_CORS=1 just devd-web
DEVD_ORIGINS=http://isolarail-devd.local:51200,http://127.0.0.1:51200 just web-dev
```

This is distinct from normal daemon mode:

- `just devd-serve`: native IPC only
- `just devd-web`: localhost Web companion for browser development and same-origin Web hosting
- Both commands invoke the Rust `isolarail-devd` binary directly from `tools/isolarail-companion/`.

Normal local device operation still goes through `isolarail`; direct `devd` startup is only for development and diagnostics.

## Hardware safety

Do not run flash, reset, monitor, port power, or replug commands against an unrelated device.
