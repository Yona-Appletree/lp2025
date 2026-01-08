//! Debug UI for visualizing textures, mappings, and LEDs

use egui::{Color32, ColorImage, Image, Painter, TextureHandle, Ui};
use lp_core::nodes::fixture::{FixtureNode, Mapping};
use lp_core::nodes::shader::{ShaderNode, ShaderNodeRuntime};
use lp_core::nodes::texture::{TextureNode, formats};
use lp_core::project::config::ProjectConfig;
use lp_core::project::runtime::ProjectRuntime;

/// Generate placeholder texture data for visualization
/// In the future, this will use actual shader-rendered data
fn generate_placeholder_texture(width: u32, height: u32, format: &str) -> Vec<u8> {
    let bytes_per_pixel = formats::bytes_per_pixel(format).unwrap_or(3);
    let mut data = Vec::with_capacity((width * height * bytes_per_pixel as u32) as usize);

    for y in 0..height {
        for x in 0..width {
            match format {
                formats::RGB8 => {
                    // Create a simple gradient pattern
                    let r = ((x as f32 / width as f32) * 255.0) as u8;
                    let g = ((y as f32 / height as f32) * 255.0) as u8;
                    let b = 128u8;
                    data.push(r);
                    data.push(g);
                    data.push(b);
                }
                formats::RGBA8 => {
                    // Create a simple gradient pattern with alpha
                    let r = ((x as f32 / width as f32) * 255.0) as u8;
                    let g = ((y as f32 / height as f32) * 255.0) as u8;
                    let b = 128u8;
                    let a = 255u8;
                    data.push(r);
                    data.push(g);
                    data.push(b);
                    data.push(a);
                }
                formats::R8 => {
                    // Grayscale gradient
                    let gray = (((x + y) as f32 / (width + height) as f32) * 255.0) as u8;
                    data.push(gray);
                }
                _ => {
                    // Default: fill with zeros
                    for _ in 0..bytes_per_pixel {
                        data.push(0);
                    }
                }
            }
        }
    }

    data
}

