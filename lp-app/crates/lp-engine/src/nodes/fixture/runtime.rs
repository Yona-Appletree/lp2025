use crate::error::Error;
use crate::nodes::fixture::mapping_compute::{PrecomputedMapping, compute_mapping};
use crate::nodes::{NodeConfig, NodeRuntime};
use crate::runtime::contexts::{NodeInitContext, OutputHandle, RenderContext, TextureHandle};
use alloc::{boxed::Box, string::String, vec::Vec};
use lp_model::FrameId;
use lp_model::nodes::fixture::mapping::{MappingConfig, PathSpec, RingOrder};
use lp_model::nodes::fixture::{ColorOrder, FixtureConfig};
use lp_shared::fs::fs_event::FsChange;

/// Mapping point representing a single LED sampling location
#[derive(Debug, Clone)]
pub struct MappingPoint {
    pub channel: u32,
    pub center: [f32; 2], // Texture space coordinates [0, 1]
    pub radius: f32,
}

/// Fixture node runtime
pub struct FixtureRuntime {
    config: Option<FixtureConfig>,
    texture_handle: Option<TextureHandle>,
    output_handle: Option<OutputHandle>,
    color_order: ColorOrder,
    mapping: Vec<MappingPoint>,
    transform: [[f32; 4]; 4],
    texture_width: Option<u32>,
    texture_height: Option<u32>,
    /// Pre-computed pixel-to-channel mapping
    precomputed_mapping: Option<PrecomputedMapping>,
    /// Last sampled lamp colors (RGB per lamp, ordered by channel index)
    lamp_colors: Vec<u8>,
}

impl FixtureRuntime {
    pub fn new() -> Self {
        Self {
            config: None,
            texture_handle: None,
            output_handle: None,
            color_order: ColorOrder::Rgb,
            mapping: Vec::new(),
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ], // Identity matrix
            texture_width: None,
            texture_height: None,
            precomputed_mapping: None,
            lamp_colors: Vec::new(),
        }
    }

    pub fn set_config(&mut self, config: FixtureConfig) {
        self.config = Some(config);
    }

    /// Get the fixture config (for state extraction)
    pub fn get_config(&self) -> Option<&FixtureConfig> {
        self.config.as_ref()
    }

    /// Get mapping points (for state extraction)
    pub fn get_mapping(&self) -> &Vec<MappingPoint> {
        &self.mapping
    }

    /// Get transform matrix (for state extraction)
    pub fn get_transform(&self) -> [[f32; 4]; 4] {
        self.transform
    }

    /// Get texture handle (for state extraction)
    pub fn get_texture_handle(&self) -> Option<TextureHandle> {
        self.texture_handle
    }

    /// Get output handle (for state extraction)
    pub fn get_output_handle(&self) -> Option<OutputHandle> {
        self.output_handle
    }

    /// Get lamp colors (for state extraction)
    /// Returns RGB values per lamp, ordered by channel index (3 bytes per lamp)
    pub fn get_lamp_colors(&self) -> &[u8] {
        &self.lamp_colors
    }

    /// Regenerate mapping when texture resolution changes or config versions change
    fn regenerate_mapping_if_needed(
        &mut self,
        texture_width: u32,
        texture_height: u32,
        our_config_ver: FrameId,
        texture_config_ver: FrameId,
    ) -> Result<(), Error> {
        let needs_regeneration = self
            .texture_width
            .map(|w| w != texture_width)
            .unwrap_or(true)
            || self
                .texture_height
                .map(|h| h != texture_height)
                .unwrap_or(true)
            || self
                .precomputed_mapping
                .as_ref()
                .map(|m| {
                    let max_config_ver = our_config_ver.max(texture_config_ver);
                    max_config_ver > m.mapping_data_ver
                })
                .unwrap_or(true);

        if needs_regeneration {
            let config = self.config.as_ref().ok_or_else(|| Error::InvalidConfig {
                node_path: String::from("fixture"),
                reason: String::from("Config not set"),
            })?;

            // Compute new pre-computed mapping
            let max_config_ver = our_config_ver.max(texture_config_ver);
            let mapping = compute_mapping(
                &config.mapping,
                texture_width,
                texture_height,
                max_config_ver,
            );

            self.precomputed_mapping = Some(mapping);

            // Update texture dimensions
            self.texture_width = Some(texture_width);
            self.texture_height = Some(texture_height);

            // Keep existing mapping points for now (used by state extraction)
            self.mapping = generate_mapping_points(&config.mapping, texture_width, texture_height);
        }

        Ok(())
    }
}

/// Generate mapping points from MappingConfig
fn generate_mapping_points(
    config: &MappingConfig,
    texture_width: u32,
    texture_height: u32,
) -> Vec<MappingPoint> {
    match config {
        MappingConfig::PathPoints {
            paths,
            sample_diameter,
        } => {
            let mut all_points = Vec::new();
            let mut channel_offset = 0u32;

            for path_spec in paths {
                let points = match path_spec {
                    PathSpec::RingArray {
                        center,
                        diameter,
                        start_ring_inclusive,
                        end_ring_exclusive,
                        ring_lamp_counts,
                        offset_angle,
                        order,
                    } => generate_ring_array_points(
                        *center,
                        *diameter,
                        *start_ring_inclusive,
                        *end_ring_exclusive,
                        ring_lamp_counts,
                        *offset_angle,
                        *order,
                        *sample_diameter,
                        texture_width,
                        texture_height,
                        channel_offset,
                    ),
                };

                channel_offset += points.len() as u32;
                all_points.extend(points);
            }

            all_points
        }
    }
}

