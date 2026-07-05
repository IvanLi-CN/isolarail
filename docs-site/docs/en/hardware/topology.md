---
title: Hardware Topology
description: Current V3 power, USB hub, I²C, port telemetry, and front-panel connections.
---

<!-- markdownlint-disable MD025 -->

# Hardware Topology

This page summarizes the current V3 baseline. The full board-level source of truth remains `docs/hardware_connection_overview.md`; the site is a published reading surface.

## Current baseline

- Controller: `ESP32-S3`
- USB hub controller: `CH335F`
- Persistent configuration: `M24C64@0x50`
- Input power protection: `TPS2490`
- Input telemetry: `Input INA226@0x44`
- Mainboard sideband expander: `Mainboard TCA6408A@0x20`
- Front-panel / LCD expander: `Front-panel TCA6408A@0x21`
- Owner-facing ports: `port1`, `port2`, `port3`, `port4`

`PCA9545A@0x70` is retained only as a compatibility naming slot. The current validation board uses direct shared I²C buses and does not rely on that mux for port addressing.

## System topology

```text
DC IN
  └─ VIN_UNSAFE -> shunt -> TPS2490 / input gate -> VIN
                      └─ Input INA226@0x44

ESP32-S3
  ├─ Sensor I2C (GPIO8/GPIO9)
  │   ├─ Front-panel TCA6408A@0x21
  │   ├─ Input INA226@0x44
  │   ├─ Port 3 INA226@0x42 + TMP112@0x4A
  │   └─ Port 4 INA226@0x43 + TMP112@0x4B
  ├─ Hub I2C (GPIO14/GPIO13)
  │   ├─ Mainboard TCA6408A@0x20
  │   ├─ M24C64@0x50
  │   ├─ Port 1 INA226@0x40 + TMP112@0x48
  │   └─ Port 2 INA226@0x41 + TMP112@0x49
  ├─ EN1..EN4 -> Port 1..4 power gate
  ├─ ISOUSB211 V1OK <- upstream isolation status
  ├─ USB D+ / D- <-> native USB
  └─ LCD + front panel + buzzer + fan

CH335F
  ├─ PWREN1#..4# -> Mainboard TCA6408A@0x20 -> MCU
  └─ OVCUR1#..4# <- Mainboard TCA6408A@0x20 <- MCU
```

The key boundary is that ESP32-S3 owns power gating, telemetry, persisted configuration, and the control plane; CH335F owns USB hub data behavior and the `PWREN#` / `OVCUR#` sideband.

## Input power

Input power follows this path:

```text
VIN_UNSAFE -> 5 mΩ shunt -> TPS2490 / input gate -> VIN
                                  └─ Input INA226@0x44
```

Key nets:

- `IN_EN`: MCU-controlled input-gate qualification.
- `IN_PG`: high means power-good.
- `VIN_ADC`: MCU ADC divider sample point.

Firmware checks input qualification during boot. If input power is unsafe, `IN_CE` stays closed, port initialization is skipped, and runtime tasks are not released.

Input bring-up reads in this order:

1. `VIN_UNSAFE` exists.
2. `IN_EN` allows the TPS2490 input gate.
3. `IN_PG` reports power-good.
4. `Input INA226@0x44` reports VIN / current / power.
5. `boot.summary` classifies VIN as `Ok`, `Warn`, or `Fatal`.

Only after VIN is ready does firmware probe the front panel, fan, mainboard sideband, and four output modules.

## Four-port gating

The four outputs are controlled by MCU-driven `EN1..EN4`:

| Port | Enable | GPIO | Telemetry |
| --- | --- | --- | --- |
| `port1` | `EN1` | `GPIO17` | `INA226@0x40` + `TMP112@0x48` |
| `port2` | `EN2` | `GPIO18` | `INA226@0x41` + `TMP112@0x49` |
| `port3` | `EN3` | `GPIO39` | `INA226@0x42` + `TMP112@0x4A` |
| `port4` | `EN4` | `GPIO40` | `INA226@0x43` + `TMP112@0x4B` |

