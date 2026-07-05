---
title: Boot and Runtime
description: Firmware boot self-check, gating decisions, runtime sampling, front-panel input, and fan path.
---

<!-- markdownlint-disable MD025 -->

# Boot and Runtime

Firmware behavior is owned by `docs/software_design.md` and the matching specs. This page compresses the boot, gating, and runtime paths into one readable map.

## Four boot phases

Boot is fixed as:

```text
Early Bring-up -> Self-Check -> Gate Apply -> Runtime
```

`Self-Check` does three things:

- Emits `boot.stage:*`, `boot.check:*`, and `boot.summary:*` logs.
- Drives the 160x50 LCD boot self-check page.
- Produces `GateDecision`, which releases or blocks runtime tasks, front-panel input, and ports.

State model:

- `SelfCheckItemState = Pending / Ok / Warn / Err / Fatal / Skipped`
- `BootOutcome = Ok / Degraded / Fatal`
- `BootFaultCode` covers mux, input power, INA, front panel, fan, and per-port faults.

The LCD page, serial logs, and gating decision all read from one `BootSelfCheckSnapshot`. That prevents the display, logs, and runtime from disagreeing about the same boot.

## Fixed self-check order

The order is stable to avoid bus contention and inconsistent logs:

1. Initialize logging, time source, LCD, two I²C buses, and basic GPIO.
2. Confirm I²C topology. The current validation board records `mux=Skipped`.
3. Start and wait for input power qualification.
4. Once VIN is ready, probe `Front-panel TCA6408A@0x21` and the fan path.
5. Once VIN is ready, initialize `Mainboard TCA6408A@0x20`, CH335F sideband, and four-port scanning.
6. Summarize `GateDecision` and emit `boot.summary:*`.

Typical log shape:

```text
app.start
init.time: embassy-timer=ok
boot.stage: stage=self-check
boot.check: name=mux state=skip fault=-
boot.check: name=vin state=ok fault=-
boot.check: name=front_panel state=warn fault=FrontPanelOffline
boot.summary: outcome=DEG first_fault=FrontPanelOffline runtime=on front_panel=off
```

If the serial monitor attaches late, `boot.summary:*` should still explain the boot result.

## Degradation policy

The default strategy is read-only probing and graded degradation:

- Unsafe input power: `Fatal`; `IN_CE` stays closed and runtime is not released.
- Front panel offline: current V3 records `Warn/FrontPanelOffline`, disables front-panel input, and continues dashboard/runtime.
- Fan unavailable: `Warn/FanUnavailable`; dashboard is not blocked.
- One port missing `INA226/TMP112`: that port is `Err`; diagnostics and telemetry are affected, but other ports are not shut off.
- `Mainboard TCA6408A@0x20` offline: degraded manual mode; sideband fault must remain visible.

| Condition | Result | Ports | Dashboard |
| --- | --- | --- | --- |
| unsafe VIN / PG | `Fatal` | all skipped / off | stays on fatal self-check page |
| missing `PCA9545A` | `Skipped` | unaffected | continues |
| front panel `0x21` offline | `Degraded` | unaffected | continues, keys disabled |
| fan unavailable | `Degraded` | unaffected | continues |
| one telemetry pair missing | `Degraded` | not shut off alone | that port is diagnostic `Err` |
| mainboard sideband offline | `Degraded` | manual mode | sideband fault remains visible |

## Runtime port gating

Each port output is controlled by:

- `VIN ready`
- healthy mainboard sideband, or explicit degraded manual release
- `V1OK` mode: standalone or upstream-managed
- matching `PWREN#` state
- software over-current latch state
- front-panel manual disconnect state

Software over-current is detected when:

- `vbus < 3.0 V && current > 0.1 A`
- or `current > 5.3 A`

On fault, firmware immediately drops the matching `ENx` and injects `OVCUR#`. The latch is released only after 4 consecutive 500 ms powered safe samples.

Each runtime port state should answer:

- Is VIN ready?
- Is `V1OK` in standalone or upstream-managed mode?
- Does `PWREN#` allow this port?
- Did the owner manually close this port?
- Is an OCP latch active?
- Is telemetry fresh, initializing, missing, or failed?

Post-shutdown `0V/0mA` samples do not clear a latch. Recovery must be based on powered safe samples.

## Front-panel interaction

The front-panel task publishes debounced key events. It does not directly own `EN1..EN4`:

- Left / Right: move selected dashboard column across the four ports.
- Center short press: toggle manual output allowance for the selected port.
- Up / Down: reserved for future display modes or detail views.

Selection changes and manual state changes must trigger an immediate dashboard redraw, not only wait for telemetry refresh.

Manual disconnect state is not persistent. On each boot all four ports default to manually allowed, while actual output is still constrained by VIN, sideband, OCP, and `PWREN#`.

## Buzzer and alarms

V3 drives the `BUZZER` net from `GPIO7` using LEDC PWM. GPIO7 must return low when idle and after every effect.

Rules:

- Play the boot tone only after a non-`Fatal` self-check reaches `Runtime`.
- Left / Right selection changes play the operation cue.
- Center success enabling a port plays the power-on tone; disabling plays the power-off tone.
- `PortPowerSet(enabled=true)` from the control plane plays the power-on tone.
- `PortPowerSet(enabled=false)` and `PortReplug` play the power-off tone.

Use the [Buzzer Audio Preview](buzzer-audio-preview) page to audition the timing
table before firmware constant changes.

Alarm priority:

1. `channel_short`
2. `over_temp`
3. `input_over_power`
4. `channel_over_5a`
5. one-shot effects

When a protection shutdown happens in the same tick, normal unplug or power-off cues are suppressed and only the alarm remains.

## Log style

Logs use single-line key/value records for serial and diagnostics parsing:

- Boot: `boot.stage:*`, `boot.check:*`, `boot.summary:*`
- Input power: `pwr.in:*`
- Front panel: `i2c.front:*`, `i2c.front_diag:*`
- Sideband: `hub.sideband:*`

New modules should keep the same single-line key/value style. `device-monitor` and diagnostics export depend on that shape.

## Verification path

| Scenario | Expected result |
| --- | --- |
| normal boot | LCD shows self-check, then dashboard |
| direct I²C board | `mux=Skipped`, port scan continues |
| VIN failure | `Fatal`, `IN_CE` closed, runtime blocked |
| front panel offline | `Warn/FrontPanelOffline`, dashboard continues |
| one telemetry pair missing | that port is `Err`, other ports continue |
| OCP fault | `ENx` drops, `OVCUR#` is injected, dashboard shows over-current |

## References

- `docs/software_design.md`
- `docs/specs/5f74j-firmware-boot-self-check/SPEC.md`
- `docs/specs/h8c4s-ch335f-sideband-power-control/SPEC.md`
- `docs/specs/7gf6b-firmware-buzzer-audio/SPEC.md`
