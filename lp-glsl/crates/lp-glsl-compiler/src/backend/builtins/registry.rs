//! This file is AUTO-GENERATED. Do not edit manually.
//!
//! To regenerate this file, run:
//!     cargo run --bin lp-builtin-gen --manifest-path lp-glsl/apps/lp-builtin-gen/Cargo.toml
//!
//! Or use the build script:
//!     scripts/build-builtins.sh

//! Builtin function registry implementation.
//!
//! Provides enum-based registry for builtin functions with support for both
//! JIT (function pointer) and emulator (ELF symbol) linking.

use crate::error::{ErrorCode, GlslError};
use cranelift_codegen::ir::{AbiParam, Signature, types};
use cranelift_codegen::isa::CallConv;
use cranelift_module::{Linkage, Module};

/// Enum identifying builtin functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinId {
    Fixed32Acos,
    Fixed32Acosh,
    Fixed32Asin,
    Fixed32Asinh,
    Fixed32Atan,
    Fixed32Atan2,
    Fixed32Atanh,
    Fixed32Cos,
    Fixed32Cosh,
    Fixed32Div,
    Fixed32Exp,
    Fixed32Exp2,
    Fixed32Fma,
    Fixed32Inversesqrt,
    Fixed32Ldexp,
    Fixed32Log,
    Fixed32Log2,
    Fixed32Mod,
    Fixed32Mul,
    Fixed32Pow,
    Fixed32Round,
    Fixed32Roundeven,
    Fixed32Sin,
    Fixed32Sinh,
    Fixed32Sqrt,
    Fixed32Tan,
    Fixed32Tanh,
}

