//! Project wrapper for managing a single project instance

extern crate alloc;

use crate::error::ServerError;
use alloc::{format, string::String};
use lp_core::app::{LpApp, Platform};
use lp_core::error::Error as CoreError;

/// A project instance wrapping an LpApp
pub struct Project {
    /// Project name/identifier
    name: String,
    /// Project filesystem path
    path: String,
    /// The underlying LpApp instance
    app: LpApp,
}

impl Project {
    /// Create a new project instance
    ///
    /// The project must already exist on the filesystem.
    pub fn new(name: String, path: String, platform: Platform) -> Result<Self, ServerError> {
        let mut app = LpApp::new(platform);

        // Load the project
        app.load_project(&path).map_err(|e| match e {
            CoreError::Filesystem(msg) => ServerError::Filesystem(msg),
            e => ServerError::Core(format!("{}", e)),
        })?;

        Ok(Self { name, path, app })
    }

    /// Get the project name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the project path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get mutable access to the underlying LpApp
    pub fn app_mut(&mut self) -> &mut LpApp {
        &mut self.app
    }

    /// Get immutable access to the underlying LpApp
    pub fn app(&self) -> &LpApp {
        &self.app
    }

    /// Reload the project from the filesystem
    pub fn reload(&mut self) -> Result<(), ServerError> {
        self.app.load_project(&self.path).map_err(|e| match e {
            CoreError::Filesystem(msg) => ServerError::Filesystem(msg),
            e => ServerError::Core(format!("{}", e)),
        })
    }
}
