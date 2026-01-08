//! LightPlayer Application - main entry point for firmware

use crate::app::{FileChange, Platform};
use crate::error::Error;
use crate::project::{config::ProjectConfig, loader, runtime::ProjectRuntime};
use alloc::string::{String, ToString};

/// LightPlayer Application
///
/// Main entry point for firmware. Manages project lifecycle, handles messages,
/// and coordinates runtime updates.
pub struct LpApp {
    /// Platform-specific trait implementations
    pub platform: Platform,
    /// Current project config (None if no project loaded)
    config: Option<ProjectConfig>,
    /// Current project runtime (None if no project loaded)
    runtime: Option<ProjectRuntime>,
}

impl LpApp {
    /// Create a new LpApp with the provided platform
    pub fn new(platform: Platform) -> Self {
        Self {
            platform,
            config: None,
            runtime: None,
        }
    }

    /// Load a project from the filesystem
    ///
    /// Uses ProjectLoader to read `/project.json` and load the project configuration.
    /// Creates a ProjectRuntime and initializes it. If a project is already loaded,
    /// it is destroyed before loading the new one.
    ///
    /// Returns an error if the project file doesn't exist. Project creation should
    /// be handled by the caller (e.g., `lp-server`).
    ///
    pub fn load_project(&mut self, _path: &str) -> Result<(), Error> {
        log::info!("Loading project from /project.json");

        // Destroy existing runtime if present
        if let Some(mut runtime) = self.runtime.take() {
            let _ = runtime.destroy();
        }

        // Check if project exists
        if !self.platform.fs.file_exists("/project.json")? {
            return Err(Error::Filesystem(
                "Project not found: /project.json does not exist".to_string(),
            ));
        }

        // Load project config using ProjectLoader
        let config = loader::load_from_filesystem(self.platform.fs.as_ref())?;

        log::info!("Project config loaded: {} ({})", config.name, config.uid);

        // Load all nodes from filesystem
        let (textures, shaders, outputs, fixtures) =
            loader::load_all_nodes(self.platform.fs.as_ref())?;

        // Create runtime
        let mut runtime = ProjectRuntime::new(config.uid.clone());

        // Initialize runtime with loaded nodes
        runtime.init(
            &config,
            &textures,
            &shaders,
            &outputs,
            &fixtures,
            self.platform.output.as_ref(),
        )?;

        // Store config and runtime
        self.config = Some(config);
        self.runtime = Some(runtime);

        log::info!("Project loaded successfully");
        Ok(())
    }

    /// Process incoming messages and update runtime
    ///
    /// Processes all incoming messages, updates the runtime if loaded,
    /// and returns any outgoing messages.
    pub fn tick(
        &mut self,
        delta_ms: u32,
        incoming: &[crate::app::MsgIn],
        file_changes: &[crate::app::FileChange],
    ) -> Result<alloc::vec::Vec<crate::app::MsgOut>, Error> {
        let mut outgoing = alloc::vec::Vec::new();

        // Process file changes
        if !file_changes.is_empty() {
            if let Err(e) = self.handle_file_changes_impl(file_changes) {
                // Log error but continue - file change errors shouldn't stop the app
                let _ = e;
            }
        }

        // Process incoming messages
        for msg in incoming {
            match self.handle_message(msg.clone()) {
                Ok(mut msgs) => outgoing.append(&mut msgs),
                Err(e) => {
                    // Log error but continue processing
                    let _ = e;
                }
            }
        }

        // Update runtime if loaded
        if let Some(ref mut runtime) = self.runtime {
            let _ = runtime.update(delta_ms, self.platform.output.as_ref());
        }

        Ok(outgoing)
    }

    /// Handle a single incoming message
    fn handle_message(
        &mut self,
        msg: crate::app::MsgIn,
    ) -> Result<alloc::vec::Vec<crate::app::MsgOut>, Error> {
        let mut outgoing = alloc::vec::Vec::new();

        // match msg {
        //  TODO: Implement message handling
        // }

        Ok(outgoing)
    }

    /// Get a reference to the current project config
    pub fn config(&self) -> Option<&ProjectConfig> {
        self.config.as_ref()
    }

    /// Get a reference to the current project runtime
    pub fn runtime(&self) -> Option<&ProjectRuntime> {
        self.runtime.as_ref()
    }

    /// Handle file changes and reload affected nodes
    ///
    /// Processes file changes, determines which nodes are affected,
    /// and reloads them. Handles project.json changes specially by
    /// reloading the entire project.
    ///
    /// For node file changes, reloads all nodes and reinitializes the runtime.
    /// This is simpler and ensures consistency, though not optimal for performance.
    #[cfg(test)]
    pub fn handle_file_changes(&mut self, changes: &[FileChange]) -> Result<(), Error> {
        self.handle_file_changes_impl(changes)
    }

