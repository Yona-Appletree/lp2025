//! In-memory filesystem implementation for testing

use crate::error::FsError;
use crate::fs::LpFs;
use alloc::{format, string::{String, ToString}, vec::Vec};
use hashbrown::HashMap;

/// In-memory filesystem implementation for testing
pub struct LpFsMemory {
    /// File storage: path -> contents
    files: HashMap<String, Vec<u8>>,
}

impl LpFsMemory {
    /// Create a new empty in-memory filesystem
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Write a file (mutable version)
    pub fn write_file_mut(&mut self, path: &str, data: &[u8]) -> Result<(), FsError> {
        self.validate_path(path)?;
        self.files.insert(path.to_string(), data.to_vec());
        Ok(())
    }

    /// Delete a file
    pub fn delete_file(&mut self, path: &str) -> Result<(), FsError> {
        self.validate_path(path)?;
        if self.files.remove(path).is_none() {
            return Err(FsError::NotFound(path.to_string()));
        }
        Ok(())
    }

    /// Validate that a path is relative to project root (starts with /)
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

impl Default for LpFsMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl LpFs for LpFsMemory {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, FsError> {
        self.validate_path(path)?;
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| FsError::NotFound(path.to_string()))
    }

    fn write_file(&self, _path: &str, _data: &[u8]) -> Result<(), FsError> {
        // For immutable access, we can't modify, so return error
        // Use write_file_mut() for mutable access
        Err(FsError::Filesystem(
            "Use write_file_mut() for mutable filesystem".to_string(),
        ))
    }

    fn file_exists(&self, path: &str) -> Result<bool, FsError> {
        self.validate_path(path)?;
        Ok(self.files.contains_key(path))
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
        self.validate_path(path)?;
        let mut entries = Vec::new();
        let prefix = if path.ends_with('/') {
            path.to_string()
        } else {
            alloc::format!("{}/", path)
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

    fn chroot(&self, subdir: &str) -> Result<alloc::boxed::Box<dyn LpFs>, FsError> {
        // Normalize the subdirectory path
        // Remove leading "./" if present, then ensure it starts with "/"
        let normalized_subdir = if subdir.starts_with("./") {
            format!("/{}", &subdir[2..])
        } else if subdir.starts_with('/') {
            subdir.to_string()
        } else {
            format!("/{}", subdir)
        };

        // Ensure it ends with / for prefix matching
        let prefix = if normalized_subdir.ends_with('/') {
            normalized_subdir.clone()
        } else {
            format!("{}/", normalized_subdir)
        };

        // Create a new LpFsMemory with only files under the subdirectory
        let mut new_files = HashMap::new();
        for (path, data) in &self.files {
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
            files: HashMap<String, Vec<u8>>,
        }

        impl LpFs for ChrootedLpFsMemory {
            fn read_file(&self, path: &str) -> Result<alloc::vec::Vec<u8>, FsError> {
                self.validate_path(path)?;
                self.files
                    .get(path)
                    .cloned()
                    .ok_or_else(|| FsError::NotFound(path.to_string()))
            }

            fn write_file(&self, _path: &str, _data: &[u8]) -> Result<(), FsError> {
                Err(FsError::Filesystem(
                    "Use write_file_mut() for mutable filesystem".to_string(),
                ))
            }

            fn file_exists(&self, path: &str) -> Result<bool, FsError> {
                self.validate_path(path)?;
                Ok(self.files.contains_key(path))
            }

            fn list_dir(&self, path: &str) -> Result<alloc::vec::Vec<alloc::string::String>, FsError> {
                self.validate_path(path)?;
                let mut entries = Vec::new();
                let prefix = if path.ends_with('/') {
                    path.to_string()
                } else {
                    format!("{}/", path)
                };

                for file_path in self.files.keys() {
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

                Ok(entries)
            }

            fn chroot(&self, subdir: &str) -> Result<alloc::boxed::Box<dyn LpFs>, FsError> {
                // Recursive chroot - normalize path
                let normalized_subdir = if subdir.starts_with('/') {
                    subdir.to_string()
                } else {
                    format!("/{}", subdir)
                };

                let prefix = if normalized_subdir.ends_with('/') {
                    normalized_subdir.clone()
                } else {
                    format!("{}/", normalized_subdir)
                };

                let mut new_files = HashMap::new();
                for (path, data) in &self.files {
                    if path.starts_with(&prefix) || path == &normalized_subdir {
                        let relative_path = if path.starts_with(&prefix) {
                            format!("/{}", &path[prefix.len()..])
                        } else {
                            "/".to_string()
                        };
                        new_files.insert(relative_path, data.clone());
                    }
                }

                Ok(alloc::boxed::Box::new(ChrootedLpFsMemory { files: new_files }))
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

        Ok(alloc::boxed::Box::new(ChrootedLpFsMemory { files: new_files }))
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
        let mut fs = LpFsMemory::new();
        assert!(fs.write_file_mut("invalid", b"data").is_err());
        assert!(fs.write_file_mut("/valid", b"data").is_ok());
    }
}
