---
title: CH335F EEPROM bus isolation
module: hardware
problem_type: usb-hub-eeprom-programming
component: CH335F, M24C64, ESP32-S3
tags:
  - ch335f
  - eeprom
  - i2c
  - hardware-revision
  - bus-isolation
status: unresolved
related_specs:
  - docs/specs/m7gtw-ch335f-eeprom-initializer/SPEC.md
root_cause: CH335F and ESP32-S3 share the same EEPROM I2C nets through 0 ohm links, so reset-only sequencing is not a robust programming topology.
resolution_type: next-hardware-revision
---

## Context

The Rev2.3 board connects the M24C64 EEPROM to CH335F `LED3/SCL` and
`LED4/SDA`, and also connects ESP32-S3 `GPIO37/GPIO36` to the same nets through
0 ohm links. The intended manufacturing flow was to let ESP32-S3 program the
EEPROM, release the I2C bus to high impedance, and then reset or start CH335F so
it reads the customized descriptor image.

## Symptoms

- Holding or pulsing `HUB_RESET#` changes the USB topology enough to disconnect
  the ESP32-S3 USB monitor because the ESP is downstream of the CH335F hub.
- EEPROM access behavior depends on the CH335F state and host topology, so logs
  can be lost exactly during the reset window that should prove the operation.
- ESP32-S3 can observe `0x50` ACK and can read/write/verify the M24C64 in some
  states, but CH335F still enumerates as the default `USB HUB` after reset.
- Keeping CH335F reset asserted is the safer software path, but it still does
  not make the shared 0 ohm topology a reliable production programming path.
- A reset-only handoff therefore cannot prove that CH335F consumed the image,
  even when the EEPROM readback matches the expected bytes.

## Root Cause

The EEPROM bus has two active-capable masters or bus participants connected at
the same time: CH335F and ESP32-S3. Pulling CH335F reset is not a sufficient
electrical isolation strategy for this board because CH335F sideband pins, LED
loads, USB topology resets, and the ESP serial monitor are coupled into the same
debug path.

The reliable design requirement is not only "CH335F is reset"; it is "only one
side is electrically connected to the EEPROM at a time." The current 0 ohm
parallel topology does not provide that guarantee.

## Resolution

For the next board revision, route EEPROM `SCL/SDA` through an analog switch so
the EEPROM connection direction is explicit:

- ESP32-S3 programming mode: EEPROM connects to ESP32-S3 only; CH335F side is
  disconnected.
- Runtime mode: EEPROM connects to CH335F only; ESP32-S3 side is disconnected or
  high impedance.
- Use CH442E as the planned switch device for this direction select.
- Define the switch control pin default so cold boot favors CH335F runtime mode
  unless the ESP intentionally enters EEPROM programming mode.

## Guardrails / Reuse Notes

- Do not rely on reset-only sequencing when an external EEPROM is shared between
  a USB hub controller and an MCU.
- Do not treat a successful ESP readback as proof that CH335F has consumed the
  EEPROM image; verify the host USB descriptor after a real CH335F cold start.
- Keep EEPROM write-protect (`WC`) controllable or physically strapped to write
  enabled during factory initialization, then document the production state.
- If the ESP debug interface is downstream of the same hub being reset, expect
  serial logs to break during hub reset. Add persistent status reporting or an
  out-of-band debug path before depending on reset-window logs.
- Prefer a hardware mux or switch over removable 0 ohm links for any future
  factory-programmable hub EEPROM path.

## References

- `docs/specs/m7gtw-ch335f-eeprom-initializer/SPEC.md`
- `docs/specs/m7gtw-ch335f-eeprom-initializer/IMPLEMENTATION.md`
- `docs/hardware/mainboard_netlist.enet.enet`
