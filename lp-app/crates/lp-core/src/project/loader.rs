//! Project loader for filesystem-based projects

use crate::error::Error;
use lp_core_util::fs::LpFs;
use crate::nodes::{FixtureNode, OutputNode, ShaderNode, TextureNode};
use crate::project::config::ProjectConfig;
use alloc::{collections::BTreeMap, format, string::String, string::ToString, vec::Vec};

/// Load a project from the filesystem
///
/// Reads `/project.json` to get the top-level project configuration.
/// Node discovery is handled separately via `discover_nodes()`.
pub fn load_from_filesystem(fs: &dyn LpFs) -> Result<ProjectConfig, Error> {
    log::debug!("Reading project.json");
    // Read project.json
    let project_json = fs.read_file("/project.json")?;
    let project_str = core::str::from_utf8(&project_json)
        .map_err(|e| Error::Serialization(format!("Invalid UTF-8 in project.json: {}", e)))?;

    // Deserialize project config
    let config: ProjectConfig = serde_json::from_str(project_str)
        .map_err(|e| Error::Serialization(format!("Failed to parse project.json: {}", e)))?;

    log::debug!("Project config loaded: {} ({})", config.name, config.uid);
    Ok(config)
}

/// Discover all node directories in the project
///
/// Recursively scans `/src/` for directories ending with node suffixes
/// (`.shader`, `.texture`, `.output`, `.fixture`).
///
/// Returns a list of node IDs (full paths from project root with leading slash,
/// e.g., `["/src/my-shader.shader", "/src/nested/effects/rainbow.shader"]`).
pub fn discover_nodes(fs: &dyn LpFs) -> Result<Vec<String>, Error> {
    log::debug!("Discovering nodes in /src");
    let mut nodes = Vec::new();
    discover_nodes_recursive(fs, "/src", &mut nodes)?;
    log::info!("Discovered {} node(s)", nodes.len());
    Ok(nodes)
}

/// Recursively discover node directories
fn discover_nodes_recursive(
    fs: &dyn LpFs,
    dir_path: &str,
    nodes: &mut Vec<String>,
) -> Result<(), Error> {
    // Try to list directory contents
    // If the directory doesn't exist, list_dir will return an error, which is fine
    let entries = match fs.list_dir(dir_path) {
        Ok(entries) => entries,
        Err(_) => {
            // Directory doesn't exist (e.g., src/ doesn't exist yet), that's okay
            return Ok(());
        }
    };

    for entry in entries {
        // Check if this entry is a node directory
        if is_node_directory(&entry) {
            // Extract node ID (full path from project root)
            if let Some(node_id) = extract_node_id(&entry) {
                nodes.push(node_id);
            }
        } else {
            // Check if it's a subdirectory (not a file)
            // We can't easily distinguish files from directories with the current Filesystem trait,
            // so we'll try to list it. If it fails, it's probably a file and we'll skip it.
            if let Ok(_) = fs.list_dir(&entry) {
                // It's a directory, recurse into it
                discover_nodes_recursive(fs, &entry, nodes)?;
            }
        }
    }

    Ok(())
}

/// Check if a path represents a node directory
///
/// Node directories end with one of: `.shader`, `.texture`, `.output`, `.fixture`
fn is_node_directory(path: &str) -> bool {
    path.ends_with(".shader")
        || path.ends_with(".texture")
        || path.ends_with(".output")
        || path.ends_with(".fixture")
}

/// Extract node ID from a directory path
///
/// Returns the full path from project root with leading slash.
/// For example, `/src/my-shader.shader` -> `/src/my-shader.shader`
fn extract_node_id(path: &str) -> Option<String> {
    // Ensure path starts with /
    if !path.starts_with('/') {
        return None;
    }

    // Path is already the node ID (full path from project root)
    Some(path.to_string())
}

/// Enum representing all node types
#[derive(Debug, Clone)]
pub enum NodeConfig {
    Texture(TextureNode),
    Shader(ShaderNode),
    Output(OutputNode),
    Fixture(FixtureNode),
}

