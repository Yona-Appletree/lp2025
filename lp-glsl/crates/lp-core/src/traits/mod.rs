//! Platform-agnostic abstraction traits

pub mod led_output;
pub mod output_provider;
pub mod transport;

pub use crate::fs::Filesystem;
pub use led_output::{LedOutput, Rgb, Rgba};
pub use output_provider::OutputProvider;
pub use transport::Transport;
