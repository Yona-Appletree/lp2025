use crate::error::Error;
use lp_model::NodeSpecifier;
use lp_shared::fs::LpFs;

#[allow(dead_code)] // Will be used when implementing resolution

/// Handles for resolved nodes (opaque types for now)
pub struct TextureHandle(u32);
pub struct OutputHandle(u32);

impl TextureHandle {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

impl OutputHandle {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Context for node initialization
pub trait NodeInitContext {
    /// Resolve an output node specifier to a handle
    fn resolve_output(&self, _spec: &NodeSpecifier) -> Result<OutputHandle, Error> {
        todo!("Node resolution not implemented yet")
    }
    
    /// Resolve a texture node specifier to a handle
    fn resolve_texture(&self, _spec: &NodeSpecifier) -> Result<TextureHandle, Error> {
        todo!("Node resolution not implemented yet")
    }
    
    /// Get filesystem for this node
    fn get_node_fs(&self) -> &dyn LpFs {
        todo!("Filesystem access not implemented yet")
    }
}

/// Context for rendering
pub trait RenderContext {
    /// Get texture data (triggers lazy rendering if needed)
    fn get_texture(&mut self, _handle: TextureHandle) -> Result<&[u8], Error> {
        todo!("Texture rendering not implemented yet")
    }
    
    /// Get output buffer slice
    fn get_output(&mut self, _handle: OutputHandle, _universe: u32, _start_ch: u32, _ch_count: u32) -> Result<&mut [u8], Error> {
        todo!("Output access not implemented yet")
    }
}
