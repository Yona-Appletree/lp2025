use std::fs;
use std::path::{Path, PathBuf};
use syn::{Item, ItemFn, parse_file};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
struct BuiltinInfo {
    enum_variant: String,
    symbol_name: String,
    function_name: String,
    param_count: usize,
    file_name: String,
}

fn main() {
    let workspace_root = find_workspace_root().expect("Failed to find workspace root");
    let fixed32_dir = workspace_root.join("lp-glsl/crates/lp-builtins/src/builtins/fixed32");
    let shared_dir = workspace_root.join("lp-glsl/crates/lp-builtins/src/builtins/shared");

    let mut builtins = discover_builtins(&fixed32_dir).expect("Failed to discover builtins");
    let shared_builtins =
        discover_builtins(&shared_dir).expect("Failed to discover shared builtins");
    builtins.extend(shared_builtins);

    // Generate registry.rs
    let registry_path =
        workspace_root.join("lp-glsl/crates/lp-glsl-compiler/src/backend/builtins/registry.rs");
    generate_registry(&registry_path, &builtins);

    // Generate builtin_refs.rs
    let builtin_refs_path = workspace_root.join("lp-glsl/apps/lp-builtins-app/src/builtin_refs.rs");
    generate_builtin_refs(&builtin_refs_path, &builtins);

    // Generate mod.rs
    let mod_rs_path = workspace_root.join("lp-glsl/crates/lp-builtins/src/builtins/fixed32/mod.rs");
    generate_mod_rs(&mod_rs_path, &builtins);

    // Generate testcase mapping in math.rs
    let math_rs_path = workspace_root
        .join("lp-glsl/crates/lp-glsl-compiler/src/backend/transform/fixed32/converters/math.rs");
    generate_testcase_mapping(&math_rs_path, &builtins);

    // Format generated files using cargo fmt
    format_generated_files(
        &workspace_root,
        &[
            &registry_path,
            &builtin_refs_path,
            &mod_rs_path,
            &math_rs_path,
        ],
    );

    println!("Generated all builtin boilerplate files");
}

fn find_workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut current = std::env::current_dir()?;
    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            // Check if this is the workspace root by looking for [workspace] section
            let content = std::fs::read_to_string(&cargo_toml)?;
            if content.contains("[workspace]") && current.join("lp-glsl").exists() {
                return Ok(current);
            }
        }
        if !current.pop() {
            return Err("Could not find workspace root".into());
        }
    }
}

