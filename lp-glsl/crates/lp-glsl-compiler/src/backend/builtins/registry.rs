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

#[cfg(not(feature = "std"))]
use alloc::format;

/// Enum identifying builtin functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinId {
    LpFixed32Acos,
    LpFixed32Acosh,
    LpFixed32Add,
    LpFixed32Asin,
    LpFixed32Asinh,
    LpFixed32Atan,
    LpFixed32Atan2,
    LpFixed32Atanh,
    LpFixed32Cos,
    LpFixed32Cosh,
    LpFixed32Div,
    LpFixed32Exp,
    LpFixed32Exp2,
    LpFixed32Fma,
    LpFixed32Inversesqrt,
    LpFixed32Ldexp,
    LpFixed32Log,
    LpFixed32Log2,
    LpFixed32Mod,
    LpFixed32Mul,
    LpFixed32Pow,
    LpFixed32Round,
    LpFixed32Roundeven,
    LpFixed32Sin,
    LpFixed32Sinh,
    LpFixed32Sqrt,
    LpFixed32Sub,
    LpFixed32Tan,
    LpFixed32Tanh,
    LpfxHash1,
    LpfxHash2,
    LpfxHash3,
    LpfxSimplex1Float,
    LpfxSimplex1Q32,
    LpfxSimplex2Float,
    LpfxSimplex2Q32,
    LpfxSimplex3Float,
    LpfxSimplex3Q32,
}

