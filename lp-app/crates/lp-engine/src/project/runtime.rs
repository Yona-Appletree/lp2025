//! Project runtime - manages lifecycle of all nodes

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::nodes::{
    FixtureNode, FixtureNodeRuntime, OutputNode, OutputNodeRuntime, ShaderNode, ShaderNodeRuntime,
    TextureNode, TextureNodeRuntime,
};
use crate::runtime::contexts::{
    FixtureRenderContext, InitContext, OutputRenderContext, ShaderRenderContext,
};
use crate::runtime::frame_time::FrameTime;
use crate::runtime::lifecycle::NodeLifecycle;
use crate::traits::OutputProvider;
use lp_shared::nodes::handle::NodeHandle;
use lp_shared::nodes::id::{FixtureId, OutputId, ShaderId, TextureId};
use lp_shared::project::api::{EngineStats, NodeDetail};
use lp_shared::project::config::ProjectConfig;
use lp_shared::project::frame_id::FrameId;

/// Project runtime - manages lifecycle of all node runtimes
pub struct ProjectRuntime {
    #[allow(dead_code)] // Used for serialization via get_runtime_nodes
    uid: String,
    frame_time: FrameTime,
    current_frame: FrameId,
    next_handle: i32,
    // ID -> Handle mappings for resolving references at init time
    texture_id_to_handle: HashMap<TextureId, NodeHandle>,
    shader_id_to_handle: HashMap<ShaderId, NodeHandle>,
    fixture_id_to_handle: HashMap<FixtureId, NodeHandle>,
    output_id_to_handle: HashMap<OutputId, NodeHandle>,
    textures: HashMap<NodeHandle, TextureNodeRuntime>,
    shaders: HashMap<NodeHandle, ShaderNodeRuntime>,
    fixtures: HashMap<NodeHandle, FixtureNodeRuntime>,
    outputs: HashMap<NodeHandle, OutputNodeRuntime>,
}

impl ProjectRuntime {
    /// Create a new empty runtime for a project
    pub fn new(uid: String) -> Self {
        Self {
            uid,
            frame_time: FrameTime::new(0, 0),
            current_frame: FrameId(0),
            next_handle: 0,
            texture_id_to_handle: HashMap::new(),
            shader_id_to_handle: HashMap::new(),
            fixture_id_to_handle: HashMap::new(),
            output_id_to_handle: HashMap::new(),
            textures: HashMap::new(),
            shaders: HashMap::new(),
            fixtures: HashMap::new(),
            outputs: HashMap::new(),
        }
    }

