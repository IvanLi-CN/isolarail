// test_joystick_debounce.rs
//! Five-way joystick debounce test program
//!
//! This program tests the software debouncing functionality for the five-way joystick.
//! It demonstrates how the debouncer filters out mechanical bounce and provides
//! clean button press events.

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Input, Pull};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

/// Simplified joystick structure for testing
struct TestJoystick {
    up: Input<'static>,
    down: Input<'static>,
    left: Input<'static>,
    right: Input<'static>,
    center: Input<'static>,
}

impl TestJoystick {
    fn new(p: embassy_stm32::Peripherals) -> Self {
        Self {
            up: Input::new(p.PA1, Pull::Up),     // UP button on PA1
            down: Input::new(p.PA3, Pull::Up),   // DOWN button on PA3
            left: Input::new(p.PA2, Pull::Up),   // LEFT button on PA2
            right: Input::new(p.PA5, Pull::Up),  // RIGHT button on PA5
            center: Input::new(p.PA6, Pull::Up), // CENTER button on PA6
        }
    }

    fn is_up_pressed(&self) -> bool {
        self.up.is_low()
    }

    fn is_down_pressed(&self) -> bool {
        self.down.is_low()
    }

    fn is_left_pressed(&self) -> bool {
        self.left.is_low()
    }

    fn is_right_pressed(&self) -> bool {
        self.right.is_low()
    }

    fn is_center_pressed(&self) -> bool {
        self.center.is_low()
    }
}

/// Button enumeration for debouncing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Button {
    Up,
    Down,
    Left,
    Right,
    Center,
}

/// Button debounce state
#[derive(Debug, Clone, Copy)]
struct ButtonDebounceState {
    current_state: bool,
    stable_state: bool,
    counter: u8,
    last_press_time: u64,
}

impl ButtonDebounceState {
    fn new() -> Self {
        Self {
            current_state: false,
            stable_state: false,
            counter: 0,
            last_press_time: 0,
        }
    }
}

/// Simple debouncer for testing
struct TestDebouncer {
    buttons: [ButtonDebounceState; 5],
    debounce_threshold: u8,
    repeat_delay_ms: u64,
}

impl TestDebouncer {
    fn new(debounce_threshold: u8, repeat_delay_ms: u64) -> Self {
        Self {
            buttons: [ButtonDebounceState::new(); 5],
            debounce_threshold,
            repeat_delay_ms,
        }
    }

    fn update(
        &mut self,
        joystick: &TestJoystick,
        current_time_ms: u64,
    ) -> heapless::Vec<Button, 5> {
        let raw_states = [
            joystick.is_up_pressed(),
            joystick.is_down_pressed(),
            joystick.is_left_pressed(),
            joystick.is_right_pressed(),
            joystick.is_center_pressed(),
        ];

        let button_types = [
            Button::Up,
            Button::Down,
            Button::Left,
            Button::Right,
            Button::Center,
        ];

        let mut pressed_buttons = heapless::Vec::new();

        for (i, &raw_state) in raw_states.iter().enumerate() {
            let button_state = &mut self.buttons[i];

            // Update current state
            if raw_state == button_state.current_state {
                // State is consistent, increment counter
                if button_state.counter < self.debounce_threshold {
                    button_state.counter += 1;
                }
            } else {
                // State changed, reset counter and update current state
                button_state.current_state = raw_state;
                button_state.counter = 0;
            }

            // Check if state is stable
            if button_state.counter >= self.debounce_threshold {
                let previous_stable_state = button_state.stable_state;
                button_state.stable_state = button_state.current_state;

                // Detect rising edge (button press) with repeat delay
                if button_state.stable_state && !previous_stable_state {
                    if current_time_ms >= button_state.last_press_time + self.repeat_delay_ms {
                        button_state.last_press_time = current_time_ms;
                        let _ = pressed_buttons.push(button_types[i]);
                    }
                }
            }
        }

        pressed_buttons
    }

    fn get_button_state(&self, button: Button) -> bool {
        let index = match button {
            Button::Up => 0,
            Button::Down => 1,
            Button::Left => 2,
            Button::Right => 3,
            Button::Center => 4,
        };
        self.buttons[index].stable_state
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Initialize STM32
    let config = embassy_stm32::Config::default();
    let p = embassy_stm32::init(config);

    info!("Five-way joystick debounce test program started");
    info!("GPIO configuration:");
    info!("  UP    -> PA1");
    info!("  DOWN  -> PA3");
    info!("  LEFT  -> PA2");
    info!("  RIGHT -> PA5");
    info!("  CENTER-> PA6");
    info!("");
    info!("Debounce settings:");
    info!("  Threshold: 3 samples");
    info!("  Repeat delay: 200ms");
    info!("");
    info!("Press any direction to test debouncing...");

    // Initialize joystick
    let joystick = TestJoystick::new(p);

    // Initialize debouncer with 3-sample threshold and 200ms repeat delay
    let mut debouncer = TestDebouncer::new(3, 200);

    let mut loop_counter = 0u32;

    loop {
        let current_time_ms = embassy_time::Instant::now().as_millis();
        let pressed_buttons = debouncer.update(&joystick, current_time_ms);

        // Process debounced button presses
        for button in pressed_buttons {
            match button {
                Button::Up => {
                    info!(
                        "✓ DEBOUNCED: UP button pressed (PA1) at {}ms",
                        current_time_ms
                    );
                }
                Button::Down => {
                    info!(
                        "✓ DEBOUNCED: DOWN button pressed (PA3) at {}ms",
                        current_time_ms
                    );
                }
                Button::Left => {
                    info!(
                        "✓ DEBOUNCED: LEFT button pressed (PA2) at {}ms",
                        current_time_ms
                    );
                }
                Button::Right => {
                    info!(
                        "✓ DEBOUNCED: RIGHT button pressed (PA5) at {}ms",
                        current_time_ms
                    );
                }
                Button::Center => {
                    info!(
                        "✓ DEBOUNCED: CENTER button pressed (PA6) at {}ms",
                        current_time_ms
                    );
                }
            }
        }

        // Show raw vs stable states every 1000 loops (approximately every 10 seconds)
        loop_counter += 1;
        if loop_counter % 1000 == 0 {
            let raw_states = (
                joystick.is_up_pressed(),
                joystick.is_down_pressed(),
                joystick.is_left_pressed(),
                joystick.is_right_pressed(),
                joystick.is_center_pressed(),
            );

            let stable_states = (
                debouncer.get_button_state(Button::Up),
                debouncer.get_button_state(Button::Down),
                debouncer.get_button_state(Button::Left),
                debouncer.get_button_state(Button::Right),
                debouncer.get_button_state(Button::Center),
            );

            info!(
                "Status check - Raw: {:?}, Stable: {:?}",
                raw_states, stable_states
            );
        }

        // 10ms polling interval
        Timer::after(Duration::from_millis(10)).await;
    }
}
