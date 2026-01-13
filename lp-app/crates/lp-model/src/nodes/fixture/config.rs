use crate::nodes::{NodeConfig, NodeKind, NodeSpecifier};
use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Fixture node configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixtureConfig {
    /// Output node specifier
    pub output_spec: NodeSpecifier,
    /// Texture node specifier
    pub texture_spec: NodeSpecifier,
    /// Mapping configuration (simplified for now)
    pub mapping: String, // todo!() - will be structured type later
    /// Lamp type (color order, etc.)
    pub lamp_type: String, // todo!() - will be enum later
    /// Transform matrix (4x4)
    pub transform: [[f32; 4]; 4], // todo!() - will be proper matrix type later
}

impl NodeConfig for FixtureConfig {
    fn kind(&self) -> NodeKind {
        NodeKind::Fixture
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_fixture_config_kind() {
        let config = FixtureConfig {
            output_spec: NodeSpecifier::from("/src/out.output"),
            texture_spec: NodeSpecifier::from("/src/tex.texture"),
            mapping: "linear".to_string(),
            lamp_type: "rgb".to_string(),
            transform: [[1.0; 4]; 4],
        };
        assert_eq!(config.kind(), NodeKind::Fixture);
    }
}
