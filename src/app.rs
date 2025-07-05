// src/app.rs
use embassy_stm32::timer::simple_pwm::SimplePwm;
use embassy_stm32::peripherals;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::WebColors;
use tca6424::{Pin, PinState};
use defmt::*;
use crate::display::dashboard::Dashboard;

/// Buzzer control function to emit a beep sound
pub async fn beep_buzzer(buzzer_pwm: &mut SimplePwm<'_, peripherals::TIM3>, duration_ms: u64) {
    // Enable PWM channel and set 80% duty cycle for louder beep
    buzzer_pwm.ch1().enable();
    buzzer_pwm.ch1().set_duty_cycle_percent(80);

    // Wait for the specified duration
    embassy_time::Timer::after_millis(duration_ms).await;

    // Turn off the buzzer
    buzzer_pwm.ch1().set_duty_cycle_percent(0);
    buzzer_pwm.ch1().disable();
}

/// Display test pattern on startup
pub async fn display_test_pattern<'a, BUS, DC, RST, TIMER, BusE, PinE>(
    display: &mut gc9d01::GC9D01<'a, BUS, DC, RST, TIMER>
) -> Result<(), gc9d01::Error<BusE, PinE>>
where
    BUS: embedded_hal_async::spi::SpiDevice<Error = BusE>,
    DC: embedded_hal::digital::OutputPin<Error = PinE>,
    RST: embedded_hal::digital::OutputPin<Error = PinE>,
    TIMER: gc9d01::Timer,
    BusE: core::fmt::Debug + embedded_hal_async::spi::Error + defmt::Format,
    PinE: core::fmt::Debug + defmt::Format,
{
    info!("Drawing test pattern.");
    let colors = [
        Rgb565::CSS_WHITE,
        Rgb565::CSS_YELLOW,
        Rgb565::CSS_CYAN,
        Rgb565::CSS_GREEN,
        Rgb565::CSS_MAGENTA,
        Rgb565::CSS_RED,
        Rgb565::CSS_BLUE,
        Rgb565::CSS_BLACK,
    ];

    // Each stripe is 20 pixels wide and 40 pixels high
    const STRIPE_WIDTH: u16 = 20;
    const STRIPE_HEIGHT: u16 = 40;

    // Create a buffer for one stripe's pixel data
    let mut stripe_pixels = [Rgb565::CSS_BLACK; (STRIPE_WIDTH * STRIPE_HEIGHT) as usize];

    for (i, color) in colors.iter().enumerate() {
        let x = i as u16 * STRIPE_WIDTH;

        // Fill the stripe buffer with the current color
        for pixel in stripe_pixels.iter_mut() {
            *pixel = *color;
        }

        // Write the pixel data for the current stripe
        if let Err(e) = display.write_area(x, 0, STRIPE_WIDTH, STRIPE_HEIGHT, &stripe_pixels).await {
            error!("Failed to write stripe {}: {:?}", i, e);
            return Err(e);
        }
    }

    Ok(())
}

/// Main application loop
pub async fn run_application(mut hardware: crate::hardware::HardwareConfig<'static>) {
    info!("Starting application main loop");

    // Fill display with black background
    hardware.display.fill_color(Rgb565::CSS_BLACK).await.unwrap();

    // Display test pattern
    if let Err(e) = display_test_pattern(&mut hardware.display).await {
        error!("Failed to display test pattern: {:?}", e);
    }

    // Test buzzer on startup
    info!("Testing buzzer...");
    beep_buzzer(&mut hardware.buzzer_pwm, 300).await; // 300ms test beep
    info!("Buzzer test complete.");

    // Instantiate Dashboard
    let mut dashboard = Dashboard::new();

    // Initial delay before starting the loop
    embassy_time::Timer::after_secs(1).await;

    // Initialize previous UFP states for change detection
    let mut prev_port1_connected = false; // SW2303 Port 1
    let mut prev_port2_connected = false;
    let mut prev_port3_connected = false;

    loop {
        // Read data from INA226 sensors
        let voltage1 = hardware.ina226_sensors.0.bus_voltage_millivolts().await.unwrap_or(0.0);
        let current1 = hardware.ina226_sensors.0.current_amps().await.unwrap_or(None).unwrap_or(0.0);
        let power1 = hardware.ina226_sensors.0.power_watts().await.unwrap_or(None).unwrap_or(0.0);

        let voltage2 = hardware.ina226_sensors.1.bus_voltage_millivolts().await.unwrap_or(0.0);
        let current2 = hardware.ina226_sensors.1.current_amps().await.unwrap_or(None).unwrap_or(0.0);
        let power2 = hardware.ina226_sensors.1.power_watts().await.unwrap_or(None).unwrap_or(0.0);

        let voltage3 = hardware.ina226_sensors.2.bus_voltage_millivolts().await.unwrap_or(0.0);
        let current3 = hardware.ina226_sensors.2.current_amps().await.unwrap_or(None).unwrap_or(0.0);
        let power3 = hardware.ina226_sensors.2.power_watts().await.unwrap_or(None).unwrap_or(0.0);

        // Read SW2303 sink device connection status for Port 1
        let sw2303_port1_connected = match hardware.sw2303_controller.is_sink_device_connected().await {
            Ok(connected) => connected,
            Err(e) => {
                error!("Failed to read SW2303 sink device status: {:?}", e);
                false
            }
        };

        // Read P2_UFP (P01) and P3_UFP (P25) states
        let p2_ufp_state = hardware.tca6424_expander.get_pin_input_state(Pin::P01).await.unwrap();
        let p3_ufp_state = hardware.tca6424_expander.get_pin_input_state(Pin::P25).await.unwrap();

        // Px_UFP is Low Active, so Low means connected
        let port2_connected = p2_ufp_state == PinState::Low;
        let port3_connected = p3_ufp_state == PinState::Low;

        // Check for UFP status changes and trigger buzzer
        if sw2303_port1_connected != prev_port1_connected {
            info!("SW2303 PD controller Port 1 UFP status changed: {} -> {}", prev_port1_connected, sw2303_port1_connected);
            beep_buzzer(&mut hardware.buzzer_pwm, 200).await; // 200ms beep
            prev_port1_connected = sw2303_port1_connected;
        }

        if port2_connected != prev_port2_connected {
            info!("TCA6424 Port 2 UFP status changed: {} -> {}", prev_port2_connected, port2_connected);
            beep_buzzer(&mut hardware.buzzer_pwm, 200).await; // 200ms beep
            prev_port2_connected = port2_connected;
        }

        if port3_connected != prev_port3_connected {
            info!("TCA6424 Port 3 UFP status changed: {} -> {}", prev_port3_connected, port3_connected);
            beep_buzzer(&mut hardware.buzzer_pwm, 200).await; // 200ms beep
            prev_port3_connected = port3_connected;
        }

        // Prepare data for Dashboard, converting f64 to f32
        let sensor_data = [
            ((voltage1 / 1000.0) as f32, current1 as f32, power1 as f32),
            ((voltage2 / 1000.0) as f32, current2 as f32, power2 as f32),
            ((voltage3 / 1000.0) as f32, current3 as f32, power3 as f32),
        ];

        // Prepare connection status for Dashboard
        let connection_status = [sw2303_port1_connected, port2_connected, port3_connected];

        // Update Dashboard data
        dashboard.update_data(sensor_data, connection_status);

        // Draw Dashboard directly to the display
        dashboard.draw(&mut hardware.display).await.unwrap();

        // Wait for 100ms before the next update
        embassy_time::Timer::after_millis(100).await;
    }
}
