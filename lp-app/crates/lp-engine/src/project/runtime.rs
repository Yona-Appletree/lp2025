use crate::error::Error;
use crate::nodes::{FixtureRuntime, NodeRuntime, OutputRuntime, ShaderRuntime, TextureRuntime};
use crate::runtime::contexts::RenderContext;
use crate::runtime::frame_time::FrameTime;
use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use lp_model::{
    FrameId, LpPath, NodeConfig, NodeHandle, NodeKind,
    project::api::{
        ApiNodeSpecifier, NodeChange, NodeDetail, NodeState, NodeStatus as ApiNodeStatus,
        ProjectResponse,
    },
};
use lp_shared::fs::LpFs;
#[cfg(feature = "std")]
use serde_json;

/// Project runtime - manages nodes and rendering
pub struct ProjectRuntime {
    /// Current frame ID
    pub frame_id: FrameId,
    /// Frame timing information
    pub frame_time: FrameTime,
    /// Filesystem (owned for now)
    pub fs: Box<dyn LpFs>,
    /// Node entries
    pub nodes: BTreeMap<NodeHandle, NodeEntry>,
    /// Next handle to assign
    pub next_handle: i32,
}

/// Node entry in runtime
pub struct NodeEntry {
    /// Node path
    pub path: LpPath,
    /// Node kind
    pub kind: NodeKind,
    /// Node config
    pub config: Box<dyn NodeConfig>,
    /// Frame when config was last updated
    pub config_ver: FrameId,
    /// Node status
    pub status: NodeStatus,
    /// Node runtime (None until initialized)
    pub runtime: Option<Box<dyn NodeRuntime>>,
    /// Last frame state updates occurred
    pub state_ver: FrameId,
}

/// Node status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeStatus {
    /// Created but not yet initialized
    Created,
    /// Error initializing the node
    InitError(String),
    /// Node is running normally
    Ok,
    /// Node is running, but something is wrong
    Warn(String),
    /// Node cannot run
    Error(String),
}

impl ProjectRuntime {
    /// Create new project runtime
    pub fn new(fs: Box<dyn LpFs>) -> Result<Self, Error> {
        let _config = crate::project::loader::load_from_filesystem(fs.as_ref())?;

        Ok(Self {
            frame_id: FrameId::default(),
            frame_time: FrameTime::zero(),
            fs,
            nodes: BTreeMap::new(),
            next_handle: 1,
        })
    }

    /// Load nodes from filesystem (doesn't initialize them)
    pub fn load_nodes(&mut self) -> Result<(), Error> {
        let node_paths = crate::project::loader::discover_nodes(self.fs.as_ref())?;

        for path in node_paths {
            match crate::project::loader::load_node(self.fs.as_ref(), &path) {
                Ok((path, config)) => {
                    let handle = NodeHandle::new(self.next_handle);
                    self.next_handle += 1;

                    let kind = config.kind();
                    let entry = NodeEntry {
                        path,
                        kind,
                        config,
                        config_ver: self.frame_id,
                        status: NodeStatus::Created,
                        runtime: None,
                        state_ver: FrameId::default(),
                    };

                    self.nodes.insert(handle, entry);
                }
                Err(e) => {
                    // Create entry with error status
                    let handle = NodeHandle::new(self.next_handle);
                    self.next_handle += 1;

                    // Try to determine kind from path
                    let kind = match crate::project::loader::node_kind_from_path(&path) {
                        Ok(k) => k,
                        Err(_) => continue, // Skip unknown types
                    };

                    // Create a dummy config based on kind
                    // This is a temporary solution until we have a better way
                    let config: Box<dyn NodeConfig> = match kind {
                        NodeKind::Texture => Box::new(lp_model::nodes::texture::TextureConfig {
                            width: 0,
                            height: 0,
                        }),
                        NodeKind::Shader => {
                            Box::new(lp_model::nodes::shader::ShaderConfig::default())
                        }
                        NodeKind::Output => {
                            Box::new(lp_model::nodes::output::OutputConfig::GpioStrip { pin: 0 })
                        }
                        NodeKind::Fixture => Box::new(lp_model::nodes::fixture::FixtureConfig {
                            output_spec: lp_model::NodeSpecifier::from(""),
                            texture_spec: lp_model::NodeSpecifier::from(""),
                            mapping: String::new(),
                            lamp_type: String::new(),
                            color_order: lp_model::nodes::fixture::ColorOrder::Rgb,
                            transform: [[0.0; 4]; 4],
                        }),
                    };

                    let entry = NodeEntry {
                        path,
                        kind,
                        config,
                        config_ver: self.frame_id,
                        status: NodeStatus::InitError(format!("Failed to load: {}", e)),
                        runtime: None,
                        state_ver: FrameId::default(),
                    };

                    self.nodes.insert(handle, entry);
                }
            }
        }

        Ok(())
    }

