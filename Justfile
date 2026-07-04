set shell := ["zsh", "-cu"]

companion_devd_bin := justfile_directory() + "/tools/isohub-companion/scripts/devd-bin.sh"
TARGET := "xtensa-esp32s3-none-elf"
BIN := "iso-usb-hub"
FIRMWARE_ELF := justfile_directory() + "/target/" + TARGET + "/release/" + BIN
FIRMWARE_BIN := justfile_directory() + "/target/" + TARGET + "/release/" + BIN + ".app.bin"
FIRMWARE_CATALOG := justfile_directory() + "/target/" + TARGET + "/release/firmware-catalog.json"
FIRMWARE_ARTIFACT := "local-app"

default:
  @just --list

tools-build:
  cd tools/isohub-companion && ISOHUB_USB_PORT="${USB_PORT:-}" cargo build --bins

tools-test:
  cd tools/isohub-companion && ISOHUB_USB_PORT="${USB_PORT:-}" cargo test

firmware-check:
  cargo +esp check

firmware-contract-test:
  native_target="$(rustc -vV | awk '/^host:/ { print $2 }')"; \
  cargo +stable test --lib --target "$native_target"

firmware-build:
  cargo +esp build --release

host-tools-build:
  just tools-build

host-tools-test:
  just tools-test

devd-serve:
  cd tools/isohub-companion && \
  if [[ -n "${ENDPOINT:-}" ]]; then \
    ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub-devd -- serve --endpoint "${ENDPOINT}"; \
  else \
    ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub-devd -- serve; \
  fi

devd-web:
  cd tools/isohub-companion && \
  extra_args=(); \
  if [[ "${ALLOW_DEV_CORS:-}" == "1" ]]; then extra_args+=(--allow-dev-cors); fi; \
  ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub-devd -- web --bind "${BIND:-127.0.0.1:51200}" --mdns-name "${MDNS_NAME:-isohub-devd}" "${extra_args[@]}"

devd-help:
  cd tools/isohub-companion && ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub-devd -- --help

isohub +ARGS='--help':
  cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- {{ARGS}}

ports:
  USB_PORT="${USB_PORT:-}" just isohub --json discover --scan | python3 -c 'import json,sys; data=json.load(sys.stdin); rows=["{}\t{}\tdevice={}".format((d.get("transport") or {}).get("portPath") or "", d.get("displayName") or d.get("id") or "iso-usb-hub", d.get("id") or "") for d in data.get("devices", []) if (d.get("transport") or {}).get("kind") == "usb" and (d.get("transport") or {}).get("portPath")]; print("\n".join(rows) if rows else "No ESP32-S3 USB Serial/JTAG candidates found.")'

identify:
  if [[ -z "${PORT:-}" ]]; then \
    echo "error: PORT is required." >&2; \
    echo "List candidates:" >&2; \
    echo "  just ports" >&2; \
    echo "Then confirm explicitly:" >&2; \
    echo "  PORT=/dev/cu.xxx just identify" >&2; \
    exit 2; \
  fi; \
  device_id="usb-$(printf '%s' "$PORT" | sed 's/[^A-Za-z0-9]/-/g')"; \
  tmp="$(mktemp)"; \
  trap 'rm -f "$tmp"' EXIT HUP INT TERM; \
  USB_PORT="$PORT" just isohub --json discover --scan > "$tmp"; \
  identity="$(python3 -c 'import json,sys; device_id=sys.argv[1]; data=json.load(open(sys.argv[2], encoding="utf-8")); matches=[d for d in data.get("devices", []) if d.get("id") == device_id]; (print(json.dumps(matches[0])) if matches else sys.exit(f"device {device_id} not found in discovery output"))' "$device_id" "$tmp")"; \
  project_device_id="$(python3 -c 'import json,sys; v=json.loads(sys.argv[1]); print(v.get("deviceId") or v.get("device_id") or "")' "$identity")"; \
  mac="$(python3 -c 'import json,sys; v=json.loads(sys.argv[1]); print(v.get("mac") or "")' "$identity")"; \
  firmware="$(python3 -c 'import json,sys; v=json.loads(sys.argv[1]); f=v.get("firmware") or {}; print(f.get("name") or "")' "$identity")"; \
  if [[ "$firmware" != "iso-usb-hub" ]]; then \
    echo "error: selected port is not running iso-usb-hub firmware." >&2; \
    exit 2; \
  fi; \
  { \
    print -r -- "$PORT"; \
    print -r -- "port=$PORT"; \
    print -r -- "device=$device_id"; \
    if [[ -n "$project_device_id" ]]; then print -r -- "device_id=$project_device_id"; fi; \
    if [[ -n "$mac" ]]; then print -r -- "mac=$mac"; fi; \
  } > .esp32-port; \
  echo "port: $PORT"; \
  echo "device: $device_id"; \
  if [[ -n "$project_device_id" ]]; then echo "device_id: $project_device_id"; fi; \
  if [[ -n "$mac" ]]; then echo "mac: $mac"; fi; \
  echo "cached: .esp32-port"

