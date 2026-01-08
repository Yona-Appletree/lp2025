//! Error types for lp-engine

use alloc::{format, string::String};
use core::fmt;
use lp_shared::error::FsError;

/// Error type for lp-engine operations
#[derive(Debug, Clone)]
pub enum Error {
    /// Serialization/deserialization error
    Serialization(String),
    /// Filesystem error
    Filesystem(String),
    /// Protocol error
    Protocol(String),
    /// Project validation error
    Validation(String),
    /// Node-specific error
    Node(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            Error::Filesystem(msg) => write!(f, "Filesystem error: {}", msg),
            Error::Protocol(msg) => write!(f, "Protocol error: {}", msg),
            Error::Validation(msg) => write!(f, "Validation error: {}", msg),
            Error::Node(msg) => {
                // For node errors, preserve the original formatting as-is
                // GlslError and other error types already have proper formatting
                // with line numbers and carets, so we don't want to modify it
                write!(f, "{}", msg)
            }
        }
    }
}

impl From<FsError> for Error {
    fn from(err: FsError) -> Self {
        match err {
            FsError::Filesystem(msg) => Error::Filesystem(msg),
            FsError::NotFound(msg) => Error::Filesystem(format!("File not found: {}", msg)),
            FsError::InvalidPath(msg) => Error::Filesystem(format!("Invalid path: {}", msg)),
        }
    }
}
