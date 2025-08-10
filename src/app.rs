// src/app.rs
use crate::display::dashboard::Dashboard;
use defmt::*;
use embassy_stm32::peripherals;
use embassy_stm32::timer::simple_pwm::SimplePwm;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{RgbColor, WebColors};
use tca6424::{Pin, PinState};

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

/// Alarm beep function for overcurrent and fault conditions
/// Plays a different pattern than the connection beep (3 short beeps)
pub async fn play_alarm_beep(buzzer_pwm: &mut SimplePwm<'_, peripherals::TIM3>) {
    for _ in 0..3 {
        // Enable PWM channel and set 90% duty cycle for urgent alarm sound
        buzzer_pwm.ch1().enable();
        buzzer_pwm.ch1().set_duty_cycle_percent(90);

        // Short beep (150ms)
        embassy_time::Timer::after_millis(150).await;

        // Turn off the buzzer
        buzzer_pwm.ch1().set_duty_cycle_percent(0);
        buzzer_pwm.ch1().disable();

        // Short pause between beeps (100ms)
        embassy_time::Timer::after_millis(100).await;
    }
}

/// Display test pattern on startup
#[allow(dead_code)]
pub async fn display_test_pattern<BUS, DC, RST, TIMER, BusE, PinE>(
    display: &mut gc9d01::GC9D01<'_, BUS, DC, RST, TIMER>,
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

        // Write the pixel data for the current stripe to frame buffer
        display.write_area(x, 0, STRIPE_WIDTH, STRIPE_HEIGHT, &stripe_pixels);
    }

    // Flush the frame buffer to display the test pattern
    display.flush().await?;

    Ok(())
}

/// Display startup splash screen from external Flash - simplified implementation based on flash_display_test.rs
async fn display_startup_splash(
    hardware: &mut crate::hardware::HardwareConfig<'static>,
) -> Result<(), &'static str> {
    info!("=== 启动屏显示 - 基于flash_display_test.rs的成功实现 ===");

    // 图像参数 - 与flash_display_test.rs保持一致
    const BITMAP_WIDTH: u16 = 160;
    const BITMAP_HEIGHT: u16 = 40;
    const BYTES_PER_ROW: usize = 320; // 160像素 * 2字节/像素
    const STARTUP_ADDRESS: u32 = 0x000000; // 启动屏地址

    info!("启动屏图像尺寸: {}x{} 像素", BITMAP_WIDTH, BITMAP_HEIGHT);
    info!("启动屏Flash地址: 0x{:06X}", STARTUP_ADDRESS);

    // 逐行读取Flash并渲染到屏幕 - 完全参考flash_display_test.rs的成功实现
    for y in 0..BITMAP_HEIGHT {
        let row_address = STARTUP_ADDRESS + (y as u32 * BYTES_PER_ROW as u32);
        let mut row_buffer = [0u8; BYTES_PER_ROW];

        // 从Flash读取一行数据
        match hardware.flash.read(row_address, &mut row_buffer).await {
            Ok(_) => {
                // 转换为RGB565像素 - 与flash_display_test.rs完全相同的实现
                let mut pixel_row = [Rgb565::BLACK; BITMAP_WIDTH as usize];
                for (pixel_index, pixel_bytes) in row_buffer.chunks_exact(2).enumerate() {
                    if pixel_index < pixel_row.len() {
                        // RGB565小端格式 - 与flash_display_test.rs完全相同
                        let pixel_value = (pixel_bytes[0] as u16) | ((pixel_bytes[1] as u16) << 8);
                        pixel_row[pixel_index] = Rgb565::new(
                            ((pixel_value >> 11) & 0x1F) as u8, // 红色
                            ((pixel_value >> 5) & 0x3F) as u8,  // 绿色
                            (pixel_value & 0x1F) as u8,         // 蓝色
                        );
                    }
                }

                // 写入显示缓冲区 - 与flash_display_test.rs完全相同
                hardware
                    .display
                    .write_area(0, y, BITMAP_WIDTH, 1, &pixel_row);
            }
            Err(e) => {
                error!("启动屏第{}行读取失败: {:?}", y, e);

                // 错误时显示红色行 - 与flash_display_test.rs完全相同
                let error_row = [Rgb565::CSS_RED; BITMAP_WIDTH as usize];
                hardware
                    .display
                    .write_area(0, y, BITMAP_WIDTH, 1, &error_row);
            }
        }
    }

    // 刷新显示器 - 与flash_display_test.rs完全相同
    match hardware.display.flush().await {
        Ok(_) => {
            info!("✓ 启动屏显示成功！");
        }
        Err(e) => {
            error!("✗ 启动屏显示失败: {:?}", e);
            return Err("Failed to flush display");
        }
    }

    info!("启动屏显示完成");
    Ok(())
}

