use alloc::string::String;

/// Shader node state - runtime values
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderState {
    /// Actual GLSL code loaded from file
    pub glsl_code: String,
    /// Compilation/runtime errors
    pub error: Option<String>,
}
