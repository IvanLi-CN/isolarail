// src/joystick_example.rs
//! Five-way joystick usage example
//!
//! This module demonstrates how to use the five-way joystick for navigation
//! and user input in the ISO USB Hub application.

use crate::hardware::{FiveWayJoystick, HardwareConfig};
use defmt::*;
use embassy_time::{Duration, Timer};

/// Joystick direction enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoystickDirection {
    Up,
    Down,
    Left,
    Right,
    Center,
    None,
}

impl defmt::Format for JoystickDirection {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            JoystickDirection::Up => defmt::write!(fmt, "UP"),
            JoystickDirection::Down => defmt::write!(fmt, "DOWN"),
            JoystickDirection::Left => defmt::write!(fmt, "LEFT"),
            JoystickDirection::Right => defmt::write!(fmt, "RIGHT"),
            JoystickDirection::Center => defmt::write!(fmt, "CENTER"),
            JoystickDirection::None => defmt::write!(fmt, "NONE"),
        }
    }
}

/// Joystick event handler
pub struct JoystickHandler {
    last_direction: JoystickDirection,
    debounce_counter: u8,
}

impl JoystickHandler {
    /// Create a new joystick handler
    pub fn new() -> Self {
        Self {
            last_direction: JoystickDirection::None,
            debounce_counter: 0,
        }
    }

    /// Read current joystick direction with debouncing
    pub fn read_direction(&mut self, joystick: &FiveWayJoystick) -> Option<JoystickDirection> {
        let current_direction = self.get_current_direction(joystick);

        // Simple debouncing logic
        if current_direction == self.last_direction {
            if self.debounce_counter < 3 {
                self.debounce_counter += 1;
                return None;
            }
        } else {
            self.debounce_counter = 0;
            self.last_direction = current_direction;
            return None;
        }

        // Return direction only if it's not None and debounced
        if current_direction != JoystickDirection::None {
            Some(current_direction)
        } else {
            None
        }
    }

    /// Get current direction without debouncing
    fn get_current_direction(&self, joystick: &FiveWayJoystick) -> JoystickDirection {
        if joystick.is_up_pressed() {
            JoystickDirection::Up
        } else if joystick.is_down_pressed() {
            JoystickDirection::Down
        } else if joystick.is_left_pressed() {
            JoystickDirection::Left
        } else if joystick.is_right_pressed() {
            JoystickDirection::Right
        } else if joystick.is_center_pressed() {
            JoystickDirection::Center
        } else {
            JoystickDirection::None
        }
    }
}

/// Example joystick monitoring task
pub async fn joystick_monitor_task(hardware: &mut HardwareConfig<'_>) {
    info!("Starting joystick monitor task...");
    let mut handler = JoystickHandler::new();

    loop {
        // Check joystick state
        if let Some(direction) = handler.read_direction(&hardware.joystick) {
            info!("Joystick pressed: {}", direction);

            // Handle different directions
            match direction {
                JoystickDirection::Up => {
                    info!("Navigation: UP - Move cursor up or increase value");
                    // Add your UP action here
                }
                JoystickDirection::Down => {
                    info!("Navigation: DOWN - Move cursor down or decrease value");
                    // Add your DOWN action here
                }
                JoystickDirection::Left => {
                    info!("Navigation: LEFT - Move cursor left or previous item");
                    // Add your LEFT action here
                }
                JoystickDirection::Right => {
                    info!("Navigation: RIGHT - Move cursor right or next item");
                    // Add your RIGHT action here
                }
                JoystickDirection::Center => {
                    info!("Action: CENTER - Select/confirm current item");
                    // Add your CENTER/SELECT action here

                    // Example: trigger buzzer on center press
                    hardware.buzzer_pwm.ch1().set_duty_cycle_percent(50);
                    Timer::after(Duration::from_millis(100)).await;
                    hardware.buzzer_pwm.ch1().set_duty_cycle_percent(0);
                }
                JoystickDirection::None => {
                    // This shouldn't happen in this context
                }
            }
        }

        // Small delay to prevent excessive polling
        Timer::after(Duration::from_millis(10)).await;
    }
}

/// Test all joystick buttons
pub async fn test_joystick_buttons(joystick: &FiveWayJoystick) {
    info!("Testing joystick buttons...");

    let (up, down, left, right, center) = joystick.get_all_states();

    info!(
        "Joystick states - UP: {}, DOWN: {}, LEFT: {}, RIGHT: {}, CENTER: {}",
        up, down, left, right, center
    );

    if up || down || left || right || center {
        info!("At least one button is currently pressed!");
    } else {
        info!("No buttons are currently pressed.");
    }
}

/// Menu navigation example using joystick
pub struct MenuNavigator {
    current_item: usize,
    menu_items: &'static [&'static str],
}

impl MenuNavigator {
    pub fn new(items: &'static [&'static str]) -> Self {
        Self {
            current_item: 0,
            menu_items: items,
        }
    }

    pub fn handle_joystick(&mut self, direction: JoystickDirection) -> bool {
        match direction {
            JoystickDirection::Up => {
                if self.current_item > 0 {
                    self.current_item -= 1;
                } else {
                    self.current_item = self.menu_items.len() - 1; // Wrap to last item
                }
                info!(
                    "Menu: Selected item {} - {}",
                    self.current_item, self.menu_items[self.current_item]
                );
                false
            }
            JoystickDirection::Down => {
                if self.current_item < self.menu_items.len() - 1 {
                    self.current_item += 1;
                } else {
                    self.current_item = 0; // Wrap to first item
                }
                info!(
                    "Menu: Selected item {} - {}",
                    self.current_item, self.menu_items[self.current_item]
                );
                false
            }
            JoystickDirection::Center => {
                info!(
                    "Menu: Activated item {} - {}",
                    self.current_item, self.menu_items[self.current_item]
                );
                true // Return true to indicate item was selected
            }
            _ => false,
        }
    }

    pub fn get_current_item(&self) -> (usize, &'static str) {
        (self.current_item, self.menu_items[self.current_item])
    }
}
