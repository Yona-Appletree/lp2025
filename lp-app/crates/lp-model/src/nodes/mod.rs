pub mod kind;
pub mod handle;
pub mod specifier;

// Node type modules
pub mod texture;
pub mod shader;
pub mod output;
pub mod fixture;

pub use kind::NodeKind;
pub use handle::NodeHandle;
pub use specifier::NodeSpecifier;

/// Node config trait - all node configs implement this
pub trait NodeConfig: core::fmt::Debug {
    fn kind(&self) -> NodeKind;
}
