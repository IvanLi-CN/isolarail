# IsoHub Web

React SPA (Vite + React + TypeScript) for the four-port IsoHub USB Hub control surface.

## Pages

- `/` — Dashboard (multi-device grid)
- `/devices/:deviceId` — Device Dashboard
- `/devices/:deviceId/settings` — Device Settings
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
- `isohub-devd web`: opt-in localhost Web companion for browser surfaces.

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

## Local development with Web companion

The browser cannot consume native IPC directly. For Local USB and companion-backed storage flows in `web/`, start the Web companion explicitly in a separate terminal:

```bash
BIND=127.0.0.1:51200 just devd-web
```

Then start the Vite dev server:

```bash
just web-dev
```

Development behavior:

- Vite proxies `/api/v1/*` to the first explicit `DEVD_ORIGINS` entry.
- This keeps the browser on a same-origin API path during development, so Local USB flows do not require `--allow-dev-cors`.
- `just web-dev` defaults `ISOHUB_DEVD_ORIGINS` to `http://isohub-devd.local:51200,http://127.0.0.1:51200`.
- Put the mDNS URL first and an IP or localhost URL second only when overriding the default.
- The Web app never scans localhost ports; every fallback origin must be explicitly configured.
- Disable the proxy only for deliberate diagnostics with `ISOHUB_DEV_PROXY=0`.
- `just web-storybook` always disables the dev proxy and stays mock-only; it must not require a running `isohub-devd web`.

If `Local USB` still reports the service as unavailable:

- Confirm the explicit `isohub-devd web` process is running and one configured origin returns `/api/v1/bootstrap` JSON.
- Confirm the Vite server was started after the proxy config change.
- Do not treat `web` as the default daemon mode. It is an explicit browser companion layered alongside the native IPC path.

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
