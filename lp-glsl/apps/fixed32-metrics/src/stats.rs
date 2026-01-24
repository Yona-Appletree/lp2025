use anyhow::Result;
use cranelift_codegen::ir::Function;
use cranelift_jit::JITModule;
use hashbrown::HashMap;
use lp_glsl_compiler::backend::module::gl_module::GlModule;
use lp_glsl_compiler::backend::util::clif_format::format_function;

#[derive(Debug, Clone, serde::Serialize)]
pub struct FunctionStats {
    pub name: String,
    pub blocks: usize,
    pub instructions: usize,
    pub values: usize,
    pub clif_size: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModuleStats {
    pub total_blocks: usize,
    pub total_instructions: usize,
    pub total_values: usize,
    pub total_clif_size: usize,
    pub functions: Vec<FunctionStats>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StatsDelta {
    pub blocks: i32,
    pub instructions: i32,
    pub values: i32,
    pub clif_size: i32,
    pub blocks_percent: f64,
    pub instructions_percent: f64,
    pub values_percent: f64,
    pub clif_size_percent: f64,
}

pub fn collect_function_stats(
    func: &Function,
    name: &str,
    name_mapping: &HashMap<String, String>,
) -> Result<FunctionStats> {
    let blocks: Vec<_> = func.layout.blocks().collect();
    let num_blocks = blocks.len();
    let num_insts: usize = blocks
        .iter()
        .map(|block| func.layout.block_insts(*block).count())
        .sum();
    let num_values = func.dfg.num_values();

    let func_text = format_function(func, name, name_mapping)
        .map_err(|e| anyhow::anyhow!("Failed to format function: {}", e))?;
    let clif_size = func_text.len();

    Ok(FunctionStats {
        name: name.to_string(),
        blocks: num_blocks,
        instructions: num_insts,
        values: num_values,
        clif_size,
    })
}

pub fn collect_module_stats(module: &GlModule<JITModule>) -> Result<ModuleStats> {
    // Build name mapping
    let mut name_mapping: HashMap<String, String> = HashMap::new();
    for (name, gl_func) in &module.fns {
        name_mapping.insert(gl_func.func_id.as_u32().to_string(), name.clone());
    }

    let mut functions = Vec::new();
    let mut total_blocks = 0;
    let mut total_instructions = 0;
    let mut total_values = 0;
    let mut total_clif_size = 0;

    // Sort function names for deterministic output
    let mut func_names: Vec<String> = module.fns.keys().cloned().collect();
    func_names.sort();

    for name in &func_names {
        if let Some(gl_func) = module.fns.get(name) {
            let stats = collect_function_stats(&gl_func.function, name, &name_mapping)?;
            total_blocks += stats.blocks;
            total_instructions += stats.instructions;
            total_values += stats.values;
            total_clif_size += stats.clif_size;
            functions.push(stats);
        }
    }

    Ok(ModuleStats {
        total_blocks,
        total_instructions,
        total_values,
        total_clif_size,
        functions,
    })
}

pub fn calculate_deltas(before: &ModuleStats, after: &ModuleStats) -> StatsDelta {
    let blocks_diff = after.total_blocks as i32 - before.total_blocks as i32;
    let insts_diff = after.total_instructions as i32 - before.total_instructions as i32;
    let values_diff = after.total_values as i32 - before.total_values as i32;
    let size_diff = after.total_clif_size as i32 - before.total_clif_size as i32;

    let blocks_percent = if before.total_blocks > 0 {
        (blocks_diff as f64 / before.total_blocks as f64) * 100.0
    } else {
        0.0
    };
    let insts_percent = if before.total_instructions > 0 {
        (insts_diff as f64 / before.total_instructions as f64) * 100.0
    } else {
        0.0
    };
    let values_percent = if before.total_values > 0 {
        (values_diff as f64 / before.total_values as f64) * 100.0
    } else {
        0.0
    };
    let size_percent = if before.total_clif_size > 0 {
        (size_diff as f64 / before.total_clif_size as f64) * 100.0
    } else {
        0.0
    };

    StatsDelta {
        blocks: blocks_diff,
        instructions: insts_diff,
        values: values_diff,
        clif_size: size_diff,
        blocks_percent,
        instructions_percent: insts_percent,
        values_percent,
        clif_size_percent: size_percent,
    }
}

pub fn collect_function_reports(
    module_before: &GlModule<JITModule>,
    module_after: &GlModule<JITModule>,
) -> Result<Vec<crate::report::FunctionReport>> {
    // Build name mappings
    let mut name_mapping_before: HashMap<String, String> = HashMap::new();
    for (name, gl_func) in &module_before.fns {
        name_mapping_before.insert(gl_func.func_id.as_u32().to_string(), name.clone());
    }

    let mut name_mapping_after: HashMap<String, String> = HashMap::new();
    for (name, gl_func) in &module_after.fns {
        name_mapping_after.insert(gl_func.func_id.as_u32().to_string(), name.clone());
    }

    let mut reports = Vec::new();
    let mut func_names: Vec<String> = module_before.fns.keys().cloned().collect();
    func_names.sort();

    for name in &func_names {
        if let Some(gl_func_before) = module_before.fns.get(name)
            && let Some(gl_func_after) = module_after.fns.get(name)
        {
            let stats_before =
                collect_function_stats(&gl_func_before.function, name, &name_mapping_before)?;
            let stats_after =
                collect_function_stats(&gl_func_after.function, name, &name_mapping_after)?;
            let delta = calculate_function_delta(&stats_before, &stats_after);

            reports.push(crate::report::FunctionReport {
                name: name.clone(),
                before: stats_before,
                after: stats_after,
                delta,
            });
        }
    }

    Ok(reports)
}

fn calculate_function_delta(before: &FunctionStats, after: &FunctionStats) -> StatsDelta {
    let blocks_diff = after.blocks as i32 - before.blocks as i32;
    let insts_diff = after.instructions as i32 - before.instructions as i32;
    let values_diff = after.values as i32 - before.values as i32;
    let size_diff = after.clif_size as i32 - before.clif_size as i32;

    let blocks_percent = if before.blocks > 0 {
        (blocks_diff as f64 / before.blocks as f64) * 100.0
    } else {
        0.0
    };
    let insts_percent = if before.instructions > 0 {
        (insts_diff as f64 / before.instructions as f64) * 100.0
    } else {
        0.0
    };
    let values_percent = if before.values > 0 {
        (values_diff as f64 / before.values as f64) * 100.0
    } else {
        0.0
    };
    let size_percent = if before.clif_size > 0 {
        (size_diff as f64 / before.clif_size as f64) * 100.0
    } else {
        0.0
    };

    StatsDelta {
        blocks: blocks_diff,
        instructions: insts_diff,
        values: values_diff,
        clif_size: size_diff,
        blocks_percent,
        instructions_percent: insts_percent,
        values_percent,
        clif_size_percent: size_percent,
    }
}