/// Main application loop
pub async fn run_application(mut hardware: crate::hardware::HardwareConfig<'static>) {
    info!("Starting application main loop");

    // Test buzzer immediately on power-up
    info!("Testing buzzer on power-up...");
    beep_buzzer(&mut hardware.buzzer_pwm, 300).await; // 300ms power-up beep
    info!("Power-up buzzer test complete.");

    // Display startup splash screen for 3 seconds
    info!("Displaying startup splash screen for 3 seconds...");
    if let Err(e) = display_startup_splash(&mut hardware).await {
        error!("Startup splash failed: {}", e);
    }

    // Wait for 3 seconds with watchdog feeding
    info!("Waiting 3 seconds for startup splash screen...");
    for i in 0..30 {
        embassy_time::Timer::after_millis(100).await;
        hardware.watchdog.pet(); // Feed watchdog every 100ms during splash screen
        if i % 10 == 9 {
            info!("Splash screen: {}s remaining", 3 - (i + 1) / 10);
        }
    }
    info!("Startup splash screen timeout completed");

    // Clear display and continue with normal application
    hardware.display.fill_color(Rgb565::BLACK);
    hardware.display.flush().await.unwrap();

    // Instantiate Dashboard and start main application
    let mut dashboard = Dashboard::new();

    // Initialize previous UFP states for change detection
    let mut prev_port1_connected = false; // SW2303 Port 1
    let mut prev_port2_connected = false;
    let mut prev_port3_connected = false;

    // Initialize previous overcurrent states for change detection
    let mut previous_overcurrent_status = [false; 3];

    // Track USB communication state for the selected port
    let mut usb_comm_disabled = false;

    loop {
        // Read data from INA226 sensors
        let voltage1 = hardware
            .ina226_sensors
            .0
            .bus_voltage_millivolts()
            .await
            .unwrap_or(0.0);
        let current1 = hardware
            .ina226_sensors
            .0
            .current_amps()
            .await
            .unwrap_or(None)
            .unwrap_or(0.0);
        let power1 = hardware
            .ina226_sensors
            .0
            .power_watts()
            .await
            .unwrap_or(None)
            .unwrap_or(0.0);

        let voltage2 = hardware
            .ina226_sensors
            .1
            .bus_voltage_millivolts()
            .await
            .unwrap_or(0.0);
        let current2 = hardware
            .ina226_sensors
            .1
            .current_amps()
            .await
            .unwrap_or(None)
            .unwrap_or(0.0);
        let power2 = hardware
            .ina226_sensors
            .1
            .power_watts()
            .await
            .unwrap_or(None)
            .unwrap_or(0.0);

        let voltage3 = hardware
            .ina226_sensors
            .2
            .bus_voltage_millivolts()
            .await
            .unwrap_or(0.0);
        let current3 = hardware
            .ina226_sensors
            .2
            .current_amps()
            .await
            .unwrap_or(None)
            .unwrap_or(0.0);
        let power3 = hardware
            .ina226_sensors
            .2
            .power_watts()
            .await
            .unwrap_or(None)
            .unwrap_or(0.0);

        // Read SW2303 connection status for Port 1
        let sw2303_port1_connected = hardware
            .sw2303_controller
            .is_sink_device_connected()
            .await
            .unwrap_or_default();

        // Read P2_UFP (P01) and P3_UFP (P25) states
        let p2_ufp_state = hardware
            .tca6424_expander
            .get_pin_input_state(Pin::P01)
            .await
            .unwrap();
        let p3_ufp_state = hardware
            .tca6424_expander
            .get_pin_input_state(Pin::P25)
            .await
            .unwrap();

        // Px_UFP is Low Active, so Low means connected
        let port2_connected = p2_ufp_state == PinState::Low;
        let port3_connected = p3_ufp_state == PinState::Low;

        // Read SW2303 system status for Port 1 anomaly detection
        let sw2303_system_status0 = hardware.sw2303_controller.get_system_status0().await.ok();
        let sw2303_system_status1 = hardware.sw2303_controller.get_system_status_1().await.ok();
        let sw2303_system_status2 = hardware.sw2303_controller.get_system_status_2().await.ok();

        // Log SW2303 system status for debugging (every 10 seconds to avoid spam)
        static mut LAST_STATUS_LOG_TIME: u64 = 0;
        let current_time = embassy_time::Instant::now().as_millis();
        let should_log_status = unsafe {
            current_time - LAST_STATUS_LOG_TIME > 10000 // 10 seconds
        };

        if should_log_status {
            if let Some(status0) = sw2303_system_status0 {
                info!(
                    "SW2303 Status0: OPTOCOUPLER={}, CC_LOOP={}, LINE_COMP={}, PASS_TRANS={}",
                    status0.contains(sw2303::registers::SystemStatus0Flags::ABNORMAL_OPTOCOUPLER),
                    status0.contains(sw2303::registers::SystemStatus0Flags::CC_LOOP_CLOSED),
                    status0.contains(sw2303::registers::SystemStatus0Flags::LINE_COMPENSATION_OPEN),
                    status0.contains(sw2303::registers::SystemStatus0Flags::PASS_TRANSISTOR_OPEN)
                );
            }
            if let Some(status1) = sw2303_system_status1 {
                info!(
                    "SW2303 Status1: VIN_25V={}, OCP_112={}, DIE_TEMP={}, CC_LOOP={}, VIN_OVP={}, VIN_UVP={}",
                    status1.contains(sw2303::registers::SystemStatus1Flags::VIN_OVER_25V),
                    status1
                        .contains(sw2303::registers::SystemStatus1Flags::OVERCURRENT_112_5_PERCENT),
                    status1.contains(sw2303::registers::SystemStatus1Flags::DIE_OVERTEMPERATURE),
                    status1.contains(sw2303::registers::SystemStatus1Flags::CC_LOOP_OPEN),
                    status1.contains(sw2303::registers::SystemStatus1Flags::VIN_OVERVOLTAGE),
                    status1.contains(sw2303::registers::SystemStatus1Flags::VIN_UNDERVOLTAGE)
                );
            }
            if let Some(status2) = sw2303_system_status2 {
                info!(
                    "SW2303 Status2: CC1_OVP={}, CC2_OVP={}, DP_OVP={}",
                    status2.contains(sw2303::registers::SystemStatus2Flags::CC1_OVERVOLTAGE),
                    status2.contains(sw2303::registers::SystemStatus2Flags::CC2_OVERVOLTAGE),
                    status2.contains(sw2303::registers::SystemStatus2Flags::DP_OVERVOLTAGE)
                );
            }
            unsafe {
                LAST_STATUS_LOG_TIME = current_time;
            }
        }

        // Check overcurrent/fault conditions for all ports
        // Port 1: SW2303 overcurrent detection
        let port1_overcurrent = match hardware.sw2303_controller.is_overcurrent().await {
            Ok(overcurrent) => overcurrent,
            Err(e) => {
                error!("Failed to read SW2303 overcurrent status: {:?}", e);
                false
            }
        };

        // Port 2: TPS25810 fault signal via TCA6424 P06 (Low Active)
        let p2_fault_state = hardware
            .tca6424_expander
            .get_pin_input_state(Pin::P06)
            .await
            .unwrap();
        let port2_overcurrent = p2_fault_state == PinState::Low;

        // Port 3: TPS25810 fault signal via TCA6424 P20 (Low Active)
        let p3_fault_state = hardware
            .tca6424_expander
            .get_pin_input_state(Pin::P20)
            .await
            .unwrap();
        let port3_overcurrent = p3_fault_state == PinState::Low;

        // Check for new overcurrent events and trigger alarm beep
        let current_overcurrent_status = [port1_overcurrent, port2_overcurrent, port3_overcurrent];
        for (port_idx, &is_overcurrent) in current_overcurrent_status.iter().enumerate() {
            if is_overcurrent && !previous_overcurrent_status[port_idx] {
                // New overcurrent event detected - play alarm beep
                info!(
                    "Overcurrent detected on Port {}, triggering alarm beep",
                    port_idx + 1
                );
                play_alarm_beep(&mut hardware.buzzer_pwm).await;
            } else if !is_overcurrent && previous_overcurrent_status[port_idx] {
                // Overcurrent cleared
                info!("Overcurrent cleared on Port {}", port_idx + 1);
            }
        }
        previous_overcurrent_status = current_overcurrent_status;

        // Check for UFP status changes and trigger buzzer
        if sw2303_port1_connected != prev_port1_connected {
            info!(
                "Port 1 UFP status changed: {} -> {}",
                prev_port1_connected, sw2303_port1_connected
            );
            beep_buzzer(&mut hardware.buzzer_pwm, 200).await; // 200ms beep
            embassy_time::Timer::after_millis(10).await; // Small delay after beep
            prev_port1_connected = sw2303_port1_connected;
        }

        if port2_connected != prev_port2_connected {
            info!(
                "Port 2 UFP status changed: {} -> {}",
                prev_port2_connected, port2_connected
            );
            beep_buzzer(&mut hardware.buzzer_pwm, 200).await; // 200ms beep
            embassy_time::Timer::after_millis(10).await; // Small delay after beep
            prev_port2_connected = port2_connected;
        }

        if port3_connected != prev_port3_connected {
            info!(
                "Port 3 UFP status changed: {} -> {}",
                prev_port3_connected, port3_connected
            );
            beep_buzzer(&mut hardware.buzzer_pwm, 200).await; // 200ms beep
            embassy_time::Timer::after_millis(10).await; // Small delay after beep
            prev_port3_connected = port3_connected;
        }

        // Note: We now use real-time power consumption from INA226 sensors
        // instead of SW2303 negotiated power for dynamic allocation

        // Check for connection status changes and trigger power allocation recalculation
        let current_connections = [sw2303_port1_connected, port2_connected, port3_connected];
        static mut PREV_CONNECTIONS: [bool; 3] = [false, false, false];
        let prev_connections = unsafe { PREV_CONNECTIONS };

        let connections_changed = current_connections != prev_connections;
        if connections_changed {
            info!(
                "Connections: P1={}, P2={}, P3={}",
                current_connections[0], current_connections[1], current_connections[2]
            );
            unsafe {
                PREV_CONNECTIONS = current_connections;
            }
        }

        // Update connection status and apply dynamic power allocation
        dashboard.update_connection_status(current_connections);
        let power_allocation = dashboard.get_power_allocation();

        // Apply power allocation to hardware (only when connections change or every 10 seconds)
        static mut LAST_APPLY_TIME: u64 = 0;
        let current_time = embassy_time::Instant::now().as_millis();
        let should_apply =
            connections_changed || (current_time - unsafe { LAST_APPLY_TIME } > 10000);

        if should_apply {
            if connections_changed {
                info!("Applying power allocation due to connection change");
            } else {
                info!("Applying power allocation (periodic update)");
            }

            match crate::hardware::apply_power_allocation(
                &mut hardware.sw2303_controller,
                &mut hardware.tca6424_expander,
                power_allocation,
            )
            .await
            {
                Ok(_) => {
                    info!("Power allocation applied successfully");
                    unsafe {
                        LAST_APPLY_TIME = current_time;
                    }
                }
                Err(_e) => {
                    error!("Failed to apply power allocation to hardware");
                }
            }
        }

        // Prepare data for Dashboard, converting f64 to f32
        let sensor_data = [
            ((voltage1 / 1000.0) as f32, current1 as f32, power1 as f32),
            ((voltage2 / 1000.0) as f32, current2 as f32, power2 as f32),
            ((voltage3 / 1000.0) as f32, current3 as f32, power3 as f32),
        ];

        // Prepare connection status for Dashboard
        let connection_status = [sw2303_port1_connected, port2_connected, port3_connected];

        // Prepare overcurrent status for Dashboard
        let overcurrent_status = [port1_overcurrent, port2_overcurrent, port3_overcurrent];

        // Update Dashboard data
        dashboard.update_data(sensor_data, connection_status, overcurrent_status);

        // Update SW2303 system status for Port 1 anomaly detection
        dashboard.update_sw2303_status(
            sw2303_system_status0,
            sw2303_system_status1,
            sw2303_system_status2,
        );

        // Handle joystick input with software debouncing
        let current_time_ms = embassy_time::Instant::now().as_millis();
        let pressed_buttons = hardware
            .joystick_debouncer
            .update(&hardware.joystick, current_time_ms);

        // Process debounced button presses
        for button in pressed_buttons {
            match button {
                crate::hardware::JoystickButton::Left => {
                    let current_port = dashboard.get_selected_port();
                    if current_port > 0 {
                        dashboard.set_selected_port(current_port - 1);
                        info!("Joystick: LEFT pressed - Selected Port {}", current_port);
                        beep_buzzer(&mut hardware.buzzer_pwm, 100).await;
                    }
                }
                crate::hardware::JoystickButton::Right => {
                    let current_port = dashboard.get_selected_port();
                    if current_port < 2 {
                        dashboard.set_selected_port(current_port + 1);
                        info!(
                            "Joystick: RIGHT pressed - Selected Port {}",
                            current_port + 2
                        );
                        beep_buzzer(&mut hardware.buzzer_pwm, 100).await;
                    }
                }
                crate::hardware::JoystickButton::Down => {
                    dashboard.toggle_display_mode();
                    let mode = if dashboard.is_showing_power_allocation() {
                        "Power Allocation"
                    } else {
                        "Power"
                    };
                    info!("Joystick: DOWN pressed - Switched to {} display", mode);
                    beep_buzzer(&mut hardware.buzzer_pwm, 100).await;
                }
                crate::hardware::JoystickButton::Center => {
                    // Center button pressed - disable USB communication for selected port
                    let selected_port = dashboard.get_selected_port() + 1; // Convert to 1-based port number
                    let selected_port_index = dashboard.get_selected_port(); // 0-based index for dashboard
                    match crate::hardware::control_usb_communication(
                        &mut hardware.tca6424_expander,
                        selected_port as u8,
                        false,
                    )
                    .await
                    {
                        Ok(_) => {
                            usb_comm_disabled = true;
                            dashboard.set_usb_communication(selected_port_index, false); // Update dashboard state
                            info!(
                                "Joystick: CENTER pressed - USB communication disabled for Port {}",
                                selected_port
                            );
                            beep_buzzer(&mut hardware.buzzer_pwm, 200).await; // Different beep for disconnect
                        }
                        Err(e) => {
                            error!(
                                "Failed to disable USB communication for Port {}: {:?}",
                                selected_port, e
                            );
                        }
                    }
                }
                _ => {} // Handle other buttons if needed
            }
        }

        // Handle CENTER button release - restore USB communication if disabled
        let center_currently_pressed = hardware
            .joystick_debouncer
            .get_button_state(crate::hardware::JoystickButton::Center);
        if !center_currently_pressed && usb_comm_disabled {
            // Center button released - re-enable USB communication for selected port
            let selected_port = dashboard.get_selected_port() + 1; // Convert to 1-based port number
            let selected_port_index = dashboard.get_selected_port(); // 0-based index for dashboard
            match crate::hardware::control_usb_communication(
                &mut hardware.tca6424_expander,
                selected_port as u8,
                true,
            )
            .await
            {
                Ok(_) => {
                    usb_comm_disabled = false;
                    dashboard.set_usb_communication(selected_port_index, true); // Update dashboard state
                    info!(
                        "Joystick: CENTER released - USB communication restored for Port {}",
                        selected_port
                    );
                    beep_buzzer(&mut hardware.buzzer_pwm, 100).await; // Normal beep for reconnect
                }
                Err(e) => {
                    error!(
                        "Failed to restore USB communication for Port {}: {:?}",
                        selected_port, e
                    );
                }
            }
        }

        // Draw Dashboard directly to the display
        dashboard.draw(&mut hardware.display).await.unwrap();

        // Feed the watchdog to prevent system reset
        hardware.watchdog.pet();

        // Wait for 50ms before the next update (improved responsiveness for joystick)
        embassy_time::Timer::after_millis(50).await;
    }
}
