use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Light Player path - paths from project root
///
/// Supports both absolute (starting with `/`) and relative paths.
/// Paths are automatically normalized on construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LpPath(String);

impl LpPath {
    /// Create a new LpPath, normalizing the path
    pub fn new(path: String) -> Self {
        Self(normalize(&path))
    }

    /// Get the path as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if the path is absolute (starts with `/`)
    pub fn is_absolute(&self) -> bool {
        self.0.starts_with('/')
    }

    /// Check if the path is relative (!starts with `/`)
    pub fn is_relative(&self) -> bool {
        !self.0.starts_with('/')
    }

    /// Get the parent directory path
    ///
    /// Returns `None` if the path is root (`/`) or empty, or if there's no parent component.
    pub fn parent(&self) -> Option<LpPath> {
        if self.0 == "/" || self.0.is_empty() {
            return None;
        }

        // Find the last `/`
        if let Some(last_slash) = self.0.rfind('/') {
            if last_slash == 0 {
                // Only root `/` remains
                Some(LpPath("/".to_string()))
            } else {
                // Return path up to (but not including) last `/`
                Some(LpPath(self.0[..last_slash].to_string()))
            }
        } else {
            // No `/` found, check if there's a parent component
            // For relative paths like "src/test", parent is "src"
            // For relative paths like "test", there's no parent
            if self.0.contains('/') {
                // This shouldn't happen after normalization, but handle it
                if let Some(last_slash) = self.0.rfind('/') {
                    if last_slash == 0 {
                        Some(LpPath("/".to_string()))
                    } else {
                        Some(LpPath(self.0[..last_slash].to_string()))
                    }
                } else {
                    None
                }
            } else {
                // Single component relative path, no parent
                None
            }
        }
    }

    /// Get the last component (file name)
    ///
    /// Returns `None` if the path is root (`/`) or empty.
    pub fn file_name(&self) -> Option<&str> {
        if self.0 == "/" || self.0.is_empty() {
            return None;
        }

        // Find the last `/`
        if let Some(last_slash) = self.0.rfind('/') {
            if last_slash == self.0.len() - 1 {
                // Trailing `/`, this shouldn't happen after normalization, but handle it
                // Find the component before the trailing slash
                let trimmed = &self.0[..last_slash];
                if trimmed.is_empty() || trimmed == "/" {
                    None
                } else if let Some(prev_slash) = trimmed.rfind('/') {
                    Some(&trimmed[prev_slash + 1..])
                } else {
                    Some(trimmed)
                }
            } else {
                Some(&self.0[last_slash + 1..])
            }
        } else {
            // No `/` found, entire path is the file name
            Some(&self.0)
        }
    }

    /// Get the file stem (file name without extension)
    ///
    /// Returns `None` if there is no file name or no extension.
    pub fn file_stem(&self) -> Option<&str> {
        self.file_name().and_then(|name| {
            if let Some(dot_pos) = name.rfind('.') {
                if dot_pos > 0 {
                    Some(&name[..dot_pos])
                } else {
                    // File name starts with `.`, no stem
                    None
                }
            } else {
                // No extension, entire name is the stem
                Some(name)
            }
        })
    }

    /// Get the file extension (without leading dot)
    ///
    /// Returns `None` if there is no file name or no extension.
    pub fn extension(&self) -> Option<&str> {
        self.file_name().and_then(|name| {
            if let Some(dot_pos) = name.rfind('.') {
                if dot_pos < name.len() - 1 {
                    Some(&name[dot_pos + 1..])
                } else {
                    // Trailing `.`, no extension
                    None
                }
            } else {
                None
            }
        })
    }

    /// Join a path to this path
    ///
    /// Matches PathBuf::join behavior:
    /// - If `path` is absolute, replace base path
    /// - If `path` is relative, append to base (does NOT resolve `..` components)
    /// - Normalizes result
    pub fn join<P: AsRef<str>>(&self, path: P) -> LpPath {
        let path_str = path.as_ref();
        if path_str.starts_with('/') {
            // Absolute path, replace base
            LpPath::from(path_str)
        } else {
            // Relative path, append to base
            if self.0 == "/" {
                LpPath::from(format!("/{}", path_str))
            } else {
                LpPath::from(format!("{}/{}", self.0, path_str))
            }
        }
    }

