//! File change tracking types

use alloc::string::String;

/// Represents an event caused by a file or directory change
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsChange {
    /// Path affected by the change
    pub path: String,
    /// Type of change
    pub change_type: ChangeType,
}

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
