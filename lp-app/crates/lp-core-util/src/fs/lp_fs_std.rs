//! Host filesystem implementation using std::fs

use lp_core::error::Error;
use std::fs;
use std::path::PathBuf;
use crate::fs::LpFs;

/// LP filesystem implementation using std::fs
///
/// All paths are resolved relative to `root_path` and validated to ensure
/// they stay within the root directory for security.
pub struct LpFsStd {
    root_path: PathBuf,
}

impl LpFsStd {
    /// Create a new host filesystem with the given root path
    ///
    /// The root path is the project directory. All file operations are
    /// restricted to this directory and its subdirectories.
    pub fn new(root_path: PathBuf) -> Self {
        // Ensure the root directory exists
        if let Err(e) = fs::create_dir_all(&root_path) {
            log::warn!("Failed to create root directory {:?}: {}", root_path, e);
        }
        Self { root_path }
    }

    /// Resolve a path relative to the root and validate it stays within root
    ///
    /// Returns an error if the path would escape the root directory.
    fn resolve_and_validate(&self, path: &str) -> Result<PathBuf, Error> {
        // Normalize the input path (remove leading slash for joining)
        let normalized_path = if path.starts_with('/') {
            &path[1..]
        } else {
            path
        };

        // Join with root path
        let full_path = self.root_path.join(normalized_path);

        // Canonicalize to resolve any `..` components
        let canonical_path = full_path.canonicalize().or_else(|_| {
            // If canonicalize fails (path doesn't exist), use the resolved path
            // but we still need to check it's within root
            Ok(full_path)
        })?;

        // Ensure the canonical path is within the root directory
        let canonical_root = self
            .root_path
            .canonicalize()
            .map_err(|e| Error::Filesystem(format!("Failed to canonicalize root path: {}", e)))?;

        if !canonical_path.starts_with(&canonical_root) {
            return Err(Error::Filesystem(format!(
                "Path {:?} would escape root directory {:?}",
                path, self.root_path
            )));
        }

        Ok(canonical_path)
    }

    /// Get the full path for a file (without canonicalization, for non-existent paths)
    ///
    /// This is used when we need to create files that don't exist yet.
    /// Still validates that the path would be within root.
    fn get_path(&self, path: &str) -> Result<PathBuf, Error> {
        // Normalize the input path
        let normalized_path = if path.starts_with('/') {
            &path[1..]
        } else {
            path
        };

        let full_path = self.root_path.join(normalized_path);

        // Check that the normalized path doesn't contain `..` that would escape root
        // We do this by checking if any component is ".."
        let components: Vec<_> = normalized_path.split('/').collect();
        let mut depth = 0;
        for component in components {
            if component == ".." {
                if depth == 0 {
                    return Err(Error::Filesystem(format!(
                        "Path {:?} would escape root directory",
                        path
                    )));
                }
                depth -= 1;
            } else if !component.is_empty() && component != "." {
                depth += 1;
            }
        }

        Ok(full_path)
    }
}

impl LpFs for LpFsStd {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, Error> {
        let full_path = self.resolve_and_validate(path)?;
        fs::read(&full_path)
            .map_err(|e| Error::Filesystem(format!("Failed to read file {:?}: {}", full_path, e)))
    }

    fn write_file(&self, path: &str, data: &[u8]) -> Result<(), Error> {
        let full_path = self.get_path(path)?;
        // Create parent directory if it doesn't exist
        if let Some(parent) = full_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(Error::Filesystem(format!(
                    "Failed to create directory {:?}: {}",
                    parent, e
                )));
            }
        }
        fs::write(&full_path, data)
            .map_err(|e| Error::Filesystem(format!("Failed to write file {:?}: {}", full_path, e)))
    }

    fn file_exists(&self, path: &str) -> Result<bool, Error> {
        let full_path = self.get_path(path)?;
        Ok(full_path.exists())
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, Error> {
        let full_path = self.resolve_and_validate(path)?;

        // Check if it's actually a directory
        if !full_path.is_dir() {
            return Err(Error::Filesystem(format!(
                "Path {:?} is not a directory",
                path
            )));
        }

        // Read directory contents
        let entries = fs::read_dir(&full_path).map_err(|e| {
            Error::Filesystem(format!("Failed to read directory {:?}: {}", full_path, e))
        })?;

        // Get canonical root for comparison
        let canonical_root = self
            .root_path
            .canonicalize()
            .map_err(|e| Error::Filesystem(format!("Failed to canonicalize root path: {}", e)))?;

        let mut results = Vec::new();
        for entry in entries {
            let entry = entry
                .map_err(|e| Error::Filesystem(format!("Failed to read directory entry: {}", e)))?;

            let entry_path = entry.path();

            // Canonicalize the entry path
            let canonical_entry = entry_path.canonicalize().map_err(|e| {
                Error::Filesystem(format!(
                    "Failed to canonicalize entry path {:?}: {}",
                    entry_path, e
                ))
            })?;

            // Build the relative path from canonical root
            let relative_path = canonical_entry.strip_prefix(&canonical_root).map_err(|_| {
                Error::Filesystem(format!(
                    "Failed to compute relative path from root: entry={:?}, root={:?}",
                    canonical_entry, canonical_root
                ))
            })?;

            // Convert to string with leading slash
            let path_str = format!("/{}", relative_path.to_string_lossy().replace('\\', "/"));

            results.push(path_str);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_path_validation_within_root() {
        let temp_dir = TempDir::new().unwrap();
        let fs = LpFsStd::new(temp_dir.path().to_path_buf());

        // Valid paths should work
        assert!(fs.get_path("/project.json").is_ok());
        assert!(fs.get_path("/src/test.txt").is_ok());
        assert!(fs.get_path("project.json").is_ok());
    }

    #[test]
    fn test_path_validation_prevents_escape() {
        let temp_dir = TempDir::new().unwrap();
        let fs = LpFsStd::new(temp_dir.path().to_path_buf());

        // Paths with .. should be rejected
        assert!(fs.get_path("/../outside.txt").is_err());
        assert!(fs.get_path("/src/../../outside.txt").is_err());
        assert!(fs.resolve_and_validate("/../outside.txt").is_err());
    }

    #[test]
    fn test_list_dir() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test directory structure
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("project.json"), b"{}").unwrap();
        fs::write(root.join("src/file1.txt"), b"content1").unwrap();
        fs::write(root.join("src/file2.txt"), b"content2").unwrap();
        fs::create_dir_all(root.join("src/subdir")).unwrap();

        let fs = LpFsStd::new(root.to_path_buf());
        let entries = fs.list_dir("/src").unwrap();

        // Should contain the files and subdirectory
        assert!(entries.iter().any(|e| e == "/src/file1.txt"));
        assert!(entries.iter().any(|e| e == "/src/file2.txt"));
        assert!(entries.iter().any(|e| e == "/src/subdir"));
    }

    #[test]
    fn test_list_dir_security() {
        let temp_dir = TempDir::new().unwrap();
        let fs = LpFsStd::new(temp_dir.path().to_path_buf());

        // Should not be able to list outside root
        assert!(fs.list_dir("/../").is_err());
    }
}
