set shell := ["zsh", "-cu"]

companion_devd_bin := justfile_directory() + "/tools/isohub-companion/scripts/devd-bin.sh"

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
  make build

firmware-run:
  PORT="${PORT:-}" BAUD="${BAUD:-115200}" make run

firmware-attach:
  PORT="${PORT:-}" BAUD="${BAUD:-115200}" make attach

firmware-ports:
  make ports

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

ports:
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

reset:
  if [[ -z "${SELECTOR:-}" ]]; then \
    echo "Set SELECTOR='--device <device-id>' or '--hardware <saved-id>'." >&2; \
    exit 1; \
  fi; \
  selector=(${=SELECTOR}); \
  cd tools/isohub-companion && ISOHUB_DEVD_BIN='{{companion_devd_bin}}' ISOHUB_USB_PORT="${USB_PORT:-}" cargo run --bin isohub -- reset "${selector[@]}"

monitor:
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
  ISOHUB_DEVD_ORIGINS="${DEVD_ORIGINS:-}" bun run --cwd web dev

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
