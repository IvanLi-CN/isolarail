# Five-Way Joystick Software Debouncing Implementation

## Overview

This document describes the implementation of software debouncing for the five-way joystick in the ISO USB Hub project. The debouncing system eliminates mechanical button bounce and provides reliable button press detection.

## Implementation Details

### Core Components

1. **JoystickButton Enum**: Defines the five button types (Up, Down, Left, Right, Center)
2. **ButtonDebounceState**: Tracks debounce state for individual buttons
3. **JoystickDebouncer**: Main debouncing logic and state management

### Debouncing Algorithm

The debouncer uses a multi-stage approach:

1. **Sample Collection**: Continuously reads raw button states
2. **Consistency Check**: Requires multiple consecutive identical readings
3. **State Stabilization**: Updates stable state only after threshold is met
4. **Event Generation**: Produces button press events on rising edges
5. **Repeat Prevention**: Enforces minimum time between repeat presses

### Key Features

- **Configurable Threshold**: Default 3 samples, adjustable 2-5 range
- **Repeat Delay**: Default 200ms, prevents accidental rapid-fire
- **Per-Button Tracking**: Independent state for each of the 5 buttons
- **Time-Based Control**: Uses system time for repeat prevention
- **Event-Driven**: Returns only confirmed button press events

## Usage in Main Application

### Integration

The debouncer is integrated into the main application loop:

```rust
// Update debouncer and get button press events
let current_time_ms = embassy_time::Instant::now().as_millis();
let pressed_buttons = hardware.joystick_debouncer.update(&hardware.joystick, current_time_ms);

// Process debounced button presses
for button in pressed_buttons {
    match button {
        JoystickButton::Left => {
            // Handle left button press
        }
        JoystickButton::Right => {
            // Handle right button press
        }
        // ... other buttons
    }
}
```

### Button Release Handling

For buttons that need release detection (like CENTER for USB communication control):

```rust
let center_currently_pressed = hardware.joystick_debouncer.get_button_state(JoystickButton::Center);
if !center_currently_pressed && usb_comm_disabled {
    // Handle button release
}
```

## Benefits

### Before Debouncing
- Mechanical bounce caused false triggers
- Required manual edge detection with previous state tracking
- Susceptible to electrical noise
- Inconsistent user experience

### After Debouncing
- Clean, reliable button press events
- Automatic bounce filtering
- Configurable sensitivity
- Consistent timing behavior
- Simplified application logic

## Testing

### Test Programs

1. **test-joystick**: Basic GPIO functionality test
2. **test-joystick-debounce**: Debouncing functionality test (10ms polling)
3. **test-joystick-timing**: Main program timing simulation test (50ms polling)

### Running Tests

```bash
# Test basic joystick functionality
cargo run --bin test-joystick

# Test debouncing functionality (fast polling)
cargo run --bin test-joystick-debounce

# Test timing with main program settings (realistic polling)
cargo run --bin test-joystick-timing
```

### Test Output

**test-joystick-debounce** shows:
- Raw button states vs stable states
- Debounced button press events with timestamps
- Real-time bounce filtering demonstration
- Fast 10ms polling for detailed analysis

**test-joystick-timing** shows:
- Button press events with main program timing
- Delay measurements between repeat presses
- Debug information for troubleshooting
- Realistic 50ms polling simulation

### Timing Analysis

The timing test helps identify responsiveness issues:
- **Expected response time**: ~100ms (2 samples × 50ms polling)
- **Repeat delay**: 250ms minimum between presses
- **Debug output**: Shows counter progression and state changes

## Configuration Guidelines

### Debounce Threshold
- **2 samples**: Very responsive, minimal filtering
- **3 samples**: Default, good balance (recommended)
- **4-5 samples**: More filtering, slower response

### Repeat Delay
- **100ms**: Fast repeat for navigation
- **200ms**: Default, prevents accidental repeats
- **300-500ms**: Slower repeat for critical actions

### Polling Interval Considerations

The relationship between polling interval and debounce threshold is critical:

**Formula**: `Response Time = Debounce Threshold × Polling Interval`

**Examples**:
- 3 samples × 100ms polling = 300ms response time (too slow)
- 2 samples × 50ms polling = 100ms response time (good)
- 3 samples × 10ms polling = 30ms response time (very responsive)

## Troubleshooting

### Problem: Button Response Too Slow

**Symptoms**:
- Buttons take several hundred milliseconds to respond
- User needs to hold buttons for a long time

**Causes**:
- Polling interval too large for debounce threshold
- Debounce threshold too high for polling rate

**Solutions**:
1. **Reduce polling interval**: Change from 100ms to 50ms or less
2. **Reduce debounce threshold**: Use 2 samples instead of 3+
3. **Balance both**: 2 samples × 50ms = 100ms response time

**Example Fix**:
```rust
// Before: 300ms response time (too slow)
let debouncer = JoystickDebouncer::new(3, 200); // 3 samples
// Main loop: Timer::after_millis(100) // 100ms polling

// After: 100ms response time (good)
let debouncer = JoystickDebouncer::new(2, 250); // 2 samples
// Main loop: Timer::after_millis(50) // 50ms polling
```

### Problem: False Triggers

**Symptoms**:
- Multiple button presses from single physical press
- Erratic behavior during button press

**Solutions**:
1. **Increase debounce threshold**: Use 3-4 samples
2. **Increase repeat delay**: Use 300-500ms
3. **Check hardware**: Verify pull-up resistors and connections

## Performance Impact

- **Memory**: ~40 bytes per debouncer instance
- **CPU**: Minimal overhead, O(1) per button per update
- **Timing**: 10ms polling interval maintains responsiveness
- **Latency**: 30-50ms typical debounce delay (3 samples @ 10ms)

## Future Enhancements

Potential improvements for future versions:

1. **Adaptive Thresholds**: Adjust based on button behavior
2. **Noise Detection**: Identify and filter electrical interference
3. **Button Health Monitoring**: Track button wear and performance
4. **Custom Button Profiles**: Different settings per button type
5. **Interrupt-Based Updates**: Reduce polling overhead

## Conclusion

The software debouncing implementation provides a robust, configurable solution for reliable button input handling. It eliminates mechanical bounce issues while maintaining responsive user interaction, significantly improving the overall user experience of the five-way joystick interface.
