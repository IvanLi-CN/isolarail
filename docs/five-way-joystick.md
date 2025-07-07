# Five-Way Digital Joystick Integration

## Overview

This document describes the integration of a five-way digital joystick for user interface navigation in the ISO USB Hub project. The joystick is connected to STM32G431C8U6 microcontroller GPIO pins and provides directional input for menu navigation and parameter adjustment.

## GPIO Pin Assignment

| Direction | GPIO Pin | Description | Configuration |
|-----------|----------|-------------|---------------|
| **UP** | **PA1** | Up direction button | Input, internal pull-up, active low |
| **DOWN** | **PA3** | Down direction button | Input, internal pull-up, active low |
| **LEFT** | **PA2** | Left direction button | Input, internal pull-up, active low |
| **RIGHT** | **PA5** | Right direction button | Input, internal pull-up, active low |
| **CENTER** | **PA6** | Center/confirm button | Input, internal pull-up, active low |

## Hardware Connection

### Joystick Side
- Connect one terminal of each button to the corresponding GPIO pin
- Connect the other terminal of each button to GND

### MCU Side
- All GPIO pins configured as input mode
- Internal pull-up resistors enabled
- Button press reads as low level (0)
- Button release reads as high level (1)

## Software Implementation

### Core Structure

```rust
pub struct FiveWayJoystick {
    pub up: Input<'static>,      // PA1
    pub down: Input<'static>,    // PA3
    pub left: Input<'static>,    // PA2
    pub right: Input<'static>,   // PA5
    pub center: Input<'static>,  // PA6
}
```

### Initialization

```rust
let joystick = FiveWayJoystick {
    up: Input::new(p.PA1, Pull::Up),       // UP button on PA1
    down: Input::new(p.PA3, Pull::Up),     // DOWN button on PA3
    left: Input::new(p.PA2, Pull::Up),     // LEFT button on PA2
    right: Input::new(p.PA5, Pull::Up),    // RIGHT button on PA5
    center: Input::new(p.PA6, Pull::Up),   // CENTER button on PA6
};
```

### Available Methods

```rust
// Individual button state checking
joystick.is_up_pressed()      // Returns true if UP button is pressed
joystick.is_down_pressed()    // Returns true if DOWN button is pressed
joystick.is_left_pressed()    // Returns true if LEFT button is pressed
joystick.is_right_pressed()   // Returns true if RIGHT button is pressed
joystick.is_center_pressed()  // Returns true if CENTER button is pressed

// Get all button states at once
let (up, down, left, right, center) = joystick.get_all_states();
```

## Testing

### Independent Test Program

Run the dedicated test program to verify joystick functionality:

```bash
# Compile test program
cargo build --bin test-joystick

# Run test program
cargo run --bin test-joystick
```

The test program will:
- Initialize all GPIO pins
- Monitor button states in real-time
- Output button press/release events via serial
- Verify each direction's functionality

### Debounce Testing

Test the software debouncing functionality:

```bash
# Compile debounce test program
cargo build --bin test-joystick-debounce

# Run debounce test program
cargo run --bin test-joystick-debounce
```

The debounce test program will:

- Initialize joystick with software debouncing
- Show debounced button press events with timestamps
- Display raw vs stable button states periodically
- Demonstrate bounce filtering in real-time

### Integration Testing

Use the joystick with debouncing in the main program:

```rust
// Access joystick and debouncer in main program
let mut hardware = initialize_hardware(p).await;

// Update debouncer and get button press events
let current_time_ms = embassy_time::Instant::now().as_millis();
let pressed_buttons = hardware.joystick_debouncer.update(&hardware.joystick, current_time_ms);

// Process debounced button presses
for button in pressed_buttons {
    match button {
        JoystickButton::Up => {
            info!("UP button pressed (debounced)");
        }
        JoystickButton::Down => {
            info!("DOWN button pressed (debounced)");
        }
        // ... handle other buttons
    }
}

// Check stable button states
let center_pressed = hardware.joystick_debouncer.get_button_state(JoystickButton::Center);
```

## Usage Examples

### Menu Navigation

```rust
use crate::joystick_example::{JoystickHandler, JoystickDirection};

let mut handler = JoystickHandler::new();

loop {
    if let Some(direction) = handler.read_direction(&hardware.joystick) {
        match direction {
            JoystickDirection::Up => {
                // Navigate up
                menu_cursor_up();
            }
            JoystickDirection::Down => {
                // Navigate down
                menu_cursor_down();
            }
            JoystickDirection::Center => {
                // Confirm selection
                menu_select_item();
            }
            _ => {}
        }
    }
    Timer::after(Duration::from_millis(10)).await;
}
```

### Parameter Adjustment

