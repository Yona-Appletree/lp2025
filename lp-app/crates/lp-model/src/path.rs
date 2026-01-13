use alloc::string::{String, ToString};

/// Light Player path - absolute paths from project root
/// 
/// Currently supports absolute paths only. Designed to support relative paths
/// later when nodes become nestable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LpPath(pub String);

impl LpPath {
    pub fn new(path: String) -> Self {
        Self(path)
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for LpPath {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for LpPath {
    fn from(s: &str) -> Self {
        Self(s.to_string())
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
}
