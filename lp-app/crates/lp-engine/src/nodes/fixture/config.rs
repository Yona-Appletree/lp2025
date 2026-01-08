//! Fixture node configuration

use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

use crate::nodes::id::{OutputId, TextureId};

/// Fixture node types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum FixtureNode {
    #[serde(rename = "circle-list")]
    CircleList {
        #[serde(
            deserialize_with = "deserialize_output_id",
            serialize_with = "serialize_output_id"
        )]
        output_id: OutputId,
        #[serde(
            deserialize_with = "deserialize_texture_id",
            serialize_with = "serialize_texture_id"
        )]
        texture_id: TextureId,
        channel_order: String,
        mapping: Vec<Mapping>,
    },
}

/// Mapping from shader output to LED channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapping {
    pub channel: u32,
    pub center: [f32; 2],
    pub radius: f32,
}

// Custom serialization for OutputId
fn serialize_output_id<S>(id: &OutputId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    let id_str: String = id.clone().into();
    id_str.serialize(serializer)
}

fn deserialize_output_id<'de, D>(deserializer: D) -> Result<OutputId, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let id_str = String::deserialize(deserializer)?;
    Ok(OutputId::from(id_str))
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