/// Convert texture data to egui ColorImage
fn texture_data_to_color_image(data: &[u8], width: u32, height: u32, format: &str) -> ColorImage {
    let mut pixels = Vec::with_capacity((width * height) as usize);

    let bytes_per_pixel = formats::bytes_per_pixel(format).unwrap_or(3);

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * bytes_per_pixel as u32) as usize;
            if idx + bytes_per_pixel <= data.len() {
                let color = match format {
                    formats::RGB8 => {
                        let r = data[idx];
                        let g = data[idx + 1];
                        let b = data[idx + 2];
                        Color32::from_rgb(r, g, b)
                    }
                    formats::RGBA8 => {
                        let r = data[idx];
                        let g = data[idx + 1];
                        let b = data[idx + 2];
                        let a = data[idx + 3];
                        Color32::from_rgba_unmultiplied(r, g, b, a)
                    }
                    formats::R8 => {
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

/// Generate a color for a fixture based on its ID
fn fixture_color(fixture_id: &str) -> Color32 {
    // Generate distinct colors for different fixtures
    // Hash the string ID to get a consistent number
    let hash: u32 = fixture_id.chars().map(|c| c as u32).sum();
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
    fixture_id: &str,
    mapping: &Mapping,
    show_labels: bool,
) {
    let color = fixture_color(fixture_id);
    let stroke_color = Color32::from_rgb(255, 255, 255); // White outline for visibility

    // Convert normalized coordinates [0, 1] to screen coordinates
    let center_x = texture_rect.left() + mapping.center[0] * texture_rect.width();
    let center_y = texture_rect.top() + mapping.center[1] * texture_rect.height();

    // Convert normalized radius to screen coordinates
    // Radius is in normalized texture space, so multiply by texture dimension
    let radius_pixels = mapping.radius * texture_rect.width().min(texture_rect.height());

    let center = egui::pos2(center_x, center_y);

    // Draw circle outline (radius)
    painter.circle_stroke(center, radius_pixels, egui::Stroke::new(1.0, stroke_color));

    // Draw center point
    painter.circle_filled(center, 3.0, color);
    painter.circle_stroke(center, 3.0, egui::Stroke::new(1.0, stroke_color));

    // Draw label if requested
    if show_labels {
        let label = format!("Ch{}", mapping.channel);
        painter.text(
            center + egui::Vec2::new(radius_pixels + 5.0, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::monospace(10.0),
            color,
        );
    }
}

/// Render texture visualization in egui (without mappings)
pub fn render_texture(
    ui: &mut Ui,
    texture_id: &str,
    texture: &TextureNode,
    texture_data: Option<&[u8]>,
) {
    match texture {
        TextureNode::Memory { size, format } => {
            let [width, height] = *size;

            // Get texture data (placeholder for now if not provided)
            let data = if let Some(data) = texture_data {
                data.to_vec()
            } else {
                generate_placeholder_texture(width, height, format)
            };

            // Convert to egui image
            let color_image = texture_data_to_color_image(&data, width, height, format);

            // Create texture handle - egui will update if called with same name
            let texture_name = format!("texture_{}", texture_id);
            let texture_handle: TextureHandle =
                ui.ctx()
                    .load_texture(texture_name, color_image, Default::default());

            // Display metadata
            ui.group(|ui| {
                ui.label(format!("Texture ID: {}", texture_id));
                ui.label(format!("Size: {}x{}", width, height));
                ui.label(format!("Format: {}", format));
                ui.label(format!(
                    "Bytes per pixel: {}",
                    formats::bytes_per_pixel(format).unwrap_or(0)
                ));
            });

            ui.separator();

            // Display texture image
            // Scale to fit available width, max 8x native size, clamp to panel width
            let available_width = ui.available_width();
            let scale = (available_width / width as f32).min(8.0);
            let display_width = width as f32 * scale;
            let display_height = height as f32 * scale;

            ui.add(
                Image::new(&texture_handle)
                    .fit_to_exact_size(egui::Vec2::new(display_width, display_height)),
            );
        }
    }
}

/// Render all textures in a debug panel
pub fn render_textures_panel(
    ui: &mut Ui,
    _project: &ProjectConfig,
    runtime: Option<&ProjectRuntime>,
) {
    ui.heading("Textures");
    ui.separator();

    if let Some(rt) = runtime {
        let texture_ids = rt.get_texture_ids();
        if texture_ids.is_empty() {
            ui.label("No textures loaded");
        } else {
            for texture_id in texture_ids {
                let texture_id_typed = lp_core::nodes::id::TextureId(texture_id.clone());
                if let Some(texture_rt) = rt.get_texture(texture_id_typed) {
                    // Get config and texture data from runtime
                    let config = texture_rt.config();
                    let texture = texture_rt.texture();
                    let texture_data = texture.data();

                    // Call the existing render_texture() helper function
                    render_texture(ui, &texture_id, config, Some(texture_data));
                    ui.separator();
                }
            }
        }
    } else {
        ui.label("No runtime available");
    }
}

/// Render a fixture with its texture and mapping overlay
fn render_fixture(
    ui: &mut Ui,
    fixture_id: &str,
    fixture: &FixtureNode,
    _project: &ProjectConfig,
    runtime: Option<&ProjectRuntime>,
) {
    match fixture {
        FixtureNode::CircleList {
            output_id,
            texture_id,
            channel_order,
            mapping,
        } => {
            // Display fixture metadata
            ui.group(|ui| {
                ui.label(format!("Fixture ID: {}", fixture_id));
                ui.label(format!("Output ID: {}", String::from(output_id.clone())));
                ui.label(format!("Channel Order: {}", channel_order));
                ui.label(format!("Mappings: {}", mapping.len()));
            });

            ui.separator();

            // Show texture with mapping overlay
            // Note: ProjectLoader is implemented, but we get texture data from runtime
            // Display texture with this fixture's mappings overlaid
            // For now, get texture data from runtime if available
            let texture_id_str: String = texture_id.clone().into();
            if let Some(rt) = runtime {
                if let Some(texture_rt) = rt.get_texture(texture_id.clone()) {
                    let texture = texture_rt.texture();
                    let width = texture.width();
                    let height = texture.height();
                    let format = "RGB8"; // Default format
                    let data: Vec<u8> = texture.data().to_vec();
                    let color_image = texture_data_to_color_image(&data, width, height, format);

                    // Create texture handle
                    let texture_name = format!("fixture_{}_texture_{}", fixture_id, texture_id_str);
                    let texture_handle: TextureHandle =
                        ui.ctx()
                            .load_texture(texture_name, color_image, Default::default());

                    // Display texture metadata
                    ui.label(format!("Texture ID: {}", texture_id_str));
                    ui.label(format!("Size: {}x{}", width, height));

                    // Scale to fit available width
                    let available_width = ui.available_width();
                    let scale = (available_width / width as f32).min(8.0);
                    let display_width = width as f32 * scale;
                    let display_height = height as f32 * scale;

                    // Display texture image with mapping overlay
                    let image_response = ui.add(
                        Image::new(&texture_handle)
                            .fit_to_exact_size(egui::Vec2::new(display_width, display_height)),
                    );

                    // Draw mapping overlay for this fixture
                    for mapping_item in mapping {
                        draw_mapping_overlay(
                            ui.painter(),
                            image_response.rect,
                            width,
                            height,
                            fixture_id,
                            mapping_item,
                            true, // Show labels
                        );
                    }

                    ui.separator();
                } else {
                    ui.label(format!("Texture {} not found in runtime", texture_id_str));
                }
            } else {
                ui.label("Runtime not available");
            }
        }
    }
}

/// Render all fixtures in a debug panel
pub fn render_fixtures_panel(
    ui: &mut Ui,
    project: &ProjectConfig,
    runtime: Option<&ProjectRuntime>,
) {
    ui.heading("Fixtures");
    ui.separator();

    if let Some(rt) = runtime {
        let fixture_ids = rt.get_fixture_ids();
        if fixture_ids.is_empty() {
            ui.label("No fixtures loaded");
        } else {
            for fixture_id in fixture_ids {
                let fixture_id_typed = lp_core::nodes::id::FixtureId(fixture_id.clone());
                if let Some(fixture_rt) = rt.get_fixture(fixture_id_typed) {
                    // Get config and runtime
                    let config = fixture_rt.config();

                    // Call the existing render_fixture() helper function
                    render_fixture(ui, &fixture_id, config, project, Some(rt));
                    ui.separator();
                }
            }
        }
    } else {
        ui.label("No runtime available");
    }
}

/// Render shader code and errors
pub fn render_shader_panel(
    ui: &mut Ui,
    shader_id: &str,
    shader_config: &ShaderNode,
    shader_runtime: Option<&ShaderNodeRuntime>,
) {
    ui.group(|ui| {
        ui.label(format!("Shader ID: {}", shader_id));
        ui.separator();

        // Show shader code
        match shader_config {
            ShaderNode::Single { glsl, texture_id } => {
                ui.label(format!("Texture ID: {}", String::from(texture_id.clone())));
                ui.separator();
                ui.label("GLSL Code:");
                // Create a mutable string for TextEdit (it needs &mut str)
                let mut glsl_mut = glsl.clone();
                ui.add(
                    egui::TextEdit::multiline(&mut glsl_mut)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace)
                        .interactive(false),
                );
            }
        }

        ui.separator();

        // Show shader status/errors
        if let Some(runtime) = shader_runtime {
            match runtime.status() {
                lp_core::project::runtime::NodeStatus::Ok => {
                    ui.label(egui::RichText::new("Status: OK").color(egui::Color32::GREEN));
                }
                lp_core::project::runtime::NodeStatus::Error { status_message } => {
                    ui.label(
                        egui::RichText::new(format!("Status: ERROR\n{}", status_message))
                            .color(egui::Color32::RED),
                    );
                }
            }
        } else {
            ui.label("Status: Not initialized");
        }
    });
}

/// Render all shaders in a debug panel
pub fn render_shaders_panel(
    ui: &mut Ui,
    _project: &ProjectConfig,
    runtime: Option<&ProjectRuntime>,
) {
    ui.heading("Shaders");
    ui.separator();

    if let Some(rt) = runtime {
        let shader_ids = rt.get_shader_ids();
        if shader_ids.is_empty() {
            ui.label("No shaders loaded");
        } else {
            for shader_id in shader_ids {
                let shader_id_typed = lp_core::nodes::id::ShaderId(shader_id.clone());
                if let Some(shader_rt) = rt.get_shader(shader_id_typed) {
                    // Get config and runtime
                    let config = shader_rt.config();

                    // Call the existing render_shader_panel() helper function
                    render_shader_panel(ui, &shader_id, config, Some(shader_rt));
                    ui.separator();
                }
            }
        }
    } else {
        ui.label("No runtime available");
    }
}
