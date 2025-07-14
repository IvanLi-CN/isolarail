// src/display/dashboard.rs
// Dashboard 页面模块

// Keep Rgb565 for colors
use embedded_graphics::pixelcolor::Rgb565;
// Remove other embedded_graphics imports as drawing primitives won't be used directly
use embedded_graphics::prelude::RgbColor;
// Removed: use embedded_graphics::prelude::*; // Unused import
use defmt::info;
// use embedded_graphics::{
//     mono_font::{ascii::FONT_6X10, MonoTextStyle},
//     prelude::*,
//     text::{Alignment, Text},
//     geometry::Point,
// };

use crate::display::font::{
    FONT_8X12_HEIGHT, FONT_8X12_WIDTH, char_to_mono_bitmap, mono_bitmap_to_rgb565,
};
use gc9d01::GC9D01; // Import GC9D01 // Updated constant names

// Import necessary traits for GC9D01 (these are bounds on the GC9D01 struct itself)
// Removed alloc::format import to avoid dynamic allocation
use core::convert::TryInto; // Added import for try_into
use embedded_hal::digital::OutputPin;
use embedded_hal_async::spi::SpiDevice;
use gc9d01::Timer as Gc9d01Timer; // Moved this import up // Added import for alloc::format!
use sw2303::registers::{SystemStatus0Flags, SystemStatus1Flags, SystemStatus2Flags};

#[derive(Debug)]
pub enum Error {
    // Add specific error types later if needed
    DriverError, // Placeholder for errors from the GC9D01 driver
                 // Add other errors like FontError, LayoutError, etc. as needed
}

// Define colors
const COLOR_VOLTAGE: Rgb565 = Rgb565::YELLOW;
const COLOR_CURRENT: Rgb565 = Rgb565::RED;
const COLOR_POWER: Rgb565 = Rgb565::GREEN;
const COLOR_GRAY: Rgb565 = Rgb565::new(15, 30, 15); // 定义灰色

// Dashboard struct, contains data to display
pub struct Dashboard {
    // Data for 3 USB ports: (voltage, current, power)
    port_data: [(f32, f32, f32); 3],
    // Connection status for each port (true if connected, false if not)
    port_connected: [bool; 3],
    // Overcurrent status for each port (true if overcurrent detected, false if normal)
    port_overcurrent: [bool; 3],
    // USB communication status for each port (true if enabled, false if disabled)
    usb_comm_enabled: [bool; 3],
    // Power allocation data: All ports in Watts
    power_allocation: [f32; 3],
    // Total power budget in Watts
    total_power_budget: f32,
    // Display mode: false = show power (row 3), true = show power allocation (row 4)
    show_power_allocation: bool,
    // Counter for draw calls to control screen clearing frequency
    draw_count: u32,
    // Currently selected port (0, 1, or 2), default is 1 (Port 2)
    selected_port: usize,
    // Previous selected port for background clearing
    previous_selected_port: Option<usize>,
    // SW2303 system status for Port 1 anomaly detection
    sw2303_system_status0: Option<SystemStatus0Flags>,
    sw2303_system_status1: Option<SystemStatus1Flags>,
    sw2303_system_status2: Option<SystemStatus2Flags>,
}

impl Dashboard {
    // Create new Dashboard instance
    pub fn new() -> Self {
        // Read total power budget from environment variable, default to 100W
        // This reads the compile-time environment variable
        let total_power_budget = option_env!("TOTAL_POWER_BUDGET")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(100.0);

        info!(
            "Dashboard initialized with TOTAL_POWER_BUDGET={}W",
            total_power_budget
        );

        let mut dashboard = Self {
            port_data: [(0.0, 0.0, 0.0); 3],
            port_connected: [false; 3], // Initialize all ports as disconnected
            port_overcurrent: [false; 3], // Initialize all ports as normal (no overcurrent)
            usb_comm_enabled: [true; 3], // Initialize all ports with USB communication enabled
            power_allocation: [0.0, 0.0, 0.0], // Will be calculated dynamically
            total_power_budget,
            show_power_allocation: false, // Default to showing power (row 3)
            draw_count: 0,                // Initialize draw counter
            selected_port: 1,             // Default to Port 2 (index 1)
            previous_selected_port: None, // No previous selection initially
            sw2303_system_status0: None,  // Initialize SW2303 system status
            sw2303_system_status1: None,
            sw2303_system_status2: None,
        };

        // Calculate initial power allocation
        dashboard.calculate_power_allocation();
        dashboard
    }

