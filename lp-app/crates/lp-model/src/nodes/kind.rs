/// Node kind - matches directory suffixes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Texture,
    Shader,
    Output,
    Fixture,
}
