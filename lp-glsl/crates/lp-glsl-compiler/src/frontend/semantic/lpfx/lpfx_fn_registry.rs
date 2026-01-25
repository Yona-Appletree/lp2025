//! LPFX Function Registry
//!
//! Provides lookup functions for LPFX functions from the registry.

use super::lpfx_fn::LpfxFn;
use crate::semantic::types::Type;
use alloc::{boxed::Box, string::String, vec::Vec};

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
    static FUNCTIONS: std::sync::OnceLock<&'static [LpfxFn]> = std::sync::OnceLock::new();
    *FUNCTIONS.get_or_init(|| {
        let vec = super::lpfx_fns::lpfx_fns();
        Box::leak(vec.into_boxed_slice())
    })
}

/// Find an LPFX function by its GLSL name
///
/// Returns `None` if the function is not found in the registry.
pub fn find_lpfx_fn(name: &str) -> Option<&'static LpfxFn> {
    get_cached_functions()
        .iter()
        .find(|f| f.glsl_sig.name == name)
}

/// Find an LPFX function and implementation by rust function name
///
/// Returns `None` if the function is not found in the registry.
/// Returns `Some((func, impl_))` where `impl_` is the first matching implementation.
/// Find an LPFX function and implementation by rust function name
///
/// Returns `None` if the function is not found in the registry.
/// Returns `Some((func, impl_))` where `impl_` is the first matching implementation.
pub fn find_lpfx_fn_by_rust_name(
    rust_fn_name: &str,
) -> Option<(&'static LpfxFn, &'static super::lpfx_fn::LpfxFnImpl)> {
    for func in get_cached_functions().iter() {
        for impl_ in func.impls.iter() {
            if impl_.rust_fn_name == rust_fn_name {
                return Some((func, impl_));
            }
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
    func: &'static LpfxFn,
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

/// Map rust function name to BuiltinId
///
/// This maps the internal Rust function names (e.g., "__lpfx_hash_1") to BuiltinId enum variants.
/// Returns `None` if the function name doesn't correspond to a builtin.
pub fn rust_fn_name_to_builtin_id(
    rust_fn_name: &str,
) -> Option<crate::backend::builtins::registry::BuiltinId> {
    use crate::backend::builtins::registry::BuiltinId;
    match rust_fn_name {
        "__lpfx_hash_1" => Some(BuiltinId::LpHash1),
        "__lpfx_hash_2" => Some(BuiltinId::LpHash2),
        "__lpfx_hash_3" => Some(BuiltinId::LpHash3),
        "__lpfx_simplex1_q32" => Some(BuiltinId::LpSimplex1),
        "__lpfx_simplex2_q32" => Some(BuiltinId::LpSimplex2),
        "__lpfx_simplex3_q32" => Some(BuiltinId::LpSimplex3),
        _ => None,
    }
}