    // Update Dashboard display data for 3 ports: [(V1, A1, W1), (V2, A2, W2), (V3, A3, W3)]
    pub fn update_data(
        &mut self,
        data: [(f32, f32, f32); 3],
        connection_status: [bool; 3],
        overcurrent_status: [bool; 3],
    ) {
        // Check if connection status changed
        let connection_changed = self.port_connected != connection_status;

        // Check if Port 1 power changed significantly (>1W difference)
        let port1_power_changed = (data[0].2 - self.port_data[0].2).abs() > 1.0;

        self.port_data = data;
        self.port_connected = connection_status;
        self.port_overcurrent = overcurrent_status;

        // Recalculate power allocation if connection status or Port 1 power changed
        if connection_changed || port1_power_changed {
            self.calculate_power_allocation();
        }
    }

    // Update SW2303 system status for Port 1 anomaly detection
    pub fn update_sw2303_status(
        &mut self,
        status0: Option<SystemStatus0Flags>,
        status1: Option<SystemStatus1Flags>,
        status2: Option<SystemStatus2Flags>,
    ) {
        self.sw2303_system_status0 = status0;
        self.sw2303_system_status1 = status1;
        self.sw2303_system_status2 = status2;
    }

    // Get SW2303 anomaly text and color for display - ONLY real anomalies from official docs
    // Returns (text, color) if anomaly detected, None if normal
    fn get_sw2303_anomaly_text(&self) -> Option<(&'static str, Rgb565)> {
        // Only check for overcurrent detection as explicitly requested by user
        if let Some(status1) = self.sw2303_system_status1 {
            if status1.contains(SystemStatus1Flags::OVERCURRENT_112_5_PERCENT) {
                defmt::warn!("SW2303 Port 1 Anomaly: Overcurrent 112.5% detected");
                return Some(("OCP", Rgb565::RED)); // 过流保护
            }
        }

        None // No anomaly detected
    }

    // Set the selected port (0, 1, or 2)
    pub fn set_selected_port(&mut self, port: usize) {
        if port < 3 {
            self.selected_port = port;
        }
    }

    // Get the currently selected port
    pub fn get_selected_port(&self) -> usize {
        self.selected_port
    }

    // Update USB communication status for a specific port
    pub fn set_usb_communication(&mut self, port_index: usize, enabled: bool) {
        if port_index < 3 {
            self.usb_comm_enabled[port_index] = enabled;
        }
    }

    // Toggle display mode between power and power allocation
    pub fn toggle_display_mode(&mut self) {
        self.show_power_allocation = !self.show_power_allocation;
    }

    // Get current display mode
    pub fn is_showing_power_allocation(&self) -> bool {
        self.show_power_allocation
    }

    // Update connection status for all ports and recalculate allocation
    pub fn update_connection_status(&mut self, connected: [bool; 3]) {
        self.port_connected = connected;
        self.calculate_power_allocation();
    }

    // Get current power allocation
    pub fn get_power_allocation(&self) -> [f32; 3] {
        self.power_allocation
    }