/// Generate mapping points from RingArray path specification
fn generate_ring_array_points(
    center: (f32, f32),
    diameter: f32,
    start_ring_inclusive: u32,
    end_ring_exclusive: u32,
    ring_lamp_counts: &Vec<u32>,
    offset_angle: f32,
    order: RingOrder,
    sample_diameter: f32,
    texture_width: u32,
    texture_height: u32,
    channel_offset: u32,
) -> Vec<MappingPoint> {
    let (center_x, center_y) = center;
    let start_ring = start_ring_inclusive;
    let end_ring = end_ring_exclusive;

    // Calculate max ring index for spacing
    let max_ring_index = if end_ring > start_ring {
        (end_ring - start_ring - 1) as f32
    } else {
        0.0
    };

    // Convert sample_diameter (pixels) to normalized radius
    let max_dimension = texture_width.max(texture_height) as f32;
    let normalized_radius = (sample_diameter / 2.0) / max_dimension;

    // Determine ring processing order
    let ring_indices: Vec<u32> = match order {
        RingOrder::InnerFirst => (start_ring..end_ring).collect(),
        RingOrder::OuterFirst => (start_ring..end_ring).rev().collect(),
    };

    let mut points = Vec::new();
    let mut current_channel = channel_offset;

    for ring_index in ring_indices {
        // Calculate ring radius (even spacing)
        let ring_radius = if max_ring_index > 0.0 {
            (diameter / 2.0) * ((ring_index - start_ring) as f32 / max_ring_index)
        } else {
            0.0
        };

        // Get lamp count for this ring
        let lamp_count = ring_lamp_counts
            .get(ring_index as usize)
            .copied()
            .unwrap_or(0);

        // Generate points for each lamp in the ring
        for lamp_index in 0..lamp_count {
            let angle = (2.0 * core::f32::consts::PI * lamp_index as f32 / lamp_count as f32)
                + offset_angle;

            let x = center_x + ring_radius * angle.cos();
            let y = center_y + ring_radius * angle.sin();

            // Clamp to [0, 1] range
            let x = x.max(0.0).min(1.0);
            let y = y.max(0.0).min(1.0);

            points.push(MappingPoint {
                channel: current_channel,
                center: [x, y],
                radius: normalized_radius,
            });

            current_channel += 1;
        }
    }

    points
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

        // Mapping will be generated in render() when texture is available
        // Texture dimensions are not available in init() (texture is lazy-loaded)
        self.mapping = Vec::new();

        Ok(())
    }

    fn render(&mut self, ctx: &mut dyn RenderContext) -> Result<(), Error> {
        // Get texture handle
        let texture_handle = self.texture_handle.ok_or_else(|| Error::Other {
            message: String::from("Texture handle not resolved"),
        })?;

        // Get texture (triggers lazy rendering if needed)
        let texture = ctx.get_texture(texture_handle)?;

        let texture_width = texture.width();
        let texture_height = texture.height();

        // Regenerate mapping if texture resolution changed
        // TODO: Get proper config versions from context
        let our_config_ver = FrameId::new(0);
        let texture_config_ver = FrameId::new(0);
        self.regenerate_mapping_if_needed(
            texture_width,
            texture_height,
            our_config_ver,
            texture_config_ver,
        )?;

        // Get pre-computed mapping
        let mapping = self
            .precomputed_mapping
            .as_ref()
            .ok_or_else(|| Error::Other {
                message: String::from("Precomputed mapping not available"),
            })?;

        // Initialize channel accumulators (16.16 fixed-point, one per channel)
        // Find max channel from mapping entries
        let max_channel = mapping
            .entries
            .iter()
            .filter_map(|e| {
                if !e.is_skip() {
                    Some(e.channel())
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0);

        let mut ch_values_r: Vec<i32> = Vec::with_capacity((max_channel + 1) as usize);
        let mut ch_values_g: Vec<i32> = Vec::with_capacity((max_channel + 1) as usize);
        let mut ch_values_b: Vec<i32> = Vec::with_capacity((max_channel + 1) as usize);
        ch_values_r.resize((max_channel + 1) as usize, 0);
        ch_values_g.resize((max_channel + 1) as usize, 0);
        ch_values_b.resize((max_channel + 1) as usize, 0);

        // Iterate through entries and accumulate
        // Entries are ordered by pixel (x, y), with consecutive entries per pixel
        let mut pixel_index = 0u32;
        
        for entry in &mapping.entries {
            if entry.is_skip() {
                // SKIP entry - advance to next pixel
                pixel_index += 1;
                continue;
            }

            // Get pixel coordinates
            let x = pixel_index % texture_width;
            let y = pixel_index / texture_width;

            // Get pixel value from texture
            if let Some(pixel) = texture.get_pixel(x, y) {
                // Decode contribution: stored value represents (65535 - contribution_fractional)
                // We need to convert back to Q32 scale (0-65536)
                let stored = (entry.to_raw() >> 16) & 0xFFFF;
                let contribution_fractional = if stored == 0 {
                    65536u32 // 100% contribution in Q32 format
                } else {
                    // Scale from [0, 65534] to [0, 65535] in Q32 format
                    ((65535u32 - stored) as i64 * 65536 / 65535) as u32
                };

                // Accumulate: ch_value += contribution * pixel_value
                // contribution_fractional is 0-65536 in Q32 format (representing 0.0-1.0)
                // pixel values are 0-255 (u8)
                // Result should be: contribution * pixel_value (in range 0-255)
                let channel = entry.channel() as usize;
                if channel < ch_values_r.len() {
                    // Use 64-bit math to avoid overflow
                    let contribution = contribution_fractional as i64;
                    let pixel_r = pixel[0] as i64;
                    let pixel_g = pixel[1] as i64;
                    let pixel_b = pixel[2] as i64;
                    
                    // Calculate: (contribution * pixel) / 65536
                    // This gives us the weighted pixel value (0-255 range)
                    let accumulated_r = (contribution * pixel_r) / 65536;
                    let accumulated_g = (contribution * pixel_g) / 65536;
                    let accumulated_b = (contribution * pixel_b) / 65536;
                    
                    ch_values_r[channel] += accumulated_r as i32;
                    ch_values_g[channel] += accumulated_g as i32;
                    ch_values_b[channel] += accumulated_b as i32;
                }
            }

            // Advance pixel_index if this is the last entry for this pixel
            if !entry.has_more() {
                pixel_index += 1;
            }
        }

        // Get output handle
        let output_handle = self.output_handle.ok_or_else(|| Error::Other {
            message: String::from("Output handle not resolved"),
        })?;

        // Store lamp colors for state extraction
        // Create dense array: each channel uses 3 bytes (RGB)
        self.lamp_colors.clear();
        self.lamp_colors.resize((max_channel as usize + 1) * 3, 0);

        for channel in 0..=max_channel as usize {
            // Values are already in 0-255 range (accumulated as regular integers)
            // Just clamp to ensure they're in valid range
            let r = ch_values_r[channel].clamp(0, 255) as u8;
            let g = ch_values_g[channel].clamp(0, 255) as u8;
            let b = ch_values_b[channel].clamp(0, 255) as u8;

            let idx = channel * 3;
            self.lamp_colors[idx] = r;
            self.lamp_colors[idx + 1] = g;
            self.lamp_colors[idx + 2] = b;
        }

        // Write sampled values to output buffer
        // For now, use universe 0 and channel_offset 0 (sequential writing)
        // TODO: Add universe and channel_offset fields to FixtureConfig when needed
        let universe = 0u32;
        let channel_offset = 0u32;
        for channel in 0..=max_channel {
            let r = ch_values_r[channel as usize].clamp(0, 255) as u8;
            let g = ch_values_g[channel as usize].clamp(0, 255) as u8;
            let b = ch_values_b[channel as usize].clamp(0, 255) as u8;

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

        // Regenerate mapping if we have texture dimensions
        // If texture dimensions not available, mapping will be regenerated in render()
        if let (Some(width), Some(height)) = (self.texture_width, self.texture_height) {
            self.mapping = generate_mapping_points(&fixture_config.mapping, width, height);
        } else {
            // Texture dimensions not available, clear mapping - will be regenerated in render()
            self.mapping = Vec::new();
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
    use alloc::vec;
    use lp_model::nodes::fixture::mapping::{MappingConfig, PathSpec, RingOrder};

    #[test]
    fn test_fixture_runtime_creation() {
        let runtime = FixtureRuntime::new();
        let _boxed: alloc::boxed::Box<dyn NodeRuntime> = alloc::boxed::Box::new(runtime);
    }
    
    #[test]
    fn test_contribution_accumulation_math() {
        // Test the accumulation math directly
        // Simulate: pixel value = 200, contribution = 0.5 (50%)
        // Expected result: 200 * 0.5 = 100
        
        let pixel_value = 200u8;
        let contribution_fractional = 32768u32; // 0.5 in Q32 format (32768 / 65536 = 0.5)
        
        let contribution = contribution_fractional as i64;
        let pixel = pixel_value as i64;
        let accumulated = (contribution * pixel) / 65536;
        
        assert_eq!(accumulated, 100, "50% of 200 should be 100, got {}", accumulated);
    }
    
    #[test]
    fn test_contribution_accumulation_full() {
        // Test full contribution (100%)
        let pixel_value = 255u8;
        let contribution_fractional = 65536u32; // 1.0 in Q32 format
        
        let contribution = contribution_fractional as i64;
        let pixel = pixel_value as i64;
        let accumulated = (contribution * pixel) / 65536;
        
        assert_eq!(accumulated, 255, "100% of 255 should be 255, got {}", accumulated);
    }
    
    #[test]
    fn test_contribution_accumulation_zero() {
        // Test zero contribution (0%)
        let pixel_value = 255u8;
        let contribution_fractional = 0u32; // 0.0 in Q32 format
        
        let contribution = contribution_fractional as i64;
        let pixel = pixel_value as i64;
        let accumulated = (contribution * pixel) / 65536;
        
        assert_eq!(accumulated, 0, "0% of 255 should be 0, got {}", accumulated);
    }
    
    #[test]
    fn test_contribution_decoding() {
        // Test decoding stored contribution values
        use crate::nodes::fixture::mapping_compute::PixelMappingEntry;
        use lp_builtins::glsl::q32::types::q32::Q32;
        
        // Create entry with 0.5 contribution
        let entry = PixelMappingEntry::new(0, Q32::from_f32(0.5), false);
        let stored = (entry.to_raw() >> 16) & 0xFFFF;
        
        // Decode contribution using the same logic as render()
        let contribution_fractional = if stored == 0 {
            65536u32
        } else {
            ((65535u32 - stored) as i64 * 65536 / 65535) as u32
        };
        
        // Should be approximately 32768 (0.5 * 65536)
        // Allow some tolerance due to rounding
        let expected = 32768;
        let diff = (contribution_fractional as i32 - expected).abs();
        assert!(diff < 100, 
            "Decoded contribution should be ~32768 (0.5), got {} (diff: {})", 
            contribution_fractional, diff);
        
        // Test that it produces correct accumulation
        let pixel_value = 200u8;
        let contribution = contribution_fractional as i64;
        let pixel = pixel_value as i64;
        let accumulated = (contribution * pixel) / 65536;
        
        // Should be approximately 100 (0.5 * 200)
        assert!((accumulated - 100).abs() < 2, 
            "Accumulated value should be ~100, got {}", accumulated);
    }
    
    #[test]
    fn test_multiple_contributions_accumulation() {
        // Test that multiple contributions accumulate correctly
        // Simulate: pixel contributes 0.3 to channel 0, then 0.7 to channel 0
        // Expected: channel 0 should have 0.3 + 0.7 = 1.0 of the pixel value
        
        let pixel_value = 200u8;
        let contribution1 = (0.3 * 65536.0) as u32; // 0.3 in Q32
        let contribution2 = (0.7 * 65536.0) as u32; // 0.7 in Q32
        
        let mut ch_value = 0i32;
        
        // First contribution
        let acc1 = (contribution1 as i64 * pixel_value as i64) / 65536;
        ch_value += acc1 as i32;
        
        // Second contribution
        let acc2 = (contribution2 as i64 * pixel_value as i64) / 65536;
        ch_value += acc2 as i32;
        
        // Total should be approximately 200 (1.0 * 200), allowing for rounding error
        assert!((ch_value - 200).abs() <= 2, 
            "Multiple contributions should sum to ~200, got {} (rounding error)", ch_value);
    }
    
    #[test]
    fn test_simulated_rendering_loop() {
        // Simulate the actual rendering loop to catch any issues
        use crate::nodes::fixture::mapping_compute::{PixelMappingEntry, PrecomputedMapping};
        use lp_builtins::glsl::q32::types::q32::Q32;
        use lp_model::FrameId;
        
        // Create a simple mapping: one pixel contributes fully to channel 0
        let mut mapping = PrecomputedMapping::new(1, 1, FrameId::new(1));
        mapping.entries.push(PixelMappingEntry::new(0, Q32::from_f32(1.0), false));
        
        // Simulate pixel value = 200
        let pixel_value = [200u8, 200u8, 200u8, 255u8];
        
        // Simulate the rendering loop
        let mut ch_values_r: Vec<i32> = vec![0; 1];
        let mut ch_values_g: Vec<i32> = vec![0; 1];
        let mut ch_values_b: Vec<i32> = vec![0; 1];
        
        let mut pixel_index = 0u32;
        let texture_width = 1u32;
        
        for entry in &mapping.entries {
            if entry.is_skip() {
                pixel_index += 1;
                continue;
            }
            
            let x = pixel_index % texture_width;
            let y = pixel_index / texture_width;
            
            // Simulate getting pixel (we know it's pixel 0,0)
            if x == 0 && y == 0 {
                let stored = (entry.to_raw() >> 16) & 0xFFFF;
                let contribution_fractional = if stored == 0 {
                    65536u32
                } else {
                    ((65535u32 - stored) as i64 * 65536 / 65535) as u32
                };
                
                let channel = entry.channel() as usize;
                if channel < ch_values_r.len() {
                    let contribution = contribution_fractional as i64;
                    let pixel_r = pixel_value[0] as i64;
                    let pixel_g = pixel_value[1] as i64;
                    let pixel_b = pixel_value[2] as i64;
                    
                    let accumulated_r = (contribution * pixel_r) / 65536;
                    let accumulated_g = (contribution * pixel_g) / 65536;
                    let accumulated_b = (contribution * pixel_b) / 65536;
                    
                    ch_values_r[channel] += accumulated_r as i32;
                    ch_values_g[channel] += accumulated_g as i32;
                    ch_values_b[channel] += accumulated_b as i32;
                }
            }
            
            if !entry.has_more() {
                pixel_index += 1;
            }
        }
        
        // Channel 0 should have value 200 (100% of pixel value 200)
        assert_eq!(ch_values_r[0], 200, 
            "Channel 0 should have value 200, got {}", ch_values_r[0]);
        assert_eq!(ch_values_g[0], 200, 
            "Channel 0 should have value 200, got {}", ch_values_g[0]);
        assert_eq!(ch_values_b[0], 200, 
            "Channel 0 should have value 200, got {}", ch_values_b[0]);
    }
    
    #[test]
    fn test_simulated_rendering_multiple_pixels() {
        // Test with multiple pixels contributing to same channel
        // Pixel 0: contributes 0.5 to channel 0, value = 200
        // Pixel 1: contributes 0.5 to channel 0, value = 200
        // Expected: channel 0 should have 100 + 100 = 200
        
        use crate::nodes::fixture::mapping_compute::{PixelMappingEntry, PrecomputedMapping};
        use lp_builtins::glsl::q32::types::q32::Q32;
        use lp_model::FrameId;
        
        let mut mapping = PrecomputedMapping::new(2, 1, FrameId::new(1));
        // Pixel 0: 0.5 contribution to channel 0
        mapping.entries.push(PixelMappingEntry::new(0, Q32::from_f32(0.5), false));
        // Pixel 1: 0.5 contribution to channel 0
        mapping.entries.push(PixelMappingEntry::new(0, Q32::from_f32(0.5), false));
        
        let mut ch_values_r: Vec<i32> = vec![0; 1];
        let mut pixel_index = 0u32;
        let texture_width = 2u32;
        
        // Simulate pixels: both have value 200
        let pixels = [[200u8, 200u8, 200u8, 255u8], [200u8, 200u8, 200u8, 255u8]];
        
        for entry in &mapping.entries {
            if entry.is_skip() {
                pixel_index += 1;
                continue;
            }
            
            let x = pixel_index % texture_width;
            let pixel = pixels[x as usize];
            
            let stored = (entry.to_raw() >> 16) & 0xFFFF;
            let contribution_fractional = if stored == 0 {
                65536u32
            } else {
                ((65535u32 - stored) as i64 * 65536 / 65535) as u32
            };
            
            let channel = entry.channel() as usize;
            if channel < ch_values_r.len() {
                let contribution = contribution_fractional as i64;
                let pixel_r = pixel[0] as i64;
                let accumulated_r = (contribution * pixel_r) / 65536;
                ch_values_r[channel] += accumulated_r as i32;
            }
            
            if !entry.has_more() {
                pixel_index += 1;
            }
        }
        
        // Channel 0 should have value 200 (0.5 * 200 + 0.5 * 200)
        // Allow small rounding error
        assert!((ch_values_r[0] - 200).abs() <= 2, 
            "Channel 0 should have value ~200, got {}", ch_values_r[0]);
    }
    
    #[test]
    fn test_pixel_index_advancement() {
        // Test that pixel_index advances correctly
        // Simulate: pixel 0 has 2 entries (channels 0 and 1), pixel 1 has 1 entry (channel 0)
        use crate::nodes::fixture::mapping_compute::{PixelMappingEntry, PrecomputedMapping};
        use lp_builtins::glsl::q32::types::q32::Q32;
        use lp_model::FrameId;
        
        let mut mapping = PrecomputedMapping::new(2, 1, FrameId::new(1));
        // Pixel 0: channel 0 (has_more = true)
        mapping.entries.push(PixelMappingEntry::new(0, Q32::from_f32(0.5), true));
        // Pixel 0: channel 1 (has_more = false) - last entry for pixel 0
        mapping.entries.push(PixelMappingEntry::new(1, Q32::from_f32(0.5), false));
        // Pixel 1: channel 0 (has_more = false) - only entry for pixel 1
        mapping.entries.push(PixelMappingEntry::new(0, Q32::from_f32(1.0), false));
        
        let mut pixel_index = 0u32;
        let texture_width = 2u32;
        let mut processed_pixels = Vec::new();
        
        for entry in &mapping.entries {
            if entry.is_skip() {
                pixel_index += 1;
                continue;
            }
            
            let x = pixel_index % texture_width;
            processed_pixels.push((x, entry.channel()));
            
            if !entry.has_more() {
                pixel_index += 1;
            }
        }
        
        // Should process: pixel 0 (channel 0), pixel 0 (channel 1), pixel 1 (channel 0)
        assert_eq!(processed_pixels.len(), 3);
        assert_eq!(processed_pixels[0], (0, 0), "First entry should be pixel 0, channel 0");
        assert_eq!(processed_pixels[1], (0, 1), "Second entry should be pixel 0, channel 1");
        assert_eq!(processed_pixels[2], (1, 0), "Third entry should be pixel 1, channel 0");
    }
    
    #[test]
    fn test_normalization_verification() {
        // Verify that contributions decode correctly
        // NOTE: With per-channel normalization, a pixel's contributions to different channels
        // do NOT necessarily sum to 1.0. Each channel's total contribution from all pixels
        // sums to 1.0 instead.
        use crate::nodes::fixture::mapping_compute::{PixelMappingEntry, PrecomputedMapping};
        use lp_builtins::glsl::q32::types::q32::Q32;
        use lp_model::FrameId;
        
        // Create a mapping where pixel 0 contributes to channels 0 and 1
        let mut mapping = PrecomputedMapping::new(1, 1, FrameId::new(1));
        // Pixel 0: channel 0 with 0.3 contribution (has_more = true)
        mapping.entries.push(PixelMappingEntry::new(0, Q32::from_f32(0.3), true));
        // Pixel 0: channel 1 with 0.7 contribution (has_more = false)
        mapping.entries.push(PixelMappingEntry::new(1, Q32::from_f32(0.7), false));
        
        // Verify the contributions decode correctly
        let mut contributions = Vec::new();
        for entry in &mapping.entries {
            if !entry.is_skip() {
                let stored = (entry.to_raw() >> 16) & 0xFFFF;
                let contribution_fractional = if stored == 0 {
                    65536u32
                } else {
                    ((65535u32 - stored) as i64 * 65536 / 65535) as u32
                };
                let contribution_float = contribution_fractional as f64 / 65536.0;
                contributions.push(contribution_float);
            }
        }
        
        // Verify contributions decode to expected values (within rounding tolerance)
        assert_eq!(contributions.len(), 2, "Should have 2 contributions");
        assert!((contributions[0] - 0.3).abs() < 0.01, 
            "First contribution should be ~0.3, got {}", contributions[0]);
        assert!((contributions[1] - 0.7).abs() < 0.01, 
            "Second contribution should be ~0.7, got {}", contributions[1]);
    }
    
    #[test]
    fn test_channel_contribution_sum() {
        // Test that all pixel contributions to a channel sum correctly
        // Create a simple mapping: one circle (one channel) that covers some pixels
        use crate::nodes::fixture::mapping_compute::{compute_mapping, PixelMappingEntry};
        use lp_model::nodes::fixture::mapping::{MappingConfig, PathSpec, RingOrder};
        use lp_model::FrameId;
        
        // Create a simple config: one ring with 1 lamp at center
        let config = MappingConfig::PathPoints {
            paths: vec![PathSpec::RingArray {
                center: (0.5, 0.5),
                diameter: 0.2, // Small diameter
                start_ring_inclusive: 0,
                end_ring_exclusive: 1,
                ring_lamp_counts: vec![1],
                offset_angle: 0.0,
                order: RingOrder::InnerFirst,
            }],
            sample_diameter: 4.0, // Sample diameter in pixels
        };
        
        // Build mapping for a small texture
        let texture_width = 32u32;
        let texture_height = 32u32;
        let mapping = compute_mapping(&config, texture_width, texture_height, FrameId::new(1));
        
        // Sum up all contributions to channel 0 from all pixels
        let mut total_contribution_ch0 = 0.0f64;
        let mut pixel_index = 0u32;
        
        for entry in &mapping.entries {
            if entry.is_skip() {
                pixel_index += 1;
                continue;
            }
            
            if entry.channel() == 0 {
                // Decode contribution
                let stored = (entry.to_raw() >> 16) & 0xFFFF;
                let contribution_fractional = if stored == 0 {
                    65536u32
                } else {
                    ((65535u32 - stored) as i64 * 65536 / 65535) as u32
                };
                let contribution_float = contribution_fractional as f64 / 65536.0;
                total_contribution_ch0 += contribution_float;
            }
            
            if !entry.has_more() {
                pixel_index += 1;
            }
        }
        
        // After fixing normalization to be per-channel instead of per-pixel,
        // the total contribution to each channel should sum to approximately 1.0
        assert!((total_contribution_ch0 - 1.0).abs() < 0.1,
            "Total contribution to channel 0 should be ~1.0 (normalized per-channel), got {}", 
            total_contribution_ch0);
    }

    // Test helper: create RingArray path spec
    fn create_ring_array_path(
        center: (f32, f32),
        diameter: f32,
        start_ring: u32,
        end_ring: u32,
        ring_lamp_counts: Vec<u32>,
        offset_angle: f32,
        order: RingOrder,
    ) -> PathSpec {
        PathSpec::RingArray {
            center,
            diameter,
            start_ring_inclusive: start_ring,
            end_ring_exclusive: end_ring,
            ring_lamp_counts,
            offset_angle,
            order,
        }
    }

    #[test]
    fn test_single_ring_center() {
        // Single ring at center (ring_index = 0) with 8 lamps
        let path =
            create_ring_array_path((0.5, 0.5), 1.0, 0, 1, vec![8], 0.0, RingOrder::InnerFirst);
        let config = MappingConfig::PathPoints {
            paths: vec![path],
            sample_diameter: 2.0,
        };

        let points = generate_mapping_points(&config, 100, 100);

        // Verify 8 points generated
        assert_eq!(points.len(), 8);

        // Verify all points at center position (radius = 0 for single ring)
        for point in &points {
            assert!((point.center[0] - 0.5).abs() < 0.001);
            assert!((point.center[1] - 0.5).abs() < 0.001);
        }

        // Verify channels 0-7 assigned sequentially
        for (i, point) in points.iter().enumerate() {
            assert_eq!(point.channel, i as u32);
        }

        // Verify angles evenly spaced (0, π/4, π/2, ...)
        // Since all points are at center, angles don't matter, but verify structure
        assert_eq!(points[0].channel, 0);
        assert_eq!(points[7].channel, 7);
    }

    #[test]
    fn test_multiple_rings() {
        // Multiple rings with different lamp counts
        // Ring 0: 1 lamp (center)
        // Ring 1: 8 lamps
        // Ring 2: 16 lamps
        let path = create_ring_array_path(
            (0.5, 0.5),
            1.0,
            0,
            3,
            vec![1, 8, 16],
            0.0,
            RingOrder::InnerFirst,
        );
        let config = MappingConfig::PathPoints {
            paths: vec![path],
            sample_diameter: 2.0,
        };

        let points = generate_mapping_points(&config, 100, 100);

        // Verify correct number of points (1 + 8 + 16 = 25)
        assert_eq!(points.len(), 25);

        // Verify channels assigned sequentially (0-24)
        for (i, point) in points.iter().enumerate() {
            assert_eq!(point.channel, i as u32);
        }

        // Verify ring 0 (center) has 1 point at center
        assert_eq!(points[0].channel, 0);
        assert!((points[0].center[0] - 0.5).abs() < 0.001);
        assert!((points[0].center[1] - 0.5).abs() < 0.001);

        // Verify ring 1 has 8 points (channels 1-8)
        // Verify ring 2 has 16 points (channels 9-24)
        assert_eq!(points[1].channel, 1);
        assert_eq!(points[8].channel, 8);
        assert_eq!(points[9].channel, 9);
        assert_eq!(points[24].channel, 24);
    }

    #[test]
    fn test_inner_first_ordering() {
        // Multiple rings with different lamp counts
        let path = create_ring_array_path(
            (0.5, 0.5),
            1.0,
            0,
            3,
            vec![1, 4, 8],
            0.0,
            RingOrder::InnerFirst,
        );
        let config = MappingConfig::PathPoints {
            paths: vec![path],
            sample_diameter: 2.0,
        };

        let points = generate_mapping_points(&config, 100, 100);

        // Verify channels assigned inner→outer
        // Ring 0: channels 0-0 (1 lamp)
        // Ring 1: channels 1-4 (4 lamps)
        // Ring 2: channels 5-12 (8 lamps)
        assert_eq!(points[0].channel, 0); // Ring 0, first lamp
        assert_eq!(points[1].channel, 1); // Ring 1, first lamp
        assert_eq!(points[5].channel, 5); // Ring 2, first lamp
        assert_eq!(points[12].channel, 12); // Ring 2, last lamp
    }

    #[test]
    fn test_outer_first_ordering() {
        // Multiple rings with different lamp counts
        let path = create_ring_array_path(
            (0.5, 0.5),
            1.0,
            0,
            3,
            vec![1, 4, 8],
            0.0,
            RingOrder::OuterFirst,
        );
        let config = MappingConfig::PathPoints {
            paths: vec![path],
            sample_diameter: 2.0,
        };

        let points = generate_mapping_points(&config, 100, 100);

        // Verify channels assigned outer→inner
        // Ring 2: channels 0-7 (8 lamps, outer)
        // Ring 1: channels 8-11 (4 lamps)
        // Ring 0: channel 12 (1 lamp, inner)
        assert_eq!(points[0].channel, 0); // Ring 2, first lamp (outer)
        assert_eq!(points[7].channel, 7); // Ring 2, last lamp
        assert_eq!(points[8].channel, 8); // Ring 1, first lamp
        assert_eq!(points[11].channel, 11); // Ring 1, last lamp
        assert_eq!(points[12].channel, 12); // Ring 0, only lamp (inner)
    }

    #[test]
    fn test_offset_angle() {
        // Single ring with offset angle
        let path = create_ring_array_path(
            (0.5, 0.5),
            0.5,
            0,
            1,
            vec![4],
            core::f32::consts::PI / 4.0, // π/4 offset
            RingOrder::InnerFirst,
        );
        let config = MappingConfig::PathPoints {
            paths: vec![path],
            sample_diameter: 2.0,
        };

        let points = generate_mapping_points(&config, 100, 100);

        // Verify 4 points generated
        assert_eq!(points.len(), 4);

        // Verify first lamp at angle π/4 (not 0)
        // For ring at radius 0 (center), all points are at center, so angles don't affect position
        // But verify structure is correct
        assert_eq!(points[0].channel, 0);
        assert_eq!(points[3].channel, 3);
    }

    #[test]
    fn test_coordinate_correctness() {
        // Test coordinates are in [0, 1] range
        let path = create_ring_array_path(
            (0.5, 0.5),
            1.0,
            0,
            2,
            vec![1, 8],
            0.0,
            RingOrder::InnerFirst,
        );
        let config = MappingConfig::PathPoints {
            paths: vec![path],
            sample_diameter: 2.0,
        };

        let points = generate_mapping_points(&config, 100, 100);

        for point in &points {
            // Verify coordinates in [0, 1] range
            assert!(point.center[0] >= 0.0 && point.center[0] <= 1.0);
            assert!(point.center[1] >= 0.0 && point.center[1] <= 1.0);
            assert!(point.radius >= 0.0 && point.radius <= 1.0);
        }
    }

    #[test]
    fn test_coordinate_edge_cases() {
        // Test edge cases: center at (0, 0), (1, 1), (0.5, 0.5)
        for center in [(0.0, 0.0), (1.0, 1.0), (0.5, 0.5)] {
            let path =
                create_ring_array_path(center, 0.5, 0, 1, vec![4], 0.0, RingOrder::InnerFirst);
            let config = MappingConfig::PathPoints {
                paths: vec![path],
                sample_diameter: 2.0,
            };

            let points = generate_mapping_points(&config, 100, 100);

            for point in &points {
                // Verify coordinates clamped to [0, 1]
                assert!(point.center[0] >= 0.0 && point.center[0] <= 1.0);
                assert!(point.center[1] >= 0.0 && point.center[1] <= 1.0);
            }
        }
    }

    #[test]
    fn test_sample_diameter_conversion() {
        // Test sample diameter to normalized radius conversion
        let path =
            create_ring_array_path((0.5, 0.5), 1.0, 0, 1, vec![1], 0.0, RingOrder::InnerFirst);
        let config = MappingConfig::PathPoints {
            paths: vec![path],
            sample_diameter: 2.0,
        };

        // Test with square texture (100x100)
        let points_square = generate_mapping_points(&config, 100, 100);
        assert_eq!(points_square.len(), 1);
        // sample_diameter = 2.0, max_dimension = 100, normalized_radius = (2.0 / 2.0) / 100 = 0.01
        assert!((points_square[0].radius - 0.01).abs() < 0.0001);

        // Test with wide texture (200x100)
        let points_wide = generate_mapping_points(&config, 200, 100);
        assert_eq!(points_wide.len(), 1);
        // sample_diameter = 2.0, max_dimension = 200, normalized_radius = (2.0 / 2.0) / 200 = 0.005
        assert!((points_wide[0].radius - 0.005).abs() < 0.0001);

        // Test with tall texture (100x200)
        let points_tall = generate_mapping_points(&config, 100, 200);
        assert_eq!(points_tall.len(), 1);
        // sample_diameter = 2.0, max_dimension = 200, normalized_radius = (2.0 / 2.0) / 200 = 0.005
        assert!((points_tall[0].radius - 0.005).abs() < 0.0001);
    }

    #[test]
    fn test_channel_assignment_multiple_paths() {
        // Multiple paths with different LED counts
        let path1 =
            create_ring_array_path((0.5, 0.5), 1.0, 0, 1, vec![5], 0.0, RingOrder::InnerFirst);
        let path2 =
            create_ring_array_path((0.5, 0.5), 1.0, 0, 1, vec![3], 0.0, RingOrder::InnerFirst);
        let config = MappingConfig::PathPoints {
            paths: vec![path1, path2],
            sample_diameter: 2.0,
        };

        let points = generate_mapping_points(&config, 100, 100);

        // Verify channels sequential with no gaps
        // Path 1: channels 0-4 (5 LEDs)
        // Path 2: channels 5-7 (3 LEDs)
        assert_eq!(points.len(), 8);
        assert_eq!(points[0].channel, 0);
        assert_eq!(points[4].channel, 4);
        assert_eq!(points[5].channel, 5);
        assert_eq!(points[7].channel, 7);
    }

    #[test]
    fn test_channel_offset() {
        // Test with channel_offset > 0 (simulated by using generate_ring_array_points directly)
        let points = generate_ring_array_points(
            (0.5, 0.5),
            1.0,
            0,
            1,
            &vec![3],
            0.0,
            RingOrder::InnerFirst,
            2.0,
            100,
            100,
            10, // channel_offset = 10
        );

        assert_eq!(points.len(), 3);
        assert_eq!(points[0].channel, 10);
        assert_eq!(points[1].channel, 11);
        assert_eq!(points[2].channel, 12);
    }

    #[test]
    fn test_edge_cases_empty_ring() {
        // Test with zero lamp count for a ring
        let path = create_ring_array_path(
            (0.5, 0.5),
            1.0,
            0,
            2,
            vec![1, 0],
            0.0,
            RingOrder::InnerFirst,
        );
        let config = MappingConfig::PathPoints {
            paths: vec![path],
            sample_diameter: 2.0,
        };

        let points = generate_mapping_points(&config, 100, 100);

        // Should only generate 1 point (ring 0), ring 1 has 0 lamps
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].channel, 0);
    }

    #[test]
    fn test_edge_cases_invalid_ring_indices() {
        // Test with start_ring >= end_ring
        let path =
            create_ring_array_path((0.5, 0.5), 1.0, 2, 2, vec![], 0.0, RingOrder::InnerFirst);
        let config = MappingConfig::PathPoints {
            paths: vec![path],
            sample_diameter: 2.0,
        };

        let points = generate_mapping_points(&config, 100, 100);

        // Should generate no points (empty range)
        assert_eq!(points.len(), 0);
    }

    #[test]
    fn test_edge_cases_single_lamp() {
        // Test with single lamp in a ring
        let path =
            create_ring_array_path((0.5, 0.5), 1.0, 0, 1, vec![1], 0.0, RingOrder::InnerFirst);
        let config = MappingConfig::PathPoints {
            paths: vec![path],
            sample_diameter: 2.0,
        };

        let points = generate_mapping_points(&config, 100, 100);

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].channel, 0);
        assert!((points[0].center[0] - 0.5).abs() < 0.001);
        assert!((points[0].center[1] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_ring_radius_calculation() {
        // Test that ring radii increase correctly
        let path = create_ring_array_path(
            (0.5, 0.5),
            1.0,
            0,
            3,
            vec![1, 8, 16],
            0.0,
            RingOrder::InnerFirst,
        );
        let config = MappingConfig::PathPoints {
            paths: vec![path],
            sample_diameter: 2.0,
        };

        let points = generate_mapping_points(&config, 100, 100);

        // Ring 0 (center): radius should be 0
        assert!((points[0].center[0] - 0.5).abs() < 0.001);
        assert!((points[0].center[1] - 0.5).abs() < 0.001);

        // Ring 1: should have non-zero radius
        let ring1_radius =
            ((points[1].center[0] - 0.5).powi(2) + (points[1].center[1] - 0.5).powi(2)).sqrt();
        assert!(ring1_radius > 0.0);

        // Ring 2: should have larger radius than ring 1
        let ring2_radius =
            ((points[9].center[0] - 0.5).powi(2) + (points[9].center[1] - 0.5).powi(2)).sqrt();
        assert!(ring2_radius > ring1_radius);
    }
}
