//! In-memory filesystem implementation for testing

use crate::app::file_change::{ChangeType, FileChange};
use crate::error::Error;
use crate::fs::Filesystem;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use hashbrown::HashMap;

/// In-memory filesystem implementation for testing
///
/// Stores files in a HashMap and tracks all changes (create, modify, delete).
/// This makes it easy to write tests where you:
/// 1. Create filesystem and write files
/// 2. Create an LpApp with the filesystem
/// 3. Mutate the filesystem
/// 4. Get changes and pass to tick()
/// 5. Validate that the project matches expectations
pub struct InMemoryFilesystem {
    /// File storage: path -> contents
    files: HashMap<String, Vec<u8>>,
    /// Tracked changes since last get_changes() call
    changes: Vec<FileChange>,
}

impl InMemoryFilesystem {
    /// Create a new empty in-memory filesystem
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            changes: Vec::new(),
        }
    }

    /// Get all changes since last call (clears the change log)
    pub fn get_changes(&mut self) -> Vec<FileChange> {
        core::mem::take(&mut self.changes)
    }

    /// Clear change log without retrieving
    pub fn reset_changes(&mut self) {
        self.changes.clear();
    }

    /// Write a file (mutable version for change tracking)
    pub fn write_file_mut(&mut self, path: &str, data: &[u8]) -> Result<(), Error> {
        self.validate_path(path)?;
        let is_create = !self.files.contains_key(path);
        self.files.insert(path.to_string(), data.to_vec());
        self.changes.push(FileChange {
            path: path.to_string(),
            change_type: if is_create {
                ChangeType::Create
            } else {
                ChangeType::Modify
            },
        });
        Ok(())
    }

    /// Delete a file
    pub fn delete_file(&mut self, path: &str) -> Result<(), Error> {
        self.validate_path(path)?;
        if self.files.remove(path).is_some() {
            self.changes.push(FileChange {
                path: path.to_string(),
                change_type: ChangeType::Delete,
            });
            Ok(())
        } else {
            Err(Error::Filesystem(format!("File not found: {}", path)))
        }
    }

    /// Validate that a path is relative to project root (starts with /)
    fn validate_path(&self, path: &str) -> Result<(), Error> {
        if !path.starts_with('/') {
            return Err(Error::Filesystem(format!(
                "Path must be relative to project root (start with /): {}",
                path
            )));
        }
        Ok(())
    }
}

impl Default for InMemoryFilesystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Filesystem for InMemoryFilesystem {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, Error> {
        self.validate_path(path)?;
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| Error::Filesystem(format!("File not found: {}", path)))
    }

    fn write_file(&self, _path: &str, _data: &[u8]) -> Result<(), Error> {
        // Filesystem trait doesn't allow &mut self, so we can't track changes here
        // Use write_file_mut() for change tracking
        Err(Error::Filesystem(
            "Use write_file_mut() for change tracking".to_string(),
        ))
    }

    fn file_exists(&self, path: &str) -> Result<bool, Error> {
        self.validate_path(path)?;
        Ok(self.files.contains_key(path))
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, Error> {
        self.validate_path(path)?;
        let mut entries = Vec::new();
        let prefix = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{}/", path)
        };

        for file_path in self.files.keys() {
            if file_path.starts_with(&prefix) {
                // Extract the entry name (file or subdirectory)
                let remainder = &file_path[prefix.len()..];
                if let Some(slash_pos) = remainder.find('/') {
                    // It's a subdirectory - add the directory path
                    let dir_name = &remainder[..slash_pos];
                    let full_dir_path = format!("{}{}", prefix, dir_name);
                    if !entries.contains(&full_dir_path) {
                        entries.push(full_dir_path);
                    }
                } else {
                    // It's a file directly in this directory
                    entries.push(file_path.clone());
                }
            }
        }

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_create_and_read_file() {
        let mut fs = InMemoryFilesystem::new();
        fs.write_file_mut("/test.txt", b"hello").unwrap();
        assert_eq!(fs.read_file("/test.txt").unwrap(), b"hello");
    }

    #[test]
    fn test_file_exists() {
        let mut fs = InMemoryFilesystem::new();
        assert!(!fs.file_exists("/test.txt").unwrap());
        fs.write_file_mut("/test.txt", b"hello").unwrap();
        assert!(fs.file_exists("/test.txt").unwrap());
    }

    #[test]
    fn test_change_tracking() {
        let mut fs = InMemoryFilesystem::new();
        fs.write_file_mut("/test.txt", b"hello").unwrap();
        let changes = fs.get_changes();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "/test.txt");
        assert_eq!(changes[0].change_type, ChangeType::Create);

        fs.write_file_mut("/test.txt", b"world").unwrap();
        let changes = fs.get_changes();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Modify);

        fs.delete_file("/test.txt").unwrap();
        let changes = fs.get_changes();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Delete);
    }

    #[test]
    fn test_list_dir() {
        let mut fs = InMemoryFilesystem::new();
        fs.write_file_mut("/src/file1.txt", b"content1").unwrap();
        fs.write_file_mut("/src/file2.txt", b"content2").unwrap();
        fs.write_file_mut("/src/nested/file3.txt", b"content3")
            .unwrap();
        fs.write_file_mut("/other.txt", b"content").unwrap();

        let entries = fs.list_dir("/src").unwrap();
        assert!(entries.contains(&"/src/file1.txt".to_string()));
        assert!(entries.contains(&"/src/file2.txt".to_string()));
        // list_dir("/src") should show "/src/nested" as a directory, not the file inside it
        assert!(entries.contains(&"/src/nested".to_string()));
        assert!(!entries.contains(&"/src/nested/file3.txt".to_string()));
        assert!(!entries.contains(&"/other.txt".to_string()));
    }

    #[test]
    fn test_path_validation() {
        let mut fs = InMemoryFilesystem::new();
        assert!(fs.write_file_mut("invalid", b"data").is_err());
        assert!(fs.write_file_mut("/valid", b"data").is_ok());
    }
}