    // Calculate dynamic power allocation based on Port 1 real-time power consumption
    fn calculate_power_allocation(&mut self) {
        const P23_HIGH_POWER: f32 = 25.0; // P2+P3 high power allocation
        const P23_LOW_POWER: f32 = 15.0; // P2+P3 low power allocation

        // Track previous allocation to avoid duplicate logs
        static mut PREV_ALLOCATION: [f32; 3] = [0.0, 0.0, 0.0];
        static mut PREV_CONNECTIONS: [bool; 3] = [false, false, false];

        let _connections_changed = unsafe { self.port_connected != PREV_CONNECTIONS };

        // Get Port 1 real-time power consumption (from INA226 sensor)
        let p1_real_power = self.port_data[0].2; // Power is the third element (V, A, W)

        // Reset allocations
        self.power_allocation = [0.0, 0.0, 0.0];

        if !self.port_connected[0] {
            // Port 1 not connected - allocate 0W to port 1
            self.power_allocation[0] = 0.0;

            // P2 and P3 can use full budget
            let p23_available = self.total_power_budget;
            self.allocate_p23_power(p23_available);
        } else {
            // Port 1 is connected - use real-time power consumption
            // 用户策略：根据 Port 1 实时功率动态计算 P2+P3 功率

            // 计算剩余功率：Pt - P1（P1 是实时功率）
            let remaining_power = self.total_power_budget - p1_real_power;

            // 用户策略实现：功率限制设置
            // 如果 Pt - P1_real > 25W：P23 限制 = 25W
            // 如果 Pt - P1_real <= 25W：P23 限制 = 15W（P2 和 P3 各自限制 7.5W）

            let p23_limit = if remaining_power > P23_HIGH_POWER {
                P23_HIGH_POWER // P23 限制 = 25W
            } else {
                P23_LOW_POWER // P23 限制 = 15W
            };

            // P1 限制根据 P23 限制计算
            let p1_power_limit = (self.total_power_budget - p23_limit).max(0.0);

            // 存储功率限制值（用于显示）
            self.power_allocation[0] = p1_power_limit;

            // 分配 P23 功率限制
            self.allocate_p23_power(p23_limit);
        }

        // Update previous allocation tracking
        unsafe {
            PREV_ALLOCATION = self.power_allocation;
        }
    }

    // Helper method to allocate power between P2 and P3
    fn allocate_p23_power(&mut self, p23_total: f32) {
        const HIGH_CURRENT_POWER: f32 = 15.0; // 3A * 5V = 15W
        const LOW_CURRENT_POWER: f32 = 7.5; // 1.5A * 5V = 7.5W
        const MIN_STANDBY_POWER: f32 = 7.5; // Minimum power allocation for standby

        // Power allocation should always reserve capacity for both ports
        // regardless of current connection status, as this is power limit allocation

        if p23_total >= HIGH_CURRENT_POWER + LOW_CURRENT_POWER {
            // Enough power for both ports at full capacity
            // Port 2: 3A (15W), Port 3: 1.5A (7.5W)
            self.power_allocation[1] = HIGH_CURRENT_POWER;
            self.power_allocation[2] = LOW_CURRENT_POWER;
        } else if p23_total >= LOW_CURRENT_POWER * 2.0 {
            // Enough for both ports at reduced capacity
            // Prioritize Port 2 with higher allocation
            let remaining_after_p3 = p23_total - LOW_CURRENT_POWER;
            self.power_allocation[1] = remaining_after_p3.min(HIGH_CURRENT_POWER);
            self.power_allocation[2] = LOW_CURRENT_POWER;
        } else {
            // Limited power - split equally but ensure minimum allocation
            let per_port = (p23_total / 2.0).max(MIN_STANDBY_POWER / 2.0);
            self.power_allocation[1] = per_port;
            self.power_allocation[2] = per_port;
        }
    }

