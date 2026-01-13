use alloc::vec::Vec;

/// Texture node state - runtime values
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureState {
    /// Texture pixel data (RGBA, width * height * 4)
    pub texture_data: Vec<u8>,
}