```rust
match direction {
    JoystickDirection::Up => {
        current_value += 1;
        info!("Parameter increased: {}", current_value);
    }
    JoystickDirection::Down => {
        current_value -= 1;
        info!("Parameter decreased: {}", current_value);
    }
    JoystickDirection::Left => {
        // Switch to previous parameter
        previous_parameter();
    }
    JoystickDirection::Right => {
        // Switch to next parameter
        next_parameter();
    }
    JoystickDirection::Center => {
        // Save current settings
        save_settings();
    }
}
```

## Software Debouncing

### Overview

The five-way joystick includes an advanced software debouncing system that eliminates mechanical button bounce and provides reliable button press detection. The debouncer uses a multi-stage filtering approach:

1. **Sample Consistency**: Requires multiple consecutive identical readings before accepting a state change
2. **Stable State Tracking**: Maintains separate current and stable states for each button
3. **Repeat Prevention**: Implements configurable minimum time between repeat presses
4. **Event Generation**: Produces clean button press events only after debouncing

### Configuration

```rust
// Create debouncer with custom settings
let debouncer = JoystickDebouncer::new(
    5,    // debounce_threshold: 5 samples required for state change
    300   // repeat_delay_ms: 300ms minimum between repeat presses
);

// Create debouncer with default settings (3 samples, 200ms delay)
let debouncer = JoystickDebouncer::new_default();
```

### Debounce Parameters

- **Debounce Threshold**: Number of consecutive identical readings required (default: 3)
  - Lower values: More responsive but less bounce filtering
  - Higher values: Better bounce filtering but slower response
  - Recommended range: 2-5 samples

- **Repeat Delay**: Minimum time between repeat button presses in milliseconds (default: 200ms)
  - Prevents accidental rapid-fire button presses
  - Allows intentional repeated presses after delay
  - Recommended range: 100-500ms

## Features

### 1. Software Debouncing

- Advanced software debouncing mechanism implemented
- Prevents false triggers from mechanical button bounce
- Configurable debounce threshold (default: 3 samples)
- Configurable repeat delay (default: 200ms)
- Per-button state tracking with stable state detection
- Event-based button press detection with timing control

### 2. Direction Detection
- Independent detection for all five directions
- Unified state reading interface
- Support for combination button detection
- Raw and stable state access

### 3. Event Handling
- Event-driven button processing
- Support for press and release events
- Extensible event handling framework
- Time-based repeat prevention

## Troubleshooting

### 1. No Button Response

**Possible Causes:**
- Incorrect hardware connections
- GPIO pin configuration issues
- Pull-up resistors not enabled

**Solutions:**
1. Check hardware connections, ensure buttons are correctly connected to specified GPIO pins
2. Verify GPIO pin configuration:
   ```rust
   // Ensure correct configuration
   Input::new(p.PA1, Pull::Up)  // Enable internal pull-up
   ```
3. Use multimeter to test button circuits

### 2. False Triggers or Bouncing

**Possible Causes:**
- Mechanical button bounce
- Inappropriate debounce parameters
- Electrical interference

**Solutions:**
1. Adjust debounce parameters:
   ```rust
   // Increase threshold in JoystickHandler
   if self.debounce_counter < 5 {  // Increase threshold
       self.debounce_counter += 1;
   }
   ```
2. Add hardware filtering capacitors
3. Check GND connection quality

### 3. Partial Button Failure

**Possible Causes:**
- Specific GPIO pins occupied by other functions
- Hardware connection issues
- Pin multiplexing conflicts

**Solutions:**
1. Check pin multiplexing table, confirm selected pins are available
2. Test each GPIO pin individually:
   ```rust
   // Test individual pins
   info!("PA1 state: {}", p.PA1.is_low());
   ```
3. If conflicts exist, switch to available GPIO pins

## Development Recommendations

### 1. Performance Optimization
- Consider using GPIO interrupts instead of polling to reduce CPU usage
- Implement smarter debouncing algorithms
- Add long-press detection functionality

### 2. Feature Extensions
- Implement combination button detection (e.g., diagonal directions)
- Add gesture recognition functionality
- Support custom button mapping

### 3. User Experience
- Add button feedback (buzzer or LED)
- Implement button sensitivity adjustment
- Provide button calibration functionality

## File Structure

```
src/
├── hardware.rs           # Hardware initialization and joystick structure
├── joystick_example.rs   # Joystick usage examples and utility functions
├── app.rs               # Application layer joystick integration
└── main.rs              # Main program entry point

test_joystick.rs         # Independent test program
docs/
└── five-way-joystick.md # This documentation
```

## Next Steps

1. **Hardware Connection Test**: Use test program to verify all button functionality
2. **STM32CubeMX Update**: Configure PA1, PA2, PA3, PA5, PA6 as GPIO inputs in .ioc file
3. **Main Program Integration**: Add joystick event handling to main application loop
4. **Menu System**: Implement complete menu navigation system
5. **Settings Interface**: Add parameter configuration and adjustment functionality