impl BuiltinId {
    /// Get the symbol name for this builtin function.
    pub fn name(&self) -> &'static str {
        match self {
            BuiltinId::LpFixed32Acos => "__lp_fixed32_acos",
            BuiltinId::LpFixed32Acosh => "__lp_fixed32_acosh",
            BuiltinId::LpFixed32Add => "__lp_fixed32_add",
            BuiltinId::LpFixed32Asin => "__lp_fixed32_asin",
            BuiltinId::LpFixed32Asinh => "__lp_fixed32_asinh",
            BuiltinId::LpFixed32Atan => "__lp_fixed32_atan",
            BuiltinId::LpFixed32Atan2 => "__lp_fixed32_atan2",
            BuiltinId::LpFixed32Atanh => "__lp_fixed32_atanh",
            BuiltinId::LpFixed32Cos => "__lp_fixed32_cos",
            BuiltinId::LpFixed32Cosh => "__lp_fixed32_cosh",
            BuiltinId::LpFixed32Div => "__lp_fixed32_div",
            BuiltinId::LpFixed32Exp => "__lp_fixed32_exp",
            BuiltinId::LpFixed32Exp2 => "__lp_fixed32_exp2",
            BuiltinId::LpFixed32Fma => "__lp_fixed32_fma",
            BuiltinId::LpFixed32Inversesqrt => "__lp_fixed32_inversesqrt",
            BuiltinId::LpFixed32Ldexp => "__lp_fixed32_ldexp",
            BuiltinId::LpFixed32Log => "__lp_fixed32_log",
            BuiltinId::LpFixed32Log2 => "__lp_fixed32_log2",
            BuiltinId::LpFixed32Mod => "__lp_fixed32_mod",
            BuiltinId::LpFixed32Mul => "__lp_fixed32_mul",
            BuiltinId::LpFixed32Pow => "__lp_fixed32_pow",
            BuiltinId::LpFixed32Round => "__lp_fixed32_round",
            BuiltinId::LpFixed32Roundeven => "__lp_fixed32_roundeven",
            BuiltinId::LpFixed32Sin => "__lp_fixed32_sin",
            BuiltinId::LpFixed32Sinh => "__lp_fixed32_sinh",
            BuiltinId::LpFixed32Sqrt => "__lp_fixed32_sqrt",
            BuiltinId::LpFixed32Sub => "__lp_fixed32_sub",
            BuiltinId::LpFixed32Tan => "__lp_fixed32_tan",
            BuiltinId::LpFixed32Tanh => "__lp_fixed32_tanh",
            BuiltinId::LpfxHash1 => "__lpfx_hash_1",
            BuiltinId::LpfxHash2 => "__lpfx_hash_2",
            BuiltinId::LpfxHash3 => "__lpfx_hash_3",
            BuiltinId::LpfxSimplex1Float => "__lpfx_simplex1_float",
            BuiltinId::LpfxSimplex1Q32 => "__lpfx_simplex1_q32",
            BuiltinId::LpfxSimplex2Float => "__lpfx_simplex2_float",
            BuiltinId::LpfxSimplex2Q32 => "__lpfx_simplex2_q32",
            BuiltinId::LpfxSimplex3Float => "__lpfx_simplex3_float",
            BuiltinId::LpfxSimplex3Q32 => "__lpfx_simplex3_q32",
        }
    }

    /// Get the Cranelift signature for this builtin function.
    pub fn signature(&self) -> Signature {
        let mut sig = Signature::new(CallConv::SystemV);
        match self {
            BuiltinId::LpfxSimplex3Float => {
                // (f32, f32, f32, u32) -> f32 (vec3 expanded to 3 f32s)
                sig.params.push(AbiParam::new(types::F32));
                sig.params.push(AbiParam::new(types::F32));
                sig.params.push(AbiParam::new(types::F32));
                sig.params.push(AbiParam::new(types::I32)); // u32 maps to i32 in Cranelift
                sig.returns.push(AbiParam::new(types::F32));
            }
            BuiltinId::LpfxHash3 | BuiltinId::LpfxSimplex3Q32 => {
                // (i32, i32, i32, i32) -> i32
                sig.params.push(AbiParam::new(types::I32));
                sig.params.push(AbiParam::new(types::I32));
                sig.params.push(AbiParam::new(types::I32));
                sig.params.push(AbiParam::new(types::I32));
                sig.returns.push(AbiParam::new(types::I32));
            }
            BuiltinId::LpfxSimplex2Float => {
                // (f32, f32, u32) -> f32 (vec2 expanded to 2 f32s)
                sig.params.push(AbiParam::new(types::F32));
                sig.params.push(AbiParam::new(types::F32));
                sig.params.push(AbiParam::new(types::I32)); // u32 maps to i32 in Cranelift
                sig.returns.push(AbiParam::new(types::F32));
            }
            BuiltinId::LpFixed32Fma | BuiltinId::LpfxHash2 | BuiltinId::LpfxSimplex2Q32 => {
                // (i32, i32, i32) -> i32
                sig.params.push(AbiParam::new(types::I32));
                sig.params.push(AbiParam::new(types::I32));
                sig.params.push(AbiParam::new(types::I32));
                sig.returns.push(AbiParam::new(types::I32));
            }
            BuiltinId::LpfxSimplex1Float => {
                // (f32, u32) -> f32
                sig.params.push(AbiParam::new(types::F32));
                sig.params.push(AbiParam::new(types::I32)); // u32 maps to i32 in Cranelift
                sig.returns.push(AbiParam::new(types::F32));
            }
            BuiltinId::LpFixed32Add
            | BuiltinId::LpFixed32Atan2
            | BuiltinId::LpFixed32Div
            | BuiltinId::LpFixed32Ldexp
            | BuiltinId::LpFixed32Mod
            | BuiltinId::LpFixed32Mul
            | BuiltinId::LpFixed32Pow
            | BuiltinId::LpFixed32Sub
            | BuiltinId::LpfxHash1
            | BuiltinId::LpfxSimplex1Q32 => {
                // (i32, i32) -> i32
                sig.params.push(AbiParam::new(types::I32));
                sig.params.push(AbiParam::new(types::I32));
                sig.returns.push(AbiParam::new(types::I32));
            }
            BuiltinId::LpFixed32Acos
            | BuiltinId::LpFixed32Acosh
            | BuiltinId::LpFixed32Asin
            | BuiltinId::LpFixed32Asinh
            | BuiltinId::LpFixed32Atan
            | BuiltinId::LpFixed32Atanh
            | BuiltinId::LpFixed32Cos
            | BuiltinId::LpFixed32Cosh
            | BuiltinId::LpFixed32Exp
            | BuiltinId::LpFixed32Exp2
            | BuiltinId::LpFixed32Inversesqrt
            | BuiltinId::LpFixed32Log
            | BuiltinId::LpFixed32Log2
            | BuiltinId::LpFixed32Round
            | BuiltinId::LpFixed32Roundeven
            | BuiltinId::LpFixed32Sin
            | BuiltinId::LpFixed32Sinh
            | BuiltinId::LpFixed32Sqrt
            | BuiltinId::LpFixed32Tan
            | BuiltinId::LpFixed32Tanh => {
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
            BuiltinId::LpFixed32Acos,
            BuiltinId::LpFixed32Acosh,
            BuiltinId::LpFixed32Add,
            BuiltinId::LpFixed32Asin,
            BuiltinId::LpFixed32Asinh,
            BuiltinId::LpFixed32Atan,
            BuiltinId::LpFixed32Atan2,
            BuiltinId::LpFixed32Atanh,
            BuiltinId::LpFixed32Cos,
            BuiltinId::LpFixed32Cosh,
            BuiltinId::LpFixed32Div,
            BuiltinId::LpFixed32Exp,
            BuiltinId::LpFixed32Exp2,
            BuiltinId::LpFixed32Fma,
            BuiltinId::LpFixed32Inversesqrt,
            BuiltinId::LpFixed32Ldexp,
            BuiltinId::LpFixed32Log,
            BuiltinId::LpFixed32Log2,
            BuiltinId::LpFixed32Mod,
            BuiltinId::LpFixed32Mul,
            BuiltinId::LpFixed32Pow,
            BuiltinId::LpFixed32Round,
            BuiltinId::LpFixed32Roundeven,
            BuiltinId::LpFixed32Sin,
            BuiltinId::LpFixed32Sinh,
            BuiltinId::LpFixed32Sqrt,
            BuiltinId::LpFixed32Sub,
            BuiltinId::LpFixed32Tan,
            BuiltinId::LpFixed32Tanh,
            BuiltinId::LpfxHash1,
            BuiltinId::LpfxHash2,
            BuiltinId::LpfxHash3,
            BuiltinId::LpfxSimplex1Float,
            BuiltinId::LpfxSimplex1Q32,
            BuiltinId::LpfxSimplex2Float,
            BuiltinId::LpfxSimplex2Q32,
            BuiltinId::LpfxSimplex3Float,
            BuiltinId::LpfxSimplex3Q32,
        ]
    }
}

