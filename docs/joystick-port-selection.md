# Joystick Navigation Features

## Overview

This document describes the implementation of the joystick navigation features for the ISO USB Hub project. The features include port selection using left/right directions and display mode switching using the down direction of the five-way joystick, with visual and audio feedback.

## Features

### Port Selection

- **Default Selection**: Port 2 (middle port) is selected by default
- **Navigation**: Use LEFT and RIGHT joystick directions to navigate between ports
  - LEFT: Move to previous port (Port 3 → Port 2 → Port 1)
  - RIGHT: Move to next port (Port 1 → Port 2 → Port 3)
- **Boundaries**: Navigation stops at the boundaries (cannot go beyond Port 1 or Port 3)

### Display Mode Switching

- **Default Mode**: Power display (Row 3) showing actual power consumption in Watts
- **Toggle**: Use DOWN joystick direction to switch between display modes
  - **Power Mode**: Shows real-time power consumption (V, A, W)
  - **Power Allocation Mode**: Shows power/current limits (V, A, Allocation)
- **Port-Specific Allocation Display**:
  - **Port 1**: Power limit in Watts (default: 65W)
  - **Ports 2 & 3**: Current limit in Amperes (default: 3A each)

### Visual Feedback

- **Selection Indicator**: Selected port column has a unified rounded rectangle background
- **Background Shape**: Rounded rectangle with 4-pixel corner radius covering the entire column
- **Background Color**: `Rgb565::new(8, 8, 16)` - subtle dark blue that doesn't interfere with text readability
- **Padding**: 4 pixels of padding around the entire column content
- **Column Background**: The entire column (all three text lines) shares one rounded background
- **Non-selected Ports**: No background, normal black display

### Audio Feedback

- **Port Selection**: A 100ms beep is played when successfully changing port selection
- **Display Mode Toggle**: A 100ms beep is played when switching display modes
- **No Sound**: No beep when trying to navigate beyond port boundaries

## Implementation Details

### Dashboard Structure Changes

```rust
pub struct Dashboard {
    // ... existing fields ...
    selected_port: usize, // Currently selected port (0, 1, or 2)
    power_allocation: [f32; 3], // Power/current limits for each port
    show_power_allocation: bool, // Display mode toggle
}
```

### New Methods

- `set_selected_port(port: usize)`: Set the selected port (with bounds checking)
- `get_selected_port() -> usize`: Get the currently selected port
- `toggle_display_mode()`: Switch between power and power allocation display
- `update_power_allocation(allocation: [f32; 3])`: Update power/current limits
- `is_showing_power_allocation() -> bool`: Check current display mode

### Main Loop Integration

The joystick handling is integrated into the main application loop with:

- **Debouncing**: Only triggers on rising edge (button press, not hold)
- **State Tracking**: Prevents multiple triggers from single button press
- **Logging**: Debug information when port selection or display mode changes
- **Multi-button Support**: Handles LEFT, RIGHT, and DOWN directions simultaneously

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

1. **Power On**: System starts with Port 2 selected and Power display mode (default)
2. **Navigate Left**: Press joystick LEFT to select previous port
3. **Navigate Right**: Press joystick RIGHT to select next port
4. **Toggle Display**: Press joystick DOWN to switch between Power and Power Allocation modes
5. **Visual Confirmation**:
   - Selected port will have a dark blue background
   - Display mode affects the third row content (Power vs Allocation)
6. **Audio Confirmation**: Successful navigation or mode switching produces a short beep

## Technical Notes

### Joystick GPIO Mapping

- LEFT: PA2 (Port selection)
- RIGHT: PA5 (Port selection)
- DOWN: PA3 (Display mode toggle)
- UP: PA1 (Not used)
- CENTER: PA6 (Not used)

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

1. **Animation**: Smooth transition effects between port selections and display modes
2. **Different Colors**: Port-specific selection colors and mode-specific indicators
3. **Additional Actions**: CENTER button for port-specific actions or settings
4. **Status Integration**: Different selection styles based on port status (connected/disconnected)
5. **Configurable Limits**: Runtime adjustment of power/current allocation limits
6. **Visual Indicators**: Icons or symbols to distinguish between display modes

## Code Files Modified

- `src/display/dashboard.rs`: Added selection state and background rendering
- `src/app.rs`: Added joystick handling in main loop
- `docs/joystick-port-selection.md`: This documentation file

## Testing

To test the features:

1. Compile and flash the program to the STM32G431 microcontroller
2. Observe the default selection on Port 2 (middle column highlighted) in Power mode
3. Press LEFT/RIGHT on the joystick to navigate between ports
4. Press DOWN to toggle between Power and Power Allocation display modes
5. Verify visual feedback (background highlight and display content changes)
6. Verify audio feedback (beep for all successful actions)
7. Test boundary conditions (cannot go beyond Port 1 or Port 3)
8. Confirm Power Allocation displays: Port 1 shows 65W, Ports 2&3 show 3A
