# IsoHub Web

React SPA (Vite + React + TypeScript) for the four-port IsoHub USB Hub control surface.

## Pages

- `/` — Dashboard (multi-device grid)
- `/devices/:deviceId` — Device Dashboard
- `/devices/:deviceId/hardware` — Device Hardware
- `/devices/:deviceId/info` — Device Info
- `/about` — About

Compatibility redirects remain in place for legacy local links:

- `/devices/:deviceId/overview` → `/devices/:deviceId`
- `/devices/:deviceId/details` → `/devices/:deviceId/info`

## Quick start

- Install: `just web-install`
- Dev server: `just web-dev` (default: `http://127.0.0.1:45173`)
- Storybook: `just web-storybook` (default: `http://127.0.0.1:46006`)
- Build: `just web-build`

## `isohub-devd` modes

- `isohub-devd serve`: default daemon mode, local IPC only.
- `isohub-devd bridge-http`: opt-in localhost HTTP bridge for browser surfaces.

For normal local operation, users go through `isohub` CLI. It is responsible for discovering or auto-starting the `serve` singleton. Direct manual `devd` use is a development and diagnostics path, not the normal user portal.

Native IPC remains the default integration path for local companion tools. On Unix, `serve` uses a Unix domain socket under the runtime directory chosen by `default_ipc_endpoint()`. On Windows, it uses the named pipe `\\.\pipe\isohub-devd`.

## Normal local operation

For normal local USB work, use `isohub` as the entrypoint. It discovers or auto-starts the `isohub-devd serve` singleton when needed.

`isohub wifi set` and `isohub wifi clear` are intentionally Local-USB-only in the CLI path. Saved Wi-Fi/LAN hardware and direct `--url` selectors remain read-only for Wi-Fi state inspection.

## Manual devd diagnostics

Direct manual `devd` startup is for development and diagnostics only.

```bash
just devd-serve
```

Notes:

- `tools/isohub-companion/` is isolated from the firmware toolchain by its own `.cargo/config.toml` and `rust-toolchain.toml`.
- If the pinned companion toolchain is missing, install the exact channel from `tools/isohub-companion/rust-toolchain.toml`.
- The repo-root `just` command invokes the Rust `isohub-devd` binary directly from `tools/isohub-companion/`.
- If you need to keep Local USB work pinned to one serial device, run the companion commands as `USB_PORT=/dev/cu.usbmodem... just ...`.
- Override the IPC endpoint only when necessary with `ISOHUB_DEVD_ENDPOINT=<path-or-pipe>`.

## Local development with Web bridge

The browser cannot consume native IPC directly. For Local USB and companion-backed storage flows in `web/`, start the HTTP bridge explicitly in a separate terminal:

```bash
just devd-http-bridge
```

Then start the Vite dev server:

```bash
just web-dev
```

Development behavior:

- Vite proxies `/api/v1/*` to `http://127.0.0.1:51200` by default.
- This keeps the browser on a same-origin API path during development, so Local USB flows do not require `--allow-dev-cors`.
- Override the proxy target with `DEVD_ORIGIN=http://127.0.0.1:<port> just web-dev` if the bridge is not using `51200`.
- Disable the proxy only for deliberate diagnostics with `ISOHUB_DEV_PROXY=0`.

If `Local USB` still reports the service as unavailable:

- Confirm the explicit `bridge-http` process is running and `http://127.0.0.1:51200/api/v1/bootstrap` returns JSON.
- Confirm the Vite server was started after the proxy config change.
- Do not treat `bridge-http` as the default daemon mode. It is an explicit browser bridge layered alongside the native IPC path.

## Review tips

- Storybook includes viewport presets for quick layout checks:
  - `IsoHub Mobile (390×844)`
  - `IsoHub Wide (1440×900)`

## Quality gates

- `just web-check`
- `just web-build`
- `just web-test-companion-bridge`
- `just web-test-e2e`
- `just web-test-unit`
- `just web-test-storybook`