    // Draw Dashboard directly to the display driver using write_area
    // Accept GC9D01 directly
    pub async fn draw<'a, BUS, DC, RST, TIMER>(
        &mut self,
        display: &mut GC9D01<'a, BUS, DC, RST, TIMER>,
    ) -> Result<(), Error>
    where
        BUS: SpiDevice,
        DC: OutputPin<Error = core::convert::Infallible>, // Specify Infallible error type
        RST: OutputPin<Error = core::convert::Infallible>, // Specify Infallible error type
        TIMER: Gc9d01Timer,
    {
        // Clear screen manually by writing black pixels to the whole area
        let screen_width = 160; // Assuming landscape 160x40
        let screen_height = 40;
        let _black_pixel = Rgb565::BLUE;
        // Create a buffer for a 20x20 block of black pixels
        // const BLOCK_SIZE: u16 = 20; // Removed unused constant

        // Clear screen manually by writing black pixels to the whole area
        // Only clear every 1000 draws to save resources
        if self.draw_count % 1000 == 0 {
            display.fill_color(Rgb565::BLACK);
            let _ = display.flush().await;
            // Handle potential remaining rows/columns if screen dimensions are not multiples of BLOCK_SIZE
            // (Assuming 160x40 is a multiple of 20x20, so no extra handling needed for this specific case)
        }
        self.draw_count += 1;
        // (Assuming 160x40 is a multiple of 20x20, so no extra handling needed for this specific case)

        // Layout: 3 columns, hardcoded positions to avoid overlap
        // Screen width: 160 pixels, divide into 3 columns with proper spacing
        let col_positions = [
            (0, 50),    // Port 1: x=0 to x=50 (50 pixels wide)
            (55, 105),  // Port 2: x=55 to x=105 (50 pixels wide)
            (110, 160), // Port 3: x=110 to x=160 (50 pixels wide)
        ];
        let _row_height = screen_height / 3; // Approx 13 // Mark as unused
        let row_spacing = 1; // Additional spacing between rows
        let actual_row_height = FONT_8X12_HEIGHT + row_spacing; // 12 + 1 = 13

        // Buffer for character pixels (8x12)
        let mut char_pixel_buffer = [Rgb565::BLACK; FONT_8X12_WIDTH * FONT_8X12_HEIGHT]; // Updated constant names

        // Helper function to draw a string with fixed width using space padding
        async fn draw_string<'a, BUS, DC, RST, TIMER>(
            display: &mut GC9D01<'a, BUS, DC, RST, TIMER>,
            s: &str,
            right_edge_x: usize,      // Right edge of the drawing area
            fixed_width_chars: usize, // Fixed width in characters (e.g., 6 for "12.34V")
            start_y: usize,
            fg_color: Rgb565,
            bg_color: Rgb565,
            char_pixel_buffer: &mut [Rgb565], // Pass buffer as argument
        ) -> Result<(), Error>
        where
            BUS: SpiDevice,
            DC: OutputPin<Error = core::convert::Infallible>, // Specify Infallible error type
            RST: OutputPin<Error = core::convert::Infallible>, // Specify Infallible error type
            TIMER: Gc9d01Timer,
        {
            // Create a fixed-width string by padding with spaces using fixed buffer
            let mut padded_chars: [char; 16] = [' '; 16]; // Max 16 characters should be enough
            let string_len = s.chars().count();
            let actual_width = fixed_width_chars.min(16); // Limit to buffer size

            if string_len < actual_width {
                // Pad with leading spaces for right alignment
                let spaces_needed = actual_width - string_len;
                let mut char_iter = s.chars();
                for i in 0..actual_width {
                    if i < spaces_needed {
                        padded_chars[i] = ' ';
                    } else {
                        padded_chars[i] = char_iter.next().unwrap_or(' ');
                    }
                }
            } else {
                // If string is too long, truncate it
                let mut char_iter = s.chars();
                for i in 0..actual_width {
                    padded_chars[i] = char_iter.next().unwrap_or(' ');
                }
            }

            // Calculate dimensions for background rectangle
            let total_pixel_width = fixed_width_chars * FONT_8X12_WIDTH;
            let start_x = right_edge_x.saturating_sub(total_pixel_width);

            // Draw the text
            let mut current_x = start_x;
            for c in &padded_chars[..actual_width] {
                // First fill the character buffer with background color
                for pixel in char_pixel_buffer.iter_mut() {
                    *pixel = bg_color;
                }

                if let Some(bitmap) = char_to_mono_bitmap(*c) {
                    mono_bitmap_to_rgb565(bitmap, fg_color, char_pixel_buffer);

                    let x0 = current_x;
                    let y0 = start_y;

                    display.write_area(
                        x0.try_into().unwrap(),
                        y0.try_into().unwrap(),
                        FONT_8X12_WIDTH.try_into().unwrap(),
                        FONT_8X12_HEIGHT.try_into().unwrap(),
                        char_pixel_buffer,
                    );

                    current_x += FONT_8X12_WIDTH;
                } else {
                    // Handle characters not in font (draw a blank space)
                    // Background is already filled, no need to render anything for blank space
                    let x0 = current_x;
                    let y0 = start_y;

                    display.write_area(
                        x0.try_into().unwrap(),
                        y0.try_into().unwrap(),
                        FONT_8X12_WIDTH.try_into().unwrap(),
                        FONT_8X12_HEIGHT.try_into().unwrap(),
                        char_pixel_buffer,
                    );

                    current_x += FONT_8X12_WIDTH;
                }
            }
            Ok(())
        }

        // Only clear previous selection background if selection has changed
        if let Some(prev_selected) = self.previous_selected_port {
            if prev_selected != self.selected_port {
                // Clear the previous selection's background area
                let (prev_col_start_x, prev_col_end_x) = col_positions[prev_selected];
                let prev_bg_x = (prev_col_start_x as i32 - 4).max(0) as u16;
                let prev_bg_y = 0u16.saturating_sub(4);
                let prev_col_width = prev_col_end_x - prev_col_start_x;
                let prev_bg_width =
                    (prev_col_width as u16 + 8).min(screen_width as u16 - prev_bg_x);
                let prev_bg_height = screen_height as u16 + 8;

                // Clear previous background in chunks to avoid memory issues
                // Use very small fixed buffer to minimize memory usage
                const MAX_CLEAR_PIXELS: usize = 64; // Only 64 pixels = 128 bytes
                let clear_buffer: [Rgb565; MAX_CLEAR_PIXELS] = [Rgb565::BLACK; MAX_CLEAR_PIXELS];

                // Clear in very small chunks to minimize memory usage
                // Calculate how many pixels we can clear per row with our small buffer
                let pixels_per_row = prev_bg_width as usize;
                let rows_per_chunk = MAX_CLEAR_PIXELS / pixels_per_row.max(1);
                let chunk_height = rows_per_chunk.max(1) as u16;

                for y_chunk in (0..prev_bg_height).step_by(chunk_height as usize) {
                    let remaining_height = (prev_bg_height - y_chunk).min(chunk_height);
                    let chunk_pixels =
                        (prev_bg_width as usize * remaining_height as usize).min(MAX_CLEAR_PIXELS);

                    display.write_area(
                        prev_bg_x,
                        prev_bg_y + y_chunk,
                        prev_bg_width,
                        remaining_height,
                        &clear_buffer[..chunk_pixels],
                    );
                }
            }
        }

        // Update previous selection tracking
        self.previous_selected_port = Some(self.selected_port);

        // Draw values
        let mut buffer: [u8; 10] = [0; 10]; // Buffer for float to string conversion

        // Draw data for each port (column)
        for i in 0..3 {
            let (_col_start_x, col_end_x) = col_positions[i];
            let col_right_edge_x = col_end_x;

            let port_voltage = self.port_data[i].0;
            let port_current = self.port_data[i].1;
            let port_power = self.port_data[i].2;

            // Note: Dynamic color logic based on voltage/current/power values is no longer used.
            // All ports now use connection-based fixed colors for consistency.

            // Determine final colors based on connection status for all ports
            let (final_voltage_color, final_current_color, final_power_color) =
                if self.port_connected[i] {
                    // If port is connected, use fixed colors: voltage yellow, current red, power green
                    (COLOR_VOLTAGE, COLOR_CURRENT, COLOR_POWER)
                } else {
                    // If port is not connected, use gray for all
                    (COLOR_GRAY, COLOR_GRAY, COLOR_GRAY)
                };

            // Determine text background color based on selection status
            let text_bg_color = if i == self.selected_port {
                Rgb565::new(8, 8, 16) // Dark blue background for selected port
            } else {
                Rgb565::BLACK // Black background for non-selected ports
            };

            // Draw column background if this port is selected
            if i == self.selected_port {
                let (col_start_x, col_end_x) = col_positions[i];
                let selection_color = Rgb565::new(8, 8, 16); // Dark blue background for selection

                // Calculate column background dimensions with 4-pixel padding
                let bg_x = (col_start_x as i32 - 4).max(0) as u16;
                let bg_y = 0u16.saturating_sub(4);
                let col_width = col_end_x - col_start_x;
                let bg_width = (col_width as u16 + 8).min(screen_width as u16 - bg_x); // 4px padding on each side
                let bg_height = screen_height as u16 + 8; // 4px padding top and bottom

                // Draw simple rectangle background without rounded corners to save memory
                // Use very small buffer for line-by-line drawing
                const LINE_PIXELS: usize = 64; // Max 64 pixels per line = 128 bytes
                let mut line_buffer: [Rgb565; LINE_PIXELS] = [selection_color; LINE_PIXELS];

                let lines_per_chunk = LINE_PIXELS / (bg_width as usize).max(1);
                let chunk_height = lines_per_chunk.max(1) as u16;

                for y_offset in (0..bg_height).step_by(chunk_height as usize) {
                    let remaining_height = (bg_height - y_offset).min(chunk_height);
                    let pixels_needed =
                        (bg_width as usize * remaining_height as usize).min(LINE_PIXELS);

                    // Fill buffer with selection color
                    for pixel in &mut line_buffer[..pixels_needed] {
                        *pixel = selection_color;
                    }

                    display.write_area(
                        bg_x,
                        bg_y + y_offset,
                        bg_width,
                        remaining_height,
                        &line_buffer[..pixels_needed],
                    );
                }
            }

            // Draw Voltage (Row 1) - Fixed width of 6 characters (e.g., "12.34V")
            let voltage_str = self.float_with_unit_to_string(&mut buffer, port_voltage, 'V');
            draw_string(
                display,
                voltage_str,
                col_right_edge_x as usize,
                6,
                0,
                final_voltage_color,
                text_bg_color,
                &mut char_pixel_buffer,
            )
            .await?;

            // Draw Current (Row 2) - Fixed width of 6 characters (e.g., "12.34A")
            let current_str = self.float_with_unit_to_string(&mut buffer, port_current, 'A');
            draw_string(
                display,
                current_str,
                col_right_edge_x as usize,
                6,
                actual_row_height as usize,
                final_current_color,
                text_bg_color,
                &mut char_pixel_buffer,
            )
            .await?;

            // Draw Row 3 - Show power allocation or power based on display mode
            if self.show_power_allocation {
                // Display power allocation (Row 4 data) - All in Watts with white color
                let allocation_str =
                    self.float_with_unit_to_string(&mut buffer, self.power_allocation[i], 'W');
                draw_string(
                    display,
                    allocation_str,
                    col_right_edge_x as usize,
                    6,
                    (actual_row_height * 2) as usize,
                    Rgb565::WHITE,
                    text_bg_color,
                    &mut char_pixel_buffer,
                )
                .await?;
            } else {
                // Display power (Row 3 data) - Show status based on priority: USB comm > SW2303 anomalies > OCP > power value
                if !self.usb_comm_enabled[i] {
                    // Display "DISC" in orange when USB communication is disabled
                    draw_string(
                        display,
                        "DISC",
                        col_right_edge_x as usize,
                        6,
                        (actual_row_height * 2) as usize,
                        Rgb565::new(31, 32, 0),
                        text_bg_color,
                        &mut char_pixel_buffer,
                    )
                    .await?; // Orange color
                } else if i == 0 {
                    // Port 1: Check SW2303 system status for anomalies
                    let sw2303_anomaly = self.get_sw2303_anomaly_text();
                    if let Some((anomaly_text, anomaly_color)) = sw2303_anomaly {
                        draw_string(
                            display,
                            anomaly_text,
                            col_right_edge_x as usize,
                            6,
                            (actual_row_height * 2) as usize,
                            anomaly_color,
                            text_bg_color,
                            &mut char_pixel_buffer,
                        )
                        .await?;
                    } else if self.port_overcurrent[i] {
                        // Display "OCP" in red when overcurrent is detected
                        draw_string(
                            display,
                            "OCP",
                            col_right_edge_x as usize,
                            6,
                            (actual_row_height * 2) as usize,
                            Rgb565::RED,
                            text_bg_color,
                            &mut char_pixel_buffer,
                        )
                        .await?;
                    } else {
                        // Display normal power value
                        let power_str =
                            self.float_with_unit_to_string(&mut buffer, port_power, 'W');
                        draw_string(
                            display,
                            power_str,
                            col_right_edge_x as usize,
                            6,
                            (actual_row_height * 2) as usize,
                            final_power_color,
                            text_bg_color,
                            &mut char_pixel_buffer,
                        )
                        .await?;
                    }
                } else if self.port_overcurrent[i] {
                    // Display "OCP" in red when overcurrent is detected (for ports 2 and 3)
                    draw_string(
                        display,
                        "OCP",
                        col_right_edge_x as usize,
                        6,
                        (actual_row_height * 2) as usize,
                        Rgb565::RED,
                        text_bg_color,
                        &mut char_pixel_buffer,
                    )
                    .await?;
                } else {
                    // Display normal power value (for ports 2 and 3)
                    let power_str = self.float_with_unit_to_string(&mut buffer, port_power, 'W');
                    draw_string(
                        display,
                        power_str,
                        col_right_edge_x as usize,
                        6,
                        (actual_row_height * 2) as usize,
                        final_power_color,
                        text_bg_color,
                        &mut char_pixel_buffer,
                    )
                    .await?;
                }
            }
        }

        // Flush all changes to the display
        display.flush().await.map_err(|_| Error::DriverError)?;

        Ok(())
    }

    // Helper function to create float string with unit suffix without dynamic allocation
    fn float_with_unit_to_string<'a>(
        &self,
        buffer: &'a mut [u8],
        value: f32,
        unit: char,
    ) -> &'a str {
        // First convert float to string manually
        let integer_part = value as i32;
        let decimal_part = ((value - integer_part as f32).abs() * 100.0) as i32;

        let mut cursor = 0;
        if value < 0.0 {
            buffer[cursor] = b'-';
            cursor += 1;
        }
        let mut temp = integer_part.abs();
        let mut divisor = 1;
        while divisor * 10 <= temp {
            divisor *= 10;
        }
        while divisor > 0 {
            buffer[cursor] = b'0' + (temp / divisor) as u8;
            cursor += 1;
            temp %= divisor;
            divisor /= 10;
        }
        if integer_part == 0 && value.abs() < 1.0 && value >= 0.0 {
            buffer[cursor] = b'0';
            cursor += 1;
        } else if integer_part == 0 && value.abs() < 1.0 && value < 0.0 && cursor == 1 {
            buffer[cursor] = b'0';
            cursor += 1;
        }

        buffer[cursor] = b'.';
        cursor += 1;

        buffer[cursor] = b'0' + (decimal_part / 10) as u8;
        cursor += 1;
        buffer[cursor] = b'0' + (decimal_part % 10) as u8;
        cursor += 1;

        // Append unit
        if cursor < buffer.len() {
            buffer[cursor] = unit as u8;
            cursor += 1;
        }

        core::str::from_utf8(&buffer[..cursor]).unwrap_or("Err")
    }

    // Simplified float to string function (moved from inside draw)
    fn float_to_string<'a>(&self, buffer: &'a mut [u8], value: f32) -> &'a str {
        // Added &self and value parameter and lifetime
        // This is a very simplified implementation for demonstration only
        // Does not handle negative numbers, large numbers, or specific precision well
        let integer_part = value as i32;
        let decimal_part = ((value - integer_part as f32).abs() * 100.0) as i32; // Get two decimal places, handle negative input

        let mut cursor = 0;
        if value < 0.0 {
            buffer[cursor] = b'-';
            cursor += 1;
        }
        let mut temp = integer_part.abs();
        let mut divisor = 1;
        while divisor * 10 <= temp {
            divisor *= 10;
        }
        while divisor > 0 {
            buffer[cursor] = b'0' + (temp / divisor) as u8;
            cursor += 1;
            temp %= divisor;
            divisor /= 10;
        }
        if integer_part == 0 && value.abs() < 1.0 && value >= 0.0 {
            // Handle 0.x case
            buffer[cursor] = b'0';
            cursor += 1;
        } else if integer_part == 0 && value.abs() < 1.0 && value < 0.0 && cursor == 1 {
            // Handle -0.x case
            buffer[cursor] = b'0';
            cursor += 1;
        }

        buffer[cursor] = b'.';
        cursor += 1;

        buffer[cursor] = b'0' + (decimal_part / 10) as u8;
        cursor += 1;
        buffer[cursor] = b'0' + (decimal_part % 10) as u8;
        cursor += 1;

        core::str::from_utf8(&buffer[..cursor]).unwrap_or("Err")
    }
}
