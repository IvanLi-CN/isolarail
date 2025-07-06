# Joystick Port Selection Feature

## Overview

This document describes the implementation of the joystick port selection feature for the ISO USB Hub project. The feature allows users to navigate between different USB ports using the left and right directions of the five-way joystick, with visual feedback on the display.

## Features

### Port Selection
- **Default Selection**: Port 2 (middle port) is selected by default
- **Navigation**: Use LEFT and RIGHT joystick directions to navigate between ports
  - LEFT: Move to previous port (Port 3 → Port 2 → Port 1)
  - RIGHT: Move to next port (Port 1 → Port 2 → Port 3)
- **Boundaries**: Navigation stops at the boundaries (cannot go beyond Port 1 or Port 3)

### Visual Feedback

- **Selection Indicator**: Selected port column has a unified rounded rectangle background
- **Background Shape**: Rounded rectangle with 4-pixel corner radius covering the entire column
- **Background Color**: `Rgb565::new(8, 8, 16)` - subtle dark blue that doesn't interfere with text readability
- **Padding**: 4 pixels of padding around the entire column content
- **Column Background**: The entire column (all three text lines) shares one rounded background
- **Non-selected Ports**: No background, normal black display

### Audio Feedback
- **Beep Sound**: A 100ms beep is played when successfully changing port selection
- **No Sound**: No beep when trying to navigate beyond boundaries

## Implementation Details

### Dashboard Structure Changes
```rust
pub struct Dashboard {
    // ... existing fields ...
    selected_port: usize, // Currently selected port (0, 1, or 2)
}
```

### New Methods
- `set_selected_port(port: usize)`: Set the selected port (with bounds checking)
- `get_selected_port() -> usize`: Get the currently selected port

### Main Loop Integration
The joystick handling is integrated into the main application loop with:
- **Debouncing**: Only triggers on rising edge (button press, not hold)
- **State Tracking**: Prevents multiple triggers from single button press
- **Logging**: Debug information when port selection changes

### Display Rendering

- **Screen Clearing**: Entire screen is cleared before each redraw to prevent background artifacts
- **Column Positions**: Hardcoded column positions to prevent overlap:
  - Port 1: x=0 to x=50 (50 pixels wide)
  - Port 2: x=55 to x=105 (50 pixels wide)
  - Port 3: x=110 to x=160 (50 pixels wide)
- **Column Background**: Entire selected column gets a unified rounded rectangle background
- **Corner Radius**: 4-pixel corner radius for smooth rounded corners
- **Padding**: 4 pixels of padding around the entire column content
- **Text Rendering**: All text (voltage, current, power) is drawn on top of the column background
- **Performance**: Uses efficient area-based rendering with pre-calculated pixel buffers

## Usage

1. **Power On**: System starts with Port 2 selected (default)
2. **Navigate Left**: Press joystick LEFT to select previous port
3. **Navigate Right**: Press joystick RIGHT to select next port
4. **Visual Confirmation**: Selected port will have a dark blue background
5. **Audio Confirmation**: Successful navigation produces a short beep

## Technical Notes

### Joystick GPIO Mapping
- LEFT: PA2
- RIGHT: PA5
- (UP: PA1, DOWN: PA3, CENTER: PA6 - not used for port selection)

### Display Coordinates
- Screen: 160x40 pixels in landscape mode
- Columns: 3 ports, each approximately 53 pixels wide
- Background: Full column height (40 pixels)

### Performance Considerations
- Background drawing uses vector allocation for pixel buffer
- Efficient memory usage with proper buffer management
- Minimal impact on main loop timing (100ms cycle)

## Future Enhancements

Potential improvements for this feature:
1. **Animation**: Smooth transition effects between port selections
2. **Different Colors**: Port-specific selection colors
3. **Additional Actions**: CENTER button for port-specific actions
4. **Status Integration**: Different selection styles based on port status (connected/disconnected)

## Code Files Modified

- `src/display/dashboard.rs`: Added selection state and background rendering
- `src/app.rs`: Added joystick handling in main loop
- `docs/joystick-port-selection.md`: This documentation file

## Testing

To test the feature:
1. Compile and flash the program to the STM32G431 microcontroller
2. Observe the default selection on Port 2 (middle column highlighted)
3. Press LEFT/RIGHT on the joystick to navigate between ports
4. Verify visual feedback (background highlight) and audio feedback (beep)
5. Test boundary conditions (cannot go beyond Port 1 or Port 3)
