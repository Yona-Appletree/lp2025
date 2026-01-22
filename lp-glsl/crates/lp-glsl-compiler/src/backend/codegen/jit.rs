//! JIT codegen - build executable from GlModule<JITModule>

use crate::backend::module::gl_module::GlModule;
use crate::error::{ErrorCode, GlslError};
use crate::exec::jit::GlslJitModule;
use alloc::{format, string::String, vec::Vec};
use cranelift_jit::JITModule;
use cranelift_module::Module;
use hashbrown::HashMap;

/// Build JIT executable from GlModule<JITModule>
/// Called by GlModule<JITModule>::build_executable()
pub fn build_jit_executable(
    mut gl_module: GlModule<JITModule>,
) -> Result<GlslJitModule, GlslError> {
    // Builtin functions are already declared when the module was created

    // 1. Define all functions (compile them)
    // Collect function data first to avoid borrowing conflicts
    let funcs: Vec<(
        String,
        cranelift_codegen::ir::Function,
        cranelift_module::FuncId,
    )> = gl_module
        .fns
        .iter()
        .map(|(name, gl_func)| (name.clone(), gl_func.function.clone(), gl_func.func_id))
        .collect();

    for (name, func, func_id) in funcs {
        // Create context using immutable borrow
        let mut ctx = {
            let module_ref = gl_module.module_internal();
            module_ref.make_context()
        };
        ctx.func = func;
        // Define function using mutable borrow
        gl_module
            .module_mut_internal()
            .define_function(func_id, &mut ctx)
            .map_err(|e| {
                GlslError::new(
                    ErrorCode::E0400,
                    format!("Failed to define function '{name}': {e}"),
                )
            })?;
        // Clear context using immutable borrow
        {
            let module_ref = gl_module.module_internal();
            module_ref.clear_context(&mut ctx);
        }
    }

    // 2. Finalize definitions
    gl_module
        .module_mut_internal()
        .finalize_definitions()
        .map_err(|e| {
            GlslError::new(
                ErrorCode::E0400,
                format!("Failed to finalize definitions: {e}"),
            )
        })?;

    // 3. Extract function pointers
    let mut function_ptrs = HashMap::new();
    for (name, gl_func) in &gl_module.fns {
        let ptr = gl_module
            .module_internal()
            .get_finalized_function(gl_func.func_id);
        function_ptrs.insert(name.clone(), ptr);
    }

    // 3. Build signatures map from GlModule metadata
    let signatures = gl_module.glsl_signatures.clone();
    let mut cranelift_signatures = HashMap::new();
    for (name, gl_func) in &gl_module.fns {
        cranelift_signatures.insert(name.clone(), gl_func.clif_sig.clone());
    }

    // 4. Get target properties (requires mutable reference for ISA caching)
    let call_conv = gl_module
        .target
        .default_call_conv()
        .map_err(|e| GlslError::new(ErrorCode::E0400, format!("Failed to get call conv: {e}")))?;
    let pointer_type = gl_module.target.pointer_type().map_err(|e| {
        GlslError::new(ErrorCode::E0400, format!("Failed to get pointer type: {e}"))
    })?;

    // 5. Create GlslJitModule
    Ok(GlslJitModule {
        jit_module: gl_module.into_module(),
        function_ptrs,
        signatures,
        cranelift_signatures,
        call_conv,
        pointer_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::module::gl_module::GlModule;
    use crate::backend::module::test_helpers::test_helpers::build_simple_function;
    use crate::backend::target::Target;
    use cranelift_codegen::ir::{AbiParam, InstBuilder, Signature, types};
    use cranelift_codegen::isa::CallConv;
    use cranelift_module::Linkage;

    #[test]
    #[cfg(feature = "std")]
    fn test_build_jit_executable() {
        use crate::exec::executable::GlslExecutable;

        let target = Target::host_jit().unwrap();
        let mut gl_module = GlModule::new_jit(target).unwrap();

        // Build a simple function that returns 42
        let mut sig = Signature::new(CallConv::SystemV);
        sig.returns.push(AbiParam::new(types::I32));

        build_simple_function(&mut gl_module, "main", Linkage::Export, sig, |builder| {
            let val = builder.ins().iconst(types::I32, 42);
            builder.ins().return_(&[val]);
            Ok(())
        })
        .unwrap();

        // Build executable
        let mut executable = build_jit_executable(gl_module).unwrap();
        assert!(executable.function_ptrs.contains_key("main"));

        // Actually call the function and verify it returns 42
        let result = executable.call_i32("main", &[]).unwrap();
        assert_eq!(result, 42);
    }
}
