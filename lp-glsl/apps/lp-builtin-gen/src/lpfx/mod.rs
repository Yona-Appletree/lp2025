//! LPFX function codegen module

pub mod errors;
pub mod generate;
pub mod glsl_parse;
pub mod parse;
pub mod process;
pub mod validate;

pub use errors::{LpfxCodegenError, Variant};