impl BuiltinId {
    /// Get the symbol name for this builtin function.
    pub fn name(&self) -> &'static str {
        match self {
            BuiltinId::Fixed32Acos => "__lp_fixed32_acos",
            BuiltinId::Fixed32Acosh => "__lp_fixed32_acosh",
            BuiltinId::Fixed32Asin => "__lp_fixed32_asin",
            BuiltinId::Fixed32Asinh => "__lp_fixed32_asinh",
            BuiltinId::Fixed32Atan => "__lp_fixed32_atan",
            BuiltinId::Fixed32Atan2 => "__lp_fixed32_atan2",
            BuiltinId::Fixed32Atanh => "__lp_fixed32_atanh",
            BuiltinId::Fixed32Cos => "__lp_fixed32_cos",
            BuiltinId::Fixed32Cosh => "__lp_fixed32_cosh",
            BuiltinId::Fixed32Div => "__lp_fixed32_div",
            BuiltinId::Fixed32Exp => "__lp_fixed32_exp",
            BuiltinId::Fixed32Exp2 => "__lp_fixed32_exp2",
            BuiltinId::Fixed32Fma => "__lp_fixed32_fma",
            BuiltinId::Fixed32Inversesqrt => "__lp_fixed32_inversesqrt",
            BuiltinId::Fixed32Ldexp => "__lp_fixed32_ldexp",
            BuiltinId::Fixed32Log => "__lp_fixed32_log",
            BuiltinId::Fixed32Log2 => "__lp_fixed32_log2",
            BuiltinId::Fixed32Mod => "__lp_fixed32_mod",
            BuiltinId::Fixed32Mul => "__lp_fixed32_mul",
            BuiltinId::Fixed32Pow => "__lp_fixed32_pow",
            BuiltinId::Fixed32Round => "__lp_fixed32_round",
            BuiltinId::Fixed32Roundeven => "__lp_fixed32_roundeven",
            BuiltinId::Fixed32Sin => "__lp_fixed32_sin",
            BuiltinId::Fixed32Sinh => "__lp_fixed32_sinh",
            BuiltinId::Fixed32Sqrt => "__lp_fixed32_sqrt",
            BuiltinId::Fixed32Tan => "__lp_fixed32_tan",
            BuiltinId::Fixed32Tanh => "__lp_fixed32_tanh",
        }
    }

    /// Get the Cranelift signature for this builtin function.
    pub fn signature(&self) -> Signature {
        let mut sig = Signature::new(CallConv::SystemV);
        match self {
            BuiltinId::Fixed32Fma => {
                // (i32, i32, i32) -> i32
                sig.params.push(AbiParam::new(types::I32));
                sig.params.push(AbiParam::new(types::I32));
                sig.params.push(AbiParam::new(types::I32));
                sig.returns.push(AbiParam::new(types::I32));
            }
            BuiltinId::Fixed32Atan2 | BuiltinId::Fixed32Div | BuiltinId::Fixed32Ldexp | BuiltinId::Fixed32Mod | BuiltinId::Fixed32Mul | BuiltinId::Fixed32Pow => {
                // (i32, i32) -> i32
                sig.params.push(AbiParam::new(types::I32));
                sig.params.push(AbiParam::new(types::I32));
                sig.returns.push(AbiParam::new(types::I32));
            }
            BuiltinId::Fixed32Acos | BuiltinId::Fixed32Acosh | BuiltinId::Fixed32Asin | BuiltinId::Fixed32Asinh | BuiltinId::Fixed32Atan | BuiltinId::Fixed32Atanh | BuiltinId::Fixed32Cos | BuiltinId::Fixed32Cosh | BuiltinId::Fixed32Exp | BuiltinId::Fixed32Exp2 | BuiltinId::Fixed32Inversesqrt | BuiltinId::Fixed32Log | BuiltinId::Fixed32Log2 | BuiltinId::Fixed32Round | BuiltinId::Fixed32Roundeven | BuiltinId::Fixed32Sin | BuiltinId::Fixed32Sinh | BuiltinId::Fixed32Sqrt | BuiltinId::Fixed32Tan | BuiltinId::Fixed32Tanh => {
                // (i32) -> i32
                sig.params.push(AbiParam::new(types::I32));
                sig.returns.push(AbiParam::new(types::I32));
            }
        }
        sig
    }

    /// Get all builtin IDs.
    pub fn all() -> &'static [BuiltinId] {
        &[
            BuiltinId::Fixed32Acos,
            BuiltinId::Fixed32Acosh,
            BuiltinId::Fixed32Asin,
            BuiltinId::Fixed32Asinh,
            BuiltinId::Fixed32Atan,
            BuiltinId::Fixed32Atan2,
            BuiltinId::Fixed32Atanh,
            BuiltinId::Fixed32Cos,
            BuiltinId::Fixed32Cosh,
            BuiltinId::Fixed32Div,
            BuiltinId::Fixed32Exp,
            BuiltinId::Fixed32Exp2,
            BuiltinId::Fixed32Fma,
            BuiltinId::Fixed32Inversesqrt,
            BuiltinId::Fixed32Ldexp,
            BuiltinId::Fixed32Log,
            BuiltinId::Fixed32Log2,
            BuiltinId::Fixed32Mod,
            BuiltinId::Fixed32Mul,
            BuiltinId::Fixed32Pow,
            BuiltinId::Fixed32Round,
            BuiltinId::Fixed32Roundeven,
            BuiltinId::Fixed32Sin,
            BuiltinId::Fixed32Sinh,
            BuiltinId::Fixed32Sqrt,
            BuiltinId::Fixed32Tan,
            BuiltinId::Fixed32Tanh,
        ]
    }
}