select-port:
  tmp="$(mktemp)"; \
  trap 'rm -f "$tmp"' EXIT HUP INT TERM; \
  just ports | awk -F '\t' '$1 ~ /^\/dev\// || $1 ~ /^[Cc][Oo][Mm][0-9]+$/ { print $1 }' > "$tmp"; \
  if [[ ! -s "$tmp" ]]; then echo "error: no ESP32-S3 USB Serial/JTAG candidates found." >&2; exit 2; fi; \
  nl -w1 -s'  ' "$tmp"; \
  printf "Select target by number or full port path: "; read choice; \
  case "$choice" in /dev/*) port="$choice" ;; *[!0-9]*|"") echo "error: invalid selection '$choice'." >&2; exit 2 ;; *) port="$(sed -n "${choice}p" "$tmp")" ;; esac; \
  if [[ -z "$port" ]]; then echo "error: no target port selected." >&2; exit 2; fi; \
  printf "Confirm target port %s? Type 'yes' to continue: " "$port"; read confirm; \
  if [[ "$confirm" != "yes" ]]; then echo "aborted"; exit 2; fi; \
  PORT="$port" just identify

firmware-bin:
  cargo +esp build --release
  espflash save-image --chip esp32s3 {{FIRMWARE_ELF}} {{FIRMWARE_BIN}}
  python3 tools/firmware-catalog/build-catalog.py \
    --out {{FIRMWARE_CATALOG}} \
    --artifact-id {{FIRMWARE_ARTIFACT}} \
    --version "$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')" \
    --git-sha "$(git rev-parse HEAD)" \
    --build-id "local" \
    --app-bin {{FIRMWARE_BIN}} \
    --elf {{FIRMWARE_ELF}}

_selected-device:
  @if [[ ! -f .esp32-port ]]; then \
    echo "error: no port selected for this repo (.esp32-port missing)." >&2; \
    echo "Run:" >&2; \
    echo "  just select-port" >&2; \
    exit 2; \
  fi; \
  port="$(sed -n 's/^port=//p' .esp32-port | head -1)"; \
  if [[ -z "$port" ]]; then port="$(head -n 1 .esp32-port | tr -d '\r' | xargs)"; fi; \
  if [[ -z "$port" ]] || [[ ! -e "$port" ]]; then \
    echo "error: cached port '$port' is not available." >&2; \
    echo "Run:" >&2; \
    echo "  just select-port" >&2; \
    exit 2; \
  fi; \
  device="$(sed -n 's/^device=//p' .esp32-port | head -1)"; \
  if [[ -z "$device" ]]; then device="usb-$(printf '%s' "$port" | sed 's/[^A-Za-z0-9]/-/g')"; fi; \
  print -r -- "$device"

_expected-flash-args:
  @args=(); \
  device_id="$(sed -n 's/^device_id=//p' .esp32-port | head -1)"; \
  mac="$(sed -n 's/^mac=//p' .esp32-port | head -1)"; \
  if [[ -n "$device_id" ]]; then args+=(--expected-device-id "$device_id"); fi; \
  if [[ -n "$mac" ]]; then args+=(--expected-mac "$mac"); fi; \
  if [[ "${#args[@]}" -eq 0 ]]; then \
    echo "error: no confirmed device identity in .esp32-port." >&2; \
    echo "Run:" >&2; \
    echo "  PORT=/dev/cu.xxx just identify" >&2; \
    echo "For first-time/download-mode flashing, run:" >&2; \
    echo "  PORT=/dev/cu.xxx just flash-first-time" >&2; \
    exit 2; \
  fi; \
  printf '%q ' "${args[@]}"

flash:
  @device="$(just _selected-device)" || exit $?; \
  expected="$(just _expected-flash-args)" || exit $?; \
  expected_args=("${(@z)expected}"); \
  port="$(sed -n 's/^port=//p' .esp32-port | head -1)"; \
  just firmware-bin; \
  USB_PORT="$port" just isohub flash --device "$device" --catalog {{FIRMWARE_CATALOG}} --artifact {{FIRMWARE_ARTIFACT}} --real "${expected_args[@]}"

flash-first-time:
  @if [[ -z "${PORT:-}" ]]; then \
    echo "error: PORT is required for first-time/download-mode flashing." >&2; \
    exit 2; \
  fi; \
  just firmware-bin
  @set -o pipefail; \
  device="usb-$(printf '%s' "$PORT" | sed 's/[^A-Za-z0-9]/-/g')"; \
  tmp="$(mktemp)"; \
  trap 'rm -f "$tmp"' EXIT HUP INT TERM; \
  USB_PORT="$PORT" just isohub discover --scan >/dev/null || true; \
  USB_PORT="$PORT" just isohub --json flash --device "$device" --catalog {{FIRMWARE_CATALOG}} --artifact {{FIRMWARE_ARTIFACT}} --real --first-time | tee "$tmp"; \
  project_device_id="$(python3 -c 'import json,sys; data=json.load(open(sys.argv[1], encoding="utf-8")); identity=data.get("identity") or (data.get("result") or {}).get("identity") or {}; print(identity.get("deviceId") or identity.get("device_id") or "")' "$tmp")"; \
  mac="$(python3 -c 'import json,sys; data=json.load(open(sys.argv[1], encoding="utf-8")); identity=data.get("identity") or (data.get("result") or {}).get("identity") or {}; print(identity.get("mac") or "")' "$tmp")"; \
  if [[ -z "$project_device_id" && -z "$mac" ]]; then \
    echo "error: first-time flash completed but no device identity was captured." >&2; \
    echo "Run after reboot:" >&2; \
    echo "  PORT=$PORT just identify" >&2; \
    exit 2; \
  fi; \
  { \
    print -r -- "$PORT"; \
    print -r -- "port=$PORT"; \
    print -r -- "device=$device"; \
    if [[ -n "$project_device_id" ]]; then print -r -- "device_id=$project_device_id"; fi; \
    if [[ -n "$mac" ]]; then print -r -- "mac=$mac"; fi; \
  } > .esp32-port; \
  echo "cached: .esp32-port"

reset:
  @device="$(just _selected-device)" || exit $?; \
  port="$(sed -n 's/^port=//p' .esp32-port | head -1)"; \
  USB_PORT="$port" just isohub reset --device "$device"

monitor:
  @device="$(just _selected-device)" || exit $?; \
  port="$(sed -n 's/^port=//p' .esp32-port | head -1)"; \
  USB_PORT="$port" just isohub monitor --device "$device" --tail "${TAIL:-200}"

flash-monitor:
  @just flash
  @just reset
  @just monitor

discover:
  if [[ "${SCAN:-1}" == '1' ]]; then \
    cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- discover --scan; \
  else \
    cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- discover; \
  fi

devices:
  cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- devices

hardware-available:
  if [[ "${SCAN:-1}" == '1' ]]; then \
    cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- hardware available --scan; \
  else \
    cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- hardware available; \
  fi

status:
  if [[ -z "${SELECTOR:-}" ]]; then \
    echo "Set SELECTOR='--device <device-id>' or '--hardware <saved-id>'." >&2; \
    exit 1; \
  fi; \
  selector=(${=SELECTOR}); \
  cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- status "${selector[@]}"

device-ports:
  if [[ -z "${SELECTOR:-}" ]]; then \
    echo "Set SELECTOR='--device <device-id>' or '--hardware <saved-id>'." >&2; \
    exit 1; \
  fi; \
  selector=(${=SELECTOR}); \
  cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- ports "${selector[@]}"

port-power:
  if [[ -z "${SELECTOR:-}" || -z "${PORT:-}" || -z "${ENABLED:-}" ]]; then \
    echo "Set SELECTOR='--device <device-id>' or '--hardware <saved-id>', plus PORT=port1..port4 and ENABLED=true|false." >&2; \
    exit 1; \
  fi; \
  selector=(${=SELECTOR}); \
  cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- ports "${selector[@]}" power --port "${PORT}" --enabled "${ENABLED}"

port-replug:
  if [[ -z "${SELECTOR:-}" || -z "${PORT:-}" ]]; then \
    echo "Set SELECTOR='--device <device-id>' or '--hardware <saved-id>', plus PORT=port1..port4." >&2; \
    exit 1; \
  fi; \
  selector=(${=SELECTOR}); \
  cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- ports "${selector[@]}" replug --port "${PORT}"

wifi-show:
  if [[ -z "${SELECTOR:-}" ]]; then \
    echo "Set SELECTOR='--device <device-id>' or '--hardware <saved-id>'." >&2; \
    exit 1; \
  fi; \
  selector=(${=SELECTOR}); \
  cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- wifi show "${selector[@]}"

wifi-set:
  if [[ -z "${SELECTOR:-}" || -z "${SSID:-}" || -z "${PSK:-}" ]]; then \
    echo "Set SELECTOR='--device <device-id>' or '--hardware <saved-usb-id>', plus SSID and PSK." >&2; \
    exit 1; \
  fi; \
  selector=(${=SELECTOR}); \
  cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- wifi set "${selector[@]}" --ssid "${SSID}" --psk "${PSK}"

wifi-clear:
  if [[ -z "${SELECTOR:-}" ]]; then \
    echo "Set SELECTOR='--device <device-id>' or '--hardware <saved-usb-id>'." >&2; \
    exit 1; \
  fi; \
  selector=(${=SELECTOR}); \
  cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- wifi clear "${selector[@]}"

device-reset:
  if [[ -z "${SELECTOR:-}" ]]; then \
    echo "Set SELECTOR='--device <device-id>' or '--hardware <saved-id>'." >&2; \
    exit 1; \
  fi; \
  selector=(${=SELECTOR}); \
  cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- reset "${selector[@]}"

device-monitor:
  if [[ -z "${SELECTOR:-}" ]]; then \
    echo "Set SELECTOR='--device <device-id>' or '--hardware <saved-id>'." >&2; \
    exit 1; \
  fi; \
  selector=(${=SELECTOR}); \
  cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- monitor "${selector[@]}" --tail "${TAIL:-200}"

diagnostics-export:
  if [[ -z "${SELECTOR:-}" ]]; then \
    echo "Set SELECTOR='--device <device-id>' or '--hardware <saved-id>'." >&2; \
    exit 1; \
  fi; \
  selector=(${=SELECTOR}); \
  cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- diagnostics export "${selector[@]}"

web-install:
  bun install --cwd web

web-dev:
  ISOHUB_DEVD_ORIGINS="${DEVD_ORIGINS:-http://isohub-devd.local:51200,http://127.0.0.1:51200}" bun run --cwd web dev

web-storybook:
  bun run --cwd web storybook

web-build:
  bun run --cwd web build

web-check:
  bun run --cwd web check

web-test-unit:
  bun run --cwd web test:unit

web-lint:
  bun run --cwd web lint

web-format:
  bun run --cwd web format

web-test-companion-bridge:
  bun test --cwd web ./src/domain/companionBridge.test.ts

web-test-e2e:
  bun run --cwd web test:e2e

web-test-storybook:
  bun run --cwd web build-storybook
  bun run --cwd web test:storybook