/// Get function pointer for a builtin (JIT mode only).
///
/// Returns the function pointer that can be registered with JITModule.
pub fn get_function_pointer(builtin: BuiltinId) -> *const u8 {
    use lp_builtins::builtins::{
        fixed32,
        lpfx::{hash, simplex},
    };
    match builtin {
        BuiltinId::LpFixed32Acos => fixed32::__lp_fixed32_acos as *const u8,
        BuiltinId::LpFixed32Acosh => fixed32::__lp_fixed32_acosh as *const u8,
        BuiltinId::LpFixed32Add => fixed32::__lp_fixed32_add as *const u8,
        BuiltinId::LpFixed32Asin => fixed32::__lp_fixed32_asin as *const u8,
        BuiltinId::LpFixed32Asinh => fixed32::__lp_fixed32_asinh as *const u8,
        BuiltinId::LpFixed32Atan => fixed32::__lp_fixed32_atan as *const u8,
        BuiltinId::LpFixed32Atan2 => fixed32::__lp_fixed32_atan2 as *const u8,
        BuiltinId::LpFixed32Atanh => fixed32::__lp_fixed32_atanh as *const u8,
        BuiltinId::LpFixed32Cos => fixed32::__lp_fixed32_cos as *const u8,
        BuiltinId::LpFixed32Cosh => fixed32::__lp_fixed32_cosh as *const u8,
        BuiltinId::LpFixed32Div => fixed32::__lp_fixed32_div as *const u8,
        BuiltinId::LpFixed32Exp => fixed32::__lp_fixed32_exp as *const u8,
        BuiltinId::LpFixed32Exp2 => fixed32::__lp_fixed32_exp2 as *const u8,
        BuiltinId::LpFixed32Fma => fixed32::__lp_fixed32_fma as *const u8,
        BuiltinId::LpFixed32Inversesqrt => fixed32::__lp_fixed32_inversesqrt as *const u8,
        BuiltinId::LpFixed32Ldexp => fixed32::__lp_fixed32_ldexp as *const u8,
        BuiltinId::LpFixed32Log => fixed32::__lp_fixed32_log as *const u8,
        BuiltinId::LpFixed32Log2 => fixed32::__lp_fixed32_log2 as *const u8,
        BuiltinId::LpFixed32Mod => fixed32::__lp_fixed32_mod as *const u8,
        BuiltinId::LpFixed32Mul => fixed32::__lp_fixed32_mul as *const u8,
        BuiltinId::LpFixed32Pow => fixed32::__lp_fixed32_pow as *const u8,
        BuiltinId::LpFixed32Round => fixed32::__lp_fixed32_round as *const u8,
        BuiltinId::LpFixed32Roundeven => fixed32::__lp_fixed32_roundeven as *const u8,
        BuiltinId::LpFixed32Sin => fixed32::__lp_fixed32_sin as *const u8,
        BuiltinId::LpFixed32Sinh => fixed32::__lp_fixed32_sinh as *const u8,
        BuiltinId::LpFixed32Sqrt => fixed32::__lp_fixed32_sqrt as *const u8,
        BuiltinId::LpFixed32Sub => fixed32::__lp_fixed32_sub as *const u8,
        BuiltinId::LpFixed32Tan => fixed32::__lp_fixed32_tan as *const u8,
        BuiltinId::LpFixed32Tanh => fixed32::__lp_fixed32_tanh as *const u8,
        BuiltinId::LpfxHash1 => hash::__lpfx_hash_1 as *const u8,
        BuiltinId::LpfxHash2 => hash::__lpfx_hash_2 as *const u8,
        BuiltinId::LpfxHash3 => hash::__lpfx_hash_3 as *const u8,
        BuiltinId::LpfxSimplex1Float => simplex::simplex1_f32::__lpfx_simplex1_f32 as *const u8,
        BuiltinId::LpfxSimplex1Q32 => simplex::simplex1_q32::__lpfx_simplex1_q32 as *const u8,
        BuiltinId::LpfxSimplex2Float => simplex::simplex2_f32::__lpfx_simplex2_f32 as *const u8,
        BuiltinId::LpfxSimplex2Q32 => simplex::simplex2_q32::__lpfx_simplex2_q32 as *const u8,
        BuiltinId::LpfxSimplex3Float => simplex::simplex3_f32::__lpfx_simplex3_f32 as *const u8,
        BuiltinId::LpfxSimplex3Q32 => simplex::simplex3_q32::__lpfx_simplex3_q32 as *const u8,
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
                    format!("Failed to declare builtin '{name}': {e}"),
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
