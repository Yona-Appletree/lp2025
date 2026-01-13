use crate::nodes::{NodeConfig, NodeKind};
use serde::{Deserialize, Serialize};

/// Texture node configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextureConfig {
    /// Memory texture - simple buffer
    Memory {
        width: u32,
        height: u32,
        // format: todo!(), // Will add format later
    },
}

impl NodeConfig for TextureConfig {
    fn kind(&self) -> NodeKind {
        NodeKind::Texture
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_config_kind() {
        let config = TextureConfig::Memory {
            width: 100,
            height: 200,
        };
        assert_eq!(config.kind(), NodeKind::Texture);
    }
}
