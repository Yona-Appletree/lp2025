//! In-memory filesystem implementation for testing

use crate::error::FsError;
use crate::fs::{LpFs, fs_event::ChangeType, fs_event::FsChange};
use alloc::{
    format,
    rc::Rc,
    string::{String, ToString},
    vec::Vec,
};
use core::cell::RefCell;
use hashbrown::HashMap;

/// In-memory filesystem implementation for testing
pub struct LpFsMemory {
    /// File storage: path -> contents (using RefCell for interior mutability)
    files: RefCell<HashMap<String, Vec<u8>>>,
    /// Tracked filesystem changes (using RefCell for interior mutability)
    changes: RefCell<Vec<FsChange>>,
}

impl LpFsMemory {
    /// Create a new empty in-memory filesystem
    pub fn new() -> Self {
        Self {
            files: RefCell::new(HashMap::new()),
            changes: RefCell::new(Vec::new()),
        }
    }

    /// Get all filesystem changes since last reset
    pub fn get_changes(&self) -> Vec<FsChange> {
        self.changes.borrow().clone()
    }

    /// Reset the change tracking (clear all tracked changes)
    pub fn reset_changes(&mut self) {
        self.changes.borrow_mut().clear();
    }

    /// Record a filesystem change
    fn record_change(&self, path: String, change_type: ChangeType) {
        self.changes
            .borrow_mut()
            .push(FsChange { path, change_type });
    }

    /// Normalize a path string
    ///
    /// - Removes leading "./" or "."
    /// - Ensures path starts with "/"
    /// - Collapses "//" to "/"
    /// - Removes trailing "/" (except for root "/")
    fn normalize_path(path: &str) -> String {
        let mut normalized = path.trim();

        // Remove leading "./" or "."
        if normalized.starts_with("./") {
            normalized = &normalized[2..];
        } else if normalized == "." {
            normalized = "";
        }

        // Ensure it starts with "/"
        let normalized = if normalized.is_empty() {
            "/".to_string()
        } else if normalized.starts_with('/') {
            normalized.to_string()
        } else {
            format!("/{}", normalized)
        };

        // Collapse multiple slashes
        let normalized = normalized.replace("//", "/");

        // Remove trailing "/" unless it's the root
        if normalized.len() > 1 && normalized.ends_with('/') {
            normalized[..normalized.len() - 1].to_string()
        } else {
            normalized
        }
    }

    /// Write a file (mutable version)
    pub fn write_file_mut(&mut self, path: &str, data: &[u8]) -> Result<(), FsError> {
        self.validate_path(path)?;
        let normalized = Self::normalize_path(path);
        let mut files = self.files.borrow_mut();
        let existed = files.contains_key(&normalized);
        files.insert(normalized.clone(), data.to_vec());
        drop(files); // Release borrow before recording change

        // Record change
        let change_type = if existed {
            ChangeType::Modify
        } else {
            ChangeType::Create
        };
        self.record_change(normalized, change_type);

        Ok(())
    }

    /// Delete a file (mutable version)
    pub fn delete_file_mut(&mut self, path: &str) -> Result<(), FsError> {
        // Validate path is safe to delete (explicitly reject "/")
        Self::validate_path_for_deletion(path)?;
        self.validate_path(path)?;
        let normalized = Self::normalize_path(path);

        // Check if it's a directory (by checking if any file starts with normalized + "/")
        let dir_prefix = format!("{}/", normalized);
        let mut files = self.files.borrow_mut();
        for file_path in files.keys() {
            if file_path.starts_with(&dir_prefix) {
                return Err(FsError::Filesystem(format!(
                    "Path {:?} is a directory, use delete_dir_mut() instead",
                    path
                )));
            }
        }

        if files.remove(&normalized).is_none() {
            return Err(FsError::NotFound(path.to_string()));
        }
        drop(files); // Release borrow before recording change

        // Record change
        self.record_change(normalized, ChangeType::Delete);

        Ok(())
    }

