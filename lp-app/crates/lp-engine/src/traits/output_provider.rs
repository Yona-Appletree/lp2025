//! Output provider trait for creating LED output handles

use crate::error::Error;
use crate::traits::LedOutput;
use lp_shared::nodes::id::OutputId;
use lp_shared::nodes::output::config::OutputNode;

/// Trait for creating LED output handles from configuration
///
/// Implemented by firmware-specific code (ESP32, host, etc.)
/// to set up hardware based on output configuration.
pub trait OutputProvider {
    /// Create and configure an LED output handle from configuration
    ///
    /// For `GpioStrip`: configures GPIO pin from `config.gpio_pin`,
    /// sets up chip driver (ws2812, etc.), and returns a handle.
    ///
    /// The `output_id` parameter is optional and can be used for tracking
    /// outputs (e.g., in HostOutputProvider for UI display).
    ///
    /// Returns an error if the output cannot be created.
    fn create_output(
        &self,
        config: &OutputNode,
        output_id: Option<OutputId>,
    ) -> Result<alloc::boxed::Box<dyn LedOutput>, Error>;
}