/// Get function pointer for a builtin (JIT mode only).
///
/// Returns the function pointer that can be registered with JITModule.
pub fn get_function_pointer(builtin: BuiltinId) -> *const u8 {
    use lp_builtins::builtins::fixed32;
    match builtin {
        BuiltinId::Fixed32Acos => fixed32::__lp_fixed32_acos as *const u8,
        BuiltinId::Fixed32Acosh => fixed32::__lp_fixed32_acosh as *const u8,
        BuiltinId::Fixed32Asin => fixed32::__lp_fixed32_asin as *const u8,
        BuiltinId::Fixed32Asinh => fixed32::__lp_fixed32_asinh as *const u8,
        BuiltinId::Fixed32Atan => fixed32::__lp_fixed32_atan as *const u8,
        BuiltinId::Fixed32Atan2 => fixed32::__lp_fixed32_atan2 as *const u8,
        BuiltinId::Fixed32Atanh => fixed32::__lp_fixed32_atanh as *const u8,
        BuiltinId::Fixed32Cos => fixed32::__lp_fixed32_cos as *const u8,
        BuiltinId::Fixed32Cosh => fixed32::__lp_fixed32_cosh as *const u8,
        BuiltinId::Fixed32Div => fixed32::__lp_fixed32_div as *const u8,
        BuiltinId::Fixed32Exp => fixed32::__lp_fixed32_exp as *const u8,
        BuiltinId::Fixed32Exp2 => fixed32::__lp_fixed32_exp2 as *const u8,
        BuiltinId::Fixed32Fma => fixed32::__lp_fixed32_fma as *const u8,
        BuiltinId::Fixed32Inversesqrt => fixed32::__lp_fixed32_inversesqrt as *const u8,
        BuiltinId::Fixed32Ldexp => fixed32::__lp_fixed32_ldexp as *const u8,
        BuiltinId::Fixed32Log => fixed32::__lp_fixed32_log as *const u8,
        BuiltinId::Fixed32Log2 => fixed32::__lp_fixed32_log2 as *const u8,
        BuiltinId::Fixed32Mod => fixed32::__lp_fixed32_mod as *const u8,
        BuiltinId::Fixed32Mul => fixed32::__lp_fixed32_mul as *const u8,
        BuiltinId::Fixed32Pow => fixed32::__lp_fixed32_pow as *const u8,
        BuiltinId::Fixed32Round => fixed32::__lp_fixed32_round as *const u8,
        BuiltinId::Fixed32Roundeven => fixed32::__lp_fixed32_roundeven as *const u8,
        BuiltinId::Fixed32Sin => fixed32::__lp_fixed32_sin as *const u8,
        BuiltinId::Fixed32Sinh => fixed32::__lp_fixed32_sinh as *const u8,
        BuiltinId::Fixed32Sqrt => fixed32::__lp_fixed32_sqrt as *const u8,
        BuiltinId::Fixed32Tan => fixed32::__lp_fixed32_tan as *const u8,
        BuiltinId::Fixed32Tanh => fixed32::__lp_fixed32_tanh as *const u8,
    }
}

/// Declare builtin functions as external symbols.
///
/// This is the same for both JIT and emulator - they both use Linkage::Import.
/// The difference is only in how they're linked:
/// - JIT: Function pointers are registered via symbol_lookup_fn during module creation
/// - Emulator: Symbols are resolved by the linker when linking the static library
pub fn declare_builtins<M: Module>(module: &mut M) -> Result<(), GlslError> {
    for builtin in BuiltinId::all() {
        let name = builtin.name();
        let sig = builtin.signature();

        module
            .declare_function(name, Linkage::Import, &sig)
            .map_err(|e| {
                GlslError::new(
                    ErrorCode::E0400,
                    format!("Failed to declare builtin '{}': {}", name, e),
                )
            })?;
    }

    Ok(())
}

/// Declare and link builtin functions for JIT mode.
///
/// This declares all builtins as external functions. The function pointers
/// are registered via a symbol lookup function that's added during module creation.
pub fn declare_for_jit<M: Module>(module: &mut M) -> Result<(), GlslError> {
    declare_builtins(module)
}

/// Declare builtin functions as external symbols for emulator mode.
///
/// This declares all builtins as external symbols (Linkage::Import) that will
/// be resolved by the linker when linking the static library.
pub fn declare_for_emulator<M: Module>(module: &mut M) -> Result<(), GlslError> {
    declare_builtins(module)
}
