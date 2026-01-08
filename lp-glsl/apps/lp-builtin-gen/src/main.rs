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

    let builtins = discover_builtins(&fixed32_dir).expect("Failed to discover builtins");

    // Generate registry.rs
    let registry_path =
        workspace_root.join("lp-glsl/crates/lp-glsl-compiler/src/backend/builtins/registry.rs");
    generate_registry(&registry_path, &builtins);

    // Generate builtin_refs.rs
    let builtin_refs_path =
        workspace_root.join("lp-glsl/apps/lp-builtins-app/src/builtin_refs.rs");
    generate_builtin_refs(&builtin_refs_path, &builtins);

    // Generate mod.rs
    let mod_rs_path = workspace_root.join("lp-glsl/crates/lp-builtins/src/builtins/fixed32/mod.rs");
    generate_mod_rs(&mod_rs_path, &builtins);

    // Generate testcase mapping in math.rs
    let math_rs_path = workspace_root
        .join("lp-glsl/crates/lp-glsl-compiler/src/backend/transform/fixed32/converters/math.rs");
    generate_testcase_mapping(&math_rs_path, &builtins);

    println!("Generated all builtin boilerplate files");
}

fn find_workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut current = std::env::current_dir()?;
    loop {
        if current.join("lp-glsl/Cargo.toml").exists() {
            return Ok(current);
        }
        if !current.pop() {
            return Err("Could not find workspace root".into());
        }
    }
}

fn discover_builtins(dir: &Path) -> Result<Vec<BuiltinInfo>, Box<dyn std::error::Error>> {
    let mut builtins = Vec::new();

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
                if let Some(builtin) = extract_builtin(&func, file_name) {
                    builtins.push(builtin);
                }
            }
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

    // Check if function name starts with __lp_fixed32_
    let func_name = func.sig.ident.to_string();
    if !func_name.starts_with("__lp_fixed32_") {
        return None;
    }

    // Extract symbol name (function name)
    let symbol_name = func_name.clone();

    // Derive enum variant name: __lp_fixed32_sqrt -> Fixed32Sqrt
    let enum_variant = symbol_name
        .strip_prefix("__lp_fixed32_")
        .unwrap()
        .split('_')
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + &chars.as_str(),
            }
        })
        .collect::<String>();

    // Count parameters (extern "C" functions don't have self)
    let param_count = func.sig.inputs.len();

    Some(BuiltinInfo {
        enum_variant: format!("Fixed32{}", capitalize_first(&enum_variant)),
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
    output.push_str("\n");

    output.push_str("use crate::error::{ErrorCode, GlslError};\n");
    output.push_str("use cranelift_codegen::ir::{AbiParam, Signature, types};\n");
    output.push_str("use cranelift_codegen::isa::CallConv;\n");
    output.push_str("use cranelift_module::{Linkage, Module};\n\n");

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
        let ternary_ops: Vec<_> = builtins.iter().filter(|b| b.param_count == 3).collect();
        let binary_ops: Vec<_> = builtins.iter().filter(|b| b.param_count == 2).collect();
        let unary_ops: Vec<_> = builtins.iter().filter(|b| b.param_count == 1).collect();

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
    output.push_str(
        "                    format!(\"Failed to declare builtin '{}': {}\", name, e),\n",
    );
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
        let fn_type = if builtin.param_count == 3 {
            "extern \"C\" fn(i32, i32, i32) -> i32"
        } else if builtin.param_count == 2 {
            "extern \"C\" fn(i32, i32) -> i32"
        } else {
            "extern \"C\" fn(i32) -> i32"
        };
        let var_suffix = builtin.function_name.strip_prefix("__lp_fixed32_").unwrap();
        output.push_str(&format!(
            "        let _{var_suffix}_fn: {fn_type} = {};\n",
            builtin.function_name
        ));
    }

    output.push_str("\n");
    output.push_str("        // Force these to be included by using them in a way that can't be optimized away\n");
    output.push_str("        // We'll use volatile reads to prevent optimization\n");

    // Generate read_volatile calls
    for builtin in builtins {
        let var_name = format!(
            "_{}_fn",
            builtin.function_name.strip_prefix("__lp_fixed32_").unwrap()
        );
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
    output.push_str("\n");

    // Generate mod declarations
    for builtin in builtins {
        output.push_str(&format!("mod {};\n", builtin.file_name));
    }
    output.push_str("\n");
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
        for builtin in builtins {
        let base_name = builtin.symbol_name.strip_prefix("__lp_fixed32_").unwrap();

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

    new_function.push_str("        _ => None,\n");
    new_function.push_str("    }\n");
    new_function.push_str("}\n");

    let new_content = format!("{}{}{}", before, new_function, after);
    fs::write(path, new_content).expect("Failed to write math.rs");
}