/// Load a single node from the filesystem
///
/// Reads `node_path/node.json` and related files (e.g., `main.glsl` for shaders).
/// Validates that the directory suffix matches the node type.
///
/// Returns the node ID (full path) and the node configuration.
pub fn load_node(fs: &dyn LpFs, node_path: &str) -> Result<(String, NodeConfig), Error> {
    log::debug!("Loading node: {}", node_path);
    // Validate node path format
    let node_id = extract_node_id(node_path)
        .ok_or_else(|| Error::Validation(format!("Invalid node path: {}", node_path)))?;

    // Determine expected node type from directory suffix
    let expected_type = if node_path.ends_with(".shader") {
        "shader"
    } else if node_path.ends_with(".texture") {
        "texture"
    } else if node_path.ends_with(".output") {
        "output"
    } else if node_path.ends_with(".fixture") {
        "fixture"
    } else {
        return Err(Error::Validation(format!(
            "Invalid node directory suffix: {}",
            node_path
        )));
    };

    // Read node.json
    let node_json_path = format!("{}/node.json", node_path);
    let node_json = fs
        .read_file(&node_json_path)
        .map_err(|e| Error::Filesystem(format!("Failed to read {}: {}", node_json_path, e)))?;
    let node_json_str = core::str::from_utf8(&node_json)
        .map_err(|e| Error::Serialization(format!("Invalid UTF-8 in {}: {}", node_json_path, e)))?;

    // Parse node config based on type
    let node_config = match expected_type {
        "shader" => {
            // For shaders, we need to read main.glsl separately
            let glsl_path = format!("{}/main.glsl", node_path);
            let glsl_source = fs
                .read_file(&glsl_path)
                .map_err(|e| Error::Filesystem(format!("Failed to read {}: {}", glsl_path, e)))?;
            let glsl_str = core::str::from_utf8(&glsl_source).map_err(|e| {
                Error::Serialization(format!("Invalid UTF-8 in {}: {}", glsl_path, e))
            })?;

            // Parse shader config from node.json
            // We'll parse it as a JSON value first, then inject the glsl field
            let mut shader_value: serde_json::Value =
                serde_json::from_str(node_json_str).map_err(|e| {
                    Error::Serialization(format!("Failed to parse {}: {}", node_json_path, e))
                })?;

            // Validate $type field matches
            if let Some(node_type) = shader_value.get("$type") {
                if node_type != "Single" {
                    return Err(Error::Validation(format!(
                        "Invalid shader type in {}: expected 'Single', got '{}'",
                        node_json_path, node_type
                    )));
                }
            }

            // Inject glsl field
            shader_value["glsl"] = serde_json::Value::String(glsl_str.to_string());

            // Deserialize the complete shader config
            let shader: ShaderNode = serde_json::from_value(shader_value).map_err(|e| {
                Error::Serialization(format!("Failed to deserialize shader: {}", e))
            })?;

            NodeConfig::Shader(shader)
        }
        "texture" => {
            let texture: TextureNode = serde_json::from_str(node_json_str)
                .map_err(|e| Error::Serialization(format!("Failed to parse texture: {}", e)))?;

            // Validate $type field
            match &texture {
                TextureNode::Memory { .. } => {}
            }

            log::debug!("Loaded texture node: {}", node_id);
            NodeConfig::Texture(texture)
        }
        "output" => {
            let output: OutputNode = serde_json::from_str(node_json_str)
                .map_err(|e| Error::Serialization(format!("Failed to parse output: {}", e)))?;

            // Validate $type field
            match &output {
                OutputNode::GpioStrip { .. } => {}
            }

            log::debug!("Loaded output node: {}", node_id);
            NodeConfig::Output(output)
        }
        "fixture" => {
            let fixture: FixtureNode = serde_json::from_str(node_json_str)
                .map_err(|e| Error::Serialization(format!("Failed to parse fixture: {}", e)))?;

            // Validate $type field
            match &fixture {
                FixtureNode::CircleList { .. } => {}
            }

            log::debug!("Loaded fixture node: {}", node_id);
            NodeConfig::Fixture(fixture)
        }
        _ => unreachable!(),
    };

    Ok((node_id, node_config))
}

