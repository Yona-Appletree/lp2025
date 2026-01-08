//! Shader node configuration

use alloc::string::String;
use serde::{Deserialize, Serialize};

use crate::nodes::id::TextureId;

/// Shader node types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum ShaderNode {
    #[serde(rename = "Single")]
    Single {
        glsl: String,
        #[serde(
            deserialize_with = "deserialize_texture_id",
            serialize_with = "serialize_texture_id"
        )]
        texture_id: TextureId,
    },
}

// Custom serialization for TextureId
fn serialize_texture_id<S>(id: &TextureId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    let id_str: String = id.clone().into();
    id_str.serialize(serializer)
}

fn deserialize_texture_id<'de, D>(deserializer: D) -> Result<TextureId, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let id_str = String::deserialize(deserializer)?;
    Ok(TextureId::from(id_str))
}