    /// Initialize runtime with project config and loaded nodes
    ///
    /// Initializes nodes in order: textures → shaders → fixtures → outputs
    /// Allows partial failures (nodes handle their own failures)
    pub fn init(
        &mut self,
        config: &ProjectConfig,
        textures: &BTreeMap<String, TextureNode>,
        shaders: &BTreeMap<String, ShaderNode>,
        outputs: &BTreeMap<String, OutputNode>,
        fixtures: &BTreeMap<String, FixtureNode>,
        output_provider: &dyn OutputProvider,
    ) -> Result<(), Error> {
        log::info!(
            "Initializing runtime for project: {} ({})",
            config.name,
            config.uid
        );
        let init_ctx = InitContext::new(config, textures, shaders, outputs, fixtures);
        let current_frame = self.current_frame;

        // Initialize textures (first pass - assign handles and build ID mapping)
        log::debug!("Initializing {} texture(s)", textures.len());
        for (id_str, texture_config) in textures {
            let texture_id = TextureId(id_str.clone());
            let handle = self.assign_next_handle();
            let path = id_str.clone();
            self.texture_id_to_handle.insert(texture_id.clone(), handle);

            let mut texture_runtime = TextureNodeRuntime::new(handle, path);
            if let Err(e) = texture_runtime.init(texture_config, &init_ctx) {
                log::warn!("Failed to initialize texture {}: {}", id_str, e);
                // Continue - node status is set internally
            } else {
                texture_runtime.set_creation_frame(current_frame);
            }
            self.textures.insert(handle, texture_runtime);
        }

        // Initialize shaders (resolve texture references to handles)
        log::debug!("Initializing {} shader(s)", shaders.len());
        for (id_str, shader_config) in shaders {
            let shader_id = ShaderId(id_str.clone());
            let handle = self.assign_next_handle();
            let path = id_str.clone();
            self.shader_id_to_handle.insert(shader_id.clone(), handle);

            let mut shader_runtime = ShaderNodeRuntime::new(handle, path);
            // Resolve texture_id to handle before init
            if let Err(e) = shader_runtime.init_with_handle_resolution(
                shader_config,
                &init_ctx,
                &self.texture_id_to_handle,
            ) {
                log::warn!("Failed to initialize shader {}: {}", id_str, e);
                // Continue - node status is set internally
            } else {
                shader_runtime.set_creation_frame(current_frame);
            }
            self.shaders.insert(handle, shader_runtime);
        }

        // Initialize fixtures (resolve texture and output references to handles)
        log::debug!("Initializing {} fixture(s)", fixtures.len());
        for (id_str, fixture_config) in fixtures {
            let fixture_id = FixtureId(id_str.clone());
            let handle = self.assign_next_handle();
            let path = id_str.clone();
            self.fixture_id_to_handle.insert(fixture_id.clone(), handle);

            let mut fixture_runtime = FixtureNodeRuntime::new(handle, path);
            // Resolve texture_id and output_id to handles before init
            if let Err(e) = fixture_runtime.init_with_handle_resolution(
                fixture_config,
                &init_ctx,
                &self.texture_id_to_handle,
                &self.output_id_to_handle,
            ) {
                log::warn!("Failed to initialize fixture {}: {}", id_str, e);
                // Continue - node status is set internally
            } else {
                fixture_runtime.set_creation_frame(current_frame);
            }
            self.fixtures.insert(handle, fixture_runtime);
        }

        // Initialize outputs and create LED handles
        log::debug!("Initializing {} output(s)", outputs.len());
        for (id_str, output_config) in outputs {
            let output_id = OutputId(id_str.clone());
            let handle = self.assign_next_handle();
            let path = id_str.clone();
            self.output_id_to_handle.insert(output_id.clone(), handle);

            let mut output_runtime = OutputNodeRuntime::new(handle, path);
            if let Err(e) = output_runtime.init(output_config, &init_ctx) {
                log::warn!("Failed to initialize output {}: {}", id_str, e);
                // Continue - node status is set internally
            } else {
                output_runtime.set_creation_frame(current_frame);
                // Create LED output handle via OutputProvider
                // Note: OutputProvider still uses OutputId for now, will be updated later
                match output_provider.create_output(output_config, Some(output_id.clone())) {
                    Ok(led_handle) => {
                        output_runtime.set_handle(led_handle);
                    }
                    Err(e) => {
                        log::warn!("Failed to create output handle for {}: {}", id_str, e);
                        // Continue
                    }
                }
            }
            self.outputs.insert(handle, output_runtime);
        }

        log::info!(
            "Runtime initialized: {} texture(s), {} shader(s), {} fixture(s), {} output(s)",
            self.textures.len(),
            self.shaders.len(),
            self.fixtures.len(),
            self.outputs.len()
        );

        Ok(())
    }

    /// Update all nodes
    ///
    /// Updates nodes in order: shaders → fixtures → outputs
    /// Updates frame_time: total_ms += delta_ms, delta_ms = delta_ms
    /// Increments current_frame each update cycle
    pub fn update(
        &mut self,
        delta_ms: u32,
        _output_provider: &dyn OutputProvider,
    ) -> Result<(), Error> {
        // Increment frame ID
        self.current_frame = FrameId(self.current_frame.0 + 1);

        // Update frame time
        self.frame_time.total_ms += delta_ms;
        self.frame_time.delta_ms = delta_ms;

        // Update shaders (write to textures)
        for shader_runtime in self.shaders.values_mut() {
            let mut ctx = ShaderRenderContext::new(self.frame_time, &mut self.textures);
            if let Err(_e) = shader_runtime.render(&mut ctx) {
                // Error status is set internally
            }
        }

        // Update fixtures (sample textures, write to outputs)
        for fixture_runtime in self.fixtures.values_mut() {
            let mut ctx =
                FixtureRenderContext::new(self.frame_time, &self.textures, &mut self.outputs);
            if let Err(_e) = fixture_runtime.render(&mut ctx) {
                // Error status is set internally
            }
        }

        // Update outputs (send buffer to hardware)
        for output_runtime in self.outputs.values_mut() {
            let mut ctx = OutputRenderContext::new(self.frame_time);
            if let Err(_e) = output_runtime.render(&mut ctx) {
                // Error status is set internally
            }
        }

        Ok(())
    }

