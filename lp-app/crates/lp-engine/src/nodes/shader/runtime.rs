use crate::error::Error;
use crate::nodes::NodeRuntime;
use crate::runtime::contexts::{NodeInitContext, RenderContext};

/// Shader node runtime
pub struct ShaderRuntime {
    // Will add fields later
}

impl ShaderRuntime {
    pub fn new() -> Self {
        Self {}
    }
}

impl NodeRuntime for ShaderRuntime {
    fn init(&mut self, _ctx: &dyn NodeInitContext) -> Result<(), Error> {
        // todo!("Shader initialization - load GLSL, compile")
        Ok(())
    }
    
    fn render(&mut self, _ctx: &dyn RenderContext) -> Result<(), Error> {
        // todo!("Shader rendering - execute GLSL")
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_runtime_creation() {
        let runtime = ShaderRuntime::new();
        let _boxed: alloc::boxed::Box<dyn NodeRuntime> = alloc::boxed::Box::new(runtime);
    }
}
