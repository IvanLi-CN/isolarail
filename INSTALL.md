# Development setup

## Firmware toolchain

Install the ESP Rust toolchain and flashing tools:

```bash
cargo install espup
espup install
source ~/export-esp.sh
cargo install espflash
```

Verify the setup:

```bash
rustup toolchain list
just firmware-check
```

## Web workspace

Use the repo-root `just` commands for web development and checks.

```bash
just web-install
just web-check
```

## Companion workspace

The local companion workspace is isolated under `tools/isohub-companion/` so it does not inherit the firmware Xtensa target defaults.

Verify it with:

```bash
just tools-build
just tools-test
just isohub --help
just devd-help
```

Notes:

- Prefer the repo-root `just` commands for all normal companion development.
- If you need raw Cargo, `cd tools/isohub-companion` first and run Cargo there.
- Avoid `cargo --manifest-path tools/isohub-companion/Cargo.toml ...` from the repo root; the firmware Xtensa target default can leak into that invocation.
- To constrain companion discovery and Local USB operations to one serial device, set `USB_PORT=/dev/cu.usbmodem...` on the `just` command. The workspace forwards it as `ISOHUB_USB_PORT`.
- `just wifi-set` and `just wifi-clear` only support `--device` or USB-backed `--hardware`; Wi-Fi/LAN `--url` selectors remain read-only.
- `just diagnostics-export` emits a companion-side diagnostics bundle derived from the current Local USB `status`, `ports`, `wifi`, and recent serial session activity.

## Local browser development

When the browser needs Local USB or companion-backed storage, start the explicit HTTP bridge separately:

```bash
just devd-http-bridge
just web-dev
```

This is distinct from normal daemon mode:

- `just devd-serve`: native IPC only
- `just devd-http-bridge`: localhost HTTP bridge for browser development
- Both commands invoke the Rust `isohub-devd` binary directly from `tools/isohub-companion/`.

Normal local device operation still goes through `isohub`; direct `devd` startup is only for development and diagnostics.

## Hardware safety

Do not run flash, reset, monitor, port power, or replug commands against an unrelated device.
