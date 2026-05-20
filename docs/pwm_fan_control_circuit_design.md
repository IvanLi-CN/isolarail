# ESP32-S3 PWM Fan Control Circuit

This note documents the mainboard fan circuit. The hardware uses a TI `TPS62933DRLR` buck regulator for `FAN_VCC`.

## Current Hardware

- Fan connector: `H1`
  - Pin 1: `FAN_VCC`
  - Pin 2: `GND`
  - Pin 3: `FAN_TOUCH`
- Regulator: `U8 TPS62933DRLR`
  - `VIN`: `VIN_UNSAFE`
  - `EN`: `FAN_EN`
  - `SW`: inductor `L2` to `FAN_VCC`
  - `FB`: feedback node `$1N78095`
- MCU signals:
  - `GPIO1`: `FAN_PWM`
  - `GPIO2`: `FAN_EN`
  - `GPIO6`: `FAN_TOUCH`

## Control Topology

```text
VIN_UNSAFE
  |
  +-- U8 TPS62933DRLR buck
        EN <- FAN_EN <- ESP32-S3 GPIO2
        SW -> L2 -> FAN_VCC -> H1 pin 1
        FB <- R5/R18/R19 network

ESP32-S3 GPIO1 -> FAN_PWM -> R15 10k -> VCTRL
VCTRL -> C16 1uF -> GND
VCTRL -> R18 75k -> FB

FB -> R5 47k -> FAN_VCC
FB -> R19 10k -> GND
FB -> C24 12pF -> FAN_VCC

FAN_TOUCH -> R105 4.7k -> 3V3
FAN_TOUCH -> C121 100pF -> GND
FAN_TOUCH -> ESP32-S3 GPIO6 / PCNT
```

## Polarity

- `FAN_EN` is active-high.
  - `GPIO2 = high`: enables `TPS62933`.
  - `GPIO2 = low`: disables the fan buck.
- `FAN_PWM` controls the buck feedback through `VCTRL` and `R18`.
  - Higher `FAN_PWM` average voltage raises the FB node and reduces regulator output voltage.
  - Lower `FAN_PWM` average voltage lets the FB node sit lower and raises regulator output voltage.
- Firmware speed polarity is therefore inverted at the hardware duty layer:
  - `speed=100%` -> `hw_duty=0%`
  - `speed=50%` -> `hw_duty=50%`
  - `speed=20%` -> `hw_duty=80%`

## Firmware Expectations

- Configure `GPIO1` as push-pull LEDC PWM at 25 kHz.
- Configure `GPIO2` as push-pull output and drive high before expecting fan power.
- Configure `GPIO6` as tach input with pull-up and PCNT edge counting.
- Treat missing tach pulses as a warning, not a system-fatal condition.

## Bring-Up Checks

When the fan does not spin:

1. Drive `FAN_EN=high` and `speed=100%` (`hw_duty=0%`).
2. Measure `H1 pin 1 FAN_VCC` to `GND`.
3. Measure `U8 EN` and confirm it follows `FAN_EN`.
4. Measure `FAN_PWM` and `VCTRL` while cycling speed.
5. If `FAN_VCC` is present but tach is zero, inspect `FAN_TOUCH`, connector orientation, and fan tach wire compatibility.