    /// Destroy all nodes
    ///
    /// Calls destroy() on all nodes in reverse order: outputs → fixtures → shaders → textures
    pub fn destroy(&mut self) -> Result<(), Error> {
        // Destroy outputs
        for output_runtime in self.outputs.values_mut() {
            let _ = output_runtime.destroy();
        }

        // Destroy fixtures
        for fixture_runtime in self.fixtures.values_mut() {
            let _ = fixture_runtime.destroy();
        }

        // Destroy shaders
        for shader_runtime in self.shaders.values_mut() {
            let _ = shader_runtime.destroy();
        }

        // Destroy textures
        for texture_runtime in self.textures.values_mut() {
            let _ = texture_runtime.destroy();
        }

        Ok(())
    }

    /// Get runtime nodes status for serialization
    ///
    /// Derives RuntimeNodes from runtime instances
    pub fn get_runtime_nodes(&self) -> RuntimeNodes {
        let mut runtime_nodes = RuntimeNodes {
            outputs: BTreeMap::new(),
            textures: BTreeMap::new(),
            shaders: BTreeMap::new(),
            fixtures: BTreeMap::new(),
        };

        // Collect status from texture runtimes (use path from runtime)
        for (handle, runtime) in &self.textures {
            runtime_nodes
                .textures
                .insert(runtime.path().to_string(), runtime.status().clone());
        }

        // Collect status from shader runtimes
        for (handle, runtime) in &self.shaders {
            runtime_nodes
                .shaders
                .insert(runtime.path().to_string(), runtime.status().clone());
        }

        // Collect status from fixture runtimes
        for (handle, runtime) in &self.fixtures {
            runtime_nodes
                .fixtures
                .insert(runtime.path().to_string(), runtime.status().clone());
        }

        // Collect status from output runtimes
        for (handle, runtime) in &self.outputs {
            runtime_nodes
                .outputs
                .insert(runtime.path().to_string(), runtime.status().clone());
        }

        runtime_nodes
    }

    /// Get the status for a node by path (looks up by iterating)
    pub fn get_status(&self, node_type: NodeType, node_id: &str) -> Option<&NodeStatus> {
        match node_type {
            NodeType::Output => self
                .outputs
                .values()
                .find(|r| r.path() == node_id)
                .map(|r| r.status()),
            NodeType::Texture => self
                .textures
                .values()
                .find(|r| r.path() == node_id)
                .map(|r| r.status()),
            NodeType::Shader => self
                .shaders
                .values()
                .find(|r| r.path() == node_id)
                .map(|r| r.status()),
            NodeType::Fixture => self
                .fixtures
                .values()
                .find(|r| r.path() == node_id)
                .map(|r| r.status()),
        }
    }

    /// Get a texture runtime by ID (looks up via ID->handle mapping)
    pub fn get_texture(&self, id: TextureId) -> Option<&TextureNodeRuntime> {
        self.texture_id_to_handle
            .get(&id)
            .and_then(|handle| self.textures.get(handle))
    }

    /// Get a shader runtime by ID (looks up via ID->handle mapping)
    pub fn get_shader(&self, id: ShaderId) -> Option<&ShaderNodeRuntime> {
        self.shader_id_to_handle
            .get(&id)
            .and_then(|handle| self.shaders.get(handle))
    }

    /// Get a fixture runtime by ID (looks up via ID->handle mapping)
    pub fn get_fixture(&self, id: FixtureId) -> Option<&FixtureNodeRuntime> {
        self.fixture_id_to_handle
            .get(&id)
            .and_then(|handle| self.fixtures.get(handle))
    }

    /// Get an output runtime by ID (looks up via ID->handle mapping)
    pub fn get_output(&self, id: OutputId) -> Option<&OutputNodeRuntime> {
        self.output_id_to_handle
            .get(&id)
            .and_then(|handle| self.outputs.get(handle))
    }

    /// Get the current frame time
    pub fn frame_time(&self) -> FrameTime {
        self.frame_time
    }

    /// Get all texture paths
    pub fn get_texture_ids(&self) -> alloc::vec::Vec<String> {
        self.textures
            .values()
            .map(|r| r.path().to_string())
            .collect()
    }

    /// Get all shader paths
    pub fn get_shader_ids(&self) -> alloc::vec::Vec<String> {
        self.shaders
            .values()
            .map(|r| r.path().to_string())
            .collect()
    }

    /// Get all fixture paths
    pub fn get_fixture_ids(&self) -> alloc::vec::Vec<String> {
        self.fixtures
            .values()
            .map(|r| r.path().to_string())
            .collect()
    }

    /// Get all output paths
    pub fn get_output_ids(&self) -> alloc::vec::Vec<String> {
        self.outputs
            .values()
            .map(|r| r.path().to_string())
            .collect()
    }

