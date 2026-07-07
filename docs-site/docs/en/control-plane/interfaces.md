---
title: Interfaces and Tools
description: USB JSONL, HTTP, isolarail CLI, isolarail-devd daemon, and Web companion boundaries.
---

<!-- markdownlint-disable MD025 -->

# Interfaces and Tools

The IsolaRail control plane combines device firmware, a local daemon, a CLI, and the Web app. The current owner-facing entrypoint is the `isolarail` CLI; `isolarail-devd` is the local service and should not require ordinary users to manage it manually.

## Layering

```text
user
  ├─ isolarail CLI
  │    └─ native IPC -> isolarail-devd serve
  │          └─ USB JSONL / flash / reset / monitor
  ├─ Web app
  │    ├─ Wi-Fi / LAN HTTP
  │    ├─ Web Serial
  │    └─ explicit Local USB bridge from isolarail-devd web
  └─ direct firmware links
       ├─ USB CDC JSONL
       └─ HTTP / LAN v1
```

The default path is `isolarail` automatically finding or starting `isolarail-devd serve`. `isolarail-devd web` is explicit and only for browser development or same-origin Web hosting.

## Naming and identity

| Scope | Current name |
| --- | --- |
| Firmware identity | `isolarail` |
| CLI | `isolarail` |
| Daemon | `isolarail-devd` |
| Companion source | `tools/isolarail-companion/` |
| Device hostname | `isolarail-<shortid>` |
| Port IDs | `port1`, `port2`, `port3`, `port4` |

These names are owned by `docs/specs/pw97u-control-plane-alignment/SPEC.md`. Web, CLI, README, and diagnostics should not invent alternate product names.

## USB JSONL

Firmware exposes core operations over USB Serial/JTAG JSONL:

- `info`
- `ports.get`
- `port.power_set`
- `port.replug`
- `wifi.get`
- `wifi.set`
- `wifi.clear`
- `reboot`

The `info` response must include device identity, MAC, firmware name, version, and uptime. The companion uses those fields for identity checks so flashing and control commands are not sent to an unrelated board.

| Command | Kind | Meaning |
| --- | --- | --- |
| `info` | read | Firmware identity, hostname, MAC, version, uptime |
| `ports.get` | read | `port1..port4` power, sideband, OCP, and telemetry state |
| `port.power_set` | write | Set one port's owner manual output allowance |
| `port.replug` | write | Controlled power-off then restore for one port |
| `wifi.get` | read | Wi-Fi configuration state without exposing PSK |
| `wifi.set` | write | Write Wi-Fi credentials through a USB-backed path |
| `wifi.clear` | write | Clear Wi-Fi credentials |
| `reboot` | write | Reboot the device |

`port.power_set` and `port.replug` only accept `port1..port4`; historical `USB-A`, `USB-C`, and `route` models are not current control-plane IDs.

## HTTP / LAN

The target device-side HTTP v1 surface includes:

- `GET /api/v1/health`
- `GET /api/v1/info`
- `GET /api/v1/ports`
- `GET /api/v1/ports/{portId}`
- `POST /api/v1/ports/{portId}/power`
- `POST /api/v1/ports/{portId}/actions/replug`
- `GET /api/v1/wifi`
- `POST /api/v1/reboot`

Device HTTP v1 is not an account or cloud-auth surface. Wi-Fi writes still require a USB-backed device path.

| Path | Use | Write boundary |
| --- | --- | --- |
| `/api/v1/health` | Liveness | stateless |
| `/api/v1/info` | identity, hostname, version, network state | read-only |
| `/api/v1/ports` | four-port summary | read-only |
| `/api/v1/ports/{portId}` | one-port detail | read-only |
| `/api/v1/ports/{portId}/power` | maintenance power toggle | explicit action |
| `/api/v1/ports/{portId}/actions/replug` | controlled power cycle | explicit action |
| `/api/v1/wifi` | Wi-Fi state | read-only |
| `/api/v1/reboot` | maintenance reboot | explicit action |

Seeing the device on LAN does not authorize Wi-Fi credential writes.

## `isolarail-devd` modes

`isolarail-devd` has two modes:

- `serve`: default native IPC daemon for local CLI/desktop paths.
- `web`: explicit localhost Web companion for browser development and same-origin Web hosting.

Do not treat localhost HTTP as the default daemon transport. The Web runtime must not scan localhost ports; usable origins must come from same-origin bootstrap or explicit `DEVD_ORIGINS`.

| Mode | Default exposure | Use |
| --- | --- | --- |
| `isolarail-devd serve` | Unix domain socket / Windows named pipe | CLI, future desktop, local singleton daemon |
| `isolarail-devd web` | explicit localhost Web companion | browser development and same-origin Web hosting |

Ordinary users should not need to start the daemon first; the CLI connects to an existing instance or starts `serve`.

## CLI selectors

The CLI separates temporary devices from saved hardware:

- `--device <device-id>`: currently connected temporary USB target.
- `--hardware <saved-id>`: saved hardware profile.

Wi-Fi write and clear operations require `--device` or USB-backed `--hardware`. `--url` and Wi-Fi/LAN saved hardware stay read-only.

| Selector | Source | Wi-Fi writes | Port writes | Notes |
| --- | --- | --- | --- | --- |
| `--device <device-id>` | current USB device | yes | yes | preferred for bring-up and service |
| `--hardware <saved-id>` | saved profile | USB-backed only | yes | confirm the active channel |
| `--url <http-url>` | LAN HTTP | no | limited maintenance actions | no credential writes |

## Common commands

Read-only checks:

```bash
just discover
just devices
just hardware-available
SELECTOR='--device <device-id>' just status
SELECTOR='--device <device-id>' just device-ports
SELECTOR='--device <device-id>' just wifi-show
```

Device actions:

```bash
SELECTOR='--device <device-id>' PORT=port1 ENABLED=true just port-power
SELECTOR='--device <device-id>' PORT=port1 just port-replug
SELECTOR='--device <device-id>' just device-reset
```

Diagnostics:

```bash
SELECTOR='--device <device-id>' TAIL=200 just device-monitor
SELECTOR='--device <device-id>' just diagnostics-export
```

State-changing commands should run sequentially. The companion enforces mutual exclusion for the same serial path, so overlapping requests can legitimately return `device busy`.

## Diagnostics export

`just diagnostics-export` should aggregate:

- `status`
- `ports`
- `wifi`
- recent Local USB serial session traces
- daemon-observed identity and selector information

The goal is to reproduce what state one board was in, not just capture the last error line.

## Web app boundary

The Web app arbitrates three channels:

- Wi-Fi / LAN
- Web Serial
- Local USB bridge

It does not scan localhost, does not point users at implicit port discovery, and does not bypass `isolarail-devd` identity checks.

Web runtime arbitration:

- Wi-Fi / LAN, Web Serial, and Local USB bridge may all be available.
- The same device must not appear as duplicates just because channels differ.
- Last successful channel wins; if it fails, promote another available channel.
- unsupported, busy, offline, and USB-only states must show clear next actions.

## References

- `README.md`
- `docs/specs/pw97u-control-plane-alignment/SPEC.md`
- `docs/specs/q9d7h-cli-devd-flash-migration/SPEC.md`
