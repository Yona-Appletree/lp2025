use crate::error::Error;
use crate::nodes::NodeRuntime;
use crate::runtime::contexts::{NodeInitContext, RenderContext};

/// Texture node runtime
pub struct TextureRuntime {
    // Will add fields later
}

impl TextureRuntime {
    pub fn new() -> Self {
        Self {}
    }
}

impl NodeRuntime for TextureRuntime {
    fn init(&mut self, _ctx: &dyn NodeInitContext) -> Result<(), Error> {
        // todo!("Texture initialization")
        Ok(())
    }
    
    fn render(&mut self, _ctx: &dyn RenderContext) -> Result<(), Error> {
        // todo!("Texture rendering")
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_runtime_creation() {
        let runtime = TextureRuntime::new();
        // Just verify it compiles and can be created
        let _boxed: alloc::boxed::Box<dyn NodeRuntime> = alloc::boxed::Box::new(runtime);
    }
}
