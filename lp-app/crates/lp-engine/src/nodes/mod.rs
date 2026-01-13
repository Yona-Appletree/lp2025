use crate::error::Error;
use crate::runtime::contexts::{NodeInitContext, RenderContext};

pub mod texture;
pub mod shader;
pub mod output;
pub mod fixture;

pub use texture::TextureRuntime;
pub use shader::ShaderRuntime;
pub use output::OutputRuntime;
pub use fixture::FixtureRuntime;

/// Node runtime trait - all node runtimes implement this
pub trait NodeRuntime: Send + Sync {
    /// Initialize the node
    fn init(&mut self, ctx: &dyn NodeInitContext) -> Result<(), Error>;
    
    /// Render the node
    fn render(&mut self, ctx: &dyn RenderContext) -> Result<(), Error>;
    
    /// Destroy the node (cleanup)
    fn destroy(&mut self) -> Result<(), Error> {
        todo!("Node cleanup not implemented yet")
    }
}

// Re-export NodeConfig from lp-model
pub use lp_model::NodeConfig;