fn discover_builtins(dir: &Path) -> Result<Vec<BuiltinInfo>, Box<dyn std::error::Error>> {
    use lp_glsl_compiler::frontend::semantic::lp_lib_fns::LpLibFn;

    // First, discover all functions from files
    let mut discovered_functions = Vec::new();

    for entry in WalkDir::new(dir).max_depth(1) {
        let entry = entry?;
        let path = entry.path();

        if path.extension() != Some(std::ffi::OsStr::new("rs")) {
            continue;
        }

        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("Invalid file name")?;

        // Skip mod.rs and test_helpers.rs
        if file_name == "mod" || file_name == "test_helpers" {
            continue;
        }

        let content = fs::read_to_string(path)?;
        let ast = parse_file(&content)?;

        for item in ast.items {
            if let Item::Fn(func) = item {
                let func_name = func.sig.ident.to_string();
                // Check for #[unsafe(no_mangle)] attribute
                let has_no_mangle = func.attrs.iter().any(|attr| attr.path().is_ident("unsafe"));
                if has_no_mangle {
                    let param_count = func.sig.inputs.len();
                    discovered_functions.push((
                        func_name.clone(),
                        func_name,
                        param_count,
                        file_name.to_string(),
                    ));
                }
            }
        }
    }

    // Now match discovered functions to LpLibFn enum variants
    let mut builtins = Vec::new();
    let lp_lib_fns = [
        LpLibFn::Hash1,
        LpLibFn::Hash2,
        LpLibFn::Hash3,
        LpLibFn::Simplex1,
        LpLibFn::Simplex2,
        LpLibFn::Simplex3,
    ];

    for lp_fn in lp_lib_fns {
        // Determine expected function name
        let expected_name = lp_fn.fixed32_name().unwrap_or_else(|| lp_fn.symbol_name());

        // Find matching discovered function
        if let Some((_, func_name, param_count, file_name)) = discovered_functions
            .iter()
            .find(|(name, _, _, _)| name == expected_name)
        {
            // Get BuiltinId variant name from LpLibFn
            // LpLibFn::Simplex1 -> BuiltinId::LpSimplex1
            // LpLibFn::Hash1 -> BuiltinId::LpHash1
            let enum_variant = match lp_fn {
                LpLibFn::Hash1 => "LpHash1",
                LpLibFn::Hash2 => "LpHash2",
                LpLibFn::Hash3 => "LpHash3",
                LpLibFn::Simplex1 => "LpSimplex1",
                LpLibFn::Simplex2 => "LpSimplex2",
                LpLibFn::Simplex3 => "LpSimplex3",
            };

            builtins.push(BuiltinInfo {
                enum_variant: enum_variant.to_string(),
                symbol_name: func_name.clone(),
                function_name: func_name.clone(),
                param_count: *param_count,
                file_name: file_name.clone(),
            });
        }
    }

    // Also discover regular fixed32 functions (not LP library functions)
    for (func_name, symbol_name, param_count, file_name) in discovered_functions {
        // Skip if already matched as LP library function
        if builtins.iter().any(|b| b.function_name == func_name) {
            continue;
        }

        // Check if it's a regular fixed32 function
        if func_name.starts_with("__lp_fixed32_") {
            let suffix = func_name.strip_prefix("__lp_fixed32_").unwrap();
            let enum_variant = suffix
                .split('_')
                .map(|s| {
                    let mut chars = s.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<String>();

            builtins.push(BuiltinInfo {
                enum_variant: format!("Fixed32{}", enum_variant),
                symbol_name: symbol_name.clone(),
                function_name: func_name.clone(),
                param_count,
                file_name: file_name.clone(),
            });
        }
    }

    // Sort by symbol name for consistent output
    builtins.sort_by(|a, b| a.symbol_name.cmp(&b.symbol_name));

    Ok(builtins)
}

fn extract_builtin(func: &ItemFn, file_name: &str) -> Option<BuiltinInfo> {
    // Check for #[unsafe(no_mangle)] attribute
    let has_no_mangle = func.attrs.iter().any(|attr| attr.path().is_ident("unsafe"));

    if !has_no_mangle {
        return None;
    }

    let func_name = func.sig.ident.to_string();

    // Check if function name starts with __lp_fixed32_, __lp_hash_, or __lp_simplex
    let (prefix, enum_prefix) = if func_name.starts_with("__lp_fixed32_") {
        ("__lp_fixed32_", "Fixed32")
    } else if func_name.starts_with("__lp_hash_") {
        ("__lp_hash_", "LpHash")
    } else if func_name.starts_with("__lp_simplex") {
        ("__lp_simplex", "LpSimplex")
    } else {
        return None;
    };

    // Extract symbol name (function name)
    let symbol_name = func_name.clone();

    // Derive enum variant name
    // __lp_fixed32_sqrt -> Fixed32Sqrt
    // __lp_hash_1 -> LpHash1
    // __lp_simplex2 -> LpSimplex2
    let suffix = symbol_name.strip_prefix(prefix).unwrap();
    let enum_variant = if prefix == "__lp_fixed32_" {
        // For fixed32 functions, capitalize each word
        suffix
            .split('_')
            .map(|s| {
                let mut chars = s.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<String>()
    } else {
        // For hash and simplex, just capitalize first letter and keep rest
        capitalize_first(suffix)
    };

    // Count parameters (extern "C" functions don't have self)
    let param_count = func.sig.inputs.len();

    Some(BuiltinInfo {
        enum_variant: format!("{}{}", enum_prefix, enum_variant),
        symbol_name,
        function_name: func_name,
        param_count,
        file_name: file_name.to_string(),
    })
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Strip the prefix from a function name to get the base name
fn strip_function_prefix(name: &str) -> &str {
    if name.starts_with("__lp_fixed32_") {
        name.strip_prefix("__lp_fixed32_").unwrap()
    } else if name.starts_with("__lp_hash_") {
        name.strip_prefix("__lp_hash_").unwrap()
    } else if name.starts_with("__lp_simplex") {
        name.strip_prefix("__lp_simplex").unwrap()
    } else {
        name
    }
}

fn generate_registry(path: &Path, builtins: &[BuiltinInfo]) {
    let header = r#"//! This file is AUTO-GENERATED. Do not edit manually.
//!
//! To regenerate this file, run:
//!     cargo run --bin lp-builtin-gen --manifest-path lp-glsl/apps/lp-builtin-gen/Cargo.toml
//!
//! Or use the build script:
//!     scripts/build-builtins.sh

"#;

    let mut output = String::from(header);
    output.push_str("//! Builtin function registry implementation.\n");
    output.push_str("//!\n");
    output
        .push_str("//! Provides enum-based registry for builtin functions with support for both\n");
    output.push_str("//! JIT (function pointer) and emulator (ELF symbol) linking.\n");
    output.push('\n');

    output.push_str("use crate::error::{ErrorCode, GlslError};\n");
    output.push_str("use cranelift_codegen::ir::{AbiParam, Signature, types};\n");
    output.push_str("use cranelift_codegen::isa::CallConv;\n");
    output.push_str("use cranelift_module::{Linkage, Module};\n\n");
    output.push_str("#[cfg(not(feature = \"std\"))]\n");
    output.push_str("use alloc::format;\n\n");

    // Generate enum
    output.push_str("/// Enum identifying builtin functions.\n");
    output.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
    output.push_str("pub enum BuiltinId {\n");
    if builtins.is_empty() {
        output.push_str("    // No builtins defined yet\n");
        output.push_str("    #[allow(dead_code)]\n");
        output.push_str("    _Placeholder,\n");
    } else {
        for builtin in builtins {
            output.push_str(&format!("    {},\n", builtin.enum_variant));
        }
    }
    output.push_str("}\n\n");

    // Generate impl BuiltinId
    output.push_str("impl BuiltinId {\n");
    output.push_str("    /// Get the symbol name for this builtin function.\n");
    output.push_str("    pub fn name(&self) -> &'static str {\n");
    output.push_str("        match self {\n");
    if builtins.is_empty() {
        output.push_str("            BuiltinId::_Placeholder => \"\",\n");
    } else {
        for builtin in builtins {
            output.push_str(&format!(
                "            BuiltinId::{} => \"{}\",\n",
                builtin.enum_variant, builtin.symbol_name
            ));
        }
    }
    output.push_str("        }\n");
    output.push_str("    }\n\n");

    // Generate signature() method
    output.push_str("    /// Get the Cranelift signature for this builtin function.\n");
    output.push_str("    pub fn signature(&self) -> Signature {\n");
    output.push_str("        let mut sig = Signature::new(CallConv::SystemV);\n");
    output.push_str("        match self {\n");

    if builtins.is_empty() {
        output.push_str("            BuiltinId::_Placeholder => {\n");
        output.push_str("                // Placeholder - no builtins defined\n");
        output.push_str("            }\n");
    } else {
        // Group by parameter count
        let quaternary_ops: Vec<_> = builtins.iter().filter(|b| b.param_count == 4).collect();
        let ternary_ops: Vec<_> = builtins.iter().filter(|b| b.param_count == 3).collect();
        let binary_ops: Vec<_> = builtins.iter().filter(|b| b.param_count == 2).collect();
        let unary_ops: Vec<_> = builtins.iter().filter(|b| b.param_count == 1).collect();

        if !quaternary_ops.is_empty() {
            output.push_str("            ");
            for (i, builtin) in quaternary_ops.iter().enumerate() {
                if i > 0 {
                    output.push_str(" | ");
                }
                output.push_str(&format!("BuiltinId::{}", builtin.enum_variant));
            }
            output.push_str(" => {\n");
            output.push_str("                // (i32, i32, i32, i32) -> i32\n");
            output.push_str("                sig.params.push(AbiParam::new(types::I32));\n");
            output.push_str("                sig.params.push(AbiParam::new(types::I32));\n");
            output.push_str("                sig.params.push(AbiParam::new(types::I32));\n");
            output.push_str("                sig.params.push(AbiParam::new(types::I32));\n");
            output.push_str("                sig.returns.push(AbiParam::new(types::I32));\n");
            output.push_str("            }\n");
        }

        if !ternary_ops.is_empty() {
            output.push_str("            ");
            for (i, builtin) in ternary_ops.iter().enumerate() {
                if i > 0 {
                    output.push_str(" | ");
                }
                output.push_str(&format!("BuiltinId::{}", builtin.enum_variant));
            }
            output.push_str(" => {\n");
            output.push_str("                // (i32, i32, i32) -> i32\n");
            output.push_str("                sig.params.push(AbiParam::new(types::I32));\n");
            output.push_str("                sig.params.push(AbiParam::new(types::I32));\n");
            output.push_str("                sig.params.push(AbiParam::new(types::I32));\n");
            output.push_str("                sig.returns.push(AbiParam::new(types::I32));\n");
            output.push_str("            }\n");
        }

        if !binary_ops.is_empty() {
            output.push_str("            ");
            for (i, builtin) in binary_ops.iter().enumerate() {
                if i > 0 {
                    output.push_str(" | ");
                }
                output.push_str(&format!("BuiltinId::{}", builtin.enum_variant));
            }
            output.push_str(" => {\n");
            output.push_str("                // (i32, i32) -> i32\n");
            output.push_str("                sig.params.push(AbiParam::new(types::I32));\n");
            output.push_str("                sig.params.push(AbiParam::new(types::I32));\n");
            output.push_str("                sig.returns.push(AbiParam::new(types::I32));\n");
            output.push_str("            }\n");
        }

        if !unary_ops.is_empty() {
            output.push_str("            ");
            for (i, builtin) in unary_ops.iter().enumerate() {
                if i > 0 {
                    output.push_str(" | ");
                }
                output.push_str(&format!("BuiltinId::{}", builtin.enum_variant));
            }
            output.push_str(" => {\n");
            output.push_str("                // (i32) -> i32\n");
            output.push_str("                sig.params.push(AbiParam::new(types::I32));\n");
            output.push_str("                sig.returns.push(AbiParam::new(types::I32));\n");
            output.push_str("            }\n");
        }
    }

    output.push_str("        }\n");
    output.push_str("        sig\n");
    output.push_str("    }\n\n");

    // Generate all() method
    output.push_str("    /// Get all builtin IDs.\n");
    output.push_str("    pub fn all() -> &'static [BuiltinId] {\n");
    output.push_str("        &[\n");
    if builtins.is_empty() {
        output.push_str("            BuiltinId::_Placeholder,\n");
    } else {
        for builtin in builtins {
            output.push_str(&format!(
                "            BuiltinId::{},\n",
                builtin.enum_variant
            ));
        }
    }
    output.push_str("        ]\n");
    output.push_str("    }\n");
    output.push_str("}\n\n");

    // Generate get_function_pointer()
    output.push_str("/// Get function pointer for a builtin (JIT mode only).\n");
    output.push_str("///\n");
    output.push_str("/// Returns the function pointer that can be registered with JITModule.\n");
    output.push_str("pub fn get_function_pointer(builtin: BuiltinId) -> *const u8 {\n");
    if !builtins.is_empty() {
        output.push_str("    use lp_builtins::builtins::fixed32;\n");
    }
    output.push_str("    match builtin {\n");
    if builtins.is_empty() {
        output.push_str("        BuiltinId::_Placeholder => core::ptr::null(),\n");
    } else {
        for builtin in builtins {
            output.push_str(&format!(
                "        BuiltinId::{} => fixed32::{} as *const u8,\n",
                builtin.enum_variant, builtin.function_name
            ));
        }
    }
    output.push_str("    }\n");
    output.push_str("}\n\n");

    // Generate declare_builtins and related functions
    output.push_str("/// Declare builtin functions as external symbols.\n");
    output.push_str("///\n");
    output.push_str(
        "/// This is the same for both JIT and emulator - they both use Linkage::Import.\n",
    );
    output.push_str("/// The difference is only in how they're linked:\n");
    output.push_str(
        "/// - JIT: Function pointers are registered via symbol_lookup_fn during module creation\n",
    );
    output.push_str(
        "/// - Emulator: Symbols are resolved by the linker when linking the static library\n",
    );
    output.push_str(
        "pub fn declare_builtins<M: Module>(module: &mut M) -> Result<(), GlslError> {\n",
    );
    output.push_str("    for builtin in BuiltinId::all() {\n");
    output.push_str("        let name = builtin.name();\n");
    output.push_str("        let sig = builtin.signature();\n\n");
    output.push_str("        module\n");
    output.push_str("            .declare_function(name, Linkage::Import, &sig)\n");
    output.push_str("            .map_err(|e| {\n");
    output.push_str("                GlslError::new(\n");
    output.push_str("                    ErrorCode::E0400,\n");
    output.push_str("                    format!(\"Failed to declare builtin '{name}': {e}\"),\n");
    output.push_str("                )\n");
    output.push_str("            })?;\n");
    output.push_str("    }\n\n");
    output.push_str("    Ok(())\n");
    output.push_str("}\n\n");

    output.push_str("/// Declare and link builtin functions for JIT mode.\n");
    output.push_str("///\n");
    output
        .push_str("/// This declares all builtins as external functions. The function pointers\n");
    output.push_str(
        "/// are registered via a symbol lookup function that's added during module creation.\n",
    );
    output
        .push_str("pub fn declare_for_jit<M: Module>(module: &mut M) -> Result<(), GlslError> {\n");
    output.push_str("    declare_builtins(module)\n");
    output.push_str("}\n\n");

    output.push_str("/// Declare builtin functions as external symbols for emulator mode.\n");
    output.push_str("///\n");
    output.push_str(
        "/// This declares all builtins as external symbols (Linkage::Import) that will\n",
    );
    output.push_str("/// be resolved by the linker when linking the static library.\n");
    output.push_str(
        "pub fn declare_for_emulator<M: Module>(module: &mut M) -> Result<(), GlslError> {\n",
    );
    output.push_str("    declare_builtins(module)\n");
    output.push_str("}\n");

    fs::write(path, output).expect("Failed to write registry.rs");
}

fn generate_builtin_refs(path: &Path, builtins: &[BuiltinInfo]) {
    let header = r#"//! This file is AUTO-GENERATED. Do not edit manually.
//!
//! To regenerate this file, run:
//!     cargo run --bin lp-builtin-gen --manifest-path lp-glsl/apps/lp-builtin-gen/Cargo.toml
//!
//! Or use the build script:
//!     scripts/build-builtins.sh

"#;

    let mut output = String::from(header);
    if builtins.is_empty() {
        output.push_str("// No builtins to import\n\n");
    } else {
        output.push_str("use lp_builtins::builtins::fixed32::{\n");
        for (i, builtin) in builtins.iter().enumerate() {
            if i > 0 {
                output.push_str(",\n");
            }
            output.push_str(&format!("    {}", builtin.function_name));
        }
        output.push_str(",\n};\n\n");
    }

    output.push_str("/// Reference all builtin functions to prevent dead code elimination.\n");
    output.push_str("///\n");
    output.push_str(
        "/// This function ensures all builtin functions are included in the executable\n",
    );
    output
        .push_str("/// by creating function pointers and reading them with volatile operations.\n");
    output.push_str("pub fn ensure_builtins_referenced() {\n");
    output.push_str("    unsafe {\n");

    // Generate function pointer declarations
    for builtin in builtins {
        // Determine function signature based on function name
        let fn_type = if builtin.function_name.starts_with("__lp_hash_") {
            // Hash functions use u32 for all parameters and return u32
            match builtin.param_count {
                2 => "extern \"C\" fn(u32, u32) -> u32",
                3 => "extern \"C\" fn(u32, u32, u32) -> u32",
                4 => "extern \"C\" fn(u32, u32, u32, u32) -> u32",
                _ => "extern \"C\" fn(u32) -> u32",
            }
        } else if builtin.function_name.starts_with("__lp_simplex") {
            // Simplex functions use i32 for coordinates, u32 for seed, return i32
            match builtin.param_count {
                2 => "extern \"C\" fn(i32, u32) -> i32",
                3 => "extern \"C\" fn(i32, i32, u32) -> i32",
                4 => "extern \"C\" fn(i32, i32, i32, u32) -> i32",
                _ => "extern \"C\" fn(i32) -> i32",
            }
        } else {
            // Fixed32 functions use i32 for all parameters and return i32
            match builtin.param_count {
                1 => "extern \"C\" fn(i32) -> i32",
                2 => "extern \"C\" fn(i32, i32) -> i32",
                3 => "extern \"C\" fn(i32, i32, i32) -> i32",
                4 => "extern \"C\" fn(i32, i32, i32, i32) -> i32",
                _ => "extern \"C\" fn(i32) -> i32",
            }
        };
        // Use full function name suffix for unique variable names
        let var_suffix = builtin
            .function_name
            .strip_prefix("__lp_")
            .unwrap_or(&builtin.function_name);
        let var_suffix = var_suffix.replace("__", "_").replace("-", "_");
        output.push_str(&format!(
            "        let _{var_suffix}_fn: {fn_type} = {};\n",
            builtin.function_name
        ));
    }

    output.push('\n');
    output.push_str("        // Force these to be included by using them in a way that can't be optimized away\n");
    output.push_str("        // We'll use volatile reads to prevent optimization\n");

    // Generate read_volatile calls
    for builtin in builtins {
        // Use full function name suffix for unique variable names (same as above)
        let var_suffix = builtin
            .function_name
            .strip_prefix("__lp_")
            .unwrap_or(&builtin.function_name);
        let var_suffix = var_suffix.replace("__", "_").replace("-", "_");
        let var_name = format!("_{var_suffix}_fn");
        output.push_str(&format!(
            "        let _ = core::ptr::read_volatile(&{} as *const _);\n",
            var_name
        ));
    }

    output.push_str("    }\n");
    output.push_str("}\n");

    fs::write(path, output).expect("Failed to write builtin_refs.rs");
}

fn generate_mod_rs(path: &Path, builtins: &[BuiltinInfo]) {
    let header = r#"//! This file is AUTO-GENERATED. Do not edit manually.
//!
//! To regenerate this file, run:
//!     cargo run --bin lp-builtin-gen --manifest-path lp-glsl/apps/lp-builtin-gen/Cargo.toml
//!
//! Or use the build script:
//!     scripts/build-builtins.sh

"#;

    let mut output = String::from(header);
    output.push_str("//! Fixed-point 16.16 arithmetic builtins.\n");
    output.push_str("//!\n");
    output.push_str("//! Functions operate on i32 values representing fixed-point numbers\n");
    output.push_str("//! with 16 bits of fractional precision.\n");
    output.push('\n');

    // Generate mod declarations (deduplicate by file name)
    let mut seen_files = std::collections::HashSet::new();
    for builtin in builtins {
        if seen_files.insert(&builtin.file_name) {
            output.push_str(&format!("mod {};\n", builtin.file_name));
        }
    }
    output.push('\n');
    output.push_str("#[cfg(test)]\n");
    output.push_str("mod test_helpers;\n\n");

    // Generate pub use statements
    for builtin in builtins {
        output.push_str(&format!(
            "pub use {}::{};\n",
            builtin.file_name, builtin.function_name
        ));
    }

    fs::write(path, output).expect("Failed to write mod.rs");
}

fn generate_testcase_mapping(path: &Path, builtins: &[BuiltinInfo]) {
    // Read existing file
    let content = fs::read_to_string(path).expect("Failed to read math.rs");

    // Find the map_testcase_to_builtin function and replace it
    let start_marker = "/// Map TestCase function name to BuiltinId and argument count.";

    let start_idx = content
        .find(start_marker)
        .expect("Could not find map_testcase_to_builtin function");

    // Find the end of the function (look for the closing brace after the match)
    let mut end_idx = start_idx;
    let mut brace_count = 0;
    let mut in_function = false;

    for (i, ch) in content[start_idx..].char_indices() {
        if ch == '{' {
            brace_count += 1;
            in_function = true;
        } else if ch == '}' {
            brace_count -= 1;
            if in_function && brace_count == 0 {
                end_idx = start_idx + i + 1;
                break;
            }
        }
    }

    let before = &content[..start_idx];
    let after = &content[end_idx..];

    let header = "/// Map TestCase function name to BuiltinId and argument count.\n///\n/// Returns None if the function name is not a math function that should be converted.\n/// Handles both standard C math function names (sinf, cosf) and intrinsic names (__lp_sin, __lp_cos).\n/// Returns (BuiltinId, argument_count) where argument_count is 1 or 2.\n///\n/// This function is AUTO-GENERATED. Do not edit manually.\n///\n/// To regenerate this function, run:\n///     cargo run --bin lp-builtin-gen --manifest-path lp-glsl/apps/lp-builtin-gen/Cargo.toml\n///\n/// Or use the build script:\n///     scripts/build-builtins.sh\n";

    let mut new_function = String::from(header);
    new_function.push_str(
        "pub fn map_testcase_to_builtin(testcase_name: &str) -> Option<(BuiltinId, usize)> {\n",
    );
    new_function.push_str("    match testcase_name {\n");

    // Generate mappings
    if builtins.is_empty() {
        // No builtins, so no mappings
    } else {
        use lp_glsl_compiler::frontend::semantic::lp_lib_fns::LpLibFn;

        for builtin in builtins {
            // Check if this is an LP library function by matching symbol name to enum
            let lp_fn_opt = match builtin.symbol_name.as_str() {
                "__lp_hash_1" => Some(LpLibFn::Hash1),
                "__lp_hash_2" => Some(LpLibFn::Hash2),
                "__lp_hash_3" => Some(LpLibFn::Hash3),
                "__lp_simplex1" => Some(LpLibFn::Simplex1),
                "__lp_simplex2" => Some(LpLibFn::Simplex2),
                "__lp_simplex3" => Some(LpLibFn::Simplex3),
                _ => None,
            };

            if let Some(lp_fn) = lp_fn_opt {
                // LP library function - use enum to determine mapping
                if builtin.symbol_name.starts_with("__lp_hash_") {
                    // Hash functions: use testcase pattern "1f" | "__lp_1"
                    let base_name = strip_function_prefix(&builtin.symbol_name);
                    let c_name = format!("{}f", base_name);
                    let intrinsic_name = format!("__lp_{}", base_name);
                    new_function.push_str(&format!(
                        "        \"{}\" | \"{}\" => Some((BuiltinId::{}, {})),\n",
                        c_name, intrinsic_name, builtin.enum_variant, builtin.param_count
                    ));
                } else {
                    // Simplex functions: use full symbol name to avoid conflicts
                    new_function.push_str(&format!(
                        "        \"{}\" => Some((BuiltinId::{}, {})),\n",
                        lp_fn.symbol_name(),
                        builtin.enum_variant,
                        builtin.param_count
                    ));
                }
            } else {
                // Regular fixed32 functions
                let base_name = strip_function_prefix(&builtin.symbol_name);

                // Generate C math function name (e.g., sinf)
                let c_name = format!("{}f", base_name);

                // Generate intrinsic name (e.g., __lp_sin)
                let intrinsic_name = format!("__lp_{}", base_name);

                // Special case: GLSL's mod() compiles to fmodf, not modf
                let additional_names = if base_name == "mod" {
                    " | \"fmodf\""
                } else {
                    ""
                };

                new_function.push_str(&format!(
                    "        \"{}\" | \"{}\"{additional_names} => Some((BuiltinId::{}, {})),\n",
                    c_name, intrinsic_name, builtin.enum_variant, builtin.param_count
                ));
            }
        }
    }

    new_function.push_str("        _ => None,\n");
    new_function.push_str("    }\n");
    new_function.push_str("}\n");

    let new_content = format!("{}{}{}", before, new_function, after);
    fs::write(path, new_content).expect("Failed to write math.rs");
}

fn format_generated_files(workspace_root: &Path, files: &[&Path]) {
    use std::process::Command;

    // Run cargo fmt on the generated files
    let mut cmd = Command::new("cargo");
    cmd.arg("fmt");
    cmd.arg("--");

    for file in files {
        // Get relative path from workspace root
        if let Ok(relative_path) = file.strip_prefix(workspace_root) {
            cmd.arg(relative_path);
        }
    }

    // Run from workspace root
    let output = cmd
        .current_dir(workspace_root)
        .output()
        .expect("Failed to run cargo fmt");

    if !output.status.success() {
        eprintln!("Warning: cargo fmt failed on generated files:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }
}