    /// Get the current frame ID
    pub fn get_current_frame(&self) -> FrameId {
        self.current_frame
    }

    /// Assign the next available handle and increment the counter
    pub fn assign_next_handle(&mut self) -> NodeHandle {
        let handle = NodeHandle::new(self.next_handle);
        self.next_handle += 1;
        handle
    }

    /// Get all node handles from all node types
    pub fn get_all_node_handles(&self) -> alloc::vec::Vec<NodeHandle> {
        let mut handles = alloc::vec::Vec::new();
        handles.extend(self.textures.keys().copied());
        handles.extend(self.shaders.keys().copied());
        handles.extend(self.fixtures.keys().copied());
        handles.extend(self.outputs.keys().copied());
        handles
    }

    /// Get nodes that changed since the given frame
    ///
    /// Returns handles of nodes where min(last_config_frame, last_state_frame) > since_frame
    /// OR created_frame > since_frame
    pub fn get_changed_nodes_since(&self, since_frame: FrameId) -> alloc::vec::Vec<NodeHandle> {
        let mut changed = alloc::vec::Vec::new();

        // Check textures
        for (handle, runtime) in &self.textures {
            let base = &runtime.base;
            let min_frame = if base.last_config_frame.0 < base.last_state_frame.0 {
                base.last_config_frame
            } else {
                base.last_state_frame
            };
            if min_frame.0 > since_frame.0 || base.created_frame.0 > since_frame.0 {
                changed.push(*handle);
            }
        }

        // Check shaders
        for (handle, runtime) in &self.shaders {
            let base = &runtime.base;
            let min_frame = if base.last_config_frame.0 < base.last_state_frame.0 {
                base.last_config_frame
            } else {
                base.last_state_frame
            };
            if min_frame.0 > since_frame.0 || base.created_frame.0 > since_frame.0 {
                changed.push(*handle);
            }
        }

        // Check fixtures
        for (handle, runtime) in &self.fixtures {
            let base = &runtime.base;
            let min_frame = if base.last_config_frame.0 < base.last_state_frame.0 {
                base.last_config_frame
            } else {
                base.last_state_frame
            };
            if min_frame.0 > since_frame.0 || base.created_frame.0 > since_frame.0 {
                changed.push(*handle);
            }
        }

        // Check outputs
        for (handle, runtime) in &self.outputs {
            let base = &runtime.base;
            let min_frame = if base.last_config_frame.0 < base.last_state_frame.0 {
                base.last_config_frame
            } else {
                base.last_state_frame
            };
            if min_frame.0 > since_frame.0 || base.created_frame.0 > since_frame.0 {
                changed.push(*handle);
            }
        }

        changed
    }

    /// Get detailed information about a node by handle
    pub fn get_node_detail(&self, handle: NodeHandle) -> Option<NodeDetail> {
        // Try each node type
        if let Some(texture) = self.textures.get(&handle) {
            return Some(NodeDetail {
                path: texture.path().to_string(),
                state: lp_shared::nodes::state::NodeState::Texture(
                    lp_shared::nodes::texture::state::TextureState {
                        texture: texture.texture().clone(),
                    },
                ),
            });
        }

        if let Some(shader) = self.shaders.get(&handle) {
            // Extract shader state (GLSL code, errors)
            let glsl = match shader.config() {
                lp_shared::nodes::shader::config::ShaderNode::Single { glsl, .. } => glsl.clone(),
            };
            return Some(NodeDetail {
                path: shader.path().to_string(),
                state: lp_shared::nodes::state::NodeState::Shader(
                    lp_shared::nodes::shader::state::ShaderState {
                        glsl,
                        error: match shader.status() {
                            crate::project::runtime::NodeStatus::Ok => None,
                            crate::project::runtime::NodeStatus::Error { status_message } => {
                                Some(status_message.clone())
                            }
                        },
                    },
                ),
            });
        }

        if let Some(output) = self.outputs.get(&handle) {
            // Extract output state (buffer values)
            return Some(NodeDetail {
                path: output.path().to_string(),
                state: lp_shared::nodes::state::NodeState::Output(
                    lp_shared::nodes::output::state::OutputState {
                        values: output.buffer().to_vec(),
                    },
                ),
            });
        }

        if let Some(_fixture) = self.fixtures.get(&handle) {
            // TODO: Fixture state not yet defined in NodeState
            // For now, return None
            return None;
        }

        None
    }

