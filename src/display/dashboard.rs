// src/display/dashboard.rs
// Dashboard 页面模块

// Keep Rgb565 for colors
use embedded_graphics::pixelcolor::Rgb565;
// Remove other embedded_graphics imports as drawing primitives won't be used directly
use embedded_graphics::prelude::RgbColor;
// Removed: use embedded_graphics::prelude::*; // Unused import
// use embedded_graphics::{
//     mono_font::{ascii::FONT_6X10, MonoTextStyle},
//     prelude::*,
//     text::{Alignment, Text},
//     geometry::Point,
// };

use gc9d01::GC9D01; // Import GC9D01
use crate::display::font::{char_to_mono_bitmap, mono_bitmap_to_rgb565, FONT_8X12_WIDTH, FONT_8X12_HEIGHT}; // Updated constant names

// Import necessary traits for GC9D01 (these are bounds on the GC9D01 struct itself)
use embedded_hal::digital::OutputPin;
use embedded_hal_async::spi::SpiDevice;
use gc9d01::Timer as Gc9d01Timer; // Moved this import up
use core::convert::TryInto; // Added import for try_into
use alloc::format; // Added import for alloc::format!



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
    // Counter for draw calls to control screen clearing frequency
    draw_count: u32,
    // Currently selected port (0, 1, or 2), default is 1 (Port 2)
    selected_port: usize,
    // Previous selected port for background clearing
    previous_selected_port: Option<usize>,
}

impl Dashboard {
    // Create new Dashboard instance
    pub fn new() -> Self {
        Self {
            port_data: [(0.0, 0.0, 0.0); 3],
            port_connected: [false; 3], // Initialize all ports as disconnected
            port_overcurrent: [false; 3], // Initialize all ports as normal (no overcurrent)
            draw_count: 0, // Initialize draw counter
            selected_port: 1, // Default to Port 2 (index 1)
            previous_selected_port: None, // No previous selection initially
        }
    }

    // Update Dashboard display data for 3 ports: [(V1, A1, W1), (V2, A2, W2), (V3, A3, W3)]
    pub fn update_data(&mut self, data: [(f32, f32, f32); 3], connection_status: [bool; 3], overcurrent_status: [bool; 3]) {
        self.port_data = data;
        self.port_connected = connection_status;
        self.port_overcurrent = overcurrent_status;
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

    // Draw Dashboard directly to the display driver using write_area
    // Accept GC9D01 directly
    pub async fn draw<'a, BUS, DC, RST, TIMER>(&mut self, display: &mut GC9D01<'a, BUS, DC, RST, TIMER>) -> Result<(), Error>
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
            let _ = display.fill_color(Rgb565::BLACK).await;
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