    /// Delete a directory (mutable version, always recursive)
    pub fn delete_dir_mut(&mut self, path: &str) -> Result<(), FsError> {
        // Validate path is safe to delete (explicitly reject "/")
        Self::validate_path_for_deletion(path)?;
        self.validate_path(path)?;
        let normalized = Self::normalize_path(path);

        // Check if it's actually a directory (has files with this prefix)
        let prefix = if normalized.ends_with('/') {
            normalized.clone()
        } else {
            format!("{}/", normalized)
        };

        let mut files = self.files.borrow_mut();
        let mut found_any = false;
        let mut files_to_remove = Vec::new();

        for file_path in files.keys() {
            if file_path.starts_with(&prefix) || file_path == &normalized {
                files_to_remove.push(file_path.clone());
                found_any = true;
            }
        }

        if !found_any {
            return Err(FsError::NotFound(path.to_string()));
        }

        // Remove all files with this prefix (recursive deletion)
        let files_to_remove_clone = files_to_remove.clone();
        for file_path in &files_to_remove {
            let normalized_path = Self::normalize_path(file_path);
            files.remove(&normalized_path);
        }
        drop(files); // Release borrow before recording changes

        // Record changes
        for file_path in files_to_remove_clone {
            let normalized_path = Self::normalize_path(&file_path);
            self.record_change(normalized_path, ChangeType::Delete);
        }

        Ok(())
    }

    /// Validate that a path is relative to project root (starts with /)
    fn validate_path(&self, path: &str) -> Result<(), FsError> {
        let normalized = Self::normalize_path(path);
        if !normalized.starts_with('/') {
            return Err(FsError::InvalidPath(format!(
                "Path must be relative to project root (start with /): {}",
                path
            )));
        }
        Ok(())
    }