    /// Get engine statistics
    pub fn get_engine_stats(&self) -> EngineStats {
        // TODO: Calculate actual memory usage when tracking is added
        EngineStats {
            frame_ms_avg: self.frame_time.delta_ms as f32,
            frame_ms_std_dev: 0.0, // TODO: Calculate standard deviation
            memory_max_usage: 0,   // TODO: Track memory usage
            memory_avg_usage: 0,   // TODO: Track memory usage
        }
    }
}

/// Collection of runtime status for all node types (for serialization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeNodes {
    pub outputs: BTreeMap<String, NodeStatus>,
    pub textures: BTreeMap<String, NodeStatus>,
    pub shaders: BTreeMap<String, NodeStatus>,
    pub fixtures: BTreeMap<String, NodeStatus>,
}

/// Status of a node at runtime
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum NodeStatus {
    #[serde(rename = "Ok")]
    Ok,
    #[serde(rename = "Error")]
    Error {
        #[serde(rename = "statusMessage")]
        status_message: String,
    },
}

/// Node type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Output,
    Texture,
    Shader,
    Fixture,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::texture::formats;
    use crate::nodes::{FixtureNode, Mapping, OutputNode, ShaderNode, TextureNode};
    use crate::traits::{LedOutput, OutputProvider};
    use alloc::string::ToString;
    use alloc::vec;

    // Mock OutputProvider for testing
    struct MockOutputProvider;

    impl OutputProvider for MockOutputProvider {
        fn create_output(
            &self,
            _config: &OutputNode,
            _output_id: Option<OutputId>,
        ) -> Result<alloc::boxed::Box<dyn LedOutput>, Error> {
            Ok(alloc::boxed::Box::new(MockLedOutput {
                pixel_count: 100,
                last_written: alloc::vec::Vec::new(),
            }))
        }
    }

    // Mock LedOutput for testing
    struct MockLedOutput {
        pixel_count: usize,
        last_written: alloc::vec::Vec<u8>,
    }

    impl LedOutput for MockLedOutput {
        fn write_pixels(&mut self, pixels: &[u8]) -> Result<(), Error> {
            self.last_written = pixels.to_vec();
            Ok(())
        }

        fn get_pixel_count(&self) -> usize {
            self.pixel_count
        }
    }

    #[test]
    fn test_project_runtime_new() {
        let runtime = ProjectRuntime::new("test-uid".to_string());
        assert_eq!(runtime.uid, "test-uid");
        assert_eq!(runtime.textures.len(), 0);
        assert_eq!(runtime.shaders.len(), 0);
        assert_eq!(runtime.fixtures.len(), 0);
        assert_eq!(runtime.outputs.len(), 0);
    }

    #[test]
    fn test_project_runtime_init() {
        let mut runtime = ProjectRuntime::new("test".to_string());
        let (builder, texture_id) = crate::project::builder::ProjectBuilder::new_test()
            .add_texture(TextureNode::Memory {
                size: [64, 64],
                format: formats::RGBA8.to_string(),
            });
        let (builder, _shader_id) = builder.add_shader(ShaderNode::Single {
            glsl: "vec4 main(vec2 fragCoord, vec2 outputSize, float time) { return vec4(1.0); }"
                .to_string(),
            texture_id,
        });
        let (builder, output_id) = builder.add_output(OutputNode::GpioStrip {
            chip: "ws2812".to_string(),
            gpio_pin: 18,
            count: 100,
        });
        let (builder, _fixture_id) = builder.add_fixture(FixtureNode::CircleList {
            output_id,
            texture_id,
            channel_order: "rgb".to_string(),
            mapping: vec![Mapping {
                channel: 0,
                center: [0.5, 0.5],
                radius: 0.1,
            }],
        });
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        let config = builder.build().unwrap();

        let output_provider = MockOutputProvider;
        assert!(
            runtime
                .init(
                    &config,
                    &textures,
                    &shaders,
                    &outputs,
                    &fixtures,
                    &output_provider
                )
                .is_ok()
        );

        // Check that nodes were initialized
        assert_eq!(runtime.textures.len(), 1);
        assert_eq!(runtime.shaders.len(), 1);
        assert_eq!(runtime.fixtures.len(), 1);
        assert_eq!(runtime.outputs.len(), 1);
    }

    #[test]
    fn test_project_runtime_update() {
        let mut runtime = ProjectRuntime::new("test".to_string());
        let (builder, texture_id) = crate::project::builder::ProjectBuilder::new_test()
            .add_texture(TextureNode::Memory {
                size: [4, 4],
                format: formats::RGBA8.to_string(),
            });
        let (builder, _shader_id) = builder.add_shader(ShaderNode::Single {
            glsl: "vec4 main(vec2 fragCoord, vec2 outputSize, float time) { return vec4(0.5, 0.5, 0.5, 1.0); }"
                .to_string(),
            texture_id,
        });
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        let config = builder.build().unwrap();

        let output_provider = MockOutputProvider;
        runtime
            .init(
                &config,
                &textures,
                &shaders,
                &outputs,
                &fixtures,
                &output_provider,
            )
            .unwrap();

        // Update with 16ms delta
        assert!(runtime.update(16, &output_provider).is_ok());

        // Check frame time was updated
        assert_eq!(runtime.frame_time.delta_ms, 16);
        assert_eq!(runtime.frame_time.total_ms, 16);
    }

    #[test]
    fn test_project_runtime_update_shader_writes_to_texture() {
        let mut runtime = ProjectRuntime::new("test".to_string());
        let (builder, texture_id) = crate::project::builder::ProjectBuilder::new_test()
            .add_texture(TextureNode::Memory {
                size: [8, 8],
                format: formats::RGBA8.to_string(),
            });
        // Shader that returns a constant color - simpler test without division
        let (builder, shader_id) = builder.add_shader(ShaderNode::Single {
            glsl: r#"
vec4 main(vec2 fragCoord, vec2 outputSize, float time) {
    // Return a constant color - should definitely produce non-zero pixels
    return vec4(0.5, 0.5, 0.5, 1.0);
}
"#
            .to_string(),
            texture_id,
        });
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        let config = builder.build().unwrap();

        let output_provider = MockOutputProvider;
        runtime
            .init(
                &config,
                &textures,
                &shaders,
                &outputs,
                &fixtures,
                &output_provider,
            )
            .unwrap();

        // Verify texture is initially zero (or at least check initial state)
        let texture_before = runtime.get_texture(texture_id).unwrap();
        let pixel_before = texture_before.texture().get_pixel(0, 0).unwrap();
        // Texture should be initialized to zero
        assert_eq!(pixel_before, [0, 0, 0, 0], "Texture should start as zero");

        // Verify shader compiled successfully
        let shader = runtime.get_shader(shader_id).unwrap();
        match shader.status() {
            NodeStatus::Ok => {
                // Good, shader compiled
            }
            NodeStatus::Error { status_message } => {
                panic!("Shader compilation failed: {}", status_message);
            }
        }

        // Update with 16ms delta - this should execute the shader and write to texture
        let update_result = runtime.update(16, &output_provider);
        if let Err(e) = &update_result {
            // Check shader status again - it might have changed during update
            let shader_after = runtime.get_shader(shader_id).unwrap();
            match shader_after.status() {
                NodeStatus::Ok => {
                    panic!("Update failed but shader status is Ok: {:?}", e);
                }
                NodeStatus::Error { status_message } => {
                    panic!(
                        "Update failed, shader error: {} (update error: {:?})",
                        status_message, e
                    );
                }
            }
        }
        assert!(update_result.is_ok(), "Update should succeed");

        // Check shader status after update - it might have changed if execution failed
        let shader_after_update = runtime.get_shader(shader_id).unwrap();
        match shader_after_update.status() {
            NodeStatus::Ok => {
                // Good, shader executed successfully
            }
            NodeStatus::Error { status_message } => {
                panic!("Shader execution failed during update: {}", status_message);
            }
        }

        // Verify texture was updated with non-zero pixels
        let texture_after = runtime.get_texture(texture_id).unwrap();

        // Check that at least some pixels are non-zero (shader executed)
        let mut found_non_zero = false;
        for y in 0..8 {
            for x in 0..8 {
                let pixel = texture_after.texture().get_pixel(x, y).unwrap();
                // Check RGB channels (alpha might be 255, but we care about color)
                if pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0 {
                    found_non_zero = true;
                    break;
                }
            }
            if found_non_zero {
                break;
            }
        }

        assert!(
            found_non_zero,
            "Shader should have written non-zero pixels to texture after update"
        );

        // Verify specific pixel values match expected shader output
        // Shader returns vec4(0.5, 0.5, 0.5, 1.0), so all RGB channels should be ~128 (0.5 * 255)
        let pixel_0_0 = texture_after.texture().get_pixel(0, 0).unwrap();
        // Allow some tolerance for fixed-point math (0.5 * 255 = 127.5, so expect ~127-128)
        assert!(
            pixel_0_0[0] >= 120 && pixel_0_0[0] <= 135,
            "Pixel (0,0) red channel should be around 128: got {}",
            pixel_0_0[0]
        );
        assert!(
            pixel_0_0[1] >= 120 && pixel_0_0[1] <= 135,
            "Pixel (0,0) green channel should be around 128: got {}",
            pixel_0_0[1]
        );
        assert!(
            pixel_0_0[2] >= 120 && pixel_0_0[2] <= 135,
            "Pixel (0,0) blue channel should be around 128: got {}",
            pixel_0_0[2]
        );
        assert_eq!(
            pixel_0_0[3], 255,
            "Pixel (0,0) alpha channel should be 255: got {}",
            pixel_0_0[3]
        );

        // Verify frame time was updated
        assert_eq!(runtime.frame_time.delta_ms, 16);
        assert_eq!(runtime.frame_time.total_ms, 16);
    }

    #[test]
    fn test_project_runtime_get_runtime_nodes() {
        let mut runtime = ProjectRuntime::new("test".to_string());
        let (builder, _texture_id) = crate::project::builder::ProjectBuilder::new_test()
            .add_texture(TextureNode::Memory {
                size: [64, 64],
                format: formats::RGBA8.to_string(),
            });
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        let config = builder.build().unwrap();

        let output_provider = MockOutputProvider;
        runtime
            .init(
                &config,
                &textures,
                &shaders,
                &outputs,
                &fixtures,
                &output_provider,
            )
            .unwrap();

        let runtime_nodes = runtime.get_runtime_nodes();
        assert_eq!(runtime_nodes.textures.len(), 1);
    }

    #[test]
    fn test_project_runtime_destroy() {
        let mut runtime = ProjectRuntime::new("test".to_string());
        let (builder, _texture_id) = crate::project::builder::ProjectBuilder::new_test()
            .add_texture(TextureNode::Memory {
                size: [64, 64],
                format: formats::RGBA8.to_string(),
            });
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        let config = builder.build().unwrap();

        let output_provider = MockOutputProvider;
        runtime
            .init(
                &config,
                &textures,
                &shaders,
                &outputs,
                &fixtures,
                &output_provider,
            )
            .unwrap();

        assert!(runtime.destroy().is_ok());
    }

    #[test]
    fn test_complete_project_lifecycle() {
        // Build project
        let (builder, texture_id) = crate::project::builder::ProjectBuilder::new_test()
            .add_texture(TextureNode::Memory {
                size: [8, 8],
                format: formats::RGBA8.to_string(),
            });
        let (builder, _shader_id) = builder.add_shader(ShaderNode::Single {
            glsl: "vec4 main(vec2 fragCoord, vec2 outputSize, float time) { return vec4(0.5, 0.5, 0.5, 1.0); }"
                .to_string(),
            texture_id,
        });
        let (builder, output_id) = builder.add_output(OutputNode::GpioStrip {
            chip: "ws2812".to_string(),
            gpio_pin: 18,
            count: 10,
        });
        let (builder, _fixture_id) = builder.add_fixture(FixtureNode::CircleList {
            output_id,
            texture_id,
            channel_order: "rgb".to_string(),
            mapping: vec![Mapping {
                channel: 0,
                center: [0.5, 0.5],
                radius: 0.1,
            }],
        });
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        let config = builder.build().unwrap();

        // Init runtime
        let mut runtime = ProjectRuntime::new("test".to_string());
        let output_provider = MockOutputProvider;
        assert!(
            runtime
                .init(
                    &config,
                    &textures,
                    &shaders,
                    &outputs,
                    &fixtures,
                    &output_provider
                )
                .is_ok()
        );

        // Update multiple times
        assert!(runtime.update(16, &output_provider).is_ok());
        assert_eq!(runtime.frame_time.delta_ms, 16);
        assert_eq!(runtime.frame_time.total_ms, 16);

        assert!(runtime.update(16, &output_provider).is_ok());
        assert_eq!(runtime.frame_time.delta_ms, 16);
        assert_eq!(runtime.frame_time.total_ms, 32);

        // Destroy
        assert!(runtime.destroy().is_ok());
    }

    #[test]
    fn test_shader_fixture_output_pipeline() {
        // Build: texture → shader → fixture → output
        let (builder, texture_id) = crate::project::builder::ProjectBuilder::new_test()
            .add_texture(TextureNode::Memory {
                size: [4, 4],
                format: formats::RGBA8.to_string(),
            });
        let (builder, _shader_id) = builder.add_shader(ShaderNode::Single {
            glsl: "vec4 main(vec2 fragCoord, vec2 outputSize, float time) { return vec4(1.0, 0.0, 0.0, 1.0); }"
                .to_string(),
            texture_id,
        });
        let (builder, output_id) = builder.add_output(OutputNode::GpioStrip {
            chip: "ws2812".to_string(),
            gpio_pin: 18,
            count: 5,
        });
        let (builder, _fixture_id) = builder.add_fixture(FixtureNode::CircleList {
            output_id,
            texture_id,
            channel_order: "rgb".to_string(),
            mapping: vec![Mapping {
                channel: 0,
                center: [0.5, 0.5],
                radius: 0.2,
            }],
        });
        let config = builder.build().unwrap();

        // Init and update
        let mut runtime = ProjectRuntime::new("test".to_string());
        let output_provider = MockOutputProvider;
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        runtime
            .init(
                &config,
                &textures,
                &shaders,
                &outputs,
                &fixtures,
                &output_provider,
            )
            .unwrap();
        runtime.update(16, &output_provider).unwrap();

        // Verify pipeline worked: shader wrote to texture, fixture sampled texture, output got data
        let runtime_nodes = runtime.get_runtime_nodes();
        // All nodes should be Ok status
        assert!(matches!(
            runtime_nodes.shaders.values().next(),
            Some(NodeStatus::Ok)
        ));
        assert!(matches!(
            runtime_nodes.fixtures.values().next(),
            Some(NodeStatus::Ok)
        ));
        assert!(matches!(
            runtime_nodes.outputs.values().next(),
            Some(NodeStatus::Ok)
        ));
    }

    #[test]
    fn test_multiple_fixtures_same_output() {
        // Build: one output, multiple fixtures
        let (builder, texture_id) = crate::project::builder::ProjectBuilder::new_test()
            .add_texture(TextureNode::Memory {
                size: [4, 4],
                format: formats::RGBA8.to_string(),
            });
        let (builder, output_id) = builder.add_output(OutputNode::GpioStrip {
            chip: "ws2812".to_string(),
            gpio_pin: 18,
            count: 10,
        });
        let (builder, _fixture1_id) = builder.add_fixture(FixtureNode::CircleList {
            output_id,
            texture_id,
            channel_order: "rgb".to_string(),
            mapping: vec![Mapping {
                channel: 0,
                center: [0.3, 0.3],
                radius: 0.1,
            }],
        });
        let (builder, _fixture2_id) = builder.add_fixture(FixtureNode::CircleList {
            output_id,
            texture_id,
            channel_order: "rgb".to_string(),
            mapping: vec![Mapping {
                channel: 1,
                center: [0.7, 0.7],
                radius: 0.1,
            }],
        });
        let config = builder.build().unwrap();

        let mut runtime = ProjectRuntime::new("test".to_string());
        let output_provider = MockOutputProvider;
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        runtime
            .init(
                &config,
                &textures,
                &shaders,
                &outputs,
                &fixtures,
                &output_provider,
            )
            .unwrap();
        runtime.update(16, &output_provider).unwrap();

        // Both fixtures should have written to the same output
        let runtime_nodes = runtime.get_runtime_nodes();
        assert_eq!(runtime_nodes.fixtures.len(), 2);
        assert_eq!(runtime_nodes.outputs.len(), 1);
    }

    #[test]
    fn test_frame_time_tracking() {
        let (builder, _texture_id) = crate::project::builder::ProjectBuilder::new_test()
            .add_texture(TextureNode::Memory {
                size: [4, 4],
                format: formats::RGBA8.to_string(),
            });
        let config = builder.build().unwrap();

        let mut runtime = ProjectRuntime::new("test".to_string());
        let output_provider = MockOutputProvider;
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        runtime
            .init(
                &config,
                &textures,
                &shaders,
                &outputs,
                &fixtures,
                &output_provider,
            )
            .unwrap();

        // Initial state
        assert_eq!(runtime.frame_time.delta_ms, 0);
        assert_eq!(runtime.frame_time.total_ms, 0);

        // First update
        runtime.update(16, &output_provider).unwrap();
        assert_eq!(runtime.frame_time.delta_ms, 16);
        assert_eq!(runtime.frame_time.total_ms, 16);

        // Second update
        runtime.update(17, &output_provider).unwrap();
        assert_eq!(runtime.frame_time.delta_ms, 17);
        assert_eq!(runtime.frame_time.total_ms, 33);

        // Third update
        runtime.update(16, &output_provider).unwrap();
        assert_eq!(runtime.frame_time.delta_ms, 16);
        assert_eq!(runtime.frame_time.total_ms, 49);
    }
}
