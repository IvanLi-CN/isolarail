# Implementation

## Current coverage

- Firmware already exposes USB Serial/JTAG JSONL `info` with `device_id`, `mac`, firmware name/version, and uptime through the runtime `usb_jsonl` path.
- `tools/isohub-companion` provides the source-local `isohub` and `isohub-devd` host tools over native IPC.
- The existing companion flash path validates firmware catalogs, checks project identity before normal app flashing, writes app images at `0x10000`, supports first-time full flashing with typed confirmation, and exposes reset/monitor flows.
- `Justfile`, `tools/isohub-runner`, `.cargo/config.toml`, Makefile compatibility helpers, and user-facing docs now route default flashing through `isohub` / `isohub-devd`.
- `mcu-agentd` is retained only as a legacy/emergency passthrough.

## Remaining gaps

- Hardware validation depends on an attached ESP32-S3 target.
- Release installers and user-machine host-tool packaging are intentionally out of scope for this v1 source workflow.
