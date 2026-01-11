use crate::nodes::output::state::OutputState;
use crate::nodes::shader::state::ShaderState;
use crate::nodes::texture::state::TextureState;

pub enum NodeState {
    Output(OutputState),
    Shader(ShaderState),
    Texture(TextureState),
}