    /// Join and resolve a relative path
    ///
    /// Convenience method beyond PathBuf API:
    /// - Similar to `join()` but resolves `.` and `..` components
    /// - Returns `None` if result would be invalid (e.g., goes above root for absolute paths)
    pub fn join_relative<P: AsRef<str>>(&self, path: P) -> Option<LpPath> {
        let path_str = path.as_ref();
        if path_str.starts_with('/') {
            // Absolute path, just normalize
            return Some(LpPath::from(path_str));
        }

        // Split into components
        let mut components: Vec<&str> = self
            .components()
            .collect::<Vec<_>>()
            .iter()
            .map(|s| *s)
            .collect();

        // Add relative path components
        let relative_components: Vec<&str> = path_str.split('/').collect();
        for component in relative_components {
            match component {
                "." => {
                    // Current directory - no change
                }
                ".." => {
                    // Parent directory - remove last component
                    if components.is_empty() {
                        // Going above root for absolute path
                        if self.is_absolute() {
                            return None;
                        }
                        // For relative paths, allow going "up"
                    } else {
                        components.pop();
                    }
                }
                "" => {
                    // Empty component (e.g., leading/trailing slash) - ignore
                }
                name => {
                    // Regular component - add it
                    components.push(name);
                }
            }
        }

        // Reconstruct path
        let resolved_path = if components.is_empty() {
            if self.is_absolute() {
                "/".to_string()
            } else {
                ".".to_string()
            }
        } else if self.is_absolute() {
            format!("/{}", components.join("/"))
        } else {
            components.join("/")
        };

        Some(LpPath::from(resolved_path))
    }

    /// Strip a prefix from this path
    ///
    /// Returns `None` if the prefix doesn't match.
    pub fn strip_prefix<P: AsRef<str>>(&self, prefix: P) -> Option<LpPath> {
        let prefix_str = prefix.as_ref();
        let prefix_path = LpPath::from(prefix_str);

        if self.starts_with(prefix_str) {
            // Get components after prefix
            let self_components: Vec<&str> = self.components().collect();
            let prefix_components: Vec<&str> = prefix_path.components().collect();

            if prefix_components.len() > self_components.len() {
                return None;
            }

            let remaining: Vec<&str> = self_components[prefix_components.len()..].to_vec();

            if remaining.is_empty() {
                Some(LpPath::from(if self.is_absolute() { "/" } else { "." }))
            } else if self.is_absolute() {
                Some(LpPath::from(format!("/{}", remaining.join("/"))))
            } else {
                Some(LpPath::from(remaining.join("/")))
            }
        } else {
            None
        }
    }

    /// Check if this path starts with the given base path
    ///
    /// Only considers whole path components to match.
    pub fn starts_with<P: AsRef<str>>(&self, base: P) -> bool {
        let base_str = base.as_ref();
        let base_path = LpPath::from(base_str);

        let self_components: Vec<&str> = self.components().collect();
        let base_components: Vec<&str> = base_path.components().collect();

        if base_components.len() > self_components.len() {
            return false;
        }

        self_components[..base_components.len()] == base_components[..]
    }

    /// Check if this path ends with the given child path
    ///
    /// Only considers whole path components to match.
    pub fn ends_with<P: AsRef<str>>(&self, child: P) -> bool {
        let child_str = child.as_ref();
        let child_path = LpPath::from(child_str);

        let self_components: Vec<&str> = self.components().collect();
        let child_components: Vec<&str> = child_path.components().collect();

        if child_components.len() > self_components.len() {
            return false;
        }

        let start_idx = self_components.len() - child_components.len();
        self_components[start_idx..] == child_components[..]
    }

    /// Get an iterator over path components
    ///
    /// Skips root `/` for absolute paths and empty components.
    pub fn components(&self) -> Components<'_> {
        Components {
            path: &self.0,
            start: if self.0.starts_with('/') { 1 } else { 0 },
        }
    }
}

