//! File change tracking types

use alloc::string::String;

/// Type of file change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    /// File was created
    Create,
    /// File was modified
    Modify,
    /// File was deleted
    Delete,
}

/// Represents a change to a file in the filesystem
///
/// Path is relative to project root (e.g., `/project.json`, `/src/my-shader.shader/main.glsl`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    /// Path relative to project root
    pub path: String,
    /// Type of change
    pub change_type: ChangeType,
}