`port.power_set` drives the matching `ENx`. `port.replug` means controlled power-off and power-on; it does not promise true per-port data disconnect.

## CH335F sideband

`CH335F` sideband signals are connected through `Mainboard TCA6408A@0x20`:

- `P0/P2/P4/P6` read low-active `PWREN1#..4#`.
- `P1/P3/P5/P7` inject low-active `OVCUR1#..4#`.
- `ISOUSB211 V1OK` is read on `GPIO21` and separates standalone/no-upstream from upstream-managed mode.

When `V1OK=low`, the product keeps independent output capability and does not shut ports off just because `PWREN#` is high. When `V1OK=high`, only ports whose `PWREN#` is low are allowed to output power.

| Signal | TCA6408A bit | Direction | Meaning |
| --- | --- | --- | --- |
| `PWREN1#` | P0 | input | Low means CH335F allows `port1` |
| `OVCUR1#` | P1 | inject | Output low reports over-current to CH335F |
| `PWREN2#` | P2 | input | Low means CH335F allows `port2` |
| `OVCUR2#` | P3 | inject | Output low reports over-current to CH335F |
| `PWREN3#` | P4 | input | Low means CH335F allows `port3` |
| `OVCUR3#` | P5 | inject | Output low reports over-current to CH335F |
| `PWREN4#` | P6 | input | Low means CH335F allows `port4` |
| `OVCUR4#` | P7 | inject | Output low reports over-current to CH335F |

Initialization releases all `OVCUR#` lines by writing output `0xFF`, polarity `0x00`, and direction `0xFF`.

## Two I²C buses

### Sensor / front-panel I²C

- `I2C_SDA = GPIO8`
- `I2C_SCL = GPIO9`
- Devices: `Input INA226@0x44`, `Front-panel TCA6408A@0x21`, and `port3/port4` telemetry devices.

### Hub-sideband / output I²C

- `HUB_SDA = GPIO14`
- `HUB_SCL = GPIO13`
- Devices: `Mainboard TCA6408A@0x20`, `M24C64@0x50`, and `port1/port2` telemetry devices.

## Front panel and display

The front-panel expander is `Front-panel TCA6408A@0x21`:

- `P0..P4`: five-way switch, mapped to Center, Right, Down, Left, Up.
- `P5 = LCD_RES`
- `P6 = LCD_CS`

LCD signals directly connected to the MCU:

- `LCD_DC = GPIO10`
- `LCD_MOSI = GPIO11`
- `LCD_SCLK = GPIO12`
- `LCD_BLK = GPIO15`

The panel is a `160x50 LCD`, the driver IC is `GC9D01`, and `LCD_BLK` is currently low-active.

Two V3 display details are easy to get wrong:

- `LCD_BLK` is low-active; driving it as high-active turns the backlight off.
- Display orientation uses `Orientation::LandscapeSwapped`, not the old `Landscape` mapping.

Current V3 hardware cannot hard-reset `Front-panel TCA6408A@0x21` from the MCU. If bus-clear succeeds but only `0x21` does not ACK, firmware records `Warn/FrontPanelOffline`, continues runtime, and disables only front-panel input.

## Maintenance actions and hardware capability

| Control-plane action | Hardware action | Does not promise |
| --- | --- | --- |
| `port.power_set` | Drive matching `ENx` high or low | USB data topology changes |
| `port.replug` | Controlled power-off then power-on on `ENx` | true per-port data disconnect |
| `hub.reset` | hub-level maintenance reset | replacement for single-port replug |
| `wifi.set` / `wifi.clear` | Write `M24C64@0x50` | LAN-only credential writes |

## Historical boundary

The following terms are historical or migration-only and do not represent current control-plane hardware:

- `SC8815 + SW2303`
- `PSTOP_CTL1..4`
- `PSTOP1..4`
- historical `USB-C route` / two-port product abstractions

## References

- `docs/hardware_connection_overview.md`
- `docs/ch335f_tca6408a_appnote.md`
- `docs/specs/pw97u-control-plane-alignment/SPEC.md`
- `docs/specs/j6nvw-hardware-v3-pin-assignment/SPEC.md`