    fn handle_file_changes_impl(&mut self, changes: &[FileChange]) -> Result<(), Error> {
        if changes.is_empty() {
            return Ok(());
        }

        log::info!("Processing {} file change(s)", changes.len());

        // Check if project.json changed - if so, reload entire project
        let project_json_changed = changes.iter().any(|c| c.path == "/project.json");
        if project_json_changed {
            log::info!("project.json changed, reloading entire project");
            return self.load_project("/project.json");
        }

        // Check if any node files changed
        let has_node_changes = changes
            .iter()
            .any(|c| get_node_path_from_file_path(&c.path).is_some());

        if !has_node_changes {
            // No node changes, nothing to do
            log::debug!("No node file changes detected");
            return Ok(());
        }

        log::info!("Node file changes detected, reloading nodes");

        // Reload all nodes and reinitialize runtime
        // This ensures consistency since InitContext needs all nodes
        let config = self.config.as_ref().ok_or_else(|| {
            Error::Validation("Cannot reload nodes: no project config".to_string())
        })?;

        // Destroy existing runtime
        if let Some(mut runtime) = self.runtime.take() {
            let _ = runtime.destroy();
        }

        // Reload all nodes from filesystem
        let (textures, shaders, outputs, fixtures) =
            loader::load_all_nodes(self.platform.fs.as_ref())?;

        // Create new runtime and initialize with reloaded nodes
        let mut runtime = ProjectRuntime::new(config.uid.clone());
        if let Err(e) = runtime.init(
            config,
            &textures,
            &shaders,
            &outputs,
            &fixtures,
            self.platform.output.as_ref(),
        ) {
            log::warn!("Failed to reinitialize runtime after reload: {}", e);
            return Err(e);
        }

        self.runtime = Some(runtime);
        log::info!("Nodes reloaded successfully");

        Ok(())
    }
}

/// Extract node path from a file path
///
/// For example:
/// - `/src/my-shader.shader/main.glsl` -> `/src/my-shader.shader`
/// - `/src/my-shader.shader/node.json` -> `/src/my-shader.shader`
/// - `/project.json` -> None (not a node file)
fn get_node_path_from_file_path(file_path: &str) -> Option<String> {
    // Handle project.json specially
    if file_path == "/project.json" {
        return None;
    }

    // Find the node directory by looking for node suffixes
    let node_suffixes = [".shader", ".texture", ".output", ".fixture"];
    for suffix in &node_suffixes {
        if let Some(pos) = file_path.find(suffix) {
            // Extract path up to and including the suffix
            let node_path = &file_path[..pos + suffix.len()];
            if node_path.starts_with('/') {
                return Some(node_path.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{ChangeType, FileChange};
    use lp_core_util::fs::LpFsMemory;
    use crate::traits::OutputProvider;
    use alloc::string::ToString;

    struct MockOutputProvider;

    impl OutputProvider for MockOutputProvider {
        fn create_output(
            &self,
            _config: &crate::nodes::OutputNode,
            _id: Option<crate::nodes::id::OutputId>,
        ) -> Result<alloc::boxed::Box<dyn crate::traits::LedOutput>, crate::error::Error> {
            Err(crate::error::Error::Node(
                "Mock output provider".to_string(),
            ))
        }
    }

    #[test]
    fn test_get_node_path_from_file_path() {
        assert_eq!(
            get_node_path_from_file_path("/src/my-shader.shader/main.glsl"),
            Some("/src/my-shader.shader".to_string())
        );
        assert_eq!(
            get_node_path_from_file_path("/src/my-shader.shader/node.json"),
            Some("/src/my-shader.shader".to_string())
        );
        assert_eq!(
            get_node_path_from_file_path("/src/nested/effects/rainbow.shader/main.glsl"),
            Some("/src/nested/effects/rainbow.shader".to_string())
        );
        assert_eq!(
            get_node_path_from_file_path("/src/my-texture.texture/node.json"),
            Some("/src/my-texture.texture".to_string())
        );
        assert_eq!(
            get_node_path_from_file_path("/src/my-output.output/node.json"),
            Some("/src/my-output.output".to_string())
        );
        assert_eq!(
            get_node_path_from_file_path("/src/my-fixture.fixture/node.json"),
            Some("/src/my-fixture.fixture".to_string())
        );
        assert_eq!(get_node_path_from_file_path("/project.json"), None);
        assert_eq!(get_node_path_from_file_path("/src/other-file.txt"), None);
    }

    #[test]
    fn test_handle_file_changes_project_json() {
        let mut fs = LpFsMemory::new();
        // Create project.json first
        fs.write_file_mut("/project.json", br#"{"uid":"test","name":"Test"}"#)
            .unwrap();
        let fs_box = alloc::boxed::Box::new(fs);
        let output = alloc::boxed::Box::new(MockOutputProvider);
        let platform = Platform::new(fs_box, output);
        let mut app = LpApp::new(platform);

        // Load the project
        app.load_project("/project.json").unwrap();

        // Change project.json
        let changes = vec![FileChange {
            path: "/project.json".to_string(),
            change_type: ChangeType::Modify,
        }];

        // Should reload entire project
        assert!(app.handle_file_changes(&changes).is_ok());
    }

    #[test]
    fn test_handle_file_changes_node_file() {
        let mut fs = LpFsMemory::new();
        // Create a project with a shader
        fs.write_file_mut("/project.json", br#"{"uid":"test","name":"Test"}"#)
            .unwrap();
        fs.write_file_mut(
            "/src/shader.shader/node.json",
            br#"{"$type":"Single","texture_id":"/src/texture.texture"}"#,
        )
        .unwrap();
        fs.write_file_mut("/src/shader.shader/main.glsl", b"void main() {}")
            .unwrap();
        fs.write_file_mut(
            "/src/texture.texture/node.json",
            br#"{"$type":"Memory","size":[64,64],"format":"RGB8"}"#,
        )
        .unwrap();

        let fs_box = alloc::boxed::Box::new(fs);
        let output = alloc::boxed::Box::new(MockOutputProvider);
        let platform = Platform::new(fs_box, output);
        let mut app = LpApp::new(platform);

        // Load project
        app.load_project("/project.json").unwrap();

        // Modify shader GLSL file
        let changes = alloc::vec::Vec::from([FileChange {
            path: "/src/shader.shader/main.glsl".to_string(),
            change_type: ChangeType::Modify,
        }]);

        // Should reload nodes
        assert!(app.handle_file_changes(&changes).is_ok());
    }
}
