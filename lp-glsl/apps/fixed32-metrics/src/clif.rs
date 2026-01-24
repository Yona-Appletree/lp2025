use anyhow::Result;
use cranelift_jit::JITModule;
use hashbrown::HashMap;
use lp_glsl_compiler::backend::module::gl_module::GlModule;
use lp_glsl_compiler::backend::util::clif_format::format_function;
use std::fs;
use std::path::Path;

pub fn write_clif_files(
    test_dir: &Path,
    module_before: &GlModule<JITModule>,
    module_after: &GlModule<JITModule>,
    verbose: bool,
) -> Result<()> {
    // Build name mappings
    let mut name_mapping_before: HashMap<String, String> = HashMap::new();
    for (name, gl_func) in &module_before.fns {
        name_mapping_before.insert(gl_func.func_id.as_u32().to_string(), name.clone());
    }

    let mut name_mapping_after: HashMap<String, String> = HashMap::new();
    for (name, gl_func) in &module_after.fns {
        name_mapping_after.insert(gl_func.func_id.as_u32().to_string(), name.clone());
    }

    // Sort function names for deterministic output
    let mut func_names: Vec<String> = module_before.fns.keys().cloned().collect();
    func_names.sort();

    for name in &func_names {
        if let Some(gl_func_before) = module_before.fns.get(name)
            && let Some(gl_func_after) = module_after.fns.get(name)
        {
            // Format CLIF IR
            let clif_before = format_function(&gl_func_before.function, name, &name_mapping_before)
                .map_err(|e| {
                    anyhow::anyhow!("Failed to format function {} (before): {}", name, e)
                })?;
            let clif_after = format_function(&gl_func_after.function, name, &name_mapping_after)
                .map_err(|e| {
                    anyhow::anyhow!("Failed to format function {} (after): {}", name, e)
                })?;

            // Write files
            let pre_file = test_dir.join(format!("{}.pre.clif", name));
            let post_file = test_dir.join(format!("{}.post.clif", name));

            fs::write(&pre_file, &clif_before)
                .map_err(|e| anyhow::anyhow!("Failed to write {}: {}", pre_file.display(), e))?;
            fs::write(&post_file, &clif_after)
                .map_err(|e| anyhow::anyhow!("Failed to write {}: {}", post_file.display(), e))?;

            if verbose {
                eprintln!("  Wrote {}.pre.clif and {}.post.clif", name, name);
            }
        }
    }

    Ok(())
}
