use std::fs;
use std::path::{Path, PathBuf};
use syn::{Item, ItemFn, parse_file};
use walkdir::WalkDir;

mod discovery;
mod lpfx;

use discovery::discover_lpfx_functions;
use lpfx::generate::generate_lpfx_fns;
use lpfx::process::process_lpfx_functions;
use lpfx::validate::validate_lpfx_functions;

#[derive(Debug, Clone)]
struct BuiltinInfo {
    enum_variant: String,
    symbol_name: String,
    function_name: String,
    param_count: usize,
    file_name: String,
    /// Rust function signature types as strings (e.g., "extern \"C\" fn(f32, u32) -> f32")
    rust_signature: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = find_workspace_root().expect("Failed to find workspace root");
    let fixed32_dir = workspace_root.join("lp-glsl/crates/lp-builtins/src/builtins/fixed32");
    let lpfx_dir = workspace_root.join("lp-glsl/crates/lp-builtins/src/builtins/lpfx");

    let mut builtins = discover_builtins(&fixed32_dir).expect("Failed to discover builtins");
    let lpfx_builtins = discover_builtins(&lpfx_dir).expect("Failed to discover lpfx builtins");
    builtins.extend(lpfx_builtins);

    // Generate registry.rs
    let registry_path =
        workspace_root.join("lp-glsl/crates/lp-glsl-compiler/src/backend/builtins/registry.rs");
    generate_registry(&registry_path, &builtins);

    // Generate builtin_refs.rs
    let builtin_refs_path = workspace_root.join("lp-glsl/apps/lp-builtins-app/src/builtin_refs.rs");
    generate_builtin_refs(&builtin_refs_path, &builtins);

    // Generate mod.rs (only fixed32 functions, not lpfx functions)
    let fixed32_builtins: Vec<BuiltinInfo> = builtins
        .iter()
        .filter(|b| !b.function_name.starts_with("__lpfx_"))
        .cloned()
        .collect();
    let mod_rs_path = workspace_root.join("lp-glsl/crates/lp-builtins/src/builtins/fixed32/mod.rs");
    generate_mod_rs(&mod_rs_path, &fixed32_builtins);

    // Generate testcase mapping in math.rs
    let math_rs_path = workspace_root
        .join("lp-glsl/crates/lp-glsl-compiler/src/backend/transform/fixed32/converters/math.rs");
    generate_testcase_mapping(&math_rs_path, &builtins);

    // Generate lpfx_fns.rs
    let lpfx_fns_path = workspace_root
        .join("lp-glsl/crates/lp-glsl-compiler/src/frontend/semantic/lpfx/lpfx_fns.rs");
    generate_lpfx_fns_file(&lpfx_fns_path, &lpfx_dir)?;

    // Format generated files using cargo fmt
    format_generated_files(
        &workspace_root,
        &[
            &registry_path,
            &builtin_refs_path,
            &mod_rs_path,
            &math_rs_path,
            &lpfx_fns_path,
        ],
    );

    println!("Generated all builtin boilerplate files");
    Ok(())
}

/// Generate lpfx_fns.rs file
fn generate_lpfx_fns_file(
    output_path: &Path,
    lpfx_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Discover LPFX functions
    let discovered = discover_lpfx_functions(lpfx_dir)?;

    // Process: parse attributes and GLSL signatures
    let parsed = process_lpfx_functions(&discovered)?;

    // Validate
    validate_lpfx_functions(&parsed)?;

    // Generate code
    let code = generate_lpfx_fns(&parsed);

    // Write to file
    fs::write(output_path, code)?;

    Ok(())
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
    // Discover all functions from files using extract_builtin
    let mut builtins: Vec<BuiltinInfo> = Vec::new();

    for entry in WalkDir::new(dir) {
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
            if let Item::Fn(func) = item
                && let Some(builtin_info) = extract_builtin(&func, file_name)
            {
                // Skip if already added
                if !builtins
                    .iter()
                    .any(|b| b.function_name == builtin_info.function_name)
                {
                    builtins.push(builtin_info);
                }
            }
        }
    }

    // Sort by symbol name for consistent output
    builtins.sort_by(|a, b| a.symbol_name.cmp(&b.symbol_name));

    Ok(builtins)
}

#[allow(dead_code)]
fn extract_builtin(func: &ItemFn, file_name: &str) -> Option<BuiltinInfo> {
    // Check for #[unsafe(no_mangle)] attribute
    let has_no_mangle = func.attrs.iter().any(|attr| attr.path().is_ident("unsafe"));

    if !has_no_mangle {
        return None;
    }

    let func_name = func.sig.ident.to_string();

    // Only process functions that start with __ (builtin convention)
    if !func_name.starts_with("__") {
        return None;
    }

    // Extract symbol name (function name)
    let symbol_name = func_name.clone();

    // Derive enum variant name by:
    // 1. Strip leading __
    // 2. Split by _ and capitalize each word
    // 3. Join together
    // Examples:
    // __lp_fixed32_sqrt -> LpFixed32Sqrt
    // __lpfx_hash_1 -> LpfxHash1
    // __lpfx_simplex1_q32 -> LpfxSimplex1Q32
    let name_without_prefix = func_name.strip_prefix("__").unwrap();
    let enum_variant = name_without_prefix
        .split('_')
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<String>();

    // Count parameters (extern "C" functions don't have self)
    let param_count = func.sig.inputs.len();

    // Extract Rust function signature
    let rust_signature = format_rust_function_signature(func);

    Some(BuiltinInfo {
        enum_variant,
        symbol_name,
        function_name: func_name,
        param_count,
        file_name: file_name.to_string(),
        rust_signature,
    })
}

