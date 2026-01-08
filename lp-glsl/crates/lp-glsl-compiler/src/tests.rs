//! Tests for backend2 Phase 1

#[cfg(feature = "std")]
mod backend2_phase1 {
    use crate::backend2::codegen::build_jit_executable;
    use crate::backend2::module::builder::build_simple_function_jit;
    use crate::backend2::module::GlJitModule;
    use crate::backend2::target::TargetSpec;
    use crate::exec::executable::GlslExecutable;
    use cranelift_codegen::ir::{types, AbiParam, InstBuilder, Signature};
    use cranelift_codegen::isa::CallConv;
    use cranelift_module::{Linkage, Module};

    /// Build the test functions (add and main) into a GlJitModule
    /// This simulates what the frontend will do - build functions directly in the Module
    fn build_test_functions_jit(gl_module: &mut GlJitModule) -> Result<(), crate::error::GlslError> {
        // Build helper: add(a: i32, b: i32) -> i32
        let mut add_sig = Signature::new(CallConv::SystemV);
        add_sig.params.push(AbiParam::new(types::I32));
        add_sig.params.push(AbiParam::new(types::I32));
        add_sig.returns.push(AbiParam::new(types::I32));

        build_simple_function_jit(gl_module, "add", Linkage::Local, add_sig, |builder, _module| {
            let entry = builder.current_block().unwrap();
            let a = builder.block_params(entry)[0];
            let b = builder.block_params(entry)[1];
            let sum = builder.ins().iadd(a, b);
            builder.ins().return_(&[sum]);
            Ok(())
        })?;

        // Build main: main() -> add(10, 20)
        let mut main_sig = Signature::new(CallConv::SystemV);
        main_sig.returns.push(AbiParam::new(types::I32));

        // Extract func_id before closure to avoid borrowing conflicts
        let add_func_id = gl_module.get_func("add")
            .ok_or_else(|| crate::error::GlslError::new(crate::error::ErrorCode::E0400, "Function 'add' not found"))?
            .func_id;

        build_simple_function_jit(gl_module, "main", Linkage::Export, main_sig, move |builder, module| {
            let ten = builder.ins().iconst(types::I32, 10);
            let twenty = builder.ins().iconst(types::I32, 20);

            // Create FuncRef using the captured func_id
            let add_ref = module.declare_func_in_func(add_func_id, builder.func);

            // Call add(10, 20)
            let call_result = builder.ins().call(add_ref, &[ten, twenty]);
            let result = builder.inst_results(call_result)[0];

            builder.ins().return_(&[result]);
            Ok(())
        })?;

        Ok(())
    }

    /// Test 1: JIT - Simple Function Call
    #[test]
    fn test_jit_function_call() {
        // Create JIT target spec
        let target = TargetSpec::host_jit().expect("Failed to create JIT target spec");

        // Create GlModule
        let mut gl_module = GlJitModule::new(target).expect("Failed to create GlJitModule");

        // Build functions (same logic works for both JIT and emulator!)
        build_test_functions_jit(&mut gl_module).expect("Failed to build test functions");

        // Build executable and test
        let mut executable = build_jit_executable(gl_module).expect("Failed to build JIT executable");
        let result = executable.call_i32("main", &[]).expect("Failed to call main");
        assert_eq!(result, 30);
    }
}
