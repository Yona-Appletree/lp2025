//! Platform-agnostic abstraction traits

pub mod led_output;
pub mod output_provider;
pub mod transport;

pub use lp_core_util::fs::LpFs;
pub use led_output::{LedOutput, Rgb, Rgba};
pub use output_provider::OutputProvider;
pub use transport::Transport;
