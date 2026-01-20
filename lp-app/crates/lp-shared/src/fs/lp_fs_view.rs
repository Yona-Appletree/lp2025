//! Filesystem view wrapper for chroot functionality
//!
//! `LpFsView` wraps any `LpFs` implementation with a prefix path, translating
//! all operations between chrooted-relative paths and parent-absolute paths.

use crate::error::FsError;
use crate::fs::LpFs;
use crate::fs::fs_event::{FsChange, FsVersion};
use alloc::{
    format,
    rc::Rc,
    string::{String, ToString},
    vec::Vec,
};
use core::cell::RefCell;

/// A filesystem view that translates paths relative to a prefix
///
/// All paths passed to this view are relative to the `prefix` path in the parent filesystem.
/// Operations are delegated to the parent with paths translated accordingly.
pub struct LpFsView {
    /// Parent filesystem to delegate operations to
    parent: Rc<RefCell<dyn LpFs>>,
    /// Prefix path in parent filesystem (e.g., "/projects/my-project/")
    /// Always ends with `/` except for root case
    prefix: String,
}

impl LpFsView {
    /// Create a new filesystem view with the given parent and prefix
    ///
    /// The prefix should be normalized and end with `/` (except for root).
    /// Paths passed to this view will be translated to `prefix + path` in the parent.
    pub fn new(parent: Rc<RefCell<dyn LpFs>>, prefix: String) -> Self {
        Self { parent, prefix }
    }

    /// Translate a chrooted-relative path to a parent-absolute path
    ///
    /// Examples:
    /// - `/src/file.txt` with prefix `/projects/my-project/` → `/projects/my-project/src/file.txt`
    /// - `/` with prefix `/projects/my-project/` → `/projects/my-project`
    fn parent_path(&self, chrooted_path: &str) -> String {
        let normalized = chrooted_path; // TODO: Phase 6 - convert to LpPathBuf::from()
        if normalized == "/" {
            // Root path - use prefix without trailing /
            self.prefix.trim_end_matches('/').to_string()
        } else {
            // Remove leading / from normalized path and prepend prefix
            format!("{}{}", self.prefix, &normalized[1..])
        }
    }

    /// Translate a parent-absolute path to a chrooted-relative path
    ///
    /// Used for translating results from parent operations (e.g., `list_dir`).
    /// Returns `None` if the path doesn't start with the prefix.
    fn chrooted_path(&self, parent_path: &str) -> Option<String> {
        if parent_path.starts_with(&self.prefix) {
            let relative_path = &parent_path[self.prefix.len()..];
            let normalized = if relative_path.is_empty() {
                "/".to_string()
            } else if relative_path.starts_with('/') {
                relative_path.to_string()
            } else {
                format!("/{}", relative_path)
            };
            Some(normalized)
        } else {
            None
        }
    }

    /// Validate that a path is valid for this view
    ///
    /// Paths must start with `/` (absolute from chroot root).
    fn validate_path(&self, path: &str) -> Result<(), FsError> {
        let normalized = Self::normalize_path(path);
        if !normalized.starts_with('/') {
            return Err(FsError::InvalidPath(format!(
                "Path must be relative to view root (start with /): {}",
                path
            )));
        }
        Ok(())
    }
}

impl LpFs for LpFsView {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, FsError> {
        let normalized = Self::normalize_path(path);
        self.validate_path(&normalized)?;
        let parent_path = self.parent_path(&normalized);
        self.parent.borrow().read_file(&parent_path)
    }

    fn write_file(&self, path: &str, data: &[u8]) -> Result<(), FsError> {
        let normalized = Self::normalize_path(path);
        self.validate_path(&normalized)?;
        let parent_path = self.parent_path(&normalized);
        self.parent.borrow().write_file(&parent_path, data)
    }

    fn file_exists(&self, path: &str) -> Result<bool, FsError> {
        let normalized = Self::normalize_path(path);
        self.validate_path(&normalized)?;
        let parent_path = self.parent_path(&normalized);
        self.parent.borrow().file_exists(&parent_path)
    }

    fn is_dir(&self, path: &str) -> Result<bool, FsError> {
        let normalized = Self::normalize_path(path);
        self.validate_path(&normalized)?;
        let parent_path = self.parent_path(&normalized);
        self.parent.borrow().is_dir(&parent_path)
    }

    fn list_dir(&self, path: &str, recursive: bool) -> Result<Vec<String>, FsError> {
        let normalized = Self::normalize_path(path);
        self.validate_path(&normalized)?;
        let parent_path = self.parent_path(&normalized);
        let parent_entries = self.parent.borrow().list_dir(&parent_path, recursive)?;

        // Translate parent paths to chrooted-relative paths
        let mut entries = Vec::new();
        for parent_entry in parent_entries {
            if let Some(chrooted_path) = self.chrooted_path(&parent_entry) {
                entries.push(chrooted_path);
            }
        }

        Ok(entries)
    }

    fn delete_file(&self, path: &str) -> Result<(), FsError> {
        let normalized = Self::normalize_path(path);
        self.validate_path(&normalized)?;

        if normalized == "/" {
            return Err(FsError::InvalidPath(
                "Cannot delete root directory".to_string(),
            ));
        }

        let parent_path = self.parent_path(&normalized);
        self.parent.borrow().delete_file(&parent_path)
    }

