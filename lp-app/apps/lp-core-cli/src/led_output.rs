//! Host LED output implementation with egui visualization

use lp_core::error::Error;
use lp_core::traits::LedOutput;
use std::sync::{Arc, Mutex};

/// Host LED output implementation that stores pixels and visualizes them in egui
pub struct HostLedOutput {
    pixels: Arc<Mutex<Vec<u8>>>,
    pixel_count: usize,
    bytes_per_pixel: usize,
}

impl HostLedOutput {
    /// Create a new host LED output
    ///
    /// `pixel_count` is the number of LEDs
    /// `bytes_per_pixel` is typically 3 (RGB) or 4 (RGBA)
    pub fn new(pixel_count: usize, bytes_per_pixel: usize) -> Self {
        Self {
            pixels: Arc::new(Mutex::new(vec![0; pixel_count * bytes_per_pixel])),
            pixel_count,
            bytes_per_pixel,
        }
    }

    /// Get a reference to the pixel data for rendering
    pub fn get_pixels(&self) -> Arc<Mutex<Vec<u8>>> {
        Arc::clone(&self.pixels)
    }

    /// Get bytes per pixel
    pub fn bytes_per_pixel(&self) -> usize {
        self.bytes_per_pixel
    }
}

impl LedOutput for HostLedOutput {
    fn write_pixels(&mut self, pixels: &[u8]) -> Result<(), Error> {
        let expected_len = self.pixel_count * self.bytes_per_pixel;
        if pixels.len() != expected_len {
            return Err(Error::Node(format!(
                "Invalid pixel data length: expected {} bytes, got {}",
                expected_len,
                pixels.len()
            )));
        }

        let mut pixel_data = self.pixels.lock().unwrap();
        pixel_data.copy_from_slice(pixels);
        Ok(())
    }

    fn get_pixel_count(&self) -> usize {
        self.pixel_count
    }
}

/// Helper to render LEDs as simple circles
pub fn render_leds(
    ui: &mut egui::Ui,
    led_output: &HostLedOutput,
    _selected_led: Option<usize>,
) -> Option<usize> {
    let pixels = led_output.get_pixels();
    let pixel_data = pixels.lock().unwrap();
    let bytes_per_pixel = led_output.bytes_per_pixel();
    let pixel_count = led_output.get_pixel_count();

    // Calculate layout: try to make a roughly square grid
    let cols = (pixel_count as f32).sqrt().ceil() as usize;
    let rows = (pixel_count + cols - 1) / cols; // Ceiling division

    // Size of each LED circle (adjustable)
    let led_size = 25.0;
    let spacing = 8.0;

    // Calculate total size needed
    let total_width = (cols as f32) * (led_size + spacing) - spacing;
    let total_height = (rows as f32) * (led_size + spacing) - spacing;

    // Constrain to available width
    let available_width = ui.available_width();
    let constrained_width = total_width.min(available_width);

    // Allocate space for the LED grid
    let allocated_size =
        egui::Vec2::new(constrained_width, total_height.min(ui.available_height()));

    // Use scroll area with size constraints
    egui::ScrollArea::both()
        .max_width(available_width)
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            let (_id, rect) = ui.allocate_space(allocated_size);
            let painter = ui.painter().with_clip_rect(rect);

            // Center the grid within allocated space
            let start_x =
                rect.left() + (allocated_size.x - total_width.min(allocated_size.x)) / 2.0;
            let start_y = rect.top() + (allocated_size.y - total_height) / 2.0;

            let mut y = start_y;
            for row in 0..rows {
                let mut x = start_x;
                for col in 0..cols {
                    let idx = row * cols + col;
                    if idx >= pixel_count {
                        break;
                    }

                    // Get pixel color
                    let pixel_start = idx * bytes_per_pixel;
                    let r = pixel_data[pixel_start];
                    let g = if bytes_per_pixel > 1 {
                        pixel_data[pixel_start + 1]
                    } else {
                        0
                    };
                    let b = if bytes_per_pixel > 2 {
                        pixel_data[pixel_start + 2]
                    } else {
                        0
                    };

                    let color = egui::Color32::from_rgb(r, g, b);
                    let center = egui::pos2(x + led_size / 2.0, y + led_size / 2.0);
                    let radius = led_size / 2.0;

                    // Draw LED circle
                    painter.circle_filled(center, radius, color);
                    painter.circle_stroke(
                        center,
                        radius,
                        egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
                    );

                    // Draw LED index label
                    painter.text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        format!("{}", idx),
                        egui::FontId::monospace(10.0),
                        egui::Color32::BLACK,
                    );

                    x += led_size + spacing;
                }
                y += led_size + spacing;
            }
        });

    None // No interaction
}
