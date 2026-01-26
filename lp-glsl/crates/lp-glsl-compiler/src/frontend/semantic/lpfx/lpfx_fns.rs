//! LPFX function registry
//!
//! This module contains the array of all LPFX functions.
//! This file is AUTO-GENERATED. Do not edit manually.
//!
//! To regenerate this file, run:
//!     cargo run --bin lp-builtin-gen --manifest-path lp-glsl/apps/lp-builtin-gen/Cargo.toml
//!
//! Or use the build script:
//!     scripts/build-builtins.sh

use super::lpfx_fn::{LpfxFn, LpfxFnImpl};
use crate::backend::builtins::registry::BuiltinId;
use crate::semantic::functions::{FunctionSignature, ParamQualifier, Parameter};
use crate::semantic::types::Type;
use alloc::{boxed::Box, string::String, vec, vec::Vec};

/// Registry of all LPFX functions
///
/// This is the single source of truth for all LPFX function definitions.
/// Functions are looked up by name from this array.
///
/// Returns a static reference to avoid recreating the Vec on every call.
pub fn lpfx_fns() -> &'static [LpfxFn] {
    #[cfg(feature = "std")]
    {
        static FUNCTIONS: std::sync::OnceLock<&'static [LpfxFn]> = std::sync::OnceLock::new();
        *FUNCTIONS.get_or_init(|| init_functions())
    }
    #[cfg(not(feature = "std"))]
    {
        // In no_std, use a static initialized on first access
        // This is safe because the data is immutable after initialization
        static mut FUNCTIONS: Option<&'static [LpfxFn]> = None;
        unsafe {
            let ptr = core::ptr::addr_of_mut!(FUNCTIONS);
            if (*ptr).is_none() {
                *ptr = Some(init_functions());
            }
            (*ptr).unwrap_unchecked()
        }
    }
}

fn init_functions() -> &'static [LpfxFn] {
    let vec: Vec<LpfxFn> = vec![
        LpfxFn {
            glsl_sig: FunctionSignature {
                name: String::from("lpfx_hash1"),
                return_type: Type::UInt,
                parameters: vec![
                    Parameter {
                        name: String::from("x"),
                        ty: Type::UInt,
                        qualifier: ParamQualifier::In,
                    },
                    Parameter {
                        name: String::from("seed"),
                        ty: Type::UInt,
                        qualifier: ParamQualifier::In,
                    },
                ],
            },
            impls: LpfxFnImpl::NonDecimal(BuiltinId::LpfxHash1),
        },
        LpfxFn {
            glsl_sig: FunctionSignature {
                name: String::from("lpfx_hash3"),
                return_type: Type::UInt,
                parameters: vec![
                    Parameter {
                        name: String::from("x"),
                        ty: Type::UInt,
                        qualifier: ParamQualifier::In,
                    },
                    Parameter {
                        name: String::from("y"),
                        ty: Type::UInt,
                        qualifier: ParamQualifier::In,
                    },
                    Parameter {
                        name: String::from("z"),
                        ty: Type::UInt,
                        qualifier: ParamQualifier::In,
                    },
                    Parameter {
                        name: String::from("seed"),
                        ty: Type::UInt,
                        qualifier: ParamQualifier::In,
                    },
                ],
            },
            impls: LpfxFnImpl::NonDecimal(BuiltinId::LpfxHash3),
        },
        LpfxFn {
            glsl_sig: FunctionSignature {
                name: String::from("lpfx_simplex2"),
                return_type: Type::Float,
                parameters: vec![
                    Parameter {
                        name: String::from("p"),
                        ty: Type::Vec2,
                        qualifier: ParamQualifier::In,
                    },
                    Parameter {
                        name: String::from("seed"),
                        ty: Type::UInt,
                        qualifier: ParamQualifier::In,
                    },
                ],
            },
            impls: LpfxFnImpl::Decimal {
                float_impl: BuiltinId::LpfxSimplex2F32,
                q32_impl: BuiltinId::LpfxSimplex2Q32,
            },
        },
        LpfxFn {
            glsl_sig: FunctionSignature {
                name: String::from("lpfx_hash2"),
                return_type: Type::UInt,
                parameters: vec![
                    Parameter {
                        name: String::from("x"),
                        ty: Type::UInt,
                        qualifier: ParamQualifier::In,
                    },
                    Parameter {
                        name: String::from("y"),
                        ty: Type::UInt,
                        qualifier: ParamQualifier::In,
                    },
                    Parameter {
                        name: String::from("seed"),
                        ty: Type::UInt,
                        qualifier: ParamQualifier::In,
                    },
                ],
            },
            impls: LpfxFnImpl::NonDecimal(BuiltinId::LpfxHash2),
        },
        LpfxFn {
            glsl_sig: FunctionSignature {
                name: String::from("lpfx_simplex3"),
                return_type: Type::Float,
                parameters: vec![
                    Parameter {
                        name: String::from("p"),
                        ty: Type::Vec3,
                        qualifier: ParamQualifier::In,
                    },
                    Parameter {
                        name: String::from("seed"),
                        ty: Type::UInt,
                        qualifier: ParamQualifier::In,
                    },
                ],
            },
            impls: LpfxFnImpl::Decimal {
                float_impl: BuiltinId::LpfxSimplex3F32,
                q32_impl: BuiltinId::LpfxSimplex3Q32,
            },
        },
        LpfxFn {
            glsl_sig: FunctionSignature {
                name: String::from("lpfx_simplex1"),
                return_type: Type::Float,
                parameters: vec![
                    Parameter {
                        name: String::from("x"),
                        ty: Type::Float,
                        qualifier: ParamQualifier::In,
                    },
                    Parameter {
                        name: String::from("seed"),
                        ty: Type::UInt,
                        qualifier: ParamQualifier::In,
                    },
                ],
            },
            impls: LpfxFnImpl::Decimal {
                float_impl: BuiltinId::LpfxSimplex1F32,
                q32_impl: BuiltinId::LpfxSimplex1Q32,
            },
        },
    ];
    Box::leak(vec.into_boxed_slice())
}
