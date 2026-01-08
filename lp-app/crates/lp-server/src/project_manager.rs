//! Project manager for handling multiple projects

extern crate alloc;

use crate::error::ServerError;
use crate::project::Project;
use crate::template;
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use hashbrown::HashMap;
use lp_core::app::Platform;
use lp_core::project::config::ProjectConfig;
use lp_core::traits::LpFs;

/// Manages multiple project instances
pub struct ProjectManager {
    /// Map of project name -> Project instance
    projects: HashMap<String, Project>,
    /// Base directory where projects are stored (relative path)
    projects_base_dir: String,
}

impl ProjectManager {
    /// Create a new project manager
    ///
    /// `projects_base_dir` is the base directory where all projects are stored.
    /// Each project will have its own subdirectory.
    pub fn new(projects_base_dir: String) -> Self {
        Self {
            projects: HashMap::new(),
            projects_base_dir,
        }
    }

    /// Create a new project
    ///
    /// Creates the project directory structure using the provided filesystem.
    /// The caller must provide a Platform with the appropriate filesystem (at server root) and OutputProvider.
    pub fn create_project(&mut self, name: String, platform: Platform) -> Result<(), ServerError> {
        // Check if project already exists
        if self.projects.contains_key(&name) {
            return Err(ServerError::ProjectExists(name));
        }

        let project_path = format!("{}/{}", self.projects_base_dir, name);
        let project_json_path = format!("{}/project.json", project_path);

        // Check if project already exists on filesystem
        if platform.fs.file_exists(&project_json_path).unwrap_or(false) {
            return Err(ServerError::ProjectExists(name));
        }

        // Create project.json
        let config = ProjectConfig {
            uid: name.clone(),
            name: format!("{} Project", name),
        };
        let json = serde_json::to_string_pretty(&config).map_err(|e| {
            ServerError::Serialization(format!("Failed to serialize project config: {}", e))
        })?;
        platform
            .fs
            .write_file(&project_json_path, json.as_bytes())
            .map_err(|e| ServerError::Filesystem(format!("Failed to write project.json: {}", e)))?;

        // Chroot the filesystem to the project directory to create the template
        let project_fs = platform.fs.chroot(&project_path).map_err(|e| {
            ServerError::Filesystem(format!("Failed to chroot to project directory: {}", e))
        })?;

        // Create the default project template
        template::create_default_project_template(project_fs.as_ref()).map_err(|e| {
            ServerError::Filesystem(format!("Failed to create project template: {}", e))
        })?;

        // Load the newly created project
        self.load_project(name, platform)
    }

    /// Load a project from the filesystem
    ///
    /// Creates a Project instance and loads it into memory.
    /// The caller must provide a Platform with a filesystem at the server root and an OutputProvider.
    /// This method will chroot the filesystem to the project directory.
    pub fn load_project(&mut self, name: String, platform: Platform) -> Result<(), ServerError> {
        // Check if already loaded
        if self.projects.contains_key(&name) {
            return Ok(()); // Already loaded
        }

        let project_path = format!("{}/{}", self.projects_base_dir, name);
        let project_json_path = format!("{}/project.json", project_path);

        // Check if project exists
        if !platform.fs.file_exists(&project_json_path).map_err(|e| {
            ServerError::Filesystem(format!("Failed to check project existence: {}", e))
        })? {
            return Err(ServerError::ProjectNotFound(name.clone()));
        }

        // Chroot the filesystem to the project directory
        // This creates a new filesystem view where paths are relative to the project root
        let project_fs = platform.fs.chroot(&project_path).map_err(|e| {
            ServerError::Filesystem(format!("Failed to chroot to project directory: {}", e))
        })?;

        // Create a new Platform with the chrooted filesystem
        let project_platform = Platform::new(project_fs, platform.output);

        // Create and load project
        // Now paths like "/project.json" will resolve relative to the project directory
        let project = Project::new(name.clone(), "/project.json".to_string(), project_platform)?;
        self.projects.insert(name, project);

        Ok(())
    }

    /// Unload a project
    ///
    /// Removes the project from memory but doesn't delete it from the filesystem.
    pub fn unload_project(&mut self, name: &str) -> Result<(), ServerError> {
        self.projects
            .remove(name)
            .ok_or_else(|| ServerError::ProjectNotFound(name.to_string()))?;
        Ok(())
    }

    /// Get a project by name
    pub fn get_project(&self, name: &str) -> Option<&Project> {
        self.projects.get(name)
    }

    /// Get a mutable reference to a project by name
    pub fn get_project_mut(&mut self, name: &str) -> Option<&mut Project> {
        self.projects.get_mut(name)
    }

    /// List all loaded projects
    pub fn list_loaded_projects(&self) -> Vec<String> {
        self.projects.keys().cloned().collect()
    }

    /// List all available projects on the filesystem
    ///
    /// Returns project names that exist on disk but may not be loaded.
    /// Requires a filesystem to query.
    pub fn list_available_projects(&self, fs: &dyn LpFs) -> Result<Vec<String>, ServerError> {
        // List entries in the base directory
        let entries = fs.list_dir(&self.projects_base_dir).map_err(|e| {
            ServerError::Filesystem(format!("Failed to read projects directory: {}", e))
        })?;

        let mut projects = Vec::new();
        for entry in entries {
            // Check if this entry is a project directory (has project.json)
            let project_json_path = format!("{}/project.json", entry);
            if fs.file_exists(&project_json_path).unwrap_or(false) {
                // Extract project name from path
                // Entry format: "/base/project-name" or "/base/project-name/"
                let name = entry
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or(&entry)
                    .to_string();
                if !name.is_empty() {
                    projects.push(name);
                }
            }
        }

        Ok(projects)
    }
}
