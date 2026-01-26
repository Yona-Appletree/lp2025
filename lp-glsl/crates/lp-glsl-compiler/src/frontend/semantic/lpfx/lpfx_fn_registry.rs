//! LPFX Function Registry
//!
//! Provides lookup functions for LPFX functions from the registry.

use super::lpfx_fn::LpfxFn;
use crate::semantic::types::Type;
use alloc::{format, string::String};

/// Check if a function name is an LPFX function
///
/// Returns `true` if the name starts with "lpfx_".
pub fn is_lpfx_fn(name: &str) -> bool {
    name.starts_with("lpfx_")
}

/// Find an LPFX function by its GLSL name
///
/// Returns `None` if the function is not found in the registry.
/// Get cached functions array
fn get_cached_functions() -> &'static [LpfxFn] {
    super::lpfx_fns::lpfx_fns()
}

/// Find an LPFX function by its GLSL name
///
/// Returns `None` if the function is not found in the registry.
pub fn find_lpfx_fn(name: &str) -> Option<&'static LpfxFn> {
    get_cached_functions()
        .iter()
        .find(|f| f.glsl_sig.name == name)
}

/// Find an LPFX function by BuiltinId
///
/// Returns `None` if the function is not found in the registry.
pub fn find_lpfx_fn_by_builtin_id(
    builtin_id: crate::backend::builtins::registry::BuiltinId,
) -> Option<&'static LpfxFn> {
    for func in get_cached_functions().iter() {
        match &func.impls {
            super::lpfx_fn::LpfxFnImpl::NonDecimal(id) if *id == builtin_id => {
                return Some(func);
            }
            super::lpfx_fn::LpfxFnImpl::Decimal { float_impl, q32_impl } => {
                if *float_impl == builtin_id || *q32_impl == builtin_id {
                    return Some(func);
                }
            }
            _ => {}
        }
    }
    None
}

/// Check if an LPFX function call is valid and return the return type
///
/// Validates that the function exists and that the argument types match the signature.
/// Handles vector types by comparing component counts.
///
/// # Returns
/// - `Ok(return_type)` if the call is valid
/// - `Err(error_message)` if the call is invalid
pub fn check_lpfx_fn_call(name: &str, arg_types: &[Type]) -> Result<Type, String> {
    let func = find_lpfx_fn(name).ok_or_else(|| format!("unknown LPFX function: {name}"))?;

    // Check parameter count matches
    if func.glsl_sig.parameters.len() != arg_types.len() {
        return Err(format!(
            "function `{}` expects {} arguments, got {}",
            name,
            func.glsl_sig.parameters.len(),
            arg_types.len()
        ));
    }

    // Check each parameter type matches
    for (param, arg_ty) in func.glsl_sig.parameters.iter().zip(arg_types) {
        // For vectors, check if the base type matches and component count matches
        if param.ty.is_vector() && arg_ty.is_vector() {
            // Both are vectors - check they're the same type
            if param.ty != *arg_ty {
                return Err(format!(
                    "function `{}` parameter `{}` expects type `{:?}`, got `{:?}`",
                    name, param.name, param.ty, arg_ty
                ));
            }
        } else if param.ty.is_vector() {
            // Parameter is vector but argument is not
            return Err(format!(
                "function `{}` parameter `{}` expects vector type `{:?}`, got scalar `{:?}`",
                name, param.name, param.ty, arg_ty
            ));
        } else if arg_ty.is_vector() {
            // Argument is vector but parameter is not
            return Err(format!(
                "function `{}` parameter `{}` expects scalar type `{:?}`, got vector `{:?}`",
                name, param.name, param.ty, arg_ty
            ));
        } else {
            // Both are scalars - check exact match
            if param.ty != *arg_ty {
                return Err(format!(
                    "function `{}` parameter `{}` expects type `{:?}`, got `{:?}`",
                    name, param.name, param.ty, arg_ty
                ));
            }
        }
    }

    Ok(func.glsl_sig.return_type.clone())
}

/// Get the BuiltinId for a function with a specific decimal format
///
/// Returns `None` if no implementation exists for the given format.
pub fn get_builtin_id_for_format(
    func: &'static LpfxFn,
    format: crate::DecimalFormat,
) -> Option<crate::backend::builtins::registry::BuiltinId> {
    match &func.impls {
        super::lpfx_fn::LpfxFnImpl::NonDecimal(builtin_id) => Some(*builtin_id),
        super::lpfx_fn::LpfxFnImpl::Decimal { float_impl, q32_impl } => {
            match format {
                crate::DecimalFormat::Float => Some(*float_impl),
                crate::DecimalFormat::Fixed32 => Some(*q32_impl),
            }
        }
    }
}
