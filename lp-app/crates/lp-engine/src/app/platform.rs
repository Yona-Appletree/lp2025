//! Platform-specific trait implementations wrapper

use crate::traits::{LpFs, OutputProvider};

/// Platform-specific trait implementations
///
/// Wraps the platform-specific implementations of Filesystem and OutputProvider
/// that firmware provides. LpApp uses these to interact with hardware.
pub struct Platform {
    /// Filesystem implementation for loading projects
    pub fs: alloc::boxed::Box<dyn LpFs>,
    /// Output provider for creating LED outputs
    pub output: alloc::boxed::Box<dyn OutputProvider>,
}

impl Platform {
    /// Create a new Platform with the provided trait implementations
    pub fn new(
        fs: alloc::boxed::Box<dyn LpFs>,
        output: alloc::boxed::Box<dyn OutputProvider>,
    ) -> Self {
        Self { fs, output }
    }
}
