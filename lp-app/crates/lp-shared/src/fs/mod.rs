//! Central filesystem abstraction module

pub mod fs_event;
pub mod lp_fs;
pub mod lp_fs_mem;
#[cfg(feature = "std")]
pub mod lp_fs_std;

pub use lp_fs::LpFs;
pub use lp_fs_mem::{LpFsMemory, LpFsMemoryShared};

#[cfg(feature = "std")]
pub use lp_fs_std::LpFsStd;
