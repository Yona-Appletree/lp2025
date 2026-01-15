use crate::error::Error;
use crate::nodes::fixture::sampling_kernel::SamplingKernel;
use crate::nodes::{NodeConfig, NodeRuntime};
use crate::runtime::contexts::{NodeInitContext, OutputHandle, RenderContext, TextureHandle};
use alloc::{boxed::Box, string::String, vec, vec::Vec};
use lp_model::nodes::fixture::{ColorOrder, FixtureConfig};
use lp_shared::fs::fs_event::FsChange;

// Simplified mapping point (will be replaced with structured type later)
#[derive(Debug, Clone)]
struct MappingPoint {
    channel: u32,
    center: [f32; 2], // UV coordinates in fixture space [-1,-1] to [1,1]
    radius: f32,
}

/// Fixture node runtime
pub struct FixtureRuntime {
    config: Option<FixtureConfig>,
    texture_handle: Option<TextureHandle>,
    output_handle: Option<OutputHandle>,
    kernel: SamplingKernel,
    color_order: ColorOrder,
    mapping: Vec<MappingPoint>,
    transform: [[f32; 4]; 4],
}

impl FixtureRuntime {
    pub fn new() -> Self {
        Self {
            config: None,
            texture_handle: None,
            output_handle: None,
            kernel: SamplingKernel::new(0.1), // Default small radius
            color_order: ColorOrder::Rgb,
            mapping: Vec::new(),
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ], // Identity matrix
        }
    }

    pub fn set_config(&mut self, config: FixtureConfig) {
        self.config = Some(config);
    }

    /// Get the fixture config (for state extraction)
    pub fn get_config(&self) -> Option<&FixtureConfig> {
        self.config.as_ref()
    }
}

impl NodeRuntime for FixtureRuntime {
    fn init(&mut self, ctx: &dyn NodeInitContext) -> Result<(), Error> {
        // Get config
        let config = self.config.as_ref().ok_or_else(|| Error::InvalidConfig {
            node_path: String::from("fixture"),
            reason: String::from("Config not set"),
        })?;

        // Resolve texture handle
        let texture_handle = ctx.resolve_texture(&config.texture_spec)?;
        self.texture_handle = Some(texture_handle);

        // Resolve output handle
        let output_handle = ctx.resolve_output(&config.output_spec)?;
        self.output_handle = Some(output_handle);

        // Store config values
        self.color_order = config.color_order;
        self.transform = config.transform;

        // Parse mapping (simplified for now - will be structured later)
        // For now, if mapping is "linear" or empty, create a default mapping point
        // that samples from the center of the texture (channel 0)
        if config.mapping == "linear" || config.mapping.is_empty() {
            // Default: single mapping point at center (0, 0) with small radius
            self.mapping = vec![MappingPoint {
                channel: 0,
                center: [0.0, 0.0], // Center of fixture space
                radius: 0.1,        // Small sampling radius
            }];
        } else {
            self.mapping = Vec::new(); // Other mappings not parsed yet
        }

        // Create sampling kernel based on first mapping's radius (if any)
        if let Some(first_mapping) = self.mapping.first() {
            let normalized_radius = first_mapping.radius.min(1.0).max(0.0);
            self.kernel = SamplingKernel::new(normalized_radius);
        } else {
            // No mappings, use default small radius
            self.kernel = SamplingKernel::new(0.1);
        }

        Ok(())
    }

