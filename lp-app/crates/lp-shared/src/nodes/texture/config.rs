//! Texture node configuration

use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Texture node types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum TextureNode {
    #[serde(rename = "Memory")]
    Memory { size: [u32; 2], format: String },
}

// TODO: Use this instead of the mod formats
pub enum TextureFormat {
    RGB8,
    RGBA8,
    R8,
}

/// Supported texture formats (OpenGL-style)
pub mod formats {
    /// RGB 8-bit per component (24-bit total)
    pub const RGB8: &str = "RGB8";
    /// RGBA 8-bit per component (32-bit total)
    pub const RGBA8: &str = "RGBA8";
    /// Single channel 8-bit (grayscale)
    pub const R8: &str = "R8";

    /// Check if a format string is valid
    pub fn is_valid(format: &str) -> bool {
        matches!(format, RGB8 | RGBA8 | R8)
    }

    /// Get bytes per pixel for a format
    pub fn bytes_per_pixel(format: &str) -> Option<usize> {
        match format {
            RGB8 => Some(3),
            RGBA8 => Some(4),
            R8 => Some(1),
            _ => None,
        }
    }
}
