//! Central filesystem abstraction module

pub mod filesystem;
pub mod memory;

pub use filesystem::Filesystem;
pub use memory::InMemoryFilesystem;
