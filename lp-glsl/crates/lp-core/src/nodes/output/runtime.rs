//! Output node runtime

use crate::error::Error;
use crate::nodes::output::config::OutputNode;
use crate::project::runtime::NodeStatus;
use crate::runtime::contexts::OutputRenderContext;
use crate::runtime::lifecycle::NodeLifecycle;
use crate::traits::LedOutput;
use alloc::{format, string::String, vec, vec::Vec};

/// Output node runtime
pub struct OutputNodeRuntime {
    config: OutputNode,
    handle: Option<alloc::boxed::Box<dyn LedOutput>>,
    pixel_count: usize,
    bytes_per_pixel: usize,
    buffer: Vec<u8>,
    status: NodeStatus,
}

impl OutputNodeRuntime {
    /// Create a new output node runtime (uninitialized)
    pub fn new() -> Self {
        Self {
            config: OutputNode::GpioStrip {
                chip: String::new(),
                gpio_pin: 0,
                count: 0,
            }, // Temporary, will be replaced in init
            handle: None,
            pixel_count: 0,
            bytes_per_pixel: 0,
            buffer: Vec::new(),
            status: NodeStatus::Ok,
        }
    }

    /// Get mutable access to the pixel buffer
    ///
    /// Fixtures write to this buffer, which is then sent to hardware in update().
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    /// Get the current status
    pub fn status(&self) -> &NodeStatus {
        &self.status
    }

    /// Get pixel count
    pub fn pixel_count(&self) -> usize {
        self.pixel_count
    }

    /// Get bytes per pixel
    pub fn bytes_per_pixel(&self) -> usize {
        self.bytes_per_pixel
    }

    /// Set the LED output handle
    ///
    /// Called by ProjectRuntime after creating the handle via OutputProvider.
    pub fn set_handle(&mut self, handle: alloc::boxed::Box<dyn LedOutput>) {
        self.handle = Some(handle);
    }

    /// Get the output configuration
    pub fn config(&self) -> &OutputNode {
        &self.config
    }
}

impl Default for OutputNodeRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeLifecycle for OutputNodeRuntime {
    type Config = OutputNode;
    type RenderContext<'a> = OutputRenderContext;

    fn init(
        &mut self,
        config: &Self::Config,
        _ctx: &crate::runtime::contexts::InitContext,
    ) -> Result<(), Error> {
        // Store config
        self.config = config.clone();

        match config {
            OutputNode::GpioStrip { chip, count, .. } => {
                // Derive bytes_per_pixel from chip type
                // For now, assume ws2812 = 3 bytes (RGB)
                // Future: could have a mapping or explicit format field
                self.bytes_per_pixel = match chip.as_str() {
                    "ws2812" | "ws2812b" => 3,
                    _ => {
                        self.status = NodeStatus::Error {
                            status_message: format!("Unknown chip type: {}", chip),
                        };
                        return Err(Error::Validation(format!("Unknown chip type: {}", chip)));
                    }
                };

                self.pixel_count = *count as usize;
                let buffer_size = self.pixel_count * self.bytes_per_pixel;
                self.buffer = vec![0; buffer_size];

                // Create output handle via OutputProvider
                // Note: OutputProvider will be passed from ProjectRuntime
                // For now, we'll set handle to None and it will be set later
                // Actually, we need OutputProvider in the context - let me check the design
                // The design says OutputProvider is passed to ProjectRuntime::init()
                // So we'll need to handle this differently - OutputProvider should be available
                // For now, leave handle as None - it will be set by ProjectRuntime
                self.handle = None;
                self.status = NodeStatus::Ok;
                Ok(())
            }
        }
    }

    fn render(&mut self, _ctx: &mut Self::RenderContext<'_>) -> Result<(), Error> {
        // Read buffer and send to hardware via handle
        if let Some(ref mut handle) = self.handle {
            handle.write_pixels(&self.buffer)?;
        }
        Ok(())
    }

    fn destroy(&mut self) -> Result<(), Error> {
        // Cleanup handle if needed
        self.handle = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::LedOutput;
    use alloc::string::ToString;

    // Mock LedOutput for testing
    struct MockLedOutput {
        pixel_count: usize,
        last_written: alloc::vec::Vec<u8>,
    }

    impl LedOutput for MockLedOutput {
        fn write_pixels(&mut self, pixels: &[u8]) -> Result<(), Error> {
            self.last_written = pixels.to_vec();
            Ok(())
        }

        fn get_pixel_count(&self) -> usize {
            self.pixel_count
        }
    }

    #[test]
    fn test_output_node_runtime_init() {
        let mut runtime = OutputNodeRuntime::new();
        let config = OutputNode::GpioStrip {
            chip: "ws2812".to_string(),
            gpio_pin: 4,
            count: 128,
        };
        let project_config = crate::project::config::ProjectConfig {
            uid: "test".to_string(),
            name: "Test".to_string(),
            nodes: crate::project::config::Nodes {
                outputs: hashbrown::HashMap::new(),
                textures: hashbrown::HashMap::new(),
                shaders: hashbrown::HashMap::new(),
                fixtures: hashbrown::HashMap::new(),
            },
        };
        let ctx = crate::runtime::contexts::InitContext::new(&project_config);

        assert!(runtime.init(&config, &ctx).is_ok());
        assert_eq!(runtime.pixel_count(), 128);
        assert_eq!(runtime.bytes_per_pixel(), 3);
        assert_eq!(runtime.buffer.len(), 128 * 3);
        assert!(matches!(runtime.status(), NodeStatus::Ok));
    }

    #[test]
    fn test_output_node_runtime_init_unknown_chip() {
        let mut runtime = OutputNodeRuntime::new();
        let config = OutputNode::GpioStrip {
            chip: "unknown".to_string(),
            gpio_pin: 4,
            count: 128,
        };
        let project_config = crate::project::config::ProjectConfig {
            uid: "test".to_string(),
            name: "Test".to_string(),
            nodes: crate::project::config::Nodes {
                outputs: hashbrown::HashMap::new(),
                textures: hashbrown::HashMap::new(),
                shaders: hashbrown::HashMap::new(),
                fixtures: hashbrown::HashMap::new(),
            },
        };
        let ctx = crate::runtime::contexts::InitContext::new(&project_config);

        assert!(runtime.init(&config, &ctx).is_err());
        assert!(matches!(runtime.status(), NodeStatus::Error { .. }));
    }

    #[test]
    fn test_buffer_mut() {
        let mut runtime = OutputNodeRuntime::new();
        let config = OutputNode::GpioStrip {
            chip: "ws2812".to_string(),
            gpio_pin: 4,
            count: 10,
        };
        let project_config = crate::project::config::ProjectConfig {
            uid: "test".to_string(),
            name: "Test".to_string(),
            nodes: crate::project::config::Nodes {
                outputs: hashbrown::HashMap::new(),
                textures: hashbrown::HashMap::new(),
                shaders: hashbrown::HashMap::new(),
                fixtures: hashbrown::HashMap::new(),
            },
        };
        let ctx = crate::runtime::contexts::InitContext::new(&project_config);

        runtime.init(&config, &ctx).unwrap();

        // Test buffer access
        let buffer = runtime.buffer_mut();
        assert_eq!(buffer.len(), 10 * 3);
        buffer[0] = 255;
        buffer[1] = 128;
        buffer[2] = 64;

        // Verify changes
        assert_eq!(runtime.buffer[0], 255);
        assert_eq!(runtime.buffer[1], 128);
        assert_eq!(runtime.buffer[2], 64);
    }
}
