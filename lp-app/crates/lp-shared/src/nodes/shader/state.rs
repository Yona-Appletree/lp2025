use alloc::string::String;

pub struct ShaderState {
    glsl: String,
    error: Option<String>,
}
