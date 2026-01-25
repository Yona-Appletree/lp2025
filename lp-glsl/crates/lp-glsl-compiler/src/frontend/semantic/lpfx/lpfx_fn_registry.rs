//! LPFX Function Registry
//!
//! Provides lookup functions for LPFX functions from the registry.

use super::lpfx_fn::LpfxFn;
use super::lpfx_fns::LPFX_FNS;
use crate::semantic::types::Type;
use alloc::string::String;
use alloc::vec::Vec;

/// Check if a function name is an LPFX function
///
/// Returns `true` if the name starts with "lpfx_".
pub fn is_lpfx_fn(name: &str) -> bool {
    name.starts_with("lpfx_")
}

/// Find an LPFX function by its GLSL name
///
/// Returns `None` if the function is not found in the registry.
pub fn find_lpfx_fn(name: &str) -> Option<&'static LpfxFn> {
    LPFX_FNS.iter().find(|f| f.glsl_sig.name == name)
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
    let func = find_lpfx_fn(name).ok_or_else(|| format!("unknown LPFX function: {}", name))?;

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

/// Get the implementation for a specific decimal format
///
/// Returns `None` if no implementation exists for the given format.
pub fn get_impl_for_format(
    func: &LpfxFn,
    format: crate::DecimalFormat,
) -> Option<&'static super::lpfx_fn::LpfxFnImpl> {
    // First try to find format-specific implementation
    if let Some(impl_) = func
        .impls
        .iter()
        .find(|impl_| impl_.decimal_format == Some(format))
    {
        return Some(impl_);
    }

    // Fall back to format-agnostic implementation (decimal_format == None)
    func.impls
        .iter()
        .find(|impl_| impl_.decimal_format.is_none())
}
