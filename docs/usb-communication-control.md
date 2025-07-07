# USB Communication Control Feature

## Overview

This document describes the USB communication control feature that allows temporarily disconnecting USB communication for downstream ports using the five-way joystick center button.

## Hardware Implementation

### TCA6424 GPIO Pins

The USB communication control is implemented using TCA6424 I/O expander pins:

| Pin | Signal Name | Function | Description |
|-----|-------------|----------|-------------|
| P10 | P1_DATA_CONN | Port 1 Communication Enable | Controls USB data communication for Port 1 |
| P11 | P2_DATA_CONN | Port 2 Communication Enable | Controls USB data communication for Port 2 |
| P12 | P3_DATA_CONN | Port 3 Communication Enable | Controls USB data communication for Port 3 |

### Pin States

- **High (1)**: USB communication enabled (default state)
- **Low (0)**: USB communication disabled (temporarily disconnected)

## Software Implementation

### Hardware Configuration

During initialization, the communication control pins are configured as outputs and set to High (enabled):

```rust
// Configure output pins for USB communication control
tca6424_expander.set_pin_direction(Pin::P10, PinDirection::Output).await.unwrap(); // P1_DATA_CONN
tca6424_expander.set_pin_direction(Pin::P11, PinDirection::Output).await.unwrap(); // P2_DATA_CONN
tca6424_expander.set_pin_direction(Pin::P12, PinDirection::Output).await.unwrap(); // P3_DATA_CONN

// Enable USB communication for all ports by default
tca6424_expander.set_pin_output(Pin::P10, PinState::High).await.unwrap(); // P1_DATA_CONN = High (enabled)
tca6424_expander.set_pin_output(Pin::P11, PinState::High).await.unwrap(); // P2_DATA_CONN = High (enabled)
tca6424_expander.set_pin_output(Pin::P12, PinState::High).await.unwrap(); // P3_DATA_CONN = High (enabled)
```

### Control Function

The `control_usb_communication` function provides a unified interface for controlling USB communication:

```rust
pub async fn control_usb_communication<I2C>(
    tca6424: &mut Tca6424<'_, I2C>,
    port: u8,
    enable: bool
) -> Result<(), tca6424::errors::Error<I2C::Error>>
```

**Parameters:**
- `tca6424`: TCA6424 expander instance
- `port`: Port number (1, 2, or 3)
- `enable`: true to enable communication, false to disable

## User Interface

### Joystick Control

The five-way joystick center button (PA6) controls the USB communication for the currently selected port:

1. **Port Selection**: Use LEFT/RIGHT buttons to select the target port (highlighted with blue background)
2. **Disconnect**: Press and hold CENTER button to temporarily disconnect USB communication
3. **Reconnect**: Release CENTER button to restore USB communication

### Visual Feedback

- **Port Selection**: Selected port is highlighted with a dark blue rounded rectangle background
- **Audio Feedback**: 
  - Disconnect: 200ms beep (different from normal navigation sounds)
  - Reconnect: 100ms beep (normal navigation sound)

### Status Logging

The system logs USB communication control actions:

```
INFO: Joystick: CENTER pressed - USB communication disabled for Port 2
INFO: Joystick: CENTER released - USB communication restored for Port 2
```

## Usage Examples

### Scenario 1: Troubleshooting Device Connection

1. Use LEFT/RIGHT to select the problematic port
2. Press and hold CENTER to disconnect USB communication
3. The connected device will be temporarily isolated from the USB hub
4. Release CENTER to restore communication and re-enumerate the device

### Scenario 2: Safe Device Removal

1. Select the port with the device to be removed
2. Press and hold CENTER to disconnect USB communication
3. Safely remove the USB device while communication is disabled
4. Insert new device (if needed)
5. Release CENTER to enable communication for the new device

## Technical Details

### State Management

The system maintains the following state variables:

- `prev_center_pressed`: Tracks previous center button state for edge detection
- `usb_comm_disabled`: Tracks whether USB communication is currently disabled

### Error Handling

If USB communication control fails, the system:

1. Logs an error message with the specific failure reason
2. Continues normal operation without affecting other functions
3. Does not change the `usb_comm_disabled` state

### Integration with Power Management

USB communication control is independent of power allocation:

- Disconnecting USB communication does not affect power delivery to the port
- Power monitoring and allocation continue normally
- Only the USB data lines are affected

## Safety Considerations

1. **Non-destructive**: Temporarily disconnecting USB communication is safe and reversible
2. **Power Maintained**: Power delivery continues during communication disconnect
3. **Automatic Recovery**: Communication is automatically restored when the button is released
4. **Per-Port Control**: Only the selected port is affected, other ports continue normal operation

## Future Enhancements

Potential improvements to this feature:

1. **Timeout Protection**: Automatic reconnection after a maximum disconnect time
2. **Visual Indicators**: On-screen display of communication status
3. **Multiple Port Control**: Ability to control multiple ports simultaneously
4. **Persistent State**: Remember communication state across power cycles