    /// Validate that a path is safe to delete
    ///
    /// Returns an error if:
    /// - Path is "/" (root)
    ///
    /// This is a separate function so we can test it without attempting dangerous operations.
    pub fn validate_path_for_deletion(path: &str) -> Result<(), FsError> {
        let normalized = Self::normalize_path(path);
        if normalized == "/" {
            return Err(FsError::InvalidPath(
                "Cannot delete root directory".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for LpFsMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl LpFs for LpFsMemory {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, FsError> {
        self.validate_path(path)?;
        let normalized = Self::normalize_path(path);
        self.files
            .borrow()
            .get(&normalized)
            .cloned()
            .ok_or_else(|| FsError::NotFound(path.to_string()))
    }

    fn write_file(&self, path: &str, data: &[u8]) -> Result<(), FsError> {
        // Use interior mutability to allow writes through immutable reference
        self.validate_path(path)?;
        let normalized = Self::normalize_path(path);
        let mut files = self.files.borrow_mut();
        let existed = files.contains_key(&normalized);
        files.insert(normalized.clone(), data.to_vec());
        drop(files); // Release borrow before recording change

        // Record change
        let change_type = if existed {
            ChangeType::Modify
        } else {
            ChangeType::Create
        };
        self.record_change(normalized, change_type);

        Ok(())
    }

    fn file_exists(&self, path: &str) -> Result<bool, FsError> {
        self.validate_path(path)?;
        let normalized = Self::normalize_path(path);
        Ok(self.files.borrow().contains_key(&normalized))
    }

    fn is_dir(&self, path: &str) -> Result<bool, FsError> {
        self.validate_path(path)?;
        let normalized = Self::normalize_path(path);
        let files = self.files.borrow();

        // Check if it exists as a file
        if files.contains_key(&normalized) {
            return Ok(false);
        }

        // Check if any file path starts with normalized + "/" (indicating it's a directory)
        let dir_prefix = format!("{}/", normalized);
        for file_path in files.keys() {
            if file_path.starts_with(&dir_prefix) {
                return Ok(true);
            }
        }

        // Path doesn't exist
        Err(FsError::NotFound(path.to_string()))
    }

    fn list_dir(&self, path: &str, recursive: bool) -> Result<Vec<String>, FsError> {
        self.validate_path(path)?;
        let normalized = Self::normalize_path(path);
        let mut entries = Vec::new();
        let prefix = if normalized.ends_with('/') {
            normalized.clone()
        } else {
            alloc::format!("{}/", normalized)
        };
        let files = self.files.borrow();

        if recursive {
            // Recursive: return all files/directories with this prefix
            for file_path in files.keys() {
                if file_path.starts_with(&prefix) {
                    entries.push(file_path.clone());
                }
            }
            // Also include directories (paths that are prefixes of files)
            let mut dirs = hashbrown::HashSet::new();
            for file_path in files.keys() {
                if file_path.starts_with(&prefix) {
                    let remainder = &file_path[prefix.len()..];
                    if let Some(slash_pos) = remainder.find('/') {
                        let dir_path = format!("{}{}", prefix, &remainder[..slash_pos]);
                        dirs.insert(dir_path);
                    }
                }
            }
            // Add directories that aren't already in entries
            for dir_path in dirs {
                if !entries.contains(&dir_path) {
                    entries.push(dir_path);
                }
            }
        } else {
            // Non-recursive: only immediate children
            for file_path in files.keys() {
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
        }

        Ok(entries)
    }

    fn delete_file(&self, path: &str) -> Result<(), FsError> {
        // Use interior mutability to allow deletes through immutable reference
        Self::validate_path_for_deletion(path)?;
        self.validate_path(path)?;
        let normalized = Self::normalize_path(path);

        // Check if it's a directory (by checking if any file starts with normalized + "/")
        let dir_prefix = format!("{}/", normalized);
        let mut files = self.files.borrow_mut();
        for file_path in files.keys() {
            if file_path.starts_with(&dir_prefix) {
                return Err(FsError::Filesystem(format!(
                    "Path {:?} is a directory, use delete_dir() instead",
                    path
                )));
            }
        }

        if files.remove(&normalized).is_none() {
            return Err(FsError::NotFound(path.to_string()));
        }
        drop(files); // Release borrow before recording change

        // Record change
        self.record_change(normalized, ChangeType::Delete);

        Ok(())
    }

    fn delete_dir(&self, path: &str) -> Result<(), FsError> {
        // Use interior mutability to allow deletes through immutable reference
        Self::validate_path_for_deletion(path)?;
        self.validate_path(path)?;
        let normalized = Self::normalize_path(path);

        // Check if it's actually a directory (has files with this prefix)
        let prefix = if normalized.ends_with('/') {
            normalized.clone()
        } else {
            format!("{}/", normalized)
        };

        let mut files = self.files.borrow_mut();
        let mut found_any = false;
        let mut files_to_remove = Vec::new();

        for file_path in files.keys() {
            if file_path.starts_with(&prefix) || file_path == &normalized {
                files_to_remove.push(file_path.clone());
                found_any = true;
            }
        }

        if !found_any {
            return Err(FsError::NotFound(path.to_string()));
        }

        // Remove all files with this prefix (recursive deletion)
        for file_path in &files_to_remove {
            let normalized_path = Self::normalize_path(file_path);
            files.remove(&normalized_path);
        }
        drop(files); // Release borrow before recording changes

        // Record changes
        for file_path in files_to_remove {
            let normalized_path = Self::normalize_path(&file_path);
            self.record_change(normalized_path, ChangeType::Delete);
        }

        Ok(())
    }

    fn chroot(
        &self,
        subdir: &str,
    ) -> Result<alloc::rc::Rc<core::cell::RefCell<dyn LpFs>>, FsError> {
        // Normalize the subdirectory path
        let normalized_subdir = Self::normalize_path(subdir);

        // Ensure it ends with / for prefix matching
        let prefix = if normalized_subdir.ends_with('/') {
            normalized_subdir.clone()
        } else {
            format!("{}/", normalized_subdir)
        };

        // Create a new LpFsMemory with only files under the subdirectory
        let mut new_files = HashMap::new();
        let files = self.files.borrow();
        for (path, data) in files.iter() {
            if path.starts_with(&prefix) || path == &normalized_subdir {
                // Remove the prefix from the path to make it relative to the new root
                let relative_path = if path.starts_with(&prefix) {
                    format!("/{}", &path[prefix.len()..])
                } else {
                    "/".to_string() // Root file
                };
                new_files.insert(relative_path, data.clone());
            }
        }

        // Create a new LpFsMemory with the filtered files
        // We need to wrap it in a way that implements LpFs
        // Since we can't create a new struct here, we'll create a wrapper
        struct ChrootedLpFsMemory {
            files: RefCell<HashMap<String, Vec<u8>>>,
            changes: RefCell<Vec<FsChange>>,
        }

        impl LpFs for ChrootedLpFsMemory {
            fn read_file(&self, path: &str) -> Result<alloc::vec::Vec<u8>, FsError> {
                // Normalize path first (handles relative paths by prepending /)
                let normalized = LpFsMemory::normalize_path(path);
                self.validate_path(&normalized)?;
                self.files
                    .borrow()
                    .get(&normalized)
                    .cloned()
                    .ok_or_else(|| FsError::NotFound(path.to_string()))
            }

            fn write_file(&self, path: &str, data: &[u8]) -> Result<(), FsError> {
                let normalized = LpFsMemory::normalize_path(path);
                self.validate_path(&normalized)?;
                self.files
                    .borrow_mut()
                    .insert(normalized.clone(), data.to_vec());
                self.changes.borrow_mut().push(FsChange {
                    path: normalized,
                    change_type: ChangeType::Modify,
                });
                Ok(())
            }

            fn file_exists(&self, path: &str) -> Result<bool, FsError> {
                // Normalize path first (handles relative paths by prepending /)
                let normalized = LpFsMemory::normalize_path(path);
                self.validate_path(&normalized)?;
                Ok(self.files.borrow().contains_key(&normalized))
            }

            fn is_dir(&self, path: &str) -> Result<bool, FsError> {
                // Normalize path first (handles relative paths by prepending /)
                let normalized = LpFsMemory::normalize_path(path);
                self.validate_path(&normalized)?;

                let files = self.files.borrow();
                // Check if it exists as a file
                if files.contains_key(&normalized) {
                    return Ok(false);
                }

                // Check if any file path starts with normalized + "/" (indicating it's a directory)
                let dir_prefix = format!("{}/", normalized);
                for file_path in files.keys() {
                    if file_path.starts_with(&dir_prefix) {
                        return Ok(true);
                    }
                }

                // Path doesn't exist
                Err(FsError::NotFound(path.to_string()))
            }

            fn list_dir(
                &self,
                path: &str,
                recursive: bool,
            ) -> Result<alloc::vec::Vec<alloc::string::String>, FsError> {
                // Normalize path first (handles relative paths by prepending /)
                let normalized = LpFsMemory::normalize_path(path);
                self.validate_path(&normalized)?;
                let mut entries = Vec::new();
                let prefix = if normalized.ends_with('/') {
                    normalized.clone()
                } else {
                    format!("{}/", normalized)
                };

                let files = self.files.borrow();
                if recursive {
                    // Recursive: return all files with this prefix
                    for file_path in files.keys() {
                        if file_path.starts_with(&prefix) {
                            entries.push(file_path.clone());
                        }
                    }
                } else {
                    // Non-recursive: only immediate children
                    for file_path in files.keys() {
                        if file_path.starts_with(&prefix) {
                            let remainder = &file_path[prefix.len()..];
                            if let Some(slash_pos) = remainder.find('/') {
                                let dir_name = &remainder[..slash_pos];
                                let full_dir_path = format!("{}{}", prefix, dir_name);
                                if !entries.contains(&full_dir_path) {
                                    entries.push(full_dir_path);
                                }
                            } else {
                                entries.push(file_path.clone());
                            }
                        }
                    }
                }

                Ok(entries)
            }

            fn delete_file(&self, path: &str) -> Result<(), FsError> {
                let normalized = LpFsMemory::normalize_path(path);
                self.validate_path(&normalized)?;

                if normalized == "/" {
                    return Err(FsError::InvalidPath(
                        "Cannot delete root directory".to_string(),
                    ));
                }

                if !self.files.borrow().contains_key(&normalized) {
                    return Err(FsError::NotFound(path.to_string()));
                }

                self.files.borrow_mut().remove(&normalized);
                self.changes.borrow_mut().push(FsChange {
                    path: normalized,
                    change_type: ChangeType::Delete,
                });
                Ok(())
            }

            fn delete_dir(&self, path: &str) -> Result<(), FsError> {
                let normalized = LpFsMemory::normalize_path(path);
                self.validate_path(&normalized)?;

                if normalized == "/" {
                    return Err(FsError::InvalidPath(
                        "Cannot delete root directory".to_string(),
                    ));
                }

                let prefix = if normalized.ends_with('/') {
                    normalized.clone()
                } else {
                    format!("{}/", normalized)
                };

                let mut files_to_remove = Vec::new();
                {
                    let files = self.files.borrow();
                    for file_path in files.keys() {
                        if file_path == &normalized || file_path.starts_with(&prefix) {
                            files_to_remove.push(file_path.clone());
                        }
                    }
                }

                if files_to_remove.is_empty() {
                    return Err(FsError::NotFound(path.to_string()));
                }

                let mut files = self.files.borrow_mut();
                let mut changes = self.changes.borrow_mut();
                for file_path in files_to_remove {
                    files.remove(&file_path);
                    changes.push(FsChange {
                        path: file_path,
                        change_type: ChangeType::Delete,
                    });
                }

                Ok(())
            }

            fn chroot(
                &self,
                subdir: &str,
            ) -> Result<alloc::rc::Rc<core::cell::RefCell<dyn LpFs>>, FsError> {
                // Recursive chroot - normalize path
                let normalized_subdir = LpFsMemory::normalize_path(subdir);

                let prefix = if normalized_subdir.ends_with('/') {
                    normalized_subdir.clone()
                } else {
                    format!("{}/", normalized_subdir)
                };

                let mut new_files = HashMap::new();
                {
                    let files = self.files.borrow();
                    for (path, data) in files.iter() {
                        if path.starts_with(&prefix) || path == &normalized_subdir {
                            let relative_path = if path.starts_with(&prefix) {
                                format!("/{}", &path[prefix.len()..])
                            } else {
                                "/".to_string()
                            };
                            new_files.insert(relative_path, data.clone());
                        }
                    }
                }

                Ok(Rc::new(RefCell::new(ChrootedLpFsMemory {
                    files: RefCell::new(new_files),
                    changes: RefCell::new(Vec::new()),
                })))
            }
        }

        impl ChrootedLpFsMemory {
            fn validate_path(&self, path: &str) -> Result<(), FsError> {
                if !path.starts_with('/') {
                    return Err(FsError::InvalidPath(format!(
                        "Path must be relative to project root (start with /): {}",
                        path
                    )));
                }
                Ok(())
            }
        }

        Ok(Rc::new(RefCell::new(ChrootedLpFsMemory {
            files: RefCell::new(new_files),
            changes: RefCell::new(Vec::new()),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_create_and_read_file() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/test.txt", b"hello").unwrap();
        assert_eq!(fs.read_file("/test.txt").unwrap(), b"hello");
    }

    #[test]
    fn test_file_exists() {
        let mut fs = LpFsMemory::new();
        assert!(!fs.file_exists("/test.txt").unwrap());
        fs.write_file_mut("/test.txt", b"hello").unwrap();
        assert!(fs.file_exists("/test.txt").unwrap());
    }

    #[test]
    fn test_list_dir() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/src/file1.txt", b"content1").unwrap();
        fs.write_file_mut("/src/file2.txt", b"content2").unwrap();
        fs.write_file_mut("/src/nested/file3.txt", b"content3")
            .unwrap();
        fs.write_file_mut("/other.txt", b"content").unwrap();

        let entries = fs.list_dir("/src", false).unwrap();
        assert!(entries.contains(&"/src/file1.txt".to_string()));
        assert!(entries.contains(&"/src/file2.txt".to_string()));
        // list_dir("/src") should show "/src/nested" as a directory, not the file inside it
        assert!(entries.contains(&"/src/nested".to_string()));
        assert!(!entries.contains(&"/src/nested/file3.txt".to_string()));
        assert!(!entries.contains(&"/other.txt".to_string()));
    }

    #[test]
    fn test_list_dir_recursive() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/src/file1.txt", b"content1").unwrap();
        fs.write_file_mut("/src/nested/file2.txt", b"content2")
            .unwrap();
        fs.write_file_mut("/src/nested/deep/file3.txt", b"content3")
            .unwrap();

        let entries = fs.list_dir("/src", true).unwrap();
        assert!(entries.contains(&"/src/file1.txt".to_string()));
        assert!(entries.contains(&"/src/nested/file2.txt".to_string()));
        assert!(entries.contains(&"/src/nested/deep/file3.txt".to_string()));
    }

    #[test]
    fn test_delete_file() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/test.txt", b"content").unwrap();
        assert!(fs.file_exists("/test.txt").unwrap());

        fs.delete_file_mut("/test.txt").unwrap();
        assert!(!fs.file_exists("/test.txt").unwrap());
    }

    #[test]
    fn test_delete_dir() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/dir/file1.txt", b"content1").unwrap();
        fs.write_file_mut("/dir/nested/file2.txt", b"content2")
            .unwrap();
        assert!(fs.file_exists("/dir/file1.txt").unwrap());
        assert!(fs.file_exists("/dir/nested/file2.txt").unwrap());

        fs.delete_dir_mut("/dir").unwrap();
        assert!(!fs.file_exists("/dir/file1.txt").unwrap());
        assert!(!fs.file_exists("/dir/nested/file2.txt").unwrap());
    }

    #[test]
    fn test_delete_root_rejected() {
        let mut fs = LpFsMemory::new();
        assert!(fs.delete_file_mut("/").is_err());
        assert!(fs.delete_dir_mut("/").is_err());
    }

    #[test]
    fn test_validate_path_for_deletion() {
        assert!(LpFsMemory::validate_path_for_deletion("/").is_err());
        assert!(LpFsMemory::validate_path_for_deletion("/file.txt").is_ok());
        assert!(LpFsMemory::validate_path_for_deletion("/dir").is_ok());
    }

    #[test]
    fn test_path_validation() {
        let mut fs = LpFsMemory::new();
        // Paths without leading slash are normalized to have one, so they're valid
        fs.write_file_mut("relative", b"data").unwrap();
        assert!(fs.file_exists("/relative").unwrap());
        assert!(fs.write_file_mut("/valid", b"data").is_ok());

        // Test that normalization works correctly
        fs.write_file_mut("./normalized", b"data2").unwrap();
        assert!(fs.file_exists("/normalized").unwrap());
    }

    #[test]
    fn test_chroot_basic() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/projects/test/project.json", b"{}")
            .unwrap();
        fs.write_file_mut("/projects/test/src/file.txt", b"content")
            .unwrap();
        fs.write_file_mut("/projects/other/file.txt", b"other")
            .unwrap();

        let chrooted = fs.chroot("/projects/test").unwrap();
        assert!(chrooted.borrow().file_exists("/project.json").unwrap());
        assert!(chrooted.borrow().file_exists("/src/file.txt").unwrap());
        assert!(
            !chrooted
                .borrow()
                .file_exists("/projects/other/file.txt")
                .unwrap()
        );
    }

    #[test]
    fn test_chroot_with_relative_path() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/test-projects/test/project.json", b"{}")
            .unwrap();
        fs.write_file_mut("/test-projects/test/src/file.txt", b"content")
            .unwrap();

        // Test with "./test-projects/test"
        let chrooted = fs.chroot("./test-projects/test").unwrap();
        assert!(chrooted.borrow().file_exists("/project.json").unwrap());
        assert!(chrooted.borrow().file_exists("/src/file.txt").unwrap());
    }

    #[test]
    fn test_chroot_path_normalization() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/a/b/c/file.txt", b"content").unwrap();

        // All these should work and point to the same directory
        let chroot1 = fs.chroot("/a/b").unwrap();
        let chroot2 = fs.chroot("./a/b").unwrap();
        let chroot3 = fs.chroot("a/b").unwrap();

        assert!(chroot1.borrow().file_exists("/c/file.txt").unwrap());
        assert!(chroot2.borrow().file_exists("/c/file.txt").unwrap());
        assert!(chroot3.borrow().file_exists("/c/file.txt").unwrap());
    }

    #[test]
    fn test_chroot_relative_paths() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/src/test.shader/main.glsl", b"shader code")
            .unwrap();
        fs.write_file_mut("/src/test.shader/node.json", b"{}")
            .unwrap();

        // Chroot to node directory
        let chrooted = fs.chroot("/src/test.shader").unwrap();

        // Relative paths should work in chrooted filesystem
        assert!(chrooted.borrow().file_exists("main.glsl").unwrap());
        assert!(chrooted.borrow().file_exists("/main.glsl").unwrap());
        assert!(chrooted.borrow().file_exists("./main.glsl").unwrap());

        // Read file with relative path
        let content = chrooted.borrow().read_file("main.glsl").unwrap();
        assert_eq!(content, b"shader code");

        // Read file with absolute path (normalized)
        let content2 = chrooted.borrow().read_file("/main.glsl").unwrap();
        assert_eq!(content2, b"shader code");

        // Read file with ./ prefix
        let content3 = chrooted.borrow().read_file("./main.glsl").unwrap();
        assert_eq!(content3, b"shader code");
    }

    #[test]
    fn test_chroot_path_normalization_relative() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/a/b/c/file.txt", b"content").unwrap();
        fs.write_file_mut("/a/b/other.txt", b"other").unwrap();

        let chrooted = fs.chroot("/a/b").unwrap();

        // Test various relative path formats
        assert!(chrooted.borrow().file_exists("c/file.txt").unwrap());
        assert!(chrooted.borrow().file_exists("./c/file.txt").unwrap());
        assert!(chrooted.borrow().file_exists("/c/file.txt").unwrap());

        assert!(chrooted.borrow().file_exists("other.txt").unwrap());
        assert!(chrooted.borrow().file_exists("./other.txt").unwrap());
        assert!(chrooted.borrow().file_exists("/other.txt").unwrap());

        // Read with relative path
        let content = chrooted.borrow().read_file("c/file.txt").unwrap();
        assert_eq!(content, b"content");

        // Read with normalized absolute path
        let content2 = chrooted.borrow().read_file("/c/file.txt").unwrap();
        assert_eq!(content2, b"content");
    }

    #[test]
    fn test_chroot_list_dir_relative_paths() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/src/shader/main.glsl", b"code").unwrap();
        fs.write_file_mut("/src/shader/node.json", b"{}").unwrap();
        fs.write_file_mut("/src/shader/util.glsl", b"util").unwrap();

        let chrooted = fs.chroot("/src/shader").unwrap();

        // List root directory
        let entries = chrooted.borrow().list_dir("/", false).unwrap();
        assert!(entries.contains(&"/main.glsl".to_string()));
        assert!(entries.contains(&"/node.json".to_string()));
        assert!(entries.contains(&"/util.glsl".to_string()));

        // List with relative path (should normalize to /)
        let entries2 = chrooted.borrow().list_dir(".", false).unwrap();
        assert!(entries2.contains(&"/main.glsl".to_string()));
    }

