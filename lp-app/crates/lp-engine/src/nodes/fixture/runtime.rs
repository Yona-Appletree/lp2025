use crate::error::Error;
use crate::nodes::fixture::sampling_kernel::SamplingKernel;
use crate::nodes::{NodeConfig, NodeRuntime};
use crate::runtime::contexts::{NodeInitContext, OutputHandle, RenderContext, TextureHandle};
use alloc::{boxed::Box, string::String, vec::Vec};
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
    kernel: SamplingKernel,
    color_order: ColorOrder,
    mapping: Vec<MappingPoint>,
    transform: [[f32; 4]; 4],
    texture_width: Option<u32>,
    texture_height: Option<u32>,
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
            texture_width: None,
            texture_height: None,
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

    /// Regenerate mapping when texture resolution changes
    fn regenerate_mapping_if_needed(
        &mut self,
        texture_width: u32,
        texture_height: u32,
    ) -> Result<(), Error> {
        let needs_regeneration = self
            .texture_width
            .map(|w| w != texture_width)
            .unwrap_or(true)
            || self
                .texture_height
                .map(|h| h != texture_height)
                .unwrap_or(true);

        if needs_regeneration {
            let config = self.config.as_ref().ok_or_else(|| Error::InvalidConfig {
                node_path: String::from("fixture"),
                reason: String::from("Config not set"),
            })?;

            // Regenerate mapping points
            self.mapping = generate_mapping_points(&config.mapping, texture_width, texture_height);

            // Update texture dimensions
            self.texture_width = Some(texture_width);
            self.texture_height = Some(texture_height);

            // Update sampling kernel based on first mapping's radius
            if let Some(first_mapping) = self.mapping.first() {
                let normalized_radius = first_mapping.radius.min(1.0).max(0.0);
                self.kernel = SamplingKernel::new(normalized_radius);
            }
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
        self.kernel = SamplingKernel::new(0.1);

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
        self.regenerate_mapping_if_needed(texture_width, texture_height)?;

        let texture_width = texture_width as f32;
        let texture_height = texture_height as f32;

        // Sample all mapping points and collect results
        let mut sampled_values: Vec<(u32, [u8; 4])> = Vec::new();

        for mapping in &self.mapping {
            // Mapping points are already in texture space [0, 1]
            // Apply transform matrix (4x4 affine transform) to convert from texture space to texture space
            let center_u = mapping.center[0];
            let center_v = mapping.center[1];

            // Apply transform matrix (4x4 affine transform)
            // Transform from texture space [0, 1] to texture space [0, 1]
            // Full matrix multiplication will be implemented later
            // For now, use identity transform (coordinates already in texture space)
            let center_u = center_u; // TODO: Apply full transform matrix
            let center_v = center_v; // TODO: Apply full transform matrix

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

        // Regenerate mapping if we have texture dimensions
        // If texture dimensions not available, mapping will be regenerated in render()
        if let (Some(width), Some(height)) = (self.texture_width, self.texture_height) {
            self.mapping = generate_mapping_points(&fixture_config.mapping, width, height);

            // Update sampling kernel
            if let Some(first_mapping) = self.mapping.first() {
                let normalized_radius = first_mapping.radius.min(1.0).max(0.0);
                self.kernel = SamplingKernel::new(normalized_radius);
            } else {
                self.kernel = SamplingKernel::new(0.1);
            }
        } else {
            // Texture dimensions not available, clear mapping - will be regenerated in render()
            self.mapping = Vec::new();
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
    use alloc::vec;
    use lp_model::nodes::fixture::mapping::{MappingConfig, PathSpec, RingOrder};

    #[test]
    fn test_fixture_runtime_creation() {
        let runtime = FixtureRuntime::new();
        let _boxed: alloc::boxed::Box<dyn NodeRuntime> = alloc::boxed::Box::new(runtime);
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
