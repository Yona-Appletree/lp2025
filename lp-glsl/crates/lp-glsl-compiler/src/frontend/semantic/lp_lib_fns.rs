//! LP Library Function signatures and type checking
//!
//! LightPlayer Library Functions (LpLibFns) are user-facing functions that map to
//! internal builtin implementations. These functions provide noise generation and
//! other utility functions for shaders.

use crate::backend::builtins::registry::BuiltinId;
use crate::frontend::semantic::types::Type;
use alloc::{format, string::String, vec, vec::Vec};

/// LP Library Function identifier - single source of truth for all LP library functions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LpLibFn {
    /// lp_hash(u32, u32) -> u32
    Hash1,
    /// lp_hash(u32, u32, u32) -> u32
    Hash2,
    /// lp_hash(u32, u32, u32, u32) -> u32
    Hash3,
    /// lp_simplex1(float, uint) -> float
    Simplex1,
    /// lp_simplex2(vec2, uint) -> float
    Simplex2,
    /// lp_simplex3(vec3, uint) -> float
    Simplex3,
}

impl LpLibFn {
    /// Get the user-facing function name
    pub fn user_name(&self) -> &'static str {
        match self {
            LpLibFn::Hash1 | LpLibFn::Hash2 | LpLibFn::Hash3 => "lp_hash",
            LpLibFn::Simplex1 => "lp_simplex1",
            LpLibFn::Simplex2 => "lp_simplex2",
            LpLibFn::Simplex3 => "lp_simplex3",
        }
    }

    /// Get the internal BuiltinId for this function
    pub fn builtin_id(&self) -> BuiltinId {
        match self {
            LpLibFn::Hash1 => BuiltinId::LpHash1,
            LpLibFn::Hash2 => BuiltinId::LpHash2,
            LpLibFn::Hash3 => BuiltinId::LpHash3,
            LpLibFn::Simplex1 => BuiltinId::LpSimplex1,
            LpLibFn::Simplex2 => BuiltinId::LpSimplex2,
            LpLibFn::Simplex3 => BuiltinId::LpSimplex3,
        }
    }

    /// Get the parameter types for this function
    pub fn param_types(&self) -> Vec<Type> {
        match self {
            LpLibFn::Hash1 => vec![Type::UInt, Type::UInt],
            LpLibFn::Hash2 => vec![Type::UInt, Type::UInt, Type::UInt],
            LpLibFn::Hash3 => vec![Type::UInt, Type::UInt, Type::UInt, Type::UInt],
            LpLibFn::Simplex1 => vec![Type::Float, Type::UInt],
            LpLibFn::Simplex2 => vec![Type::Vec2, Type::UInt],
            LpLibFn::Simplex3 => vec![Type::Vec3, Type::UInt],
        }
    }

    /// Get the return type for this function
    pub fn return_type(&self) -> Type {
        match self {
            LpLibFn::Hash1 | LpLibFn::Hash2 | LpLibFn::Hash3 => Type::UInt,
            LpLibFn::Simplex1 | LpLibFn::Simplex2 | LpLibFn::Simplex3 => Type::Float,
        }
    }

    /// Get the number of GLSL arguments (before vector flattening)
    pub fn glsl_arg_count(&self) -> usize {
        self.param_types().len()
    }

    /// Get the internal symbol name (for testcase mapping)
    pub fn symbol_name(&self) -> &'static str {
        match self {
            LpLibFn::Hash1 => "__lp_hash_1",
            LpLibFn::Hash2 => "__lp_hash_2",
            LpLibFn::Hash3 => "__lp_hash_3",
            LpLibFn::Simplex1 => "__lp_simplex1",
            LpLibFn::Simplex2 => "__lp_simplex2",
            LpLibFn::Simplex3 => "__lp_simplex3",
        }
    }

    /// Get all variants for a given user-facing name
    pub fn variants_for_name(name: &str) -> Vec<LpLibFn> {
        match name {
            "lp_hash" => vec![LpLibFn::Hash1, LpLibFn::Hash2, LpLibFn::Hash3],
            "lp_simplex1" => vec![LpLibFn::Simplex1],
            "lp_simplex2" => vec![LpLibFn::Simplex2],
            "lp_simplex3" => vec![LpLibFn::Simplex3],
            _ => vec![],
        }
    }

    /// Find LP library function by name and argument count
    pub fn from_name_and_args(name: &str, arg_count: usize) -> Option<LpLibFn> {
        match name {
            "lp_hash" => match arg_count {
                2 => Some(LpLibFn::Hash1),
                3 => Some(LpLibFn::Hash2),
                4 => Some(LpLibFn::Hash3),
                _ => None,
            },
            "lp_simplex1" => {
                if arg_count == 2 {
                    Some(LpLibFn::Simplex1)
                } else {
                    None
                }
            }
            "lp_simplex2" => {
                if arg_count == 2 {
                    Some(LpLibFn::Simplex2)
                } else {
                    None
                }
            }
            "lp_simplex3" => {
                if arg_count == 2 {
                    Some(LpLibFn::Simplex3)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// LP Library Function signature
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LpLibFnSignature {
    pub name: &'static str,
    pub param_types: Vec<Type>,
    pub return_type: Type,
}

/// Check if a name is an LP library function
pub fn is_lp_lib_fn(name: &str) -> bool {
    name.starts_with("lp_")
}

/// Lookup LP library function signatures by name
pub fn lookup_lp_lib_fn(name: &str) -> Option<Vec<LpLibFnSignature>> {
    let variants = LpLibFn::variants_for_name(name);
    if variants.is_empty() {
        return None;
    }

    Some(
        variants
            .into_iter()
            .map(|variant| LpLibFnSignature {
                name: variant.user_name(),
                param_types: variant.param_types(),
                return_type: variant.return_type(),
            })
            .collect(),
    )
}

/// Check if an LP library function call matches a signature
pub fn check_lp_lib_fn_call(name: &str, arg_types: &[Type]) -> Result<Type, String> {
    let signatures =
        lookup_lp_lib_fn(name).ok_or_else(|| format!("Unknown LP library function: {}", name))?;

    // Find matching signature
    for sig in &signatures {
        if sig.param_types.len() == arg_types.len() {
            let mut matches = true;
            for (expected, actual) in sig.param_types.iter().zip(arg_types.iter()) {
                if expected != actual {
                    matches = false;
                    break;
                }
            }
            if matches {
                return Ok(sig.return_type.clone());
            }
        }
    }

    // No matching signature found
    Err(format!(
        "No matching signature for {} with arguments: {:?}",
        name, arg_types
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_lp_lib_fn() {
        assert!(is_lp_lib_fn("lp_hash"));
        assert!(is_lp_lib_fn("lp_simplex1"));
        assert!(is_lp_lib_fn("lp_simplex2"));
        assert!(!is_lp_lib_fn("hash"));
        assert!(!is_lp_lib_fn("sin"));
    }

    #[test]
    fn test_lookup_lp_hash() {
        let sigs = lookup_lp_lib_fn("lp_hash").unwrap();
        assert_eq!(sigs.len(), 3);
        assert_eq!(sigs[0].param_types.len(), 2);
        assert_eq!(sigs[1].param_types.len(), 3);
        assert_eq!(sigs[2].param_types.len(), 4);
    }

    #[test]
    fn test_check_lp_hash_call() {
        // lp_hash(u32, u32) -> u32
        let result = check_lp_lib_fn_call("lp_hash", &[Type::UInt, Type::UInt]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Type::UInt);

        // lp_hash(u32, u32, u32) -> u32
        let result = check_lp_lib_fn_call("lp_hash", &[Type::UInt, Type::UInt, Type::UInt]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Type::UInt);

        // lp_hash(u32, u32, u32, u32) -> u32
        let result =
            check_lp_lib_fn_call("lp_hash", &[Type::UInt, Type::UInt, Type::UInt, Type::UInt]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Type::UInt);

        // Wrong argument type
        let result = check_lp_lib_fn_call("lp_hash", &[Type::Int]);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_lp_simplex_call() {
        // lp_simplex1(float, uint) -> float
        let result = check_lp_lib_fn_call("lp_simplex1", &[Type::Float, Type::UInt]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Type::Float);

        // lp_simplex2(vec2, uint) -> float
        let result = check_lp_lib_fn_call("lp_simplex2", &[Type::Vec2, Type::UInt]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Type::Float);

        // lp_simplex3(vec3, uint) -> float
        let result = check_lp_lib_fn_call("lp_simplex3", &[Type::Vec3, Type::UInt]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Type::Float);
    }

    #[test]
    fn test_lp_lib_fn_enum() {
        let hash1 = LpLibFn::Hash1;
        assert_eq!(hash1.user_name(), "lp_hash");
        assert_eq!(hash1.builtin_id(), BuiltinId::LpHash1);
        assert_eq!(hash1.param_types(), vec![Type::UInt, Type::UInt]);
        assert_eq!(hash1.return_type(), Type::UInt);
        assert_eq!(hash1.glsl_arg_count(), 2);
        assert_eq!(hash1.symbol_name(), "__lp_hash_1");

        let simplex2 = LpLibFn::Simplex2;
        assert_eq!(simplex2.user_name(), "lp_simplex2");
        assert_eq!(simplex2.builtin_id(), BuiltinId::LpSimplex2);
        assert_eq!(simplex2.param_types(), vec![Type::Vec2, Type::UInt]);
        assert_eq!(simplex2.return_type(), Type::Float);
        assert_eq!(simplex2.glsl_arg_count(), 2);
        assert_eq!(simplex2.symbol_name(), "__lp_simplex2");
    }

    #[test]
    fn test_from_name_and_args() {
        assert_eq!(
            LpLibFn::from_name_and_args("lp_hash", 2),
            Some(LpLibFn::Hash1)
        );
        assert_eq!(
            LpLibFn::from_name_and_args("lp_hash", 3),
            Some(LpLibFn::Hash2)
        );
        assert_eq!(
            LpLibFn::from_name_and_args("lp_hash", 4),
            Some(LpLibFn::Hash3)
        );
        assert_eq!(
            LpLibFn::from_name_and_args("lp_simplex2", 2),
            Some(LpLibFn::Simplex2)
        );
        assert_eq!(LpLibFn::from_name_and_args("lp_simplex2", 3), None);
        assert_eq!(LpLibFn::from_name_and_args("unknown", 2), None);
    }
}
