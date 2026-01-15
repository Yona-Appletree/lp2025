use crate::nodes::{NodeConfig, NodeKind, NodeSpecifier};
use alloc::string::{String, ToString};
use serde::{Deserialize, Serialize};

/// Shader node configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShaderConfig {
    /// Path to GLSL file (relative to node directory)
    pub glsl_path: String,
    /// Texture to render to (specifier)
    pub texture_spec: NodeSpecifier,
    /// Render order - lower numbers render first (default 0)
    pub render_order: i32,
}

impl Default for ShaderConfig {
    fn default() -> Self {
        Self {
            glsl_path: "main.glsl".to_string(),
            texture_spec: NodeSpecifier::from(""),
            render_order: 0,
        }
    }
}

impl NodeConfig for ShaderConfig {
    fn kind(&self) -> NodeKind {
        NodeKind::Shader
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_config_kind() {
        let config = ShaderConfig {
            glsl_path: "main.glsl".to_string(),
            texture_spec: NodeSpecifier::from("/src/tex.texture"),
            render_order: 0,
        };
        assert_eq!(config.kind(), NodeKind::Shader);
    }

    #[test]
    fn test_shader_config_default() {
        let config = ShaderConfig::default();
        assert_eq!(config.glsl_path, "main.glsl");
        assert_eq!(config.render_order, 0);
    }
}
