#![no_std]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod error;
pub mod fs;
pub mod log;

pub use error::FsError;