        // Helper function to draw a rounded rectangle background
        async fn draw_rounded_rect<'a, BUS, DC, RST, TIMER>(
            display: &mut GC9D01<'a, BUS, DC, RST, TIMER>,
            x: u16,
            y: u16,
            width: u16,
            height: u16,
            bg_color: Rgb565,
            corner_radius: u16,
        ) -> Result<(), Error>
        where
            BUS: SpiDevice,
            DC: OutputPin<Error = core::convert::Infallible>,
            RST: OutputPin<Error = core::convert::Infallible>,
            TIMER: Gc9d01Timer,
        {
            let total_pixels = (width * height) as usize;
            let mut bg_buffer = alloc::vec![Rgb565::BLACK; total_pixels]; // Default to black (transparent)

            // Fill the rounded rectangle
            for py in 0..height {
                for px in 0..width {
                    let mut inside_rect = true;

                    // Check corners for rounding
                    if corner_radius > 0 {
                        // Top-left corner
                        if px < corner_radius && py < corner_radius {
                            let dx = corner_radius - px;
                            let dy = corner_radius - py;
                            if (dx * dx + dy * dy) > (corner_radius * corner_radius) {
                                inside_rect = false;
                            }
                        }
                        // Top-right corner
                        else if px >= width - corner_radius && py < corner_radius {
                            let dx = px - (width - corner_radius - 1);
                            let dy = corner_radius - py;
                            if (dx * dx + dy * dy) > (corner_radius * corner_radius) {
                                inside_rect = false;
                            }
                        }
                        // Bottom-left corner
                        else if px < corner_radius && py >= height - corner_radius {
                            let dx = corner_radius - px;
                            let dy = py - (height - corner_radius - 1);
                            if (dx * dx + dy * dy) > (corner_radius * corner_radius) {
                                inside_rect = false;
                            }
                        }
                        // Bottom-right corner
                        else if px >= width - corner_radius && py >= height - corner_radius {
                            let dx = px - (width - corner_radius - 1);
                            let dy = py - (height - corner_radius - 1);
                            if (dx * dx + dy * dy) > (corner_radius * corner_radius) {
                                inside_rect = false;
                            }
                        }
                    }

                    if inside_rect {
                        let index = (py * width + px) as usize;
                        bg_buffer[index] = bg_color;
                    }
                }
            }

            display.write_area(x, y, width, height, &bg_buffer).await.map_err(|_| Error::DriverError)?;
            Ok(())
        }

        // Helper function to draw a string with fixed width using space padding
        async fn draw_string<'a, BUS, DC, RST, TIMER>(
            display: &mut GC9D01<'a, BUS, DC, RST, TIMER>,
            s: &str,
            right_edge_x: usize, // Right edge of the drawing area
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
            // Create a fixed-width string by padding with spaces
            let mut padded_string = alloc::string::String::new();
            let string_len = s.chars().count();

            if string_len < fixed_width_chars {
                // Pad with leading spaces for right alignment
                let spaces_needed = fixed_width_chars - string_len;
                for _ in 0..spaces_needed {
                    padded_string.push(' ');
                }
                padded_string.push_str(s);
            } else {
                // If string is too long, truncate it
                padded_string = s.chars().take(fixed_width_chars).collect();
            }

            // Calculate dimensions for background rectangle
            let total_pixel_width = fixed_width_chars * FONT_8X12_WIDTH;
            let start_x = right_edge_x.saturating_sub(total_pixel_width);

            // Draw the text
            let mut current_x = start_x;
            for c in padded_string.chars() {
                if let Some(bitmap) = char_to_mono_bitmap(c) {
                    mono_bitmap_to_rgb565(bitmap, fg_color, bg_color, char_pixel_buffer);

                    let x0 = current_x;
                    let y0 = start_y;

                    display.write_area(
                        x0.try_into().unwrap(),
                        y0.try_into().unwrap(),
                        FONT_8X12_WIDTH.try_into().unwrap(),
                        FONT_8X12_HEIGHT.try_into().unwrap(),
                        char_pixel_buffer,
                    ).await.map_err(|_| Error::DriverError)?;

                    current_x += FONT_8X12_WIDTH;
                } else {
                    // Handle characters not in font (draw a blank space)
                    mono_bitmap_to_rgb565(&[0u8; 12], fg_color, bg_color, char_pixel_buffer); // All zeros = blank

                    let x0 = current_x;
                    let y0 = start_y;

                    display.write_area(
                        x0.try_into().unwrap(),
                        y0.try_into().unwrap(),
                        FONT_8X12_WIDTH.try_into().unwrap(),
                        FONT_8X12_HEIGHT.try_into().unwrap(),
                        char_pixel_buffer,
                    ).await.map_err(|_| Error::DriverError)?;

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
                let prev_bg_width = (prev_col_width as u16 + 8).min(screen_width as u16 - prev_bg_x);
                let prev_bg_height = screen_height as u16 + 8;

                // Clear previous background in chunks to avoid memory issues
                let chunk_height = 8;
                let chunk_size = (prev_bg_width as usize * chunk_height) as usize;
                let clear_buffer = alloc::vec![Rgb565::BLACK; chunk_size];

                for y_chunk in (0..prev_bg_height).step_by(chunk_height) {
                    let remaining_height = (prev_bg_height - y_chunk).min(chunk_height as u16);
                    let chunk_pixels = (prev_bg_width as usize * remaining_height as usize).min(chunk_size);
                    display.write_area(
                        prev_bg_x,
                        prev_bg_y + y_chunk,
                        prev_bg_width,
                        remaining_height,
                        &clear_buffer[..chunk_pixels]
                    ).await.map_err(|_| Error::DriverError)?;
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
            let (final_voltage_color, final_current_color, final_power_color) = if self.port_connected[i] {
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

                draw_rounded_rect(display, bg_x, bg_y, bg_width, bg_height, selection_color, 4).await?;
            }

            // Draw Voltage (Row 1) - Fixed width of 6 characters (e.g., "12.34V")
            let voltage_str = self.float_to_string(&mut buffer, port_voltage);
            draw_string(display, &format!("{}V", voltage_str), col_right_edge_x as usize, 6, 0, final_voltage_color, text_bg_color, &mut char_pixel_buffer).await?;

            // Draw Current (Row 2) - Fixed width of 6 characters (e.g., "12.34A")
            let current_str = self.float_to_string(&mut buffer, port_current);
            draw_string(display, &format!("{}A", current_str), col_right_edge_x as usize, 6, actual_row_height as usize, final_current_color, text_bg_color, &mut char_pixel_buffer).await?;

            // Draw Power (Row 3) - Show "OCP" if overcurrent detected, otherwise show power value
            if self.port_overcurrent[i] {
                // Display "OCP" in red when overcurrent is detected
                draw_string(display, "OCP", col_right_edge_x as usize, 6, (actual_row_height * 2) as usize, Rgb565::RED, text_bg_color, &mut char_pixel_buffer).await?;
            } else {
                // Display normal power value
                let power_str = self.float_to_string(&mut buffer, port_power);
                draw_string(display, &format!("{}W", power_str), col_right_edge_x as usize, 6, (actual_row_height * 2) as usize, final_power_color, text_bg_color, &mut char_pixel_buffer).await?;
            }
        }

        Ok(())
    }

    // Simplified float to string function (moved from inside draw)
    fn float_to_string<'a>(&self, buffer: &'a mut [u8], value: f32) -> &'a str { // Added &self and value parameter and lifetime
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
         if integer_part == 0 && value.abs() < 1.0 && value >= 0.0 { // Handle 0.x case
             buffer[cursor] = b'0';
             cursor += 1;
        } else if integer_part == 0 && value.abs() < 1.0 && value < 0.0 && cursor == 1 { // Handle -0.x case
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
