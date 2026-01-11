//! Type-safe node ID wrappers
//!
//! Node IDs are path-based strings relative to project root (e.g., "/src/my-shader.shader").

use alloc::string::{String, ToString};
use serde::{Deserialize, Serialize};

/// Texture node ID (path-based, e.g., "/src/my-texture.texture")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TextureId(pub String);

/// Output node ID (path-based, e.g., "/src/my-output.output")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OutputId(pub String);

/// Shader node ID (path-based, e.g., "/src/my-shader.shader")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ShaderId(pub String);

/// Fixture node ID (path-based, e.g., "/src/my-fixture.fixture")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FixtureId(pub String);

// From/Into implementations for String

impl From<String> for TextureId {
    fn from(value: String) -> Self {
        TextureId(value)
    }
}

impl From<&str> for TextureId {
    fn from(value: &str) -> Self {
        TextureId(value.to_string())
    }
}

impl From<TextureId> for String {
    fn from(value: TextureId) -> Self {
        value.0
    }
}

impl From<String> for OutputId {
    fn from(value: String) -> Self {
        OutputId(value)
    }
}

impl From<&str> for OutputId {
    fn from(value: &str) -> Self {
        OutputId(value.to_string())
    }
}

impl From<OutputId> for String {
    fn from(value: OutputId) -> Self {
        value.0
    }
}

impl From<String> for ShaderId {
    fn from(value: String) -> Self {
        ShaderId(value)
    }
}

impl From<&str> for ShaderId {
    fn from(value: &str) -> Self {
        ShaderId(value.to_string())
    }
}

impl From<ShaderId> for String {
    fn from(value: ShaderId) -> Self {
        value.0
    }
}

impl From<String> for FixtureId {
    fn from(value: String) -> Self {
        FixtureId(value)
    }
}

impl From<&str> for FixtureId {
    fn from(value: &str) -> Self {
        FixtureId(value.to_string())
    }
}

impl From<FixtureId> for String {
    fn from(value: FixtureId) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use serde_json;

    #[test]
    fn test_id_serialization() {
        let texture_id = TextureId("/src/my-texture.texture".to_string());
        let json = serde_json::to_string(&texture_id).unwrap();
        assert_eq!(json, "\"/src/my-texture.texture\"");
    }

    #[test]
    fn test_id_deserialization() {
        let json = "\"/src/my-texture.texture\"";
        let texture_id: TextureId = serde_json::from_str(json).unwrap();
        assert_eq!(texture_id, TextureId("/src/my-texture.texture".to_string()));
    }

    #[test]
    fn test_id_conversions() {
        let id = TextureId("/src/my-texture.texture".to_string());
        let string_val: String = id.clone().into();
        assert_eq!(string_val, "/src/my-texture.texture");

        let id2: TextureId = "/src/my-texture.texture".into();
        assert_eq!(id2, TextureId("/src/my-texture.texture".to_string()));
    }

    #[test]
    fn test_all_id_types() {
        let texture_id = TextureId("/src/texture.texture".to_string());
        let output_id = OutputId("/src/output.output".to_string());
        let shader_id = ShaderId("/src/shader.shader".to_string());
        let fixture_id = FixtureId("/src/fixture.fixture".to_string());

        assert_eq!(String::from(texture_id.clone()), "/src/texture.texture");
        assert_eq!(String::from(output_id.clone()), "/src/output.output");
        assert_eq!(String::from(shader_id.clone()), "/src/shader.shader");
        assert_eq!(String::from(fixture_id.clone()), "/src/fixture.fixture");
    }
}
