# iso-usb-hub

Four-port USB hub control plane for the current V3 hardware baseline:
`ESP32-S3 + CH335F + M24C64@0x50 + EN1..EN4 + PWREN#/OVCUR# + LCD/front panel`.

Owner-facing naming is fixed as:

- firmware identity: `iso-usb-hub`
- local CLI: `isohub`
- local daemon: `isohub-devd`

## Developer entrypoints

Use `just` as the only normal developer entrypoint.

### Firmware

```bash
just firmware-check
just firmware-build
just ports
PORT=/dev/cu.usbmodem1101 just identify
just firmware-bin
just flash-monitor
just firmware-ports
```

Notes:

- `just flash-monitor` builds the app image, validates the selected device identity, flashes at `0x10000`, resets, and opens the monitor path.
- `cargo run --release` uses `tools/isohub-runner` and follows the same Local USB safety boundary.
- `just flash-first-time` is the explicit download-mode/non-project firmware path and requires typed confirmation from `isohub`.
- No hardware command should be run against an unrelated board.

### Local companion tools

`isohub` is the normal local USB entrypoint. It discovers or auto-starts the native-IPC `isohub-devd serve` singleton when needed.

Important:

- Use the `just` entrypoints for companion development from the repo root.
- If you run companion Cargo commands manually, run them from `tools/isohub-companion/`.
- Do not rely on `cargo --manifest-path tools/isohub-companion/Cargo.toml ...` from the repo root; the root Xtensa default target can leak into that invocation.

```bash
just tools-build
just tools-test
just isohub --help
just devd-help
```

Common device commands:

```bash
just discover
just devices
just hardware-available

SELECTOR='--device <device-id>' just status
SELECTOR='--device <device-id>' just device-ports
SELECTOR='--device <device-id>' just wifi-show

SELECTOR='--device <device-id>' PORT=port1 ENABLED=true just port-power
SELECTOR='--device <device-id>' PORT=port1 just port-replug
SELECTOR='--device <device-id>' just device-reset

SELECTOR='--device <device-id>' SSID='Lab WiFi' PSK='secret' just wifi-set
SELECTOR='--device <device-id>' just wifi-clear

SELECTOR='--device <device-id>' TAIL=200 just device-monitor
SELECTOR='--device <device-id>' just diagnostics-export
```

Notes:

- `just device-monitor` reads the recent Local USB serial activity timeline from `isohub-devd`.
- `just diagnostics-export` exports a companion-aggregated diagnostics snapshot built from the current `status`, `ports`, `wifi`, and recent serial session traces for the selected device.

To restrict companion discovery and Local USB operations to one specific serial device during development, pass `USB_PORT`:

```bash
USB_PORT=/dev/cu.usbmodem2123101 just discover
USB_PORT=/dev/cu.usbmodem2123101 just devd-serve
USB_PORT=/dev/cu.usbmodem2123101 just isohub --help
```

`USB_PORT` is forwarded to `ISOHUB_USB_PORT`, so scan, JSONL, flash, reset, and monitor paths reject other serial ports.

Recommended HIL sequence for one board:

```bash
USB_PORT=/dev/cu.usbmodem21234101 just discover
USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' just status
USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' just device-ports
USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' just wifi-show
USB_PORT=/dev/cu.usbmodem21234101 SELECTOR='--device usb--dev-cu-usbmodem21234101' TAIL=12 just device-monitor
```

Notes:

- Run Local USB commands sequentially against one board. The companion enforces per-port mutual exclusion, so overlapping `status` / `ports` / `wifi-show` / `port-power` runs can legitimately return `device busy`.
- Mutating commands such as `port-power` and `port-replug` may need a short settle window before a follow-up `just device-ports` reflects the new state.
- `just device-reset` reboots the board and temporarily drops the USB session. Treat it as a standalone command, then wait for re-enumeration before the next `discover` / `status`.

Selector rules:

- Use `SELECTOR='--device <device-id>'` for a currently connected temporary USB target.
- Use `SELECTOR='--hardware <saved-id>'` for a saved hardware profile.
- `just wifi-set` and `just wifi-clear` require `--device` or a USB-backed `--hardware` selector. `--url` and Wi-Fi/LAN saved hardware stay read-only for Wi-Fi writes.

### devd modes

`isohub-devd` has two distinct modes:

- `serve`: native IPC only, default daemon mode
- `web`: explicit localhost Web companion for browser development and same-origin Web hosting

Manual daemon startup is a development and diagnostics path, not the normal user workflow.

```bash
just devd-serve
just devd-web
```

Important:

- `just devd-serve` starts the native IPC daemon path only.
- `just devd-web` is the opt-in browser Web companion. It is not the default daemon mode.
- Both repo-root `just` commands invoke the Rust `isohub-devd` binary directly from `tools/isohub-companion/`.
- On Unix, IPC uses the runtime socket returned by `default_ipc_endpoint()`.
- On Windows, IPC uses `\\.\pipe\isohub-devd`.

### Web

```bash
just web-install
just web-check
just web-lint
just web-build
just web-test-unit
just web-storybook
```

For browser development that needs Local USB or companion-backed storage:

```bash
BIND=127.0.0.1:51200 ALLOW_DEV_CORS=1 just devd-web
DEVD_ORIGINS=http://isohub-devd.local:51200,http://127.0.0.1:51200 just web-dev
```

The Web app never scans localhost ports. `DEVD_ORIGINS` is an explicit ordered list: put the mDNS URL first, then an IP or localhost fallback. `ALLOW_DEV_CORS=1` is only needed when the Vite page directly tries multiple configured origins; same-origin `--web-root` hosting does not need it.

## Toolchain

The firmware build expects the `esp` Rust toolchain:

```bash
cargo install espup
espup install
source ~/export-esp.sh
cargo install espflash
```

## Validation status

Re-verified on the current `HEAD` in this dev environment:

- `just --list`
- `just tools-build`
- `just tools-test`
- `just web-check`
- `just web-build`
- `just web-test-unit`
- `just isohub --help`
- `just devd-help`
- `just firmware-check`
- `just firmware-contract-test`
- `cargo +esp check --release`

Additional quality gates that remain part of the expected developer workflow, but were not re-run in this pass:

- `just web-test-companion-bridge`
- `just web-test-e2e`
- `just web-test-storybook`

## Reference docs

- [docs/hardware_connection_overview.md](docs/hardware_connection_overview.md)
- [docs/specs/j6nvw-hardware-v3-pin-assignment/SPEC.md](docs/specs/j6nvw-hardware-v3-pin-assignment/SPEC.md)
- [docs/software_design.md](docs/software_design.md)
- [web/README.md](web/README.md)
- [docs/specs/README.md](docs/specs/README.md)