    /// Initialize all nodes in dependency order
    pub fn initialize_nodes(&mut self) -> Result<(), Error> {
        // Initialize in order: textures → shaders → fixtures → outputs
        let init_order = [
            NodeKind::Texture,
            NodeKind::Shader,
            NodeKind::Fixture,
            NodeKind::Output,
        ];

        for kind in init_order.iter() {
            let handles: Vec<NodeHandle> = self
                .nodes
                .iter()
                .filter(|(_, entry)| entry.kind == *kind && entry.status == NodeStatus::Created)
                .map(|(handle, _)| *handle)
                .collect();

            for handle in handles {
                // Get node path and kind before mutable borrow
                let (node_path, node_kind) = {
                    let entry = self.nodes.get(&handle).ok_or_else(|| Error::Other {
                        message: format!("Node handle {} not found", handle.as_i32()),
                    })?;
                    (entry.path.clone(), entry.kind)
                };

                // Extract config before creating runtime (for textures and fixtures)
                // Load config from filesystem since we can't extract from Box<dyn NodeConfig>
                let texture_config = if node_kind == NodeKind::Texture {
                    let entry = self.nodes.get(&handle).ok_or_else(|| Error::Other {
                        message: format!("Node handle {} not found", handle.as_i32()),
                    })?;
                    // Reload config from filesystem (workaround for trait object limitation)
                    let node_json_path = format!("{}/node.json", entry.path.as_str());
                    let data = self.fs.read_file(&node_json_path).map_err(|e| Error::Io {
                        path: node_json_path.clone(),
                        details: format!("Failed to read: {:?}", e),
                    })?;
                    Some(
                        serde_json::from_slice::<lp_model::nodes::texture::TextureConfig>(&data)
                            .map_err(|e| Error::Parse {
                                file: node_json_path,
                                error: format!("Failed to parse texture config: {}", e),
                            })?,
                    )
                } else {
                    None
                };

                let fixture_config = if node_kind == NodeKind::Fixture {
                    let entry = self.nodes.get(&handle).ok_or_else(|| Error::Other {
                        message: format!("Node handle {} not found", handle.as_i32()),
                    })?;
                    // Reload config from filesystem (workaround for trait object limitation)
                    let node_json_path = format!("{}/node.json", entry.path.as_str());
                    let data = self.fs.read_file(&node_json_path).map_err(|e| Error::Io {
                        path: node_json_path.clone(),
                        details: format!("Failed to read: {:?}", e),
                    })?;
                    Some(
                        serde_json::from_slice::<lp_model::nodes::fixture::FixtureConfig>(&data)
                            .map_err(|e| Error::Parse {
                                file: node_json_path,
                                error: format!("Failed to parse fixture config: {}", e),
                            })?,
                    )
                } else {
                    None
                };

                let shader_config = if node_kind == NodeKind::Shader {
                    let entry = self.nodes.get(&handle).ok_or_else(|| Error::Other {
                        message: format!("Node handle {} not found", handle.as_i32()),
                    })?;
                    // Reload config from filesystem (workaround for trait object limitation)
                    let node_json_path = format!("{}/node.json", entry.path.as_str());
                    let data = self.fs.read_file(&node_json_path).map_err(|e| Error::Io {
                        path: node_json_path.clone(),
                        details: format!("Failed to read: {:?}", e),
                    })?;
                    Some(
                        serde_json::from_slice::<lp_model::nodes::shader::ShaderConfig>(&data)
                            .map_err(|e| Error::Parse {
                                file: node_json_path,
                                error: format!("Failed to parse shader config: {}", e),
                            })?,
                    )
                } else {
                    None
                };

                // Create runtime based on kind
                let mut runtime: Box<dyn NodeRuntime> = match node_kind {
                    NodeKind::Texture => {
                        let mut tex_runtime = TextureRuntime::new(handle);
                        if let Some(config) = texture_config {
                            tex_runtime.set_config(config);
                        }
                        Box::new(tex_runtime)
                    }
                    NodeKind::Shader => {
                        let mut shader_runtime = ShaderRuntime::new(handle);
                        if let Some(config) = shader_config {
                            shader_runtime.set_config(config);
                        }
                        Box::new(shader_runtime)
                    }
                    NodeKind::Output => Box::new(OutputRuntime::new()),
                    NodeKind::Fixture => {
                        let mut fixture_runtime = FixtureRuntime::new();
                        if let Some(config) = fixture_config {
                            fixture_runtime.set_config(config);
                        }
                        Box::new(fixture_runtime)
                    }
                };

                // Create init context and initialize (needs immutable borrow of self)
                let init_result = {
                    let ctx = InitContext::new(self, &node_path)?;
                    runtime.init(&ctx)
                };

                // Now do mutable operations (context is dropped)
                if let Some(entry) = self.nodes.get_mut(&handle) {
                    match init_result {
                        Ok(()) => {
                            entry.status = NodeStatus::Ok;
                            entry.runtime = Some(runtime);
                        }
                        Err(e) => {
                            entry.status = NodeStatus::InitError(format!("{}", e));
                            entry.runtime = None;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Ensure all nodes initialized successfully
    ///
    /// Returns an error if any nodes failed to initialize, with details about
    /// which nodes failed and why. Warnings are ignored (nodes with warnings
    /// are considered successfully initialized).
    pub fn ensure_all_nodes_initialized(&self) -> Result<(), Error> {
        let mut failed_nodes = Vec::new();

        for (_, entry) in &self.nodes {
            match &entry.status {
                NodeStatus::Ok | NodeStatus::Warn(_) => {
                    // Node initialized successfully (warnings are acceptable)
                }
                NodeStatus::Created => {
                    failed_nodes.push(format!(
                        "{} ({:?}): not initialized",
                        entry.path.as_str(),
                        entry.kind
                    ));
                }
                NodeStatus::InitError(msg) => {
                    failed_nodes.push(format!(
                        "{} ({:?}): initialization error: {}",
                        entry.path.as_str(),
                        entry.kind,
                        msg
                    ));
                }
                NodeStatus::Error(msg) => {
                    failed_nodes.push(format!(
                        "{} ({:?}): error: {}",
                        entry.path.as_str(),
                        entry.kind,
                        msg
                    ));
                }
            }
        }

        if failed_nodes.is_empty() {
            Ok(())
        } else {
            Err(Error::Other {
                message: format!(
                    "Some nodes failed to initialize:\n  {}",
                    failed_nodes.join("\n  ")
                ),
            })
        }
    }

    /// Advance to next frame and render
    ///
    /// Updates frame ID and frame time, then renders the frame.
    /// `delta_ms` is the time elapsed since the last frame in milliseconds.
    pub fn tick(&mut self, delta_ms: u32) -> Result<(), Error> {
        // Update frame ID and time
        self.frame_id = self.frame_id.next();
        self.frame_time.total_ms += delta_ms;
        self.frame_time.delta_ms = delta_ms;
        
        // Render the frame
        // Render all fixtures
        let fixture_handles: Vec<NodeHandle> = self
            .nodes
            .iter()
            .filter(|(_, entry)| {
                entry.kind == NodeKind::Fixture
                    && entry.runtime.is_some()
                    && matches!(entry.status, NodeStatus::Ok)
            })
            .map(|(handle, _)| *handle)
            .collect();

        for handle in fixture_handles {
            // Render fixture - need to handle borrowing carefully
            // The issue: runtime.render() needs &mut runtime and &mut ctx
            // But runtime is inside ctx.nodes, so we can't have both borrows
            // Solution: use a helper that takes nodes and handle, does everything internally
            let render_result = {
                // Create context
                let mut ctx = RenderContextImpl {
                    nodes: &mut self.nodes,
                    frame_id: self.frame_id,
                    frame_time: self.frame_time,
                };
                
                // Get runtime and render in one go
                // We'll use a pattern where we get the runtime, call render, then handle errors
                // The key is that runtime.render() will borrow ctx, and ctx contains nodes
                // So we can't hold a reference to runtime (from nodes) while calling render
                // Solution: restructure so render() accesses runtime internally through ctx
                // But that would require changing the trait signature
                // For now, let's use a workaround: get runtime, call render with reborrow
                if let Some(entry) = ctx.nodes.get_mut(&handle) {
                    if let Some(runtime) = entry.runtime.as_mut() {
                        // runtime is &mut Box<dyn NodeRuntime>
                        // render() needs &mut self (runtime) and &mut ctx
                        // Both need mutable access, but runtime is inside ctx.nodes
                        // This creates a borrowing conflict
                        // Workaround: use unsafe to get raw pointer (not ideal, but works)
                        let runtime_ptr: *mut dyn NodeRuntime = runtime.as_mut();
                        // SAFETY: runtime_ptr is valid for the duration of this block
                        // We're not storing it or using it after the block
                        unsafe {
                            (*runtime_ptr).render(&mut ctx)
                        }
                    } else {
                        Ok(())
                    }
                } else {
                    Ok(())
                }
            };
            
            // Update status based on render result
            if let Some(entry) = self.nodes.get_mut(&handle) {
                if let Err(e) = render_result {
                    entry.status = NodeStatus::Error(format!("{}", e));
                }
            }
        }

        // todo!("Flush outputs with state_ver == frame_id")

        Ok(())
    }

    /// Resolve a path to a node handle
    ///
    /// Returns the handle for the node at the given path, or an error if not found.
    pub fn resolve_path_to_handle(&self, path: &str) -> Result<NodeHandle, Error> {
        let node_path = lp_model::LpPath::from(path);
        
        // Look up node by path
        for (handle, entry) in &self.nodes {
            if entry.path == node_path {
                return Ok(*handle);
            }
        }
        
        Err(Error::NotFound {
            path: path.to_string(),
        })
    }

    /// Get changes since a frame (for client sync)
    pub fn get_changes(
        &self,
        since_frame: FrameId,
        detail_specifier: &ApiNodeSpecifier,
    ) -> Result<ProjectResponse, Error> {
        let mut node_handles = Vec::new();
        let mut node_changes = Vec::new();
        let mut node_details = BTreeMap::new();

        // Collect all current handles
        for handle in self.nodes.keys() {
            node_handles.push(*handle);
        }

        // Determine which handles need detail
        let detail_handles: BTreeSet<NodeHandle> = match detail_specifier {
            ApiNodeSpecifier::None => BTreeSet::new(),
            ApiNodeSpecifier::All => self.nodes.keys().copied().collect(),
            ApiNodeSpecifier::ByHandles(handles) => handles.iter().copied().collect(),
        };

        // Collect changes and details
        for (handle, entry) in &self.nodes {
            // Check for changes since since_frame
            if entry.config_ver.as_i64() > since_frame.as_i64() {
                node_changes.push(NodeChange::ConfigUpdated {
                    handle: *handle,
                    config_ver: entry.config_ver,
                });
            }

            if entry.state_ver.as_i64() > since_frame.as_i64() {
                node_changes.push(NodeChange::StateUpdated {
                    handle: *handle,
                    state_ver: entry.state_ver,
                });
            }

            // Check if node was created after since_frame
            if entry.config_ver.as_i64() > since_frame.as_i64()
                && entry.config_ver == entry.state_ver
            {
                node_changes.push(NodeChange::Created {
                    handle: *handle,
                    path: entry.path.clone(),
                    kind: entry.kind,
                });
            }

            // Add detail if requested
            if detail_handles.contains(handle) {
                let state = match entry.kind {
                    NodeKind::Texture => {
                        // Get actual texture state from runtime
                        if let Some(runtime) = &entry.runtime {
                            // Use Any trait for downcasting (downcast_ref is from Any trait)
                            if let Some(tex_runtime) = runtime.as_any().downcast_ref::<TextureRuntime>() {
                                NodeState::Texture(tex_runtime.get_state())
                            } else {
                                // Fallback to empty state
                                NodeState::Texture(lp_model::nodes::texture::TextureState {
                                    texture_data: Vec::new(),
                                })
                            }
                        } else {
                            NodeState::Texture(lp_model::nodes::texture::TextureState {
                                texture_data: Vec::new(),
                            })
                        }
                    }
                    NodeKind::Shader => {
                        // Get actual shader state from runtime
                        if let Some(runtime) = &entry.runtime {
                            if let Some(shader_runtime) = runtime.as_any().downcast_ref::<ShaderRuntime>() {
                                NodeState::Shader(shader_runtime.get_state())
                            } else {
                                // Fallback to empty state
                                NodeState::Shader(lp_model::nodes::shader::ShaderState {
                                    glsl_code: String::new(),
                                    error: None,
                                })
                            }
                        } else {
                            NodeState::Shader(lp_model::nodes::shader::ShaderState {
                                glsl_code: String::new(),
                                error: None,
                            })
                        }
                    }
                    NodeKind::Output => {
                        // Output runtime not fully implemented yet
                        NodeState::Output(lp_model::nodes::output::OutputState {
                            channel_data: Vec::new(),
                        })
                    }
                    NodeKind::Fixture => {
                        // Fixture runtime state extraction
                        // FixtureState has lamp_colors - we'd need to extract from runtime
                        // For now, return empty (will implement when fixture state is needed)
                        NodeState::Fixture(lp_model::nodes::fixture::FixtureState {
                            lamp_colors: Vec::new(),
                        })
                    }
                };

                let api_status = match &entry.status {
                    NodeStatus::Created => ApiNodeStatus::Created,
                    NodeStatus::InitError(msg) => ApiNodeStatus::InitError(msg.clone()),
                    NodeStatus::Ok => ApiNodeStatus::Ok,
                    NodeStatus::Warn(msg) => ApiNodeStatus::Warn(msg.clone()),
                    NodeStatus::Error(msg) => ApiNodeStatus::Error(msg.clone()),
                };

                // Clone config based on kind (temporary - will use proper serialization later)
                let config: Box<dyn NodeConfig> = match entry.kind {
                    NodeKind::Texture => {
                        // todo!("Proper config cloning - use serialization or Any trait")
                        Box::new(lp_model::nodes::texture::TextureConfig {
                            width: 0,
                            height: 0,
                        })
                    }
                    NodeKind::Shader => {
                        // Extract actual shader config from runtime
                        if let Some(runtime) = &entry.runtime {
                            if let Some(shader_runtime) = runtime.as_any().downcast_ref::<ShaderRuntime>() {
                                if let Some(shader_config) = shader_runtime.get_config() {
                                    Box::new(shader_config.clone())
                                } else {
                                    Box::new(lp_model::nodes::shader::ShaderConfig::default())
                                }
                            } else {
                                Box::new(lp_model::nodes::shader::ShaderConfig::default())
                            }
                        } else {
                            Box::new(lp_model::nodes::shader::ShaderConfig::default())
                        }
                    }
                    NodeKind::Output => {
                        Box::new(lp_model::nodes::output::OutputConfig::GpioStrip { pin: 0 })
                    }
                    NodeKind::Fixture => Box::new(lp_model::nodes::fixture::FixtureConfig {
                        output_spec: lp_model::NodeSpecifier::from(""),
                        texture_spec: lp_model::NodeSpecifier::from(""),
                        mapping: String::new(),
                        lamp_type: String::new(),
                        color_order: lp_model::nodes::fixture::ColorOrder::Rgb,
                        transform: [[0.0; 4]; 4],
                    }),
                };

                node_details.insert(
                    *handle,
                    NodeDetail {
                        path: entry.path.clone(),
                        config,
                        state,
                        status: api_status,
                    },
                );
            }
        }

        Ok(ProjectResponse::GetChanges {
            current_frame: self.frame_id,
            node_handles,
            node_changes,
            node_details,
        })
    }
}

/// Init context implementation
struct InitContext<'a> {
    runtime: &'a ProjectRuntime,
    #[allow(dead_code)] // Used for chroot filesystem creation, may be needed for future features
    node_path: &'a LpPath,
    node_fs: alloc::boxed::Box<dyn LpFs>,
}

impl<'a> InitContext<'a> {
    pub fn new(runtime: &'a ProjectRuntime, node_path: &'a LpPath) -> Result<Self, Error> {
        let node_dir = node_path.as_str();
        let node_fs = runtime.fs.chroot(node_dir).map_err(|e| Error::Io {
            path: node_dir.to_string(),
            details: format!("Failed to chroot: {:?}", e),
        })?;

        Ok(Self {
            runtime,
            node_path,
            node_fs,
        })
    }
}

impl<'a> crate::runtime::contexts::NodeInitContext for InitContext<'a> {
    fn resolve_node(&self, spec: &lp_model::NodeSpecifier) -> Result<lp_model::NodeHandle, Error> {
        let spec_path = spec.as_str();
        let node_path = if spec_path.starts_with('/') {
            // Absolute path
            lp_model::LpPath::from(spec_path)
        } else {
            // Relative path - resolve from node directory
            // For now, assume relative paths are not supported (todo!)
            return Err(Error::NotFound {
                path: spec_path.to_string(),
            });
        };

        // Look up node by path
        for (handle, entry) in &self.runtime.nodes {
            if entry.path == node_path {
                return Ok(*handle);
            }
        }

        Err(Error::NotFound {
            path: spec_path.to_string(),
        })
    }

    fn resolve_output(
        &self,
        spec: &lp_model::NodeSpecifier,
    ) -> Result<crate::runtime::contexts::OutputHandle, Error> {
        let handle = self.resolve_node(spec)?;
        let entry = self
            .runtime
            .nodes
            .get(&handle)
            .ok_or_else(|| Error::NotFound {
                path: spec.as_str().to_string(),
            })?;

        if entry.kind != lp_model::NodeKind::Output {
            return Err(Error::WrongNodeKind {
                specifier: spec.as_str().to_string(),
                expected: lp_model::NodeKind::Output,
                actual: entry.kind,
            });
        }

        Ok(crate::runtime::contexts::OutputHandle::new(handle))
    }

    fn resolve_texture(
        &self,
        spec: &lp_model::NodeSpecifier,
    ) -> Result<crate::runtime::contexts::TextureHandle, Error> {
        let handle = self.resolve_node(spec)?;
        let entry = self
            .runtime
            .nodes
            .get(&handle)
            .ok_or_else(|| Error::NotFound {
                path: spec.as_str().to_string(),
            })?;

        if entry.kind != lp_model::NodeKind::Texture {
            return Err(Error::WrongNodeKind {
                specifier: spec.as_str().to_string(),
                expected: lp_model::NodeKind::Texture,
                actual: entry.kind,
            });
        }

        Ok(crate::runtime::contexts::TextureHandle::new(handle))
    }

    fn get_node_fs(&self) -> &dyn lp_shared::fs::LpFs {
        self.node_fs.as_ref()
    }
}

/// Render context implementation
struct RenderContextImpl<'a> {
    nodes: &'a mut BTreeMap<NodeHandle, NodeEntry>,
    frame_id: FrameId,
    frame_time: FrameTime,
}

impl<'a> crate::runtime::contexts::RenderContext for RenderContextImpl<'a> {
    fn get_texture(&mut self, handle: crate::runtime::contexts::TextureHandle) -> Result<&lp_shared::Texture, Error> {
        // Ensure texture is rendered (lazy rendering)
        Self::ensure_texture_rendered(self.nodes, handle, self.frame_id, self.frame_time)?;
        
        // Get texture runtime
        let node_handle = handle.as_node_handle();
        let entry = self.nodes.get_mut(&node_handle)
            .ok_or_else(|| Error::NotFound {
                path: format!("texture-{}", node_handle.as_i32()),
            })?;
        
        // Get texture from runtime
        if let Some(runtime) = &mut entry.runtime {
            if let Some(tex_runtime) = runtime.as_any_mut().downcast_mut::<crate::nodes::TextureRuntime>() {
                tex_runtime.texture()
                    .ok_or_else(|| Error::Other {
                        message: "Texture not initialized".to_string(),
                    })
            } else {
                Err(Error::Other {
                    message: "Texture runtime not found".to_string(),
                })
            }
        } else {
            Err(Error::Other {
                message: "Runtime not initialized".to_string(),
            })
        }
    }

    fn get_texture_mut(&mut self, handle: crate::runtime::contexts::TextureHandle) -> Result<&mut lp_shared::Texture, Error> {
        // Ensure texture is rendered (lazy rendering)
        Self::ensure_texture_rendered(self.nodes, handle, self.frame_id, self.frame_time)?;
        
        // Get texture runtime
        let node_handle = handle.as_node_handle();
        let entry = self.nodes.get_mut(&node_handle)
            .ok_or_else(|| Error::NotFound {
                path: format!("texture-{}", node_handle.as_i32()),
            })?;
        
        // Get mutable texture from runtime
        if let Some(runtime) = &mut entry.runtime {
            if let Some(tex_runtime) = runtime.as_any_mut().downcast_mut::<crate::nodes::TextureRuntime>() {
                tex_runtime.texture_mut()
                    .ok_or_else(|| Error::Other {
                        message: "Texture not initialized".to_string(),
                    })
            } else {
                Err(Error::Other {
                    message: "Texture runtime not found".to_string(),
                })
            }
        } else {
            Err(Error::Other {
                message: "Runtime not initialized".to_string(),
            })
        }
    }

    fn get_time(&self) -> f32 {
        // Convert total_ms to seconds
        self.frame_time.total_ms as f32 / 1000.0
    }
    
    fn get_output(&mut self, handle: crate::runtime::contexts::OutputHandle, _universe: u32, _start_ch: u32, _ch_count: u32) -> Result<&mut [u8], Error> {
        // Get output runtime
        let node_handle = handle.as_node_handle();
        let entry = self.nodes.get_mut(&node_handle)
            .ok_or_else(|| Error::NotFound {
                path: format!("output-{}", node_handle.as_i32()),
            })?;
        
        // Update output state_ver to current frame (state changed when accessed)
        entry.state_ver = self.frame_id;
        
        // Get output buffer from runtime
        // Note: OutputRuntime not implemented yet, so this will be a todo!()
        todo!("Get output buffer from OutputRuntime")
    }
}

impl<'a> RenderContextImpl<'a> {
    /// Ensure texture is rendered for current frame (lazy rendering)
    /// 
    /// This function:
    /// 1. Finds all shader nodes that target this texture
    /// 2. Renders those shaders in render_order (lowest first)
    /// 3. Marks the texture as rendered
    fn ensure_texture_rendered(
        nodes: &mut BTreeMap<NodeHandle, NodeEntry>,
        handle: crate::runtime::contexts::TextureHandle,
        frame_id: FrameId,
        frame_time: FrameTime,
    ) -> Result<(), Error> {
        let node_handle = handle.as_node_handle();
        
        // Check if already rendered
        if let Some(entry) = nodes.get(&node_handle) {
            if entry.state_ver >= frame_id {
                return Ok(());
            }
        }
        
        // Find all shader nodes that target this texture
        // Collect (handle, render_order) pairs for shaders targeting this texture
        let mut shader_handles: Vec<(NodeHandle, i32)> = Vec::new();
        
        for (shader_handle, entry) in nodes.iter() {
            if entry.kind == NodeKind::Shader 
                && entry.status == NodeStatus::Ok 
                && entry.runtime.is_some() 
            {
                // Check if this shader targets our texture
                if let Some(runtime) = entry.runtime.as_ref() {
                    if let Some(shader_runtime) = runtime.as_any().downcast_ref::<crate::nodes::ShaderRuntime>() {
                        if shader_runtime.targets_texture(handle) {
                            // Get render_order from shader runtime
                            let render_order = shader_runtime.render_order();
                            shader_handles.push((*shader_handle, render_order));
                        }
                    }
                }
            }
        }
        
        // Sort by render_order (lowest first)
        shader_handles.sort_by_key(|(_, order)| *order);
        
        // Mark texture as rendering BEFORE calling shader.render() to prevent infinite recursion
        // When shader.render() calls get_texture_mut(), it will see state_ver >= frame_id
        // and skip re-rendering
        if let Some(entry) = nodes.get_mut(&node_handle) {
            entry.state_ver = frame_id;
        }
        
        // Render each shader that targets this texture
        for (shader_handle, _) in shader_handles {
            // Create RenderContext for each shader render
            let mut ctx = RenderContextImpl {
                nodes,
                frame_id,
                frame_time,
            };
            
            // Get shader runtime and render
            // Use unsafe to work around borrow checker (same pattern as fixture rendering)
            let render_result = {
                if let Some(entry) = ctx.nodes.get_mut(&shader_handle) {
                    if let Some(runtime) = entry.runtime.as_mut() {
                        // runtime is &mut Box<dyn NodeRuntime>
                        // render() needs &mut self (runtime) and &mut ctx
                        // Both need mutable access, but runtime is inside ctx.nodes
                        // Workaround: use unsafe to get raw pointer
                        let runtime_ptr: *mut dyn NodeRuntime = runtime.as_mut();
                        // SAFETY: runtime_ptr is valid for the duration of this block
                        // We're not storing it or using it after the block
                        unsafe {
                            (*runtime_ptr).render(&mut ctx)
                        }
                    } else {
                        Ok(())
                    }
                } else {
                    Ok(())
                }
            };
            
            render_result?;
            
            // Update shader state_ver after render
            if let Some(entry) = ctx.nodes.get_mut(&shader_handle) {
                entry.state_ver = frame_id;
            }
        }
        
        Ok(())
    }
}
