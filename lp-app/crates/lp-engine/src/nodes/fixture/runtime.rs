//! Fixture node runtime

use crate::error::Error;
use crate::nodes::fixture::config::{FixtureNode, Mapping};
use crate::project::runtime::NodeStatus;
use crate::runtime::NodeRuntimeBase;
use crate::runtime::contexts::FixtureRenderContext;
use crate::runtime::lifecycle::NodeLifecycle;
use alloc::{format, string::String, vec::Vec};
use lp_shared::nodes::handle::NodeHandle;
use lp_shared::nodes::id::{OutputId, TextureId};
use lp_shared::project::frame_id::FrameId;

/// Precomputed sample point for texture sampling
#[derive(Debug, Clone)]
pub struct SamplePoint {
    /// Relative offset in U coordinate (normalized)
    pub offset_u: f32,
    /// Relative offset in V coordinate (normalized)
    pub offset_v: f32,
    /// Weight for this sample
    pub weight: f32,
}

/// Sampling kernel for texture sampling
///
/// Precomputed sample points in a circle, reused for all mapping points.
#[derive(Debug, Clone)]
pub struct SamplingKernel {
    /// Normalized sampling radius (same for all pixels)
    pub radius: f32,
    /// Precomputed sample points
    pub samples: Vec<SamplePoint>,
}

impl SamplingKernel {
    /// Create a new sampling kernel with the given radius
    ///
    /// Generates sample points in a circle using a simple grid pattern.
    /// The radius is normalized (0.0 to 1.0).
    pub fn new(radius: f32) -> Self {
        // Generate sample points in a circle
        // Use a simple approach: sample on a grid within the circle
        let mut samples = Vec::new();

        // Number of samples per dimension (creates a square grid)
        let sample_count = 5; // 5x5 = 25 samples

        // Total weight for normalization
        let mut total_weight = 0.0;

        for i in 0..sample_count {
            for j in 0..sample_count {
                // Map from [0, sample_count-1] to [-radius, radius]
                let u = (i as f32 / (sample_count - 1) as f32) * 2.0 - 1.0;
                let v = (j as f32 / (sample_count - 1) as f32) * 2.0 - 1.0;

                // Check if point is within circle
                let dist = (u * u + v * v).sqrt();
                if dist <= 1.0 {
                    // Scale by radius
                    let offset_u = u * radius;
                    let offset_v = v * radius;

                    // Weight: closer to center = higher weight (Gaussian-like)
                    let weight = 1.0 - (dist * dist);
                    total_weight += weight;

                    samples.push(SamplePoint {
                        offset_u,
                        offset_v,
                        weight,
                    });
                }
            }
        }

        // Normalize weights so they sum to 1.0
        if total_weight > 0.0 {
            for sample in &mut samples {
                sample.weight /= total_weight;
            }
        }

        Self { radius, samples }
    }
}

/// Fixture node runtime
pub struct FixtureNodeRuntime {
    pub base: NodeRuntimeBase,
    config: FixtureNode,
    output_handle: NodeHandle,
    texture_handle: NodeHandle,
    kernel: SamplingKernel,
    channel_order: String,
    mapping: Vec<Mapping>,
    status: NodeStatus,
}

impl FixtureNodeRuntime {
    /// Create a new fixture node runtime (uninitialized)
    pub fn new(handle: NodeHandle, path: String) -> Self {
        Self {
            base: NodeRuntimeBase::new(handle, path, FrameId(0)), // Will be updated in init
            config: FixtureNode::CircleList {
                output_id: OutputId(String::new()),
                texture_id: TextureId(String::new()),
                channel_order: String::new(),
                mapping: Vec::new(),
            }, // Temporary, will be replaced in init
            output_handle: NodeHandle::NONE,
            texture_handle: NodeHandle::NONE,
            kernel: SamplingKernel::new(0.1), // Default small radius
            channel_order: String::new(),
            mapping: Vec::new(),
            status: NodeStatus::Ok,
        }
    }

    /// Get the handle for this node
    pub fn handle(&self) -> NodeHandle {
        self.base.handle
    }

    /// Get the path for this node
    pub fn path(&self) -> &str {
        &self.base.path
    }

    /// Set the creation frame (called by ProjectRuntime after init)
    pub fn set_creation_frame(&mut self, frame: FrameId) {
        self.base.created_frame = frame;
        self.base.last_config_frame = frame;
        self.base.last_state_frame = frame;
    }

    /// Update the last state frame (called when fixture samples and writes)
    pub fn mark_state_changed(&mut self, frame: FrameId) {
        self.base.update_state_frame(frame);
    }

