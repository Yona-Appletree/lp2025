//! Helper functions for converting/calling lpfx functions, handling expanding vec types
//! to scalars expected by the implementation.

use super::lpfx_fn::LpfxFn;
use crate::DecimalFormat;
use crate::semantic::types::Type;
use alloc::vec::Vec;
use cranelift_codegen::ir::{AbiParam, Signature, types};
use cranelift_codegen::isa::CallConv;

/// Expand vector arguments to individual components
///
/// Takes a slice of (value, type) pairs and expands vectors to their components.
/// Returns a flat list of values.
///
/// # Panics
/// Panics if unsupported types are encountered (only Float, Int, UInt, Vec2, Vec3 are supported).
pub fn expand_vector_args(
    param_types: &[Type],
    values: &[cranelift_codegen::ir::Value],
) -> Vec<cranelift_codegen::ir::Value> {
    let mut flat_values = Vec::new();
    let mut value_idx = 0;

    for param_ty in param_types {
        match param_ty {
            Type::Vec2 | Type::IVec2 | Type::UVec2 => {
                // Extract 2 components
                if value_idx + 2 > values.len() {
                    panic!("Not enough values for vec2 parameter");
                }
                flat_values.push(values[value_idx]);
                flat_values.push(values[value_idx + 1]);
                value_idx += 2;
            }
            Type::Vec3 | Type::IVec3 | Type::UVec3 => {
                // Extract 3 components
                if value_idx + 3 > values.len() {
                    panic!("Not enough values for vec3 parameter");
                }
                flat_values.push(values[value_idx]);
                flat_values.push(values[value_idx + 1]);
                flat_values.push(values[value_idx + 2]);
                value_idx += 3;
            }
            Type::Float | Type::Int | Type::UInt => {
                // Scalar - single value
                if value_idx >= values.len() {
                    panic!("Not enough values for scalar parameter");
                }
                flat_values.push(values[value_idx]);
                value_idx += 1;
            }
            _ => {
                panic!("Unsupported parameter type for LPFX function: {param_ty:?}");
            }
        }
    }

    flat_values
}

/// Convert GLSL types to Cranelift types based on decimal format
///
/// For Fixed32 format, all numeric types are converted to i32.
/// Float → i32 (fixed32 representation)
/// UInt → i32 (Cranelift representation)
/// Int → i32
///
/// # Panics
/// Panics if unsupported types are encountered.
pub fn convert_to_cranelift_types(
    param_types: &[Type],
    format: DecimalFormat,
) -> Vec<cranelift_codegen::ir::Type> {
    let mut cranelift_types = Vec::new();

    for param_ty in param_types {
        match param_ty {
            Type::Vec2 | Type::IVec2 | Type::UVec2 => {
                // Vec2 expands to 2 components
                match format {
                    DecimalFormat::Fixed32 => {
                        cranelift_types.push(types::I32);
                        cranelift_types.push(types::I32);
                    }
                    DecimalFormat::Float => {
                        cranelift_types.push(types::F32);
                        cranelift_types.push(types::F32);
                    }
                }
            }
            Type::Vec3 | Type::IVec3 | Type::UVec3 => {
                // Vec3 expands to 3 components
                match format {
                    DecimalFormat::Fixed32 => {
                        cranelift_types.push(types::I32);
                        cranelift_types.push(types::I32);
                        cranelift_types.push(types::I32);
                    }
                    DecimalFormat::Float => {
                        cranelift_types.push(types::F32);
                        cranelift_types.push(types::F32);
                        cranelift_types.push(types::F32);
                    }
                }
            }
            Type::Float => match format {
                DecimalFormat::Fixed32 => cranelift_types.push(types::I32),
                DecimalFormat::Float => cranelift_types.push(types::F32),
            },
            Type::UInt | Type::Int => {
                // UInt and Int both map to I32 in Cranelift
                cranelift_types.push(types::I32);
            }
            _ => {
                panic!("Unsupported parameter type for LPFX function: {param_ty:?}");
            }
        }
    }

    cranelift_types
}

/// Build Cranelift signature dynamically from function signature
///
/// Expands vector parameters and converts types based on decimal format.
/// Returns a Signature ready for use in Cranelift function calls.
pub fn build_call_signature(
    func: &LpfxFn,
    _builtin_id: crate::backend::builtins::registry::BuiltinId,
    format: DecimalFormat,
) -> Signature {
    let mut sig = Signature::new(CallConv::SystemV);

    // Get parameter types from function signature
    let param_types: Vec<Type> = func
        .glsl_sig
        .parameters
        .iter()
        .map(|p| p.ty.clone())
        .collect();

    // Convert to Cranelift types
    let cranelift_param_types = convert_to_cranelift_types(&param_types, format);
    for ty in cranelift_param_types {
        sig.params.push(AbiParam::new(ty));
    }

    // Return type
    match func.glsl_sig.return_type {
        Type::Float => match format {
            DecimalFormat::Fixed32 => sig.returns.push(AbiParam::new(types::I32)),
            DecimalFormat::Float => sig.returns.push(AbiParam::new(types::F32)),
        },
        Type::UInt | Type::Int => {
            sig.returns.push(AbiParam::new(types::I32));
        }
        _ => {
            panic!(
                "Unsupported return type for LPFX function: {:?}",
                func.glsl_sig.return_type
            );
        }
    }

    sig
}
