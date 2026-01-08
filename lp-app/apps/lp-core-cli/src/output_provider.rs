//! Host OutputProvider implementation

use hashbrown::HashMap;
use lp_core::error::Error;
use lp_core::nodes::id::OutputId;
use lp_core::nodes::output::config::OutputNode;
use lp_core::traits::{LedOutput, OutputProvider};
use std::sync::{Arc, Mutex};

use crate::led_output::HostLedOutput;

/// Host implementation of OutputProvider
///
/// Creates HostLedOutput instances for visualization in egui.
/// Tracks outputs in a HashMap for UI access.
pub struct HostOutputProvider {
    outputs: Arc<Mutex<HashMap<OutputId, Arc<Mutex<HostLedOutput>>>>>,
}

impl HostOutputProvider {
    /// Create a new HostOutputProvider
    pub fn new() -> Self {
        Self {
            outputs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create and track an output with the given ID
    ///
    /// This is a convenience method that creates an output and tracks it.
    /// The trait implementation calls this internally when possible.
    #[allow(dead_code)]
    pub fn create_and_track_output(
        &self,
        id: OutputId,
        config: &OutputNode,
    ) -> Result<Arc<Mutex<HostLedOutput>>, Error> {
        let output = self.create_host_output(config)?;
        let output_arc = Arc::new(Mutex::new(output));
        self.outputs
            .lock()
            .unwrap()
            .insert(id, Arc::clone(&output_arc));
        Ok(output_arc)
    }

    /// Get an output by ID
    #[allow(dead_code)]
    pub fn get_output(&self, id: OutputId) -> Option<Arc<Mutex<HostLedOutput>>> {
        self.outputs.lock().unwrap().get(&id).map(Arc::clone)
    }

    /// Get all outputs
    pub fn get_all_outputs(&self) -> HashMap<OutputId, Arc<Mutex<HostLedOutput>>> {
        self.outputs.lock().unwrap().clone()
    }

    /// Create a HostLedOutput from config (internal helper)
    fn create_host_output(&self, config: &OutputNode) -> Result<HostLedOutput, Error> {
        match config {
            OutputNode::GpioStrip { chip, count, .. } => {
                // Derive bytes_per_pixel from chip type
                // For now, assume ws2812 = 3 bytes (RGB)
                let bytes_per_pixel = match chip.as_str() {
                    "ws2812" | "ws2812b" => 3,
                    _ => {
                        return Err(Error::Validation(format!("Unknown chip type: {}", chip)));
                    }
                };

                let pixel_count = *count as usize;
                Ok(HostLedOutput::new(pixel_count, bytes_per_pixel))
            }
        }
    }
}

impl Default for HostOutputProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputProvider for HostOutputProvider {
    fn create_output(
        &self,
        config: &OutputNode,
        output_id: Option<OutputId>,
    ) -> Result<std::boxed::Box<dyn LedOutput>, Error> {
        let output = self.create_host_output(config)?;

        // Track output if OutputId is provided
        if let Some(id) = output_id {
            let output_arc = Arc::new(Mutex::new(output));
            self.outputs
                .lock()
                .unwrap()
                .insert(id, Arc::clone(&output_arc));
            // Return a wrapper that delegates to the tracked output
            Ok(std::boxed::Box::new(HostLedOutputWrapper {
                inner: output_arc,
            }))
        } else {
            Ok(std::boxed::Box::new(output))
        }
    }
}

/// Wrapper around HostLedOutput that delegates to the tracked Arc<Mutex<HostLedOutput>>
struct HostLedOutputWrapper {
    inner: Arc<Mutex<HostLedOutput>>,
}

impl LedOutput for HostLedOutputWrapper {
    fn write_pixels(&mut self, pixels: &[u8]) -> Result<(), Error> {
        self.inner.lock().unwrap().write_pixels(pixels)
    }

    fn get_pixel_count(&self) -> usize {
        self.inner.lock().unwrap().get_pixel_count()
    }
}