    /// Get the current status
    pub fn status(&self) -> &NodeStatus {
        &self.status
    }

    /// Get the fixture configuration
    pub fn config(&self) -> &FixtureNode {
        &self.config
    }

    /// Initialize with handle resolution (called by ProjectRuntime)
    ///
    /// Resolves texture_id and output_id to handles using the provided mappings.
    pub fn init_with_handle_resolution(
        &mut self,
        config: &FixtureNode,
        ctx: &crate::runtime::contexts::InitContext,
        texture_id_to_handle: &hashbrown::HashMap<TextureId, NodeHandle>,
        output_id_to_handle: &hashbrown::HashMap<OutputId, NodeHandle>,
    ) -> Result<(), Error> {
        // Resolve IDs to handles
        match config {
            FixtureNode::CircleList {
                texture_id,
                output_id,
                ..
            } => {
                self.texture_handle = texture_id_to_handle
                    .get(texture_id)
                    .copied()
                    .unwrap_or(NodeHandle::NONE);
                self.output_handle = output_id_to_handle
                    .get(output_id)
                    .copied()
                    .unwrap_or(NodeHandle::NONE);

                if self.texture_handle == NodeHandle::NONE {
                    let texture_path: String = texture_id.clone().into();
                    return Err(Error::Validation(format!(
                        "Texture {} not found",
                        texture_path
                    )));
                }
                if self.output_handle == NodeHandle::NONE {
                    let output_path: String = output_id.clone().into();
                    return Err(Error::Validation(format!(
                        "Output {} not found",
                        output_path
                    )));
                }
            }
        }

        // Call regular init (which will set up kernel, etc.)
        self.init(config, ctx)
    }
}

impl Default for FixtureNodeRuntime {
    fn default() -> Self {
        Self::new(NodeHandle::NONE, String::new())
    }
}

impl NodeLifecycle for FixtureNodeRuntime {
    type Config = FixtureNode;
    type RenderContext<'a> = FixtureRenderContext<'a>;

    fn init(
        &mut self,
        config: &Self::Config,
        _ctx: &crate::runtime::contexts::InitContext,
    ) -> Result<(), Error> {
        // Store config
        self.config = config.clone();

        match config {
            FixtureNode::CircleList {
                channel_order,
                mapping,
                ..
            } => {
                // output_handle and texture_handle already set by init_with_handle_resolution
                self.channel_order = channel_order.clone();
                self.mapping = mapping.clone();

                // Precompute sampling kernel from the first mapping's radius
                // (all mappings use the same kernel, normalized by their radius)
                if let Some(first_mapping) = mapping.first() {
                    // Normalize radius: assume texture coordinates are [0, 1]
                    // So radius is already normalized if it's in texture space
                    // For now, use a fixed normalized radius based on the mapping radius
                    // In practice, we'd need texture dimensions to properly normalize
                    // For now, assume radius is already normalized (0.0 to 1.0)
                    let normalized_radius = first_mapping.radius.min(1.0).max(0.0);
                    self.kernel = SamplingKernel::new(normalized_radius);
                } else {
                    // No mappings, use default small radius
                    self.kernel = SamplingKernel::new(0.1);
                }

                self.status = NodeStatus::Ok;
                Ok(())
            }
        }
    }

