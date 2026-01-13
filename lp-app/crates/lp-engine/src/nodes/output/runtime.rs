use crate::error::Error;
use crate::nodes::NodeRuntime;
use crate::runtime::contexts::{NodeInitContext, RenderContext};

/// Output node runtime
pub struct OutputRuntime {
    // Will add fields later
}

impl OutputRuntime {
    pub fn new() -> Self {
        Self {}
    }
}

impl NodeRuntime for OutputRuntime {
    fn init(&mut self, _ctx: &dyn NodeInitContext) -> Result<(), Error> {
        // todo!("Output initialization - setup GPIO, etc.")
        Ok(())
    }
    
    fn render(&mut self, _ctx: &dyn RenderContext) -> Result<(), Error> {
        // todo!("Output rendering - flush buffers")
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_runtime_creation() {
        let runtime = OutputRuntime::new();
        let _boxed: alloc::boxed::Box<dyn NodeRuntime> = alloc::boxed::Box::new(runtime);
    }
}
