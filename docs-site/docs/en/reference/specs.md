---
title: Specs Index
description: Canonical specs, project docs, and implementation state related to the docs site.
---

<!-- markdownlint-disable MD025 -->

# Specs Index

The docs site curates and publishes. It does not replace canonical repository specs. When you need the current source of truth, go back to `docs/specs/**`, `docs/software_design.md`, and `docs/hardware_connection_overview.md`.

## Docs site spec

| ID | Title | Status | Role |
| --- | --- | --- | --- |
| `r6wfr` | Docs Web Site | First implementation in progress | Defines the Rspress/Bun bilingual site, publishing workflow, visual evidence, and acceptance gates |

Spec directory:

- `docs/specs/r6wfr-docs-site/SPEC.md`
- `docs/specs/r6wfr-docs-site/IMPLEMENTATION.md`
- `docs/specs/r6wfr-docs-site/HISTORY.md`

This spec only owns the site itself: Rspress structure, bilingual routes, GitHub Pages workflow, visual evidence, and documentation boundaries. It does not replace hardware, firmware, or control-plane specs.

## Product and control-plane specs

| ID | Title | Status | Site relevance |
| --- | --- | --- | --- |
| `pw97u` | Four-port USB Hub control-plane alignment | Completed / partially tracked | Firmware identity, CLI/daemon naming, USB JSONL, HTTP, Web app, port model |
| `q9d7h` | CLI/devd flash migration | Partially complete | `isohub`, `isohub-devd`, identity checks, flashing, and monitor path |
| `h8c4s` | CH335F sideband power control | Partially complete | `PWREN#`, `OVCUR#`, runtime gating, and OCP latch |
| `7gf6b` | Firmware buzzer audio | Completed | GPIO7 LEDC PWM, cue priority, alarm loops, and the in-site audio preview |

These specs explain why the control plane is named `isohub` / `isohub-devd` and why the owner-facing ports are fixed as `port1..port4`.

Recommended reading order:

1. Read `pw97u` for owner-facing names, port model, and interface boundaries.
2. Read `q9d7h` for why flash, reset, and monitor must go through `isohub` / `isohub-devd`.
3. Read `h8c4s` to connect `PWREN#`, `OVCUR#`, OCP latch, and dashboard state.
4. Read `7gf6b` when changing cue timing or reviewing the
   [Buzzer Audio Preview](../firmware/buzzer-audio-preview).

## Hardware and firmware specs

| ID | Title | Status | Site relevance |
| --- | --- | --- | --- |
| `5f74j` | Firmware robustness and boot self-check | Completed | boot self-check, degraded/fatal behavior, LCD boot page |
| `j6nvw` | Hardware V3 pin assignment and display path | Partially complete | GPIO, display path, backlight polarity, front-panel degradation boundary |

Recommended reading order:

1. Start with `docs/hardware_connection_overview.md` for the V3 board topology.
2. Read `j6nvw` for GPIO, display, backlight, and front-panel reset boundaries.
3. Read `5f74j` for why boot self-check degrades instead of panicking.
4. Return to `h8c4s` when debugging port power or sideband behavior.

## Current-truth docs

- `docs/hardware_connection_overview.md`: current V3 hardware overview.
- `docs/software_design.md`: firmware runtime behavior, boot self-check, and gating semantics.
- `docs/dashboard_spec.md`: 160x50 dashboard pixel layout and states.
- `README.md`: development entrypoints and current command set.
- `PRODUCT.md`: site product positioning.
- `DESIGN.md`: site visual and content direction.

## Source-of-truth priority

| Question | Primary source |
| --- | --- |
| Product and naming | `docs/specs/pw97u-control-plane-alignment/SPEC.md` |
| Current V3 hardware topology | `docs/hardware_connection_overview.md` |
| GPIO / display / front-panel reset | `docs/specs/j6nvw-hardware-v3-pin-assignment/SPEC.md` |
| Firmware boot and runtime gating | `docs/software_design.md` |
| Buzzer cue timing and priority | `docs/specs/7gf6b-firmware-buzzer-audio/SPEC.md` |
| Dashboard pixel layout | `docs/dashboard_spec.md` |
| Local development commands | `README.md` |
| Docs site publishing | `docs/specs/r6wfr-docs-site/SPEC.md` |

Site pages are the public guide layer. They may condense and reorganize source material, but they must not rewrite behavior beyond those sources.

## Site content boundary

Site pages may condense and rewrite the sources above, but must not change:

- Firmware identity: `iso-usb-hub`
- CLI: `isohub`
- Daemon: `isohub-devd`
- Port model: `port1..port4`
- Current V3 hardware baseline
- `port.replug` = controlled power-off and power-on
- `PCA9545A@0x70` is currently only a compatibility naming slot

If site content conflicts with specs or current-truth docs, specs/current-truth docs win and the site should be corrected.