/// Format a Rust function signature as a type string
fn format_rust_function_signature(func: &ItemFn) -> String {
    use quote::ToTokens;

    let mut sig = String::from("extern \"C\" fn(");

    // Format parameters
    let mut params = Vec::new();
    for input in &func.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = input {
            let ty_str = pat_type.ty.to_token_stream().to_string();
            // Clean up the string (remove extra spaces)
            let ty_str = ty_str.replace(" ", "");
            params.push(ty_str);
        }
    }
    sig.push_str(&params.join(", "));
    sig.push_str(") -> ");

    // Format return type
    match &func.sig.output {
        syn::ReturnType::Default => sig.push_str("()"),
        syn::ReturnType::Type(_, ty) => {
            let ty_str = ty.to_token_stream().to_string();
            sig.push_str(&ty_str.replace(" ", ""));
        }
    }

    sig
}

#[allow(dead_code)]
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Strip the prefix from a function name to get the base name.
/// This is a generic function that strips the leading `__` and returns the rest.
/// Callers should handle specific patterns as needed.
fn strip_function_prefix(name: &str) -> &str {
    name.strip_prefix("__").unwrap_or(name)
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
    let has_hash = builtins
        .iter()
        .any(|b| b.function_name.starts_with("__lpfx_hash_"));
    let has_simplex = builtins
        .iter()
        .any(|b| b.function_name.contains("lpfx_simplex"));
    let has_fixed32 = builtins
        .iter()
        .any(|b| !b.function_name.starts_with("__lpfx_"));

    // Generate imports
    let mut imports = Vec::new();
    if has_fixed32 {
        imports.push("fixed32".to_string());
    }
    if has_hash || has_simplex {
        let mut lpfx_imports = Vec::new();
        if has_hash {
            lpfx_imports.push("hash");
        }
        if has_simplex {
            lpfx_imports.push("simplex");
        }
        if lpfx_imports.len() == 1 {
            imports.push(format!("lpfx::{}", lpfx_imports[0]));
        } else {
            imports.push(format!("lpfx::{{{}}}", lpfx_imports.join(", ")));
        }
    }
    if !imports.is_empty() {
        output.push_str(&format!(
            "    use lp_builtins::builtins::{{{}}};\n",
            imports.join(", ")
        ));
    }

    output.push_str("    match builtin {\n");
    if builtins.is_empty() {
        output.push_str("        BuiltinId::_Placeholder => core::ptr::null(),\n");
    } else {
        for builtin in builtins {
            // Determine module path based on function name
            let module_path = if builtin.function_name.starts_with("__lpfx_hash_") {
                "hash"
            } else if builtin.function_name.contains("lpfx_simplex") {
                // Simplex functions are in simplex::simplex1_q32::__lpfx_simplex1_q32
                &format!("simplex::{}", builtin.file_name)
            } else {
                "fixed32"
            };
            output.push_str(&format!(
                "        BuiltinId::{} => {}::{} as *const u8,\n",
                builtin.enum_variant, module_path, builtin.function_name
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
        // Split builtins by module (fixed32 vs lpfx)
        let fixed32_builtins: Vec<_> = builtins
            .iter()
            .filter(|b| !b.function_name.starts_with("__lpfx_"))
            .collect();
        let hash_builtins: Vec<_> = builtins
            .iter()
            .filter(|b| b.function_name.starts_with("__lpfx_hash_"))
            .collect();
        let simplex_builtins: Vec<_> = builtins
            .iter()
            .filter(|b| b.function_name.contains("lpfx_simplex"))
            .collect();

        // Generate imports for fixed32 functions
        if !fixed32_builtins.is_empty() {
            output.push_str("use lp_builtins::builtins::fixed32::{\n");
            for (i, builtin) in fixed32_builtins.iter().enumerate() {
                if i > 0 {
                    output.push_str(",\n");
                }
                output.push_str(&format!("    {}", builtin.function_name));
            }
            output.push_str(",\n};\n");
        }

        // Generate imports for hash functions (lpfx::hash)
        if !hash_builtins.is_empty() {
            output.push_str("use lp_builtins::builtins::lpfx::hash::{\n");
            for (i, builtin) in hash_builtins.iter().enumerate() {
                if i > 0 {
                    output.push_str(",\n");
                }
                output.push_str(&format!("    {}", builtin.function_name));
            }
            output.push_str(",\n};\n");
        }

        // Generate imports for simplex functions (lpfx::simplex)
        if !simplex_builtins.is_empty() {
            output.push_str("use lp_builtins::builtins::lpfx::simplex::{\n");
            for (i, builtin) in simplex_builtins.iter().enumerate() {
                if i > 0 {
                    output.push_str(",\n");
                }
                // Extract module name from file_name (e.g., "simplex1_q32" -> "simplex1_q32")
                let module_name = &builtin.file_name;
                output.push_str(&format!("    {}::{}", module_name, builtin.function_name));
            }
            output.push_str(",\n};\n");
        }
        output.push('\n');
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
        // Use the extracted Rust signature - this should always be available
        let fn_type = &builtin.rust_signature;
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
        use lp_glsl_compiler::frontend::semantic::lpfx::lpfx_fns::lpfx_fns;

        // Build a map of BuiltinId -> LpfxFn for quick lookup
        use std::collections::HashMap;
        let mut builtin_to_func = HashMap::new();
        for func in lpfx_fns() {
            match &func.impls {
                lp_glsl_compiler::frontend::semantic::lpfx::lpfx_fn::LpfxFnImpl::NonDecimal(
                    builtin_id,
                ) => {
                    builtin_to_func.insert(*builtin_id, func);
                }
                lp_glsl_compiler::frontend::semantic::lpfx::lpfx_fn::LpfxFnImpl::Decimal {
                    float_impl,
                    q32_impl,
                } => {
                    builtin_to_func.insert(*float_impl, func);
                    builtin_to_func.insert(*q32_impl, func);
                }
            }
        }

        for builtin in builtins {
            // Check if this is an LPFX function by checking if symbol name starts with __lpfx_
            if builtin.symbol_name.starts_with("__lpfx_") {
                // For Fixed32 transform, skip f32 variants - only generate Q32 mappings
                if builtin.symbol_name.ends_with("_f32") {
                    continue;
                }

                // Parse BuiltinId from enum variant - try to match it
                let builtin_id_opt = builtin_to_func
                    .keys()
                    .find(|id| format!("{:?}", id) == builtin.enum_variant);

                if let Some(builtin_id) = builtin_id_opt
                    && let Some(func) = builtin_to_func.get(builtin_id)
                {
                    if builtin.symbol_name.starts_with("__lpfx_hash_") {
                        // Hash functions: use testcase pattern "1f" | "__lp_1"
                        let base_name = strip_function_prefix(&builtin.symbol_name);
                        let c_name = format!("{}f", base_name);
                        let intrinsic_name = format!("__lp_{}", base_name);
                        new_function.push_str(&format!(
                            "        \"{}\" | \"{}\" => Some((BuiltinId::{}, {})),\n",
                            c_name, intrinsic_name, builtin.enum_variant, builtin.param_count
                        ));
                    } else {
                        // Simplex functions: use testcase name (GLSL name with __ prefix)
                        // Only Q32 variants reach here (f32 filtered above)
                        let testcase_name = format!("__{}", func.glsl_sig.name);
                        new_function.push_str(&format!(
                            "        \"{}\" => Some((BuiltinId::{}, {})),\n",
                            testcase_name, builtin.enum_variant, builtin.param_count
                        ));
                    }
                    continue;
                }
            }
            // Not an LPFX function or lookup failed - treat as regular fixed32 function
            // Regular fixed32 functions
            let base_name = strip_function_prefix(&builtin.symbol_name);

            // Generate C math function name (e.g., lp_fixed32_sinf)
            let c_name = format!("{}f", base_name);

            // Generate intrinsic name - for fixed32 functions, use the symbol name directly
            // (e.g., __lp_fixed32_sin)
            let intrinsic_name = builtin.symbol_name.clone();

            // Extract standard C math function name (e.g., "sinf" from "__lp_fixed32_sin")
            // Pattern: __lp_fixed32_<name> -> <name>f
            let standard_c_name =
                if let Some(name_part) = builtin.symbol_name.strip_prefix("__lp_fixed32_") {
                    format!("{}f", name_part)
                } else {
                    String::new()
                };

            // Special case: GLSL's mod() compiles to fmodf, not modf
            // Check if function name ends with _mod (e.g., __lp_fixed32_mod)
            let additional_names = if builtin.symbol_name.ends_with("_mod")
                || builtin.symbol_name.ends_with("_mod\"")
            {
                " | \"fmodf\"".to_string()
            } else if !standard_c_name.is_empty() {
                // Add standard C math function name (e.g., "sinf", "cosf")
                format!(" | \"{}\"", standard_c_name)
            } else {
                String::new()
            };

            new_function.push_str(&format!(
                "        \"{}\" | \"{}\"{additional_names} => Some((BuiltinId::{}, {})),\n",
                c_name, intrinsic_name, builtin.enum_variant, builtin.param_count
            ));
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
