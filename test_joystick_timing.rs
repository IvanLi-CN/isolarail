// test_joystick_timing.rs
//! Joystick timing and responsiveness test program
//!
//! This program helps test and tune the joystick debouncing parameters
//! by simulating the main program's polling interval and showing timing information.

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
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

/// Debouncer for testing with timing info
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
    ) -> heapless::Vec<(Button, u64), 5> {
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
                        let delay_since_last = current_time_ms - button_state.last_press_time;
                        button_state.last_press_time = current_time_ms;
                        let _ = pressed_buttons.push((button_types[i], delay_since_last));
                    }
                }
            }
        }

        pressed_buttons
    }

    fn get_debug_info(&self, button: Button) -> (bool, bool, u8) {
        let index = match button {
            Button::Up => 0,
            Button::Down => 1,
            Button::Left => 2,
            Button::Right => 3,
            Button::Center => 4,
        };
        let state = &self.buttons[index];
        (state.current_state, state.stable_state, state.counter)
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Initialize STM32
    let config = embassy_stm32::Config::default();
    let p = embassy_stm32::init(config);

    info!("Joystick timing test program started");
    info!("This program simulates the main program's polling behavior");
    info!("");
    info!("GPIO configuration:");
    info!("  UP    -> PA1");
    info!("  DOWN  -> PA3");
    info!("  LEFT  -> PA2");
    info!("  RIGHT -> PA5");
    info!("  CENTER-> PA6");
    info!("");
    info!("Testing with main program settings:");
    info!("  Polling interval: 50ms");
    info!("  Debounce threshold: 2 samples");
    info!("  Repeat delay: 250ms");
    info!("  Expected response time: ~100ms (2 * 50ms)");
    info!("");
    info!("Press any button to test timing...");

    // Initialize joystick
    let joystick = TestJoystick::new(p);

    // Initialize debouncer with main program settings
    let mut debouncer = TestDebouncer::new(2, 250);

    let mut loop_counter = 0u32;
    let start_time = embassy_time::Instant::now().as_millis();

    loop {
        let current_time_ms = embassy_time::Instant::now().as_millis();
        let pressed_buttons = debouncer.update(&joystick, current_time_ms);

        // Process debounced button presses with timing info
        for (button, delay_since_last) in pressed_buttons {
            let elapsed_since_start = current_time_ms - start_time;
            match button {
                Button::Up => {
                    info!(
                        "✓ UP pressed at {}ms ({}ms since last press)",
                        elapsed_since_start, delay_since_last
                    );
                }
                Button::Down => {
                    info!(
                        "✓ DOWN pressed at {}ms ({}ms since last press)",
                        elapsed_since_start, delay_since_last
                    );
                }
                Button::Left => {
                    info!(
                        "✓ LEFT pressed at {}ms ({}ms since last press)",
                        elapsed_since_start, delay_since_last
                    );
                }
                Button::Right => {
                    info!(
                        "✓ RIGHT pressed at {}ms ({}ms since last press)",
                        elapsed_since_start, delay_since_last
                    );
                }
                Button::Center => {
                    info!(
                        "✓ CENTER pressed at {}ms ({}ms since last press)",
                        elapsed_since_start, delay_since_last
                    );
                }
            }
        }

        // Show debug info every 2 seconds when buttons are being pressed
        loop_counter += 1;
        if loop_counter % 40 == 0 {
            // Every 2 seconds at 50ms intervals
            let any_pressed = joystick.is_up_pressed()
                || joystick.is_down_pressed()
                || joystick.is_left_pressed()
                || joystick.is_right_pressed()
                || joystick.is_center_pressed();

            if any_pressed {
                info!("Debug info at {}ms:", current_time_ms - start_time);
                for button in [
                    Button::Up,
                    Button::Down,
                    Button::Left,
                    Button::Right,
                    Button::Center,
                ] {
                    let (current, stable, counter) = debouncer.get_debug_info(button);
                    if current || stable || counter > 0 {
                        info!(
                            "  {:?}: raw={}, stable={}, counter={}",
                            button, current, stable, counter
                        );
                    }
                }
            }
        }

        // 50ms polling interval (same as main program)
        Timer::after(Duration::from_millis(50)).await;
    }
}
