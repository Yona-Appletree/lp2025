//! Texture node runtime

use crate::error::Error;
use crate::project::runtime::NodeStatus;
use crate::runtime::contexts::{InitContext, TextureRenderContext};
use crate::runtime::lifecycle::NodeLifecycle;
use crate::runtime::NodeRuntimeBase;
use crate::util::Texture;
use alloc::{format, string::ToString};
use lp_shared::project::frame_id::FrameId;
use lp_shared::project::nodes::handle::NodeHandle;
use lp_shared::project::nodes::texture::config::TextureNode;

/// Texture node runtime
#[derive(Debug)]
pub struct TextureNodeRuntime {
    pub base: NodeRuntimeBase,
    config: TextureNode,
    texture: Texture,
    status: NodeStatus,
}

impl TextureNodeRuntime {
    /// Create a new texture node runtime (uninitialized)
    pub fn new(handle: NodeHandle, path: String) -> Self {
        Self {
            base: NodeRuntimeBase::new(handle, path, FrameId(0)), // Will be updated in init
            config: TextureNode::Memory {
                size: [1, 1],
                format: "RGB8".to_string(),
            }, // Temporary, will be replaced in init
            texture: Texture::new(1, 1, "RGB8".to_string()).unwrap(), // Temporary, will be replaced in init
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

    /// Update the last state frame (called when texture pixels change)
    pub fn mark_state_changed(&mut self, frame: FrameId) {
        self.base.update_state_frame(frame);
    }

    /// Get read-only access to the texture
    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    /// Get mutable access to the texture
    pub fn texture_mut(&mut self) -> &mut Texture {
        &mut self.texture
    }

    /// Get the current status
    pub fn status(&self) -> &NodeStatus {
        &self.status
    }

    /// Get the texture configuration
    pub fn config(&self) -> &TextureNode {
        &self.config
    }
}

impl Default for TextureNodeRuntime {
    fn default() -> Self {
        Self::new(NodeHandle::NONE, String::new())
    }
}

impl NodeLifecycle for TextureNodeRuntime {
    type Config = TextureNode;
    type RenderContext<'a> = TextureRenderContext;

    fn init(&mut self, config: &Self::Config, _ctx: &InitContext) -> Result<(), Error> {
        // Store config
        self.config = config.clone();
        // Note: last_config_frame will be updated by ProjectRuntime after init() completes

        match config {
            TextureNode::Memory { size, format } => {
                let [width, height] = *size;
                match Texture::new(width, height, format.clone()) {
                    Ok(texture) => {
                        self.texture = texture;
                        self.status = NodeStatus::Ok;
                        Ok(())
                    }
                    Err(e) => {
                        self.status = NodeStatus::Error {
                            status_message: format!("Failed to create texture: {}", e),
                        };
                        Err(e)
                    }
                }
            }
        }
    }

    fn render(&mut self, _ctx: &mut Self::RenderContext<'_>) -> Result<(), Error> {
        // Textures don't update themselves - they're updated by shaders
        Ok(())
    }

    fn destroy(&mut self) -> Result<(), Error> {
        // No cleanup needed for textures
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::texture::formats;
    use alloc::string::ToString;

    #[test]
    fn test_texture_node_runtime_init() {
        let mut runtime = TextureNodeRuntime::new(NodeHandle::NONE, "/test/texture.texture".to_string());
        let config = TextureNode::Memory {
            size: [64, 64],
            format: formats::RGB8.to_string(),
        };
        let project_config = lp_shared::project::config::ProjectConfig {
            uid: "test".to_string(),
            name: "Test".to_string(),
        };
        let textures = alloc::collections::BTreeMap::new();
        let shaders = alloc::collections::BTreeMap::new();
        let outputs = alloc::collections::BTreeMap::new();
        let fixtures = alloc::collections::BTreeMap::new();
        let ctx = InitContext::new(&project_config, &textures, &shaders, &outputs, &fixtures);

        assert!(runtime.init(&config, &ctx).is_ok());
        assert_eq!(runtime.texture().width(), 64);
        assert_eq!(runtime.texture().height(), 64);
        assert_eq!(runtime.texture().format(), formats::RGB8);
        assert!(matches!(runtime.status(), NodeStatus::Ok));
    }

    #[test]
    fn test_texture_node_runtime_init_invalid_format() {
        let mut runtime = TextureNodeRuntime::new(NodeHandle::NONE, "/test/texture.texture".to_string());
        let config = TextureNode::Memory {
            size: [64, 64],
            format: "INVALID".to_string(),
        };
        let project_config = lp_shared::project::config::ProjectConfig {
            uid: "test".to_string(),
            name: "Test".to_string(),
        };
        let textures = alloc::collections::BTreeMap::new();
        let shaders = alloc::collections::BTreeMap::new();
        let outputs = alloc::collections::BTreeMap::new();
        let fixtures = alloc::collections::BTreeMap::new();
        let ctx = InitContext::new(&project_config, &textures, &shaders, &outputs, &fixtures);

        assert!(runtime.init(&config, &ctx).is_err());
        assert!(matches!(runtime.status(), NodeStatus::Error { .. }));
    }

    #[test]
    fn test_texture_accessors() {
        let mut runtime = TextureNodeRuntime::new(NodeHandle::NONE, "/test/texture.texture".to_string());
        let config = TextureNode::Memory {
            size: [10, 10],
            format: formats::RGB8.to_string(),
        };
        let project_config = lp_shared::project::config::ProjectConfig {
            uid: "test".to_string(),
            name: "Test".to_string(),
        };
        let textures = alloc::collections::BTreeMap::new();
        let shaders = alloc::collections::BTreeMap::new();
        let outputs = alloc::collections::BTreeMap::new();
        let fixtures = alloc::collections::BTreeMap::new();
        let ctx = InitContext::new(&project_config, &textures, &shaders, &outputs, &fixtures);

        runtime.init(&config, &ctx).unwrap();

        // Test read access
        let texture = runtime.texture();
        assert_eq!(texture.width(), 10);

        // Test write access
        runtime.texture_mut().set_pixel(5, 5, [255, 128, 64, 255]);
        let pixel = runtime.texture().get_pixel(5, 5).unwrap();
        assert_eq!(pixel[0], 255);
        assert_eq!(pixel[1], 128);
        assert_eq!(pixel[2], 64);
    }
}
