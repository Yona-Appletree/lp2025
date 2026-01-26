//! LPFX function codegen module

pub mod errors;
pub mod parse;
pub mod validate;
pub mod generate;

pub use errors::{LpfxCodegenError, Variant};
