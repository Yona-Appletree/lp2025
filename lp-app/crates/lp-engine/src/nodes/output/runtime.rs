use crate::error::Error;
use crate::nodes::NodeRuntime;
use crate::output::{OutputChannelHandle, OutputFormat};
use crate::runtime::contexts::{NodeInitContext, RenderContext};
use alloc::vec::Vec;

/// Output node runtime
pub struct OutputRuntime {
    /// Channel data buffer (DMX-style, sequential bytes)
    channel_data: Vec<u8>,
    /// Output channel handle from provider (None until initialized)
    channel_handle: Option<OutputChannelHandle>,
    /// GPIO pin number
    pin: u32,
}

impl OutputRuntime {
    pub fn new() -> Self {
        Self {
            channel_data: Vec::new(),
            channel_handle: None,
            pin: 0,
        }
    }

    /// Get mutable slice to channel data, extending if needed
    pub fn get_buffer_mut(&mut self, start_ch: u32, ch_count: u32) -> &mut [u8] {
        let end = (start_ch + ch_count) as usize;
        if end > self.channel_data.len() {
            self.channel_data.resize(end, 0);
        }
        &mut self.channel_data[start_ch as usize..end]
    }

    /// Get channel data (for state extraction)
    pub fn get_channel_data(&self) -> &[u8] {
        &self.channel_data
    }
}

impl NodeRuntime for OutputRuntime {
    fn init(&mut self, ctx: &dyn NodeInitContext) -> Result<(), Error> {
        // Get output config - need to reload from filesystem since we can't extract from trait object
        // For now, we'll need to get the config from the node path
        // Actually, we can't easily get the node path from the context
        // Let's use a simpler approach: calculate byte_count from fixtures, use default pin

        // For now, use a default byte_count (will be calculated properly later)
        // Default: 3 bytes for single RGB pixel
        let byte_count = 3u32;
        let format = OutputFormat::Ws2811;

        // Get pin from config - we'll need to reload config from filesystem
        // For now, use default pin 0 (will be fixed when we have proper config access)
        self.pin = 0;

        // Open output channel with provider
        let handle = ctx.output_provider().open(self.pin, byte_count, format)?;
        self.channel_handle = Some(handle);

        // Allocate buffer
        self.channel_data.resize(byte_count as usize, 0);

        Ok(())
    }

    fn render(&mut self, ctx: &mut dyn RenderContext) -> Result<(), Error> {
        // Flush buffer to provider if handle exists
        if let Some(handle) = self.channel_handle {
            ctx.output_provider().write(handle, &self.channel_data)?;
        }
        Ok(())
    }

    fn destroy(&mut self) -> Result<(), Error> {
        // Note: Provider cleanup should happen at a higher level
        // since destroy() doesn't have access to the provider context
        // For now, just clear the handle
        self.channel_handle = None;
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
    fn test_output_runtime_creation() {
        let runtime = OutputRuntime::new();
        let _boxed: alloc::boxed::Box<dyn NodeRuntime> = alloc::boxed::Box::new(runtime);
    }
}