    fn render(&mut self, ctx: &mut dyn RenderContext) -> Result<(), Error> {
        // Get texture handle
        let texture_handle = self.texture_handle.ok_or_else(|| Error::Other {
            message: String::from("Texture handle not resolved"),
        })?;

        // Get texture (triggers lazy rendering if needed)
        let texture = ctx.get_texture(texture_handle)?;

        let texture_width = texture.width() as f32;
        let texture_height = texture.height() as f32;

        // Sample all mapping points and collect results
        let mut sampled_values: Vec<(u32, [u8; 4])> = Vec::new();

        for mapping in &self.mapping {
            // Transform fixture coordinates to texture UV coordinates
            // Fixture space: [-1, -1] to [1, 1]
            // Texture space: [0, 0] to [1, 1]
            let fixture_u = mapping.center[0];
            let fixture_v = mapping.center[1];

            // Apply transform matrix (4x4 affine transform)
            // For now, simple transform: map [-1,1] to [0,1]
            // Full matrix multiplication will be implemented later
            let center_u = (fixture_u + 1.0) * 0.5; // Simplified for now
            let center_v = (fixture_v + 1.0) * 0.5;

            let radius = mapping.radius;

            // Sample texture at kernel positions
            let mut r_sum = 0.0f32;
            let mut g_sum = 0.0f32;
            let mut b_sum = 0.0f32;
            let mut a_sum = 0.0f32;
            let mut total_weight = 0.0f32;

            for sample in &self.kernel.samples {
                // Calculate sample position (scale kernel by mapping radius)
                let sample_u = center_u + sample.offset_u * radius;
                let sample_v = center_v + sample.offset_v * radius;

                // Clamp to [0, 1]
                let sample_u = sample_u.max(0.0).min(1.0);
                let sample_v = sample_v.max(0.0).min(1.0);

                // Convert normalized coordinates to pixel coordinates
                let x = (sample_u * texture_width).clamp(0.0, texture_width - 1.0) as u32;
                let y = (sample_v * texture_height).clamp(0.0, texture_height - 1.0) as u32;

                // Sample texture
                if let Some(pixel) = texture.get_pixel(x, y) {
                    let weight = sample.weight;
                    r_sum += pixel[0] as f32 * weight;
                    g_sum += pixel[1] as f32 * weight;
                    b_sum += pixel[2] as f32 * weight;
                    a_sum += pixel[3] as f32 * weight;
                    total_weight += weight;
                }
            }

            // Normalize by total weight
            if total_weight > 0.0 {
                r_sum /= total_weight;
                g_sum /= total_weight;
                b_sum /= total_weight;
                a_sum /= total_weight;
            }

            // Convert to u8
            let r = r_sum as u8;
            let g = g_sum as u8;
            let b = b_sum as u8;
            let a = a_sum as u8;

            sampled_values.push((mapping.channel, [r, g, b, a]));
        }

        // Get output handle
        let output_handle = self.output_handle.ok_or_else(|| Error::Other {
            message: String::from("Output handle not resolved"),
        })?;

        // Write sampled values to output buffer
        // For now, use universe 0 and channel_offset 0 (sequential writing)
        // TODO: Add universe and channel_offset fields to FixtureConfig when needed
        let universe = 0u32;
        let channel_offset = 0u32;
        for (channel, [r, g, b, _a]) in sampled_values {
            let start_ch = channel_offset + channel * 3; // 3 bytes per RGB
            let buffer = ctx.get_output(output_handle, universe, start_ch, 3)?;
            self.color_order.write_rgb(buffer, 0, r, g, b);
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }

    fn update_config(
        &mut self,
        new_config: Box<dyn NodeConfig>,
        ctx: &dyn NodeInitContext,
    ) -> Result<(), Error> {
        // Downcast to FixtureConfig
        let fixture_config = new_config
            .as_any()
            .downcast_ref::<FixtureConfig>()
            .ok_or_else(|| Error::InvalidConfig {
                node_path: String::from("fixture"),
                reason: String::from("Config is not a FixtureConfig"),
            })?;

        let old_config = self.config.as_ref();
        let texture_changed = old_config
            .map(|old| old.texture_spec != fixture_config.texture_spec)
            .unwrap_or(true);
        let output_changed = old_config
            .map(|old| old.output_spec != fixture_config.output_spec)
            .unwrap_or(true);

        self.config = Some(fixture_config.clone());
        self.color_order = fixture_config.color_order;
        self.transform = fixture_config.transform;

        // Re-resolve handles if they changed
        if texture_changed {
            let texture_handle = ctx.resolve_texture(&fixture_config.texture_spec)?;
            self.texture_handle = Some(texture_handle);
        }

        if output_changed {
            let output_handle = ctx.resolve_output(&fixture_config.output_spec)?;
            self.output_handle = Some(output_handle);
        }

        // Parse mapping (simplified for now)
        if fixture_config.mapping == "linear" || fixture_config.mapping.is_empty() {
            self.mapping = vec![MappingPoint {
                channel: 0,
                center: [0.0, 0.0],
                radius: 0.1,
            }];
        } else {
            self.mapping = Vec::new();
        }

        // Update sampling kernel
        if let Some(first_mapping) = self.mapping.first() {
            let normalized_radius = first_mapping.radius.min(1.0).max(0.0);
            self.kernel = SamplingKernel::new(normalized_radius);
        } else {
            self.kernel = SamplingKernel::new(0.1);
        }

        Ok(())
    }

    fn handle_fs_change(
        &mut self,
        _change: &FsChange,
        _ctx: &dyn NodeInitContext,
    ) -> Result<(), Error> {
        // Fixtures don't currently support external mapping files
        // This is a no-op for now
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_runtime_creation() {
        let runtime = FixtureRuntime::new();
        let _boxed: alloc::boxed::Box<dyn NodeRuntime> = alloc::boxed::Box::new(runtime);
    }
}
