//! Project manager for handling multiple projects

extern crate alloc;

use crate::error::ServerError;
use crate::project::Project;
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::Vec,
};
use hashbrown::HashMap;
use lp_shared::fs::LpFs;

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
    /// todo!("Refactor to use new ProjectRuntime API")
    pub fn create_project(&mut self, _name: String, _fs: Box<dyn LpFs>) -> Result<(), ServerError> {
        todo!("Refactor to use new ProjectRuntime API")
    }

    /// Load a project from the filesystem
    ///
    /// Creates a Project instance and loads it into memory.
    /// todo!("Refactor to use new ProjectRuntime API")
    pub fn load_project(&mut self, name: String, fs: alloc::boxed::Box<dyn LpFs>) -> Result<(), ServerError> {
        // Check if already loaded
        if self.projects.contains_key(&name) {
            return Ok(()); // Already loaded
        }

        let project_path = format!("{}/{}", self.projects_base_dir, name);

        // Create a new project instance
        let project = Project::new(name.clone(), project_path, fs)?;
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
