# IsolaRail Web

React SPA (Vite + React + TypeScript) for the four-port IsolaRail USB Hub control surface.

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

## `isolarail-devd` modes

- `isolarail-devd serve`: default daemon mode, local IPC only.
- `isolarail-devd web`: opt-in localhost Web companion for browser surfaces.

For normal local operation, users go through `isolarail` CLI. It is responsible for discovering or auto-starting the `serve` singleton. Direct manual `devd` use is a development and diagnostics path, not the normal user portal.

Native IPC remains the default integration path for local companion tools. On Unix, `serve` uses a Unix domain socket under the runtime directory chosen by `default_ipc_endpoint()`. On Windows, it uses the named pipe `\\.\pipe\isolarail-devd`.

## Normal local operation

For normal local USB work, use `isolarail` as the entrypoint. It discovers or auto-starts the `isolarail-devd serve` singleton when needed.

`isolarail wifi set` and `isolarail wifi clear` are intentionally Local-USB-only in the CLI path. Saved Wi-Fi/LAN hardware and direct `--url` selectors remain read-only for Wi-Fi state inspection.

## Manual devd diagnostics

Direct manual `devd` startup is for development and diagnostics only.

```bash
just devd-serve
```

Notes:

- `tools/isolarail-companion/` is isolated from the firmware toolchain by its own `.cargo/config.toml` and `rust-toolchain.toml`.
- If the pinned companion toolchain is missing, install the exact channel from `tools/isolarail-companion/rust-toolchain.toml`.
- The repo-root `just` command invokes the Rust `isolarail-devd` binary directly from `tools/isolarail-companion/`.
- If you need to keep Local USB work pinned to one serial device, run the companion commands as `USB_PORT=/dev/cu.usbmodem... just ...`.
- Override the IPC endpoint only when necessary with `ISOLARAIL_DEVD_ENDPOINT=<path-or-pipe>`.

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
- `just web-dev` defaults `ISOLARAIL_DEVD_ORIGINS` to `http://isolarail-devd.local:51200,http://127.0.0.1:51200`.
- Put the mDNS URL first and an IP or localhost URL second only when overriding the default.
- The Web app never scans localhost ports; every fallback origin must be explicitly configured.
- Loopback-only static preview remains localStorage/mock-first when no companion responds. The app still attempts its same-origin bootstrap path, so `isolarail-devd web --web-root` hosting works on `127.0.0.1` or `localhost` without an extra origin setting.
- Disable the proxy only for deliberate diagnostics with `ISOLARAIL_DEV_PROXY=0`.
- `just web-storybook` always disables the dev proxy and stays mock-only; it must not require a running `isolarail-devd web`.

If `Local USB` still reports the service as unavailable:

- Confirm the explicit `isolarail-devd web` process is running and one configured origin returns `/api/v1/bootstrap` JSON.
- Confirm the Vite server was started after the proxy config change.
- Do not treat `web` as the default daemon mode. It is an explicit browser companion layered alongside the native IPC path.

## Review tips

- Storybook includes viewport presets for quick layout checks:
  - `IsolaRail Mobile (390×844)`
  - `IsolaRail Wide (1440×900)`

## Quality gates

- `just web-check`
- `just web-build`
- `just web-test-companion-bridge`
- `just web-test-e2e`
- `just web-test-unit`
- `just web-test-storybook`