/// Load all nodes from the filesystem
///
/// Discovers all node directories and loads each node configuration.
/// Returns maps of node ID to node configuration for each node type.
pub fn load_all_nodes(
    fs: &dyn LpFs,
) -> Result<
    (
        BTreeMap<String, TextureNode>,
        BTreeMap<String, ShaderNode>,
        BTreeMap<String, OutputNode>,
        BTreeMap<String, FixtureNode>,
    ),
    Error,
> {
    let mut textures = BTreeMap::new();
    let mut shaders = BTreeMap::new();
    let mut outputs = BTreeMap::new();
    let mut fixtures = BTreeMap::new();

    // Discover all nodes
    let node_paths = discover_nodes(fs)?;

    // Load each node
    for node_path in node_paths {
        let (node_id, node_config) = match load_node(fs, &node_path) {
            Ok(result) => result,
            Err(e) => {
                log::warn!("Failed to load node {}: {}", node_path, e);
                continue;
            }
        };

        match node_config {
            NodeConfig::Texture(texture) => {
                textures.insert(node_id, texture);
            }
            NodeConfig::Shader(shader) => {
                shaders.insert(node_id, shader);
            }
            NodeConfig::Output(output) => {
                outputs.insert(node_id, output);
            }
            NodeConfig::Fixture(fixture) => {
                fixtures.insert(node_id, fixture);
            }
        }
    }

    Ok((textures, shaders, outputs, fixtures))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lp_core_util::fs::LpFsMemory;
    use alloc::string::ToString;

    #[test]
    fn test_load_from_filesystem() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut(
            "/project.json",
            br#"{"uid":"test-uid","name":"Test Project"}"#,
        )
        .unwrap();

        let config = load_from_filesystem(&fs).unwrap();
        assert_eq!(config.uid, "test-uid");
        assert_eq!(config.name, "Test Project");
    }

    #[test]
    fn test_load_from_filesystem_missing_file() {
        let fs = LpFsMemory::new();
        let result = load_from_filesystem(&fs);
        assert!(result.is_err());
    }

    #[test]
    fn test_discover_nodes() {
        let mut fs = LpFsMemory::new();
        // Create node directories by creating files in them
        fs.write_file_mut("/src/shader1.shader/node.json", b"{}")
            .unwrap();
        fs.write_file_mut("/src/texture1.texture/node.json", b"{}")
            .unwrap();
        fs.write_file_mut("/src/output1.output/node.json", b"{}")
            .unwrap();
        fs.write_file_mut("/src/fixture1.fixture/node.json", b"{}")
            .unwrap();

        let nodes = discover_nodes(&fs).unwrap();
        assert_eq!(nodes.len(), 4);
        assert!(nodes.contains(&"/src/shader1.shader".to_string()));
        assert!(nodes.contains(&"/src/texture1.texture".to_string()));
        assert!(nodes.contains(&"/src/output1.output".to_string()));
        assert!(nodes.contains(&"/src/fixture1.fixture".to_string()));
    }

    #[test]
    fn test_discover_nodes_nested() {
        let mut fs = LpFsMemory::new();
        // Create nested node directories
        fs.write_file_mut("/src/nested/effects/rainbow.shader/node.json", b"{}")
            .unwrap();
        fs.write_file_mut("/src/nested/textures/bg.texture/node.json", b"{}")
            .unwrap();

        let nodes = discover_nodes(&fs).unwrap();
        assert_eq!(nodes.len(), 2);
        assert!(nodes.contains(&"/src/nested/effects/rainbow.shader".to_string()));
        assert!(nodes.contains(&"/src/nested/textures/bg.texture".to_string()));
    }

    #[test]
    fn test_discover_nodes_empty_src() {
        let fs = LpFsMemory::new();
        let nodes = discover_nodes(&fs).unwrap();
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn test_is_node_directory() {
        assert!(is_node_directory("/src/my-shader.shader"));
        assert!(is_node_directory("/src/my-texture.texture"));
        assert!(is_node_directory("/src/my-output.output"));
        assert!(is_node_directory("/src/my-fixture.fixture"));
        assert!(!is_node_directory("/src/regular-dir"));
        assert!(!is_node_directory("/src/file.txt"));
    }

    #[test]
    fn test_extract_node_id() {
        assert_eq!(
            extract_node_id("/src/my-shader.shader"),
            Some("/src/my-shader.shader".to_string())
        );
        assert_eq!(
            extract_node_id("/src/nested/effects/rainbow.shader"),
            Some("/src/nested/effects/rainbow.shader".to_string())
        );
        assert_eq!(extract_node_id("src/my-shader.shader"), None); // No leading slash
    }

    #[test]
    fn test_load_node_shader() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut(
            "/src/my-shader.shader/node.json",
            br#"{"$type":"Single","texture_id":"/src/my-texture.texture"}"#,
        )
        .unwrap();
        fs.write_file_mut(
            "/src/my-shader.shader/main.glsl",
            b"vec4 main(vec2 fragCoord, vec2 outputSize, float time) { return vec4(1.0); }",
        )
        .unwrap();

        let (node_id, node_config) = load_node(&fs, "/src/my-shader.shader").unwrap();
        assert_eq!(node_id, "/src/my-shader.shader");
        match node_config {
            NodeConfig::Shader(ShaderNode::Single { glsl, texture_id }) => {
                assert_eq!(
                    glsl,
                    "vec4 main(vec2 fragCoord, vec2 outputSize, float time) { return vec4(1.0); }"
                );
                assert_eq!(
                    String::from(texture_id),
                    "/src/my-texture.texture".to_string()
                );
            }
            _ => panic!("Expected Shader node"),
        }
    }

    #[test]
    fn test_load_node_texture() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut(
            "/src/my-texture.texture/node.json",
            br#"{"$type":"Memory","size":[64,64],"format":"RGB8"}"#,
        )
        .unwrap();

        let (node_id, node_config) = load_node(&fs, "/src/my-texture.texture").unwrap();
        assert_eq!(node_id, "/src/my-texture.texture");
        match node_config {
            NodeConfig::Texture(TextureNode::Memory { size, format }) => {
                assert_eq!(size, [64, 64]);
                assert_eq!(format, "RGB8");
            }
            _ => panic!("Expected Texture node"),
        }
    }

    #[test]
    fn test_load_all_nodes() {
        let mut fs = LpFsMemory::new();
        // Create a shader
        fs.write_file_mut(
            "/src/shader.shader/node.json",
            br#"{"$type":"Single","texture_id":"/src/texture.texture"}"#,
        )
        .unwrap();
        fs.write_file_mut("/src/shader.shader/main.glsl", b"void main() {}")
            .unwrap();

        // Create a texture
        fs.write_file_mut(
            "/src/texture.texture/node.json",
            br#"{"$type":"Memory","size":[32,32],"format":"RGB8"}"#,
        )
        .unwrap();

        // Create an output
        fs.write_file_mut(
            "/src/output.output/node.json",
            br#"{"$type":"gpio_strip","chip":"ws2812","gpio_pin":4,"count":128}"#,
        )
        .unwrap();

        let (textures, shaders, outputs, fixtures) = load_all_nodes(&fs).unwrap();
        assert_eq!(textures.len(), 1);
        assert_eq!(shaders.len(), 1);
        assert_eq!(outputs.len(), 1);
        assert_eq!(fixtures.len(), 0);

        assert!(textures.contains_key("/src/texture.texture"));
        assert!(shaders.contains_key("/src/shader.shader"));
        assert!(outputs.contains_key("/src/output.output"));
    }
}
