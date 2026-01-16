//! Panel rendering functions for different node types

use eframe::egui::{self, Color32, ColorImage, Image, Painter, TextureHandle};
use lp_engine_client::project::{ClientNodeEntry, ClientProjectView};
use lp_model::{
    NodeHandle, NodeKind,
    nodes::{
        fixture::{FixtureState, MappingCell},
        output::OutputState,
        shader::ShaderState,
        texture::TextureState,
    },
};

/// Convert texture data to egui ColorImage
///
/// Handles RGB8, RGBA8, and R8 formats.
pub fn texture_data_to_color_image(
    data: &[u8],
    width: u32,
    height: u32,
    format: &str,
) -> ColorImage {
    let mut pixels = Vec::with_capacity((width * height) as usize);

    let bytes_per_pixel = match format {
        "RGB8" => 3,
        "RGBA8" => 4,
        "R8" => 1,
        _ => 3, // Default to RGB8
    };

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * bytes_per_pixel) as usize;
            let bytes_per_pixel_usize = bytes_per_pixel as usize;
            if idx + bytes_per_pixel_usize <= data.len() {
                let color = match format {
                    "RGB8" => {
                        let r = data[idx];
                        let g = data[idx + 1];
                        let b = data[idx + 2];
                        Color32::from_rgb(r, g, b)
                    }
                    "RGBA8" => {
                        let r = data[idx];
                        let g = data[idx + 1];
                        let b = data[idx + 2];
                        let a = data[idx + 3];
                        Color32::from_rgba_unmultiplied(r, g, b, a)
                    }
                    "R8" => {
                        let gray = data[idx];
                        Color32::from_gray(gray)
                    }
                    _ => Color32::BLACK,
                };
                pixels.push(color);
            } else {
                pixels.push(Color32::BLACK);
            }
        }
    }

    ColorImage {
        size: [width as usize, height as usize],
        pixels,
    }
}

/// Generate a color for a fixture based on its handle
fn fixture_color(handle: &NodeHandle) -> Color32 {
    // Generate distinct colors for different fixtures
    // Hash the handle to get a consistent number
    let hash: u32 = format!("{:?}", handle).chars().map(|c| c as u32).sum();
    let hue = (hash as f32 * 137.508) % 360.0; // Golden angle for distribution
    let (r, g, b) = hsv_to_rgb(hue / 360.0, 0.8, 0.9);
    Color32::from_rgb(r, g, b)
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

/// Render status panel
pub fn render_status_panel(
    ui: &mut egui::Ui,
    frame_count: u64,
    current_fps: f32,
    avg_fps: f32,
    sync_in_progress: bool,
) {
    ui.horizontal(|ui| {
        ui.label(format!("Frame: {}", frame_count));
        ui.separator();
        ui.label(format!("FPS: {:.1}", current_fps));
        ui.separator();
        ui.label(format!("Avg FPS: {:.1}", avg_fps));
        ui.separator();
        if sync_in_progress {
            ui.label(egui::RichText::new("Syncing...").color(egui::Color32::YELLOW));
        } else {
            ui.label(egui::RichText::new("Ready").color(egui::Color32::GREEN));
        }
    });
}

/// Render nodes panel with checkboxes
///
/// Returns true if any checkbox state changed
pub fn render_nodes_panel(
    ui: &mut egui::Ui,
    view: &ClientProjectView,
    tracked_nodes: &mut std::collections::BTreeSet<NodeHandle>,
    all_detail: &mut bool,
) -> bool {
    let mut changed = false;
    ui.heading("Nodes");
    ui.separator();

    // "All detail" checkbox
    let all_detail_changed = ui.checkbox(all_detail, "All detail").changed();
    if all_detail_changed {
        changed = true;
        if *all_detail {
            // Track all nodes
            tracked_nodes.clear();
            tracked_nodes.extend(view.nodes.keys().copied());
        } else if tracked_nodes.len() == view.nodes.len() {
            // If all were tracked and we uncheck, clear tracking
            tracked_nodes.clear();
        }
    }

    ui.separator();

    // List of nodes with checkboxes
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (handle, entry) in &view.nodes {
            let is_tracked = tracked_nodes.contains(handle);
            let mut checked = is_tracked;

            let node_label = format!("{:?} ({:?}) - {:?}", entry.path, entry.kind, entry.status);

            let checkbox_response = ui.checkbox(&mut checked, node_label);

            if checkbox_response.changed() {
                changed = true;
                if checked {
                    tracked_nodes.insert(*handle);
                } else {
                    tracked_nodes.remove(handle);
                    // If we uncheck a node, also uncheck "all detail"
                    if *all_detail {
                        *all_detail = false;
                    }
                }
            }
        }
    });

    changed
}

/// Render texture panel
pub fn render_texture_panel(ui: &mut egui::Ui, entry: &ClientNodeEntry, state: &TextureState) {
    ui.heading("Texture");
    ui.separator();

    // Display metadata
    ui.group(|ui| {
        ui.label(format!("Path: {:?}", entry.path));
        ui.label(format!("Size: {}x{}", state.width, state.height));
        ui.label(format!("Format: {}", state.format));
        ui.label(format!("Data size: {} bytes", state.texture_data.len()));
    });

    ui.separator();

    // Display texture image
    if !state.texture_data.is_empty() && state.width > 0 && state.height > 0 {
        let color_image = texture_data_to_color_image(
            &state.texture_data,
            state.width,
            state.height,
            &state.format,
        );

        // Create texture handle
        let texture_name = format!("texture_{:?}", entry.path);
        let texture_handle: TextureHandle =
            ui.ctx()
                .load_texture(texture_name, color_image, Default::default());

        // Scale to fit available width, max 8x native size, but limit height
        let available_width = ui.available_width();
        let max_height = 400.0; // Limit texture height to prevent huge images
        let scale = (available_width / state.width as f32)
            .min(max_height / state.height as f32)
            .min(8.0);
        let display_width = state.width as f32 * scale;
        let display_height = state.height as f32 * scale;

        ui.add(
            Image::new(&texture_handle)
                .fit_to_exact_size(egui::Vec2::new(display_width, display_height)),
        );
    } else {
        ui.label("No texture data available");
    }
}

