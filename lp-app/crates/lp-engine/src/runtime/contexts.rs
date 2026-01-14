use crate::error::Error;
use lp_model::{NodeHandle, NodeSpecifier};
use lp_shared::fs::LpFs;

/// Handle for resolved texture nodes
pub struct TextureHandle(NodeHandle);

impl TextureHandle {
    pub fn new(handle: NodeHandle) -> Self {
        Self(handle)
    }
    
    pub fn as_node_handle(&self) -> NodeHandle {
        self.0
    }
}

/// Handle for resolved output nodes
pub struct OutputHandle(NodeHandle);

impl OutputHandle {
    pub fn new(handle: NodeHandle) -> Self {
        Self(handle)
    }
    
    pub fn as_node_handle(&self) -> NodeHandle {
        self.0
    }
}

/// Context for node initialization
pub trait NodeInitContext {
    /// Resolve a node specifier to a node handle (common method)
    fn resolve_node(&self, spec: &NodeSpecifier) -> Result<NodeHandle, Error>;
    
    /// Resolve an output node specifier to a handle
    fn resolve_output(&self, spec: &NodeSpecifier) -> Result<OutputHandle, Error>;
    
    /// Resolve a texture node specifier to a handle
    fn resolve_texture(&self, spec: &NodeSpecifier) -> Result<TextureHandle, Error>;
    
    /// Get filesystem for this node
    fn get_node_fs(&self) -> &dyn LpFs;
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
