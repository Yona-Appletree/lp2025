//! Filesystem abstraction trait
//!
//! All paths in this trait are relative to the project root. The project root is the directory
//! containing `project.json`. Leading slashes indicate paths from the project root
//! (e.g., `/project.json`, `/src/my-shader.shader/main.glsl`).
//!
//! Filesystem instances have a root path (especially for real filesystem implementations) to
//! provide security by preventing access outside the project directory.

use crate::error::FsError;

/// Platform-agnostic filesystem trait
///
/// All paths are relative to the project root. `/project.json` is always the project
/// configuration file. Filesystem instances have a root path for security.
pub trait LpFs {
    /// Read a file from the filesystem
    ///
    /// Path is relative to project root (e.g., `/project.json`, `/src/my-shader.shader/main.glsl`).
    ///
    /// Returns the file contents as a byte vector, or an error if the file doesn't exist
    /// or cannot be read.
    fn read_file(&self, path: &str) -> Result<alloc::vec::Vec<u8>, FsError>;

    /// Write data to a file in the filesystem
    ///
    /// Path is relative to project root.
    ///
    /// Creates the file if it doesn't exist, overwrites if it does.
    fn write_file(&self, path: &str, data: &[u8]) -> Result<(), FsError>;

    /// Check if a file exists in the filesystem
    ///
    /// Path is relative to project root.
    fn file_exists(&self, path: &str) -> Result<bool, FsError>;

    /// List directory contents (files and subdirectories)
    ///
    /// Path is relative to project root (e.g., `/src` or `/src/nested`).
    ///
    /// Returns paths relative to project root. The returned paths include the directory
    /// path prefix (e.g., listing `/src` might return `["/src/my-shader.shader", "/src/my-texture.texture"]`).
    fn list_dir(&self, path: &str) -> Result<alloc::vec::Vec<alloc::string::String>, FsError>;

    /// Create a new filesystem view rooted at a subdirectory
    ///
    /// Returns a new `LpFs` instance where all paths are relative to the specified subdirectory.
    /// The subdirectory path is relative to the current root.
    ///
    /// For example, if the current root is `/projects` and you chroot to `my-project`,
    /// then paths like `/project.json` in the new view will resolve to `/projects/my-project/project.json`
    /// in the original filesystem.
    fn chroot(&self, subdir: &str) -> Result<alloc::boxed::Box<dyn LpFs>, FsError>;
}