/// Render shader panel
pub fn render_shader_panel(ui: &mut egui::Ui, entry: &ClientNodeEntry, state: &ShaderState) {
    ui.heading("Shader");
    ui.separator();

    // Display metadata
    ui.group(|ui| {
        ui.label(format!("Path: {:?}", entry.path));
        ui.label(format!("Status: {:?}", entry.status));
        if let Some(error) = &state.error {
            ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
        }
    });

    ui.separator();

    // Display GLSL code
    // Don't use nested ScrollArea - we're already in a scroll area
    ui.label("GLSL Code:");
    // Create a mutable copy for display (read-only)
    let mut glsl_display = state.glsl_code.clone();
    ui.add(
        egui::TextEdit::multiline(&mut glsl_display)
            .font(egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY)
            .desired_rows(20), // Limit height instead of using ScrollArea
    );
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
                let color_image = texture_data_to_color_image(
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

                // Display texture with mapping overlay
                let (rect, _response) = ui.allocate_exact_size(
                    egui::Vec2::new(display_width, display_height),
                    egui::Sense::hover(),
                );

                // Draw texture
                ui.painter().image(
                    texture_handle.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );

                // Draw mapping overlay
                draw_mapping_overlay(
                    ui.painter(),
                    rect,
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

/// Render all nodes panel (grouped by type)
pub fn render_all_nodes_panel(ui: &mut egui::Ui, view: &ClientProjectView) {
    // Group nodes by kind
    let mut textures = Vec::new();
    let mut shaders = Vec::new();
    let mut fixtures = Vec::new();
    let mut outputs = Vec::new();

    for (handle, entry) in &view.nodes {
        match entry.kind {
            NodeKind::Texture => textures.push((*handle, entry)),
            NodeKind::Shader => shaders.push((*handle, entry)),
            NodeKind::Fixture => fixtures.push((*handle, entry)),
            NodeKind::Output => outputs.push((*handle, entry)),
        }
    }

    // Render textures
    if !textures.is_empty() {
        ui.heading("Textures");
        ui.separator();
        for (_handle, entry) in &textures {
            if let Some(lp_model::project::api::NodeState::Texture(state)) = &entry.state {
                render_texture_panel(ui, entry, state);
            } else {
                ui.group(|ui| {
                    ui.label(format!("Path: {:?}", entry.path));
                    ui.label(format!("Status: {:?}", entry.status));
                    ui.label("(Not tracked for detail - check the box to see texture data)");
                });
            }
            ui.separator();
        }
    }

    // Render shaders
    if !shaders.is_empty() {
        ui.heading("Shaders");
        ui.separator();
        for (_handle, entry) in &shaders {
            if let Some(lp_model::project::api::NodeState::Shader(state)) = &entry.state {
                render_shader_panel(ui, entry, state);
            } else {
                ui.group(|ui| {
                    ui.label(format!("Path: {:?}", entry.path));
                    ui.label(format!("Status: {:?}", entry.status));
                    ui.label("(Not tracked for detail - check the box to see GLSL code)");
                });
            }
            ui.separator();
        }
    }

    // Render fixtures
    if !fixtures.is_empty() {
        ui.heading("Fixtures");
        ui.separator();
        for (handle, entry) in &fixtures {
            if let Some(lp_model::project::api::NodeState::Fixture(state)) = &entry.state {
                render_fixture_panel(ui, view, entry, state, handle);
            } else {
                ui.group(|ui| {
                    ui.label(format!("Path: {:?}", entry.path));
                    ui.label(format!("Status: {:?}", entry.status));
                    ui.label("(Not tracked for detail - check the box to see mapping overlay)");
                });
            }
            ui.separator();
        }
    }

    // Render outputs
    if !outputs.is_empty() {
        ui.heading("Outputs");
        ui.separator();
        for (_handle, entry) in &outputs {
            if let Some(lp_model::project::api::NodeState::Output(state)) = &entry.state {
                render_output_panel(ui, entry, state);
            } else {
                ui.group(|ui| {
                    ui.label(format!("Path: {:?}", entry.path));
                    ui.label(format!("Status: {:?}", entry.status));
                    ui.label("(Not tracked for detail - check the box to see output data)");
                });
            }
            ui.separator();
        }
    }

    if textures.is_empty() && shaders.is_empty() && fixtures.is_empty() && outputs.is_empty() {
        ui.label("No nodes available");
    }
}

/// Render output panel
pub fn render_output_panel(ui: &mut egui::Ui, entry: &ClientNodeEntry, state: &OutputState) {
    ui.heading("Output");
    ui.separator();

    // Display metadata
    ui.group(|ui| {
        ui.label(format!("Path: {:?}", entry.path));
        ui.label(format!("Status: {:?}", entry.status));
        ui.label(format!("Channel data: {} bytes", state.channel_data.len()));
    });

    ui.separator();

    // Display channel data (hex dump for now)
    ui.label("Channel Data:");
    egui::ScrollArea::vertical()
        .max_height(200.0)
        .show(ui, |ui| {
            if state.channel_data.is_empty() {
                ui.label("No channel data available");
            } else {
                // Display as hex dump
                for chunk in state.channel_data.chunks(16) {
                    let hex: String = chunk
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<_>>()
                        .join(" ");
                    ui.label(format!("  {}", hex));
                }
            }
        });
}
