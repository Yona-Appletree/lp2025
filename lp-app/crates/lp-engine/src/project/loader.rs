use crate::error::Error;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;
use lp_model::{LpPath, NodeConfig, NodeKind, ProjectConfig};
use lp_shared::fs::LpFs;
use serde_json;

/// Determine node kind from path suffix
pub(crate) fn node_kind_from_path(path: &LpPath) -> Result<NodeKind, Error> {
    let path_str = path.as_str();

    // Find the last dot after the last slash
    let last_slash = path_str.rfind('/').unwrap_or(0);
    let after_slash = &path_str[last_slash..];

    // Extract suffix (part after last dot)
    let suffix = if let Some(dot_pos) = after_slash.rfind('.') {
        &after_slash[dot_pos + 1..]
    } else {
        // No type suffix found
        return Err(Error::InvalidConfig {
            node_path: path_str.to_string(),
            reason: "No type suffix on node path".to_string(),
        });
    };

    // Match suffix to node kind
    match suffix {
        "texture" => Ok(NodeKind::Texture),
        "shader" => Ok(NodeKind::Shader),
        "output" => Ok(NodeKind::Output),
        "fixture" => Ok(NodeKind::Fixture),
        _ => Err(Error::InvalidConfig {
            node_path: path_str.to_string(),
            reason: format!("Unknown node kind: {}", suffix),
        }),
    }
}

/// Check if a path is a node directory
pub(crate) fn is_node_directory(path: &str) -> bool {
    path.ends_with(".texture")
        || path.ends_with(".shader")
        || path.ends_with(".output")
        || path.ends_with(".fixture")
}

/// Load project config from filesystem
pub fn load_from_filesystem(fs: &dyn LpFs) -> Result<ProjectConfig, Error> {
    let path = "/project.json";
    let data = fs.read_file(path).map_err(|e| Error::Io {
        path: path.to_string(),
        details: format!("Failed to read: {:?}", e),
    })?;

    let config: ProjectConfig = serde_json::from_slice(&data).map_err(|e| Error::Parse {
        file: path.to_string(),
        error: format!("{}", e),
    })?;

    Ok(config)
}

/// Discover all node directories in /src/
pub fn discover_nodes(fs: &dyn LpFs) -> Result<Vec<LpPath>, Error> {
    let path = "/src";
    let entries = fs.list_dir(path).map_err(|e| Error::Io {
        path: path.to_string(),
        details: format!("Failed to list directory: {:?}", e),
    })?;

    let mut nodes = Vec::new();
    for entry in entries {
        if is_node_directory(&entry) {
            nodes.push(LpPath::from(entry));
        }
    }

    Ok(nodes)
}

/// Load a node's config from filesystem
pub fn load_node(fs: &dyn LpFs, path: &LpPath) -> Result<(LpPath, Box<dyn NodeConfig>), Error> {
    let node_json_path = format!("{}/node.json", path.as_str());

    let data = fs.read_file(&node_json_path).map_err(|e| Error::Io {
        path: node_json_path.clone(),
        details: format!("Failed to read: {:?}", e),
    })?;

    // Determine node kind from path suffix
    let kind = node_kind_from_path(path)?;

    // Parse config based on kind
    let config: Box<dyn NodeConfig> = match kind {
        NodeKind::Texture => {
            let cfg: lp_model::nodes::texture::TextureConfig = serde_json::from_slice(&data)
                .map_err(|e| Error::Parse {
                    file: node_json_path.clone(),
                    error: format!("Failed to parse texture config: {}", e),
                })?;
            Box::new(cfg)
        }
        NodeKind::Shader => {
            let cfg: lp_model::nodes::shader::ShaderConfig = serde_json::from_slice(&data)
                .map_err(|e| Error::Parse {
                    file: node_json_path.clone(),
                    error: format!("Failed to parse shader config: {}", e),
                })?;
            Box::new(cfg)
        }
        NodeKind::Output => {
            let cfg: lp_model::nodes::output::OutputConfig = serde_json::from_slice(&data)
                .map_err(|e| Error::Parse {
                    file: node_json_path.clone(),
                    error: format!("Failed to parse output config: {}", e),
                })?;
            Box::new(cfg)
        }
        NodeKind::Fixture => {
            let cfg: lp_model::nodes::fixture::FixtureConfig = serde_json::from_slice(&data)
                .map_err(|e| Error::Parse {
                    file: node_json_path.clone(),
                    error: format!("Failed to parse fixture config: {}", e),
                })?;
            Box::new(cfg)
        }
    };

    Ok((path.clone(), config))
}
