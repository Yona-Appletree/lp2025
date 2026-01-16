use crate::debug_ui::nodes::texture;
use eframe::epaint::{Color32, TextureHandle};
use egui::Painter;
use lp_engine_client::{ClientNodeEntry, ClientProjectView};
use lp_model::nodes::fixture::{FixtureState, MappingCell};
use lp_model::{NodeHandle, NodeKind};

/// Generate a color for a fixture based on its handle
pub fn fixture_color(handle: &NodeHandle) -> Color32 {
    // Generate distinct colors for different fixtures
    // Hash the handle to get a consistent number
    let hash: u32 = format!("{:?}", handle).chars().map(|c| c as u32).sum();
    let hue = (hash as f32 * 137.508) % 360.0; // Golden angle for distribution
    let (r, g, b) = hsv_to_rgb(hue / 360.0, 0.8, 0.9);
    Color32::from_rgb(r, g, b)
}

/// Render fixture panel
pub fn render_fixture_panel(
    ui: &mut egui::Ui,
    view: &ClientProjectView,
    entry: &ClientNodeEntry,
    state: &FixtureState,
    fixture_handle: &NodeHandle,
) {
    ui.heading("Fixture");
    ui.separator();

    // Display metadata
    ui.group(|ui| {
        ui.label(format!("Path: {:?}", entry.path));
        ui.label(format!("Status: {:?}", entry.status));
        ui.label(format!("Mapping cells: {}", state.mapping_cells.len()));
        ui.label(format!("Lamp colors: {} bytes", state.lamp_colors.len()));
    });

    ui.separator();

    // Find referenced texture node using resolved handle from state
    let texture_entry = state
        .texture_handle
        .and_then(|handle| view.nodes.get(&handle))
        .filter(|e| matches!(e.kind, NodeKind::Texture));

    if let Some(texture_entry) = texture_entry {
        if let Some(lp_model::project::api::NodeState::Texture(texture_state)) =
            &texture_entry.state
        {
            // Display texture with mapping overlay
            if !texture_state.texture_data.is_empty()
                && texture_state.width > 0
                && texture_state.height > 0
            {
                let color_image = texture::texture_data_to_color_image(
                    &texture_state.texture_data,
                    texture_state.width,
                    texture_state.height,
                    &texture_state.format,
                );

                // Create texture handle
                let texture_name = format!("fixture_texture_{:?}", entry.path);
                let texture_handle: TextureHandle =
                    ui.ctx()
                        .load_texture(texture_name, color_image, Default::default());

                // Scale to fit available width, but limit height
                let available_width = ui.available_width();
                let max_height = 400.0; // Limit texture height
                let scale = (available_width / texture_state.width as f32)
                    .min(max_height / texture_state.height as f32)
                    .min(8.0);
                let display_width = texture_state.width as f32 * scale;
                let display_height = texture_state.height as f32 * scale;

                // Display texture image first (using Image widget like texture.rs)
                let image_response = ui.add(
                    egui::Image::new(&texture_handle)
                        .fit_to_exact_size(egui::Vec2::new(display_width, display_height)),
                );

                // Draw mapping overlay on top of the image
                // Use the image's rect for overlay positioning
                draw_mapping_overlay(
                    ui.painter(),
                    image_response.rect,
                    texture_state.width,
                    texture_state.height,
                    fixture_handle,
                    &state.mapping_cells,
                    true, // Show labels
                );
            } else {
                ui.label("No texture data available for fixture");
            }
        } else {
            ui.label("Texture node does not have state (not tracked for detail)");
        }
    } else {
        if state.texture_handle.is_none() {
            ui.label("Fixture not initialized - no texture handle available");
        } else {
            ui.label("Texture node not found in view (may not be tracked for detail)");
        }
    }
}

/// Draw mapping overlay on a texture
fn draw_mapping_overlay(
    painter: &Painter,
    texture_rect: egui::Rect,
    _texture_width: u32,
    _texture_height: u32,
    fixture_handle: &NodeHandle,
    mapping_cells: &[MappingCell],
    show_labels: bool,
) {
    let color = fixture_color(fixture_handle);
    let stroke_color = Color32::from_rgb(255, 255, 255); // White outline for visibility

    for cell in mapping_cells {
        // Convert normalized coordinates [0, 1] to screen coordinates
        let center_x = texture_rect.left() + cell.center[0] * texture_rect.width();
        let center_y = texture_rect.top() + cell.center[1] * texture_rect.height();

        // Convert normalized radius to screen coordinates
        // Radius is in normalized texture space, so multiply by texture dimension
        let radius_pixels = cell.radius * texture_rect.width().min(texture_rect.height());

        let center = egui::pos2(center_x, center_y);

        // Draw circle outline (radius)
        painter.circle_stroke(center, radius_pixels, egui::Stroke::new(1.0, stroke_color));

        // Draw center point
        painter.circle_filled(center, 3.0, color);
        painter.circle_stroke(center, 3.0, egui::Stroke::new(1.0, stroke_color));

        // Draw label if requested
        if show_labels {
            let label = format!("Ch{}", cell.channel);
            painter.text(
                center + egui::Vec2::new(radius_pixels + 5.0, 0.0),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::monospace(10.0),
                color,
            );
        }
    }
}

/// Convert HSV to RGB (simple approximation)
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 1.0 / 6.0 {
        (c, x, 0.0)
    } else if h < 2.0 / 6.0 {
        (x, c, 0.0)
    } else if h < 3.0 / 6.0 {
        (0.0, c, x)
    } else if h < 4.0 / 6.0 {
        (0.0, x, c)
    } else if h < 5.0 / 6.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