    fn render(&mut self, ctx: &mut Self::RenderContext<'_>) -> Result<(), Error> {
        // Get texture (read-only) and sample all pixels first
        let texture = match ctx.get_texture(self.texture_handle) {
            Some(tex) => tex,
            None => {
                self.status = NodeStatus::Error {
                    status_message: format!("Texture handle {} not found", self.texture_handle.0),
                };
                return Err(Error::Node(format!(
                    "Texture handle {} not found",
                    self.texture_handle.0
                )));
            }
        };

        let texture_width = texture.width() as f32;
        let texture_height = texture.height() as f32;

        // Sample all mapping points and collect results
        let mut sampled_values: Vec<(u32, [u8; 4])> = Vec::new();

        for mapping in &self.mapping {
            let center_u = mapping.center[0];
            let center_v = mapping.center[1];
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

            // Normalize by total weight (should be ~1.0, but handle edge cases)
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

        // Now get output buffer and write all values (mutable borrow)
        let (buffer, bytes_per_pixel) = match ctx.get_output_mut(self.output_handle) {
            Some(out) => {
                let bytes_per_pixel = out.bytes_per_pixel();
                let buffer = out.buffer_mut();
                (buffer, bytes_per_pixel)
            }
            None => {
                self.status = NodeStatus::Error {
                    status_message: format!("Output handle {} not found", self.output_handle.0),
                };
                return Err(Error::Node(format!(
                    "Output handle {} not found",
                    self.output_handle.0
                )));
            }
        };

        // Write sampled values to output buffer
        for (channel, [r, g, b, a]) in sampled_values {
            let offset = (channel as usize) * bytes_per_pixel;

            if offset + bytes_per_pixel <= buffer.len() {
                match self.channel_order.as_str() {
                    "rgb" | "RGB" => {
                        buffer[offset] = r;
                        buffer[offset + 1] = g;
                        buffer[offset + 2] = b;
                    }
                    "rgba" | "RGBA" => {
                        if bytes_per_pixel >= 4 {
                            buffer[offset] = r;
                            buffer[offset + 1] = g;
                            buffer[offset + 2] = b;
                            buffer[offset + 3] = a;
                        } else {
                            // Fallback to RGB if output doesn't support alpha
                            buffer[offset] = r;
                            if bytes_per_pixel >= 2 {
                                buffer[offset + 1] = g;
                            }
                            if bytes_per_pixel >= 3 {
                                buffer[offset + 2] = b;
                            }
                        }
                    }
                    _ => {
                        // Default to RGB
                        buffer[offset] = r;
                        if bytes_per_pixel >= 2 {
                            buffer[offset + 1] = g;
                        }
                        if bytes_per_pixel >= 3 {
                            buffer[offset + 2] = b;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn destroy(&mut self) -> Result<(), Error> {
        // No cleanup needed
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::output::OutputNode;
    use crate::nodes::texture::{TextureNode, formats};
    use alloc::{string::ToString, vec};
    use hashbrown::HashMap;

    #[test]
    fn test_sampling_kernel_new() {
        let kernel = SamplingKernel::new(0.5);
        assert!(!kernel.samples.is_empty());
        assert_eq!(kernel.radius, 0.5);

        // Check that weights sum to approximately 1.0
        let total_weight: f32 = kernel.samples.iter().map(|s| s.weight).sum();
        assert!((total_weight - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_fixture_node_runtime_init() {
        let mut runtime =
            FixtureNodeRuntime::new(NodeHandle::NONE, "/test/fixture.fixture".to_string());
        let builder = crate::project::builder::ProjectBuilder::new_test();
        let (builder, output_id) = builder.add_output(OutputNode::GpioStrip {
            chip: "ws2812".to_string(),
            gpio_pin: 4,
            count: 128,
        });
        let (builder, texture_id) = builder.add_texture(TextureNode::Memory {
            size: [64, 64],
            format: formats::RGB8.to_string(),
        });
        let config = FixtureNode::CircleList {
            output_id: output_id.clone(),
            texture_id: texture_id.clone(),
            channel_order: "rgb".to_string(),
            mapping: vec![Mapping {
                channel: 0,
                center: [0.5, 0.5],
                radius: 0.1,
            }],
        };
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        let project_config = builder.build().unwrap();
        let ctx = crate::runtime::contexts::InitContext::new(
            &project_config,
            &textures,
            &shaders,
            &outputs,
            &fixtures,
        );

        assert!(runtime.init(&config, &ctx).is_ok());
        // Note: output_handle and texture_handle are set by init_with_handle_resolution, not regular init
        // For now, just verify other fields are set correctly
        assert_eq!(runtime.channel_order, "rgb");
        assert_eq!(runtime.mapping.len(), 1);
        assert!(!runtime.kernel.samples.is_empty());
        assert!(matches!(runtime.status(), NodeStatus::Ok));
    }

    #[test]
    fn test_fixture_node_runtime_update_samples_texture() {
        // Create texture runtime with a test pattern
        let mut texture_runtime = crate::nodes::texture::TextureNodeRuntime::new(
            NodeHandle::NONE,
            "/test/texture.texture".to_string(),
        );
        let texture_config = crate::nodes::texture::TextureNode::Memory {
            size: [10, 10],
            format: formats::RGBA8.to_string(),
        };
        let (builder, texture_id) =
            crate::project::builder::ProjectBuilder::new_test().add_texture(texture_config.clone());
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        let project_config = builder.build().unwrap();
        let init_ctx = crate::runtime::contexts::InitContext::new(
            &project_config,
            &textures,
            &shaders,
            &outputs,
            &fixtures,
        );
        texture_runtime.init(&texture_config, &init_ctx).unwrap();

        // Fill texture with a test pattern (red in center)
        for y in 0..10 {
            for x in 0..10 {
                let color = if x == 5 && y == 5 {
                    [255, 0, 0, 255] // Red at center
                } else {
                    [0, 0, 0, 0] // Black elsewhere
                };
                texture_runtime.texture_mut().set_pixel(x, y, color);
            }
        }

        // Create output runtime
        let mut output_runtime = crate::nodes::output::OutputNodeRuntime::new(
            NodeHandle::NONE,
            "/test/output.output".to_string(),
        );
        let output_config = crate::nodes::output::OutputNode::GpioStrip {
            chip: "ws2812".to_string(),
            gpio_pin: 18,
            count: 10,
        };
        let (builder, output_id) =
            crate::project::builder::ProjectBuilder::new_test().add_output(output_config.clone());
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        let project_config = builder.build().unwrap();
        let init_ctx = crate::runtime::contexts::InitContext::new(
            &project_config,
            &textures,
            &shaders,
            &outputs,
            &fixtures,
        );
        output_runtime.init(&output_config, &init_ctx).unwrap();

        // Create fixture runtime
        let mut fixture_runtime = FixtureNodeRuntime::new();
        let fixture_config = FixtureNode::CircleList {
            output_id: output_id.clone(),
            texture_id: texture_id.clone(),
            channel_order: "rgb".to_string(),
            mapping: vec![Mapping {
                channel: 0,
                center: [0.5, 0.5], // Center of texture
                radius: 0.2,
            }],
        };
        let (builder, _texture_id) =
            crate::project::builder::ProjectBuilder::new_test().add_texture(texture_config);
        let (builder, _output_id) = builder.add_output(output_config);
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        let project_config = builder.build().unwrap();
        let init_ctx = crate::runtime::contexts::InitContext::new(
            &project_config,
            &textures,
            &shaders,
            &outputs,
            &fixtures,
        );
        fixture_runtime.init(&fixture_config, &init_ctx).unwrap();

        // Create render context with handles
        let frame_time = crate::runtime::frame_time::FrameTime::new(16, 1000);
        let texture_handle = texture_runtime.handle();
        let output_handle = output_runtime.node_handle();
        let mut textures: HashMap<NodeHandle, crate::nodes::texture::TextureNodeRuntime> =
            HashMap::new();
        textures.insert(texture_handle, texture_runtime);
        let mut outputs: HashMap<NodeHandle, crate::nodes::output::OutputNodeRuntime> =
            HashMap::new();
        outputs.insert(output_handle, output_runtime);
        let mut ctx = FixtureRenderContext::new(frame_time, &textures, &mut outputs);

        // Set handles in fixture runtime (normally done by init_with_handle_resolution)
        fixture_runtime.texture_handle = texture_handle;
        fixture_runtime.output_handle = output_handle;

        // Update fixture
        assert!(fixture_runtime.render(&mut ctx).is_ok());

        // Check that output buffer was written
        let output = ctx.outputs.get_mut(&output_handle).unwrap();
        let buffer = output.buffer_mut();
        // Channel 0 should have some red value (sampled from center)
        assert!(buffer[0] > 0 || buffer[1] > 0 || buffer[2] > 0);
    }

    #[test]
    fn test_fixture_node_runtime_update_missing_texture() {
        let mut runtime =
            FixtureNodeRuntime::new(NodeHandle::NONE, "/test/fixture.fixture".to_string());
        runtime.texture_handle = NodeHandle::new(999); // Non-existent texture handle

        let frame_time = crate::runtime::frame_time::FrameTime::new(16, 1000);
        let textures: HashMap<NodeHandle, crate::nodes::texture::TextureNodeRuntime> =
            HashMap::new();
        let mut outputs: HashMap<NodeHandle, crate::nodes::output::OutputNodeRuntime> =
            HashMap::new();
        let mut ctx = FixtureRenderContext::new(frame_time, &textures, &mut outputs);

        assert!(runtime.render(&mut ctx).is_err());
        assert!(matches!(runtime.status(), NodeStatus::Error { .. }));
    }

    #[test]
    fn test_fixture_node_runtime_update_missing_output() {
        let mut runtime =
            FixtureNodeRuntime::new(NodeHandle::NONE, "/test/fixture.fixture".to_string());
        runtime.output_handle = NodeHandle::new(999); // Non-existent output handle

        let frame_time = crate::runtime::frame_time::FrameTime::new(16, 1000);
        let textures: HashMap<NodeHandle, crate::nodes::texture::TextureNodeRuntime> =
            HashMap::new();
        let mut outputs: HashMap<NodeHandle, crate::nodes::output::OutputNodeRuntime> =
            HashMap::new();
        let mut ctx = FixtureRenderContext::new(frame_time, &textures, &mut outputs);

        assert!(runtime.render(&mut ctx).is_err());
        assert!(matches!(runtime.status(), NodeStatus::Error { .. }));
    }
}