impl From<String> for LpPath {
    fn from(s: String) -> Self {
        Self(normalize(&s))
    }
}

impl From<&str> for LpPath {
    fn from(s: &str) -> Self {
        Self(normalize(s))
    }
}

/// Iterator over path components
pub struct Components<'a> {
    path: &'a str,
    start: usize,
}

impl<'a> Iterator for Components<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.path.len() {
            return None;
        }

        let remaining = &self.path[self.start..];
        if let Some(slash_pos) = remaining.find('/') {
            if slash_pos == 0 {
                // Skip empty component
                self.start += 1;
                return self.next();
            }
            let component = &remaining[..slash_pos];
            self.start += slash_pos + 1;
            Some(component)
        } else {
            // Last component
            if remaining.is_empty() {
                None
            } else {
                let component = remaining;
                self.start = self.path.len();
                Some(component)
            }
        }
    }
}

/// Normalize a path string
///
/// Normalization rules:
/// - Trim whitespace
/// - Remove leading `./` or `.` (if present)
/// - For absolute paths: ensure leading `/`
/// - For relative paths: keep as-is (no leading `/`)
/// - Collapse multiple consecutive slashes (`//` → `/`)
/// - Remove trailing `/` unless it's the root path (`/`)
/// - Handle empty paths: `""` → `"/"` (absolute root)
fn normalize(path: &str) -> String {
    let mut normalized = path.trim();

    // Remove leading "./" or "."
    if normalized.starts_with("./") {
        normalized = &normalized[2..];
    } else if normalized == "." {
        normalized = "";
    }

    // Handle empty path
    let normalized = if normalized.is_empty() {
        "/".to_string()
    } else {
        normalized.to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lp_path_creation() {
        let path = LpPath::new("/src/test.texture".to_string());
        assert_eq!(path.as_str(), "/src/test.texture");
    }

    #[test]
    fn test_lp_path_from_string() {
        let path = LpPath::from("/src/test.shader".to_string());
        assert_eq!(path.as_str(), "/src/test.shader");
    }

    #[test]
    fn test_lp_path_from_str() {
        let path = LpPath::from("/src/test.output");
        assert_eq!(path.as_str(), "/src/test.output");
    }

    #[test]
    fn test_normalization_absolute() {
        assert_eq!(LpPath::from("//src//test//").as_str(), "/src/test");
        assert_eq!(LpPath::from("/src/test/").as_str(), "/src/test");
        assert_eq!(LpPath::from("  /src/test  ").as_str(), "/src/test");
    }

    #[test]
    fn test_normalization_relative() {
        assert_eq!(LpPath::from("src/test").as_str(), "src/test");
        assert_eq!(LpPath::from("./src/test").as_str(), "src/test");
        assert_eq!(LpPath::from(".").as_str(), "/");
        assert_eq!(LpPath::from("").as_str(), "/");
    }

    #[test]
    fn test_is_absolute() {
        assert!(LpPath::from("/src/test").is_absolute());
        assert!(!LpPath::from("src/test").is_absolute());
        assert!(LpPath::from("/").is_absolute());
    }

    #[test]
    fn test_is_relative() {
        assert!(!LpPath::from("/src/test").is_relative());
        assert!(LpPath::from("src/test").is_relative());
        assert!(!LpPath::from("/").is_relative());
    }

    #[test]
    fn test_parent() {
        assert_eq!(
            LpPath::from("/src/test").parent(),
            Some(LpPath::from("/src"))
        );
        assert_eq!(LpPath::from("/src").parent(), Some(LpPath::from("/")));
        assert_eq!(LpPath::from("/").parent(), None);
        assert_eq!(LpPath::from("src/test").parent(), Some(LpPath::from("src")));
        assert_eq!(LpPath::from("test").parent(), None);
    }

    #[test]
    fn test_file_name() {
        assert_eq!(LpPath::from("/src/test.txt").file_name(), Some("test.txt"));
        assert_eq!(LpPath::from("/src/test").file_name(), Some("test"));
        assert_eq!(LpPath::from("/").file_name(), None);
        assert_eq!(LpPath::from("test.txt").file_name(), Some("test.txt"));
    }

    #[test]
    fn test_file_stem() {
        assert_eq!(LpPath::from("/src/test.txt").file_stem(), Some("test"));
        assert_eq!(
            LpPath::from("/src/test.tar.gz").file_stem(),
            Some("test.tar")
        );
        assert_eq!(LpPath::from("/src/test").file_stem(), Some("test"));
        assert_eq!(LpPath::from("/src/.hidden").file_stem(), None);
        assert_eq!(LpPath::from("/").file_stem(), None);
    }

    #[test]
    fn test_extension() {
        assert_eq!(LpPath::from("/src/test.txt").extension(), Some("txt"));
        assert_eq!(LpPath::from("/src/test.tar.gz").extension(), Some("gz"));
        assert_eq!(LpPath::from("/src/test").extension(), None);
        assert_eq!(LpPath::from("/src/test.").extension(), None);
        assert_eq!(LpPath::from("/").extension(), None);
    }

    #[test]
    fn test_join() {
        assert_eq!(LpPath::from("/src").join("test").as_str(), "/src/test");
        assert_eq!(LpPath::from("/src").join("/test").as_str(), "/test");
        assert_eq!(LpPath::from("/src/a").join("../b").as_str(), "/src/a/../b");
        assert_eq!(LpPath::from("/").join("test").as_str(), "/test");
        assert_eq!(LpPath::from("src").join("test").as_str(), "src/test");
    }

    #[test]
    fn test_join_relative() {
        assert_eq!(
            LpPath::from("/src/a").join_relative("../b"),
            Some(LpPath::from("/src/b"))
        );
        assert_eq!(LpPath::from("/src").join_relative("../../root"), None);
        assert_eq!(
            LpPath::from("/src").join_relative("./test"),
            Some(LpPath::from("/src/test"))
        );
        assert_eq!(
            LpPath::from("src/a").join_relative("../b"),
            Some(LpPath::from("src/b"))
        );
    }

    #[test]
    fn test_strip_prefix() {
        assert_eq!(
            LpPath::from("/projects/my-project/src").strip_prefix("/projects/my-project"),
            Some(LpPath::from("/src"))
        );
        assert_eq!(LpPath::from("/src").strip_prefix("/projects"), None);
        assert_eq!(
            LpPath::from("/projects/my-project").strip_prefix("/projects/my-project"),
            Some(LpPath::from("/"))
        );
    }

    #[test]
    fn test_starts_with() {
        assert!(LpPath::from("/etc/passwd").starts_with("/etc"));
        assert!(LpPath::from("/etc/passwd").starts_with("/etc/"));
        assert!(!LpPath::from("/etc/passwd").starts_with("/usr"));
        assert!(!LpPath::from("/etc/foo.rs").starts_with("/etc/foo"));
    }

    #[test]
    fn test_ends_with() {
        assert!(LpPath::from("/etc/resolv.conf").ends_with("resolv.conf"));
        assert!(LpPath::from("/etc/resolv.conf").ends_with("etc/resolv.conf"));
        assert!(LpPath::from("/etc/resolv.conf").ends_with("/etc/resolv.conf"));
        // /resolv.conf normalized is absolute, but we compare components
        // So /etc/resolv.conf ends with resolv.conf component
        assert!(LpPath::from("/etc/resolv.conf").ends_with("/resolv.conf"));
        assert!(!LpPath::from("/etc/resolv.conf").ends_with("conf"));
    }

    #[test]
    fn test_components() {
        let path1 = LpPath::from("/src/test");
        let components: Vec<&str> = path1.components().collect();
        assert_eq!(components, Vec::from(["src", "test"]));

        let path2 = LpPath::from("src/test");
        let components: Vec<&str> = path2.components().collect();
        assert_eq!(components, Vec::from(["src", "test"]));

        let path3 = LpPath::from("/");
        let components: Vec<&str> = path3.components().collect();
        assert_eq!(components, Vec::<&str>::new());
    }
}
