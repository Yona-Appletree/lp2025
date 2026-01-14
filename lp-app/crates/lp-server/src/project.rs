//! Project wrapper for managing a single project instance

extern crate alloc;

use crate::error::ServerError;
use alloc::{boxed::Box, format, string::String};
use lp_engine::ProjectRuntime;
use lp_shared::fs::LpFs;

/// A project instance wrapping a ProjectRuntime
pub struct Project {
    /// Project name/identifier
    name: String,
    /// Project filesystem path
    path: String,
    /// The underlying ProjectRuntime instance
    runtime: ProjectRuntime,
}

impl Project {
    /// Create a new project instance
    ///
    /// The project must already exist on the filesystem.
    pub fn new(name: String, path: String, fs: Box<dyn LpFs>) -> Result<Self, ServerError> {
        let runtime = ProjectRuntime::new(fs).map_err(|e| ServerError::Core(format!("{}", e)))?;

        Ok(Self { name, path, runtime })
    }

    /// Get the project name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the project path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get mutable access to the underlying ProjectRuntime
    pub fn runtime_mut(&mut self) -> &mut ProjectRuntime {
        &mut self.runtime
    }

    /// Get immutable access to the underlying ProjectRuntime
    pub fn runtime(&self) -> &ProjectRuntime {
        &self.runtime
    }

    /// Reload the project from the filesystem
    pub fn reload(&mut self) -> Result<(), ServerError> {
        // todo!("Implement project reload")
        Ok(())
    }
}