    #[test]
    fn test_chroot_nested() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/a/b/c/file.txt", b"content").unwrap();

        let chroot1 = fs.chroot("/a").unwrap();
        let chroot2 = chroot1.borrow().chroot("b").unwrap();
        assert!(chroot2.borrow().file_exists("/c/file.txt").unwrap());
    }

    #[test]
    fn test_chroot_read_file() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/projects/test/project.json", b"{\"name\":\"test\"}")
            .unwrap();

        let chrooted = fs.chroot("/projects/test").unwrap();
        let content = chrooted.borrow().read_file("/project.json").unwrap();
        assert_eq!(content, b"{\"name\":\"test\"}");
    }

    #[test]
    fn test_chroot_list_dir() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/projects/test/src/file1.txt", b"1")
            .unwrap();
        fs.write_file_mut("/projects/test/src/file2.txt", b"2")
            .unwrap();
        fs.write_file_mut("/projects/test/other.txt", b"other")
            .unwrap();

        let chrooted = fs.chroot("/projects/test").unwrap();
        let entries = chrooted.borrow().list_dir("/src", false).unwrap();
        assert!(entries.contains(&"/src/file1.txt".to_string()));
        assert!(entries.contains(&"/src/file2.txt".to_string()));
        assert!(!entries.contains(&"/projects/test/src/file1.txt".to_string()));
    }
}
