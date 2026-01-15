use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Texture node state - runtime values
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextureState {
    /// Texture pixel data (RGBA, width * height * 4)
    pub texture_data: Vec<u8>,
}
