# Development Notes

## Current Hardware Validation Baseline

The project is currently in an engineering validation phase. Hardware that is not
explicitly connected for the active validation target must be treated as
"intentionally absent", not as a defect.

Current baseline:

- Power input path is expected to be connected and available during bring-up.
- Front panel may be connected when front-panel behavior is under test.
- Output module channel 4 is the active validation target.
  - Current confirmed sensor pairing for channel 4:
    - `INA226 @ 0x43`
    - `TMP112 @ 0x4B`
- Other output modules may remain physically disconnected during this phase.
- Other optional peripherals that are not part of the active validation target may
  remain disconnected.

Firmware and logs should be interpreted against this baseline:

- Missing devices outside the active validation target are normal during this
  phase.
- Do not treat absent non-target hardware as a regression by default.
- When the hardware under test changes, update this note first, then interpret new
  scan results against the updated baseline.
