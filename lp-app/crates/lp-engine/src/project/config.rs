//! Project configuration structures

use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Project configuration structure
///
/// Contains only top-level project metadata. Nodes are loaded from the filesystem
/// separately via ProjectLoader.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub uid: String,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_serialize_deserialize() {
        let config = ProjectConfig {
            uid: "UID12345".to_string(),
            name: "Test Project".to_string(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ProjectConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.uid, deserialized.uid);
        assert_eq!(config.name, deserialized.name);
    }
}