    fn delete_dir(&self, path: &str) -> Result<(), FsError> {
        let normalized = Self::normalize_path(path);
        self.validate_path(&normalized)?;

        if normalized == "/" {
            return Err(FsError::InvalidPath(
                "Cannot delete root directory".to_string(),
            ));
        }

        let parent_path = self.parent_path(&normalized);
        self.parent.borrow().delete_dir(&parent_path)
    }

    fn chroot(&self, subdir: &str) -> Result<Rc<RefCell<dyn LpFs>>, FsError> {
        // Normalize the subdirectory path
        let normalized_subdir = subdir; // TODO: Phase 6 - convert to LpPathBuf::from()

        // Construct prefix relative to current chroot
        let relative_prefix = if normalized_subdir.ends_with('/') {
            normalized_subdir.clone()
        } else {
            format!("{}/", normalized_subdir)
        };

        // Construct full prefix in parent filesystem
        // Remove leading / from relative_prefix since self.prefix already ends with /
        let new_prefix = if relative_prefix == "/" {
            self.prefix.clone()
        } else {
            format!("{}{}", self.prefix, &relative_prefix[1..])
        };

        Ok(Rc::new(RefCell::new(LpFsView {
            parent: Rc::clone(&self.parent),
            prefix: new_prefix,
        })))
    }

    fn current_version(&self) -> FsVersion {
        self.parent.borrow().current_version()
    }

    fn get_changes_since(&self, since_version: FsVersion) -> Vec<FsChange> {
        let parent_changes = self.parent.borrow().get_changes_since(since_version);
        let prefix = &self.prefix;

        parent_changes
            .into_iter()
            .filter_map(|change| {
                if change.path.starts_with(prefix) {
                    // Translate to chrooted-relative path
                    if let Some(chrooted_path) = self.chrooted_path(&change.path) {
                        Some(FsChange {
                            path: chrooted_path,
                            change_type: change.change_type,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    fn clear_changes_before(&mut self, _before_version: FsVersion) {
        // No-op for views (parent manages versions)
    }

    fn record_changes(&mut self, _changes: Vec<FsChange>) {
        // No-op for views (parent manages versions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::LpFsMemory;

    #[test]
    fn test_lp_fs_view_basic() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/projects/test/src/file.txt", b"content")
            .unwrap();

        let parent_rc: Rc<RefCell<dyn LpFs>> = Rc::new(RefCell::new(fs));
        let view = LpFsView::new(Rc::clone(&parent_rc), "/projects/test/".to_string());

        assert!(view.file_exists("/src/file.txt").unwrap());
        let content = view.read_file("/src/file.txt").unwrap();
        assert_eq!(content, b"content");
    }

    #[test]
    fn test_lp_fs_view_sees_parent_changes() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/projects/test/src/file.txt", b"initial")
            .unwrap();

        let parent_rc: Rc<RefCell<dyn LpFs>> = Rc::new(RefCell::new(fs));
        let view = LpFsView::new(Rc::clone(&parent_rc), "/projects/test/".to_string());

        // Verify initial content
        let content = view.read_file("/src/file.txt").unwrap();
        assert_eq!(content, b"initial");

        // Modify file in parent filesystem
        parent_rc
            .borrow_mut()
            .write_file("/projects/test/src/file.txt", b"updated")
            .unwrap();

        // View should see the updated content
        let updated_content = view.read_file("/src/file.txt").unwrap();
        assert_eq!(updated_content, b"updated");

        // Create a new file in parent
        parent_rc
            .borrow_mut()
            .write_file("/projects/test/src/newfile.txt", b"new")
            .unwrap();

        // View should see the new file
        assert!(view.file_exists("/src/newfile.txt").unwrap());
        let new_content = view.read_file("/src/newfile.txt").unwrap();
        assert_eq!(new_content, b"new");
    }

    #[test]
    fn test_lp_fs_view_nested() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/a/b/c/file.txt", b"content").unwrap();

        let parent_rc: Rc<RefCell<dyn LpFs>> = Rc::new(RefCell::new(fs));
        let view1 = LpFsView::new(Rc::clone(&parent_rc), "/a/".to_string());
        let view2 = view1.chroot("b").unwrap();

        assert!(view2.borrow().file_exists("/c/file.txt").unwrap());
        let content = view2.borrow().read_file("/c/file.txt").unwrap();
        assert_eq!(content, b"content");
    }

    #[test]
    fn test_lp_fs_view_list_dir() {
        let mut fs = LpFsMemory::new();
        fs.write_file_mut("/projects/test/src/file1.txt", b"1")
            .unwrap();
        fs.write_file_mut("/projects/test/src/file2.txt", b"2")
            .unwrap();
        fs.write_file_mut("/projects/test/other.txt", b"other")
            .unwrap();

        let parent_rc: Rc<RefCell<dyn LpFs>> = Rc::new(RefCell::new(fs));
        let view = LpFsView::new(Rc::clone(&parent_rc), "/projects/test/".to_string());

        let entries = view.list_dir("/src", false).unwrap();
        assert!(entries.contains(&"/src/file1.txt".to_string()));
        assert!(entries.contains(&"/src/file2.txt".to_string()));
        assert!(!entries.contains(&"/projects/test/src/file1.txt".to_string()));
    }
}
