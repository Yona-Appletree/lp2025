use crate::error::Error;
use crate::nodes::NodeRuntime;
use crate::runtime::contexts::{NodeInitContext, RenderContext};

/// Fixture node runtime
pub struct FixtureRuntime {
    // Will add fields later
}

impl FixtureRuntime {
    pub fn new() -> Self {
        Self {}
    }
}

impl NodeRuntime for FixtureRuntime {
    fn init(&mut self, _ctx: &dyn NodeInitContext) -> Result<(), Error> {
        // todo!("Fixture initialization - resolve output/texture references")
        Ok(())
    }
    
    fn render(&mut self, _ctx: &dyn RenderContext) -> Result<(), Error> {
        // todo!("Fixture rendering - sample texture, write to output")
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_runtime_creation() {
        let runtime = FixtureRuntime::new();
        let _boxed: alloc::boxed::Box<dyn NodeRuntime> = alloc::boxed::Box::new(runtime);
    }
}
