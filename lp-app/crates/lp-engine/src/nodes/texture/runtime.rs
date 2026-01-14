use crate::error::Error;
use crate::nodes::NodeRuntime;
use crate::runtime::contexts::{NodeInitContext, RenderContext};
use alloc::{format, string::ToString, vec::Vec};
use lp_model::{
    NodeHandle,
    nodes::texture::{TextureConfig, TextureState},
};
use lp_shared::Texture;

/// Texture node runtime
pub struct TextureRuntime {
    config: Option<TextureConfig>,
    texture: Option<Texture>,
    node_handle: NodeHandle,
}

impl TextureRuntime {
    pub fn new(node_handle: NodeHandle) -> Self {
        Self {
            config: None,
            texture: None,
            node_handle,
        }
    }

    pub fn set_config(&mut self, config: TextureConfig) {
        self.config = Some(config);
    }

    pub fn texture(&self) -> Option<&Texture> {
        self.texture.as_ref()
    }

    pub fn texture_mut(&mut self) -> Option<&mut Texture> {
        self.texture.as_mut()
    }

    pub fn get_state(&self) -> TextureState {
        // Extract state for sync API
        if let Some(tex) = &self.texture {
            TextureState {
                texture_data: tex.data().to_vec(),
            }
        } else {
            TextureState {
                texture_data: Vec::new(),
            }
        }
    }
}

impl NodeRuntime for TextureRuntime {
    fn init(&mut self, _ctx: &dyn NodeInitContext) -> Result<(), Error> {
        let config = self.config.as_ref().ok_or_else(|| Error::InvalidConfig {
            node_path: format!("texture-{}", self.node_handle.as_i32()),
            reason: "Config not set".to_string(),
        })?;

        // Create texture with RGBA8 format (default for now)
        // Format will be added to TextureConfig later
        let format = "RGBA8".to_string();
        let texture = Texture::new(config.width, config.height, format).map_err(|e| {
            Error::InvalidConfig {
                node_path: format!("texture-{}", self.node_handle.as_i32()),
                reason: format!("Failed to create texture: {}", e),
            }
        })?;

        self.texture = Some(texture);
        Ok(())
    }

    fn render(&mut self, _ctx: &mut dyn RenderContext) -> Result<(), Error> {
        // No-op - textures don't render themselves, shaders render to textures
        Ok(())
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_runtime_creation() {
        use lp_model::NodeHandle;
        let handle = NodeHandle::new(0);
        let runtime = TextureRuntime::new(handle);
        // Just verify it compiles and can be created
        let _boxed: alloc::boxed::Box<dyn NodeRuntime> = alloc::boxed::Box::new(runtime);
    }
}
