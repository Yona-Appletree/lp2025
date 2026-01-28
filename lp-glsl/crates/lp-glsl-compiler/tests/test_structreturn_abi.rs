//! Minimal isolated test for StructReturn ABI
//!
//! This test creates a Cranelift JIT function that:
//! 1. Calls a native Rust extern "C" function that returns [i32; 3]
//! 2. Sums the three returned values
//! 3. Returns the sum
//!
//! With enable_multi_ret_implicit_sret enabled, Cranelift automatically decides:
//! - On ARM64: returns via registers (x0, x1) - matches Rust's ABI
//! - On RISC-V32: uses StructReturn - matches Rust's ABI
//!
//! Goal: Verify that Cranelift's automatic StructReturn handling matches Rust's ABI.

#[cfg(feature = "std")]
#[test]
fn test_structreturn_abi_minimal() {
    use cranelift_codegen::ir::ArgumentPurpose;
    use cranelift_codegen::ir::{AbiParam, InstBuilder, Signature, types};
    use cranelift_codegen::isa::CallConv;
    use cranelift_codegen::settings::{self, Configurable};
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module};
    use target_lexicon::Triple;

    // Setup ISA and calling convention
    let triple = Triple::host();
    let isa_builder = cranelift_native::builder().expect("Failed to create ISA builder");

    // Enable implicit StructReturn - Cranelift will automatically use StructReturn
    // when multiple return values don't fit in registers (platform-dependent)
    let mut flag_builder = settings::builder();
    flag_builder
        .set("enable_multi_ret_implicit_sret", "true")
        .expect("Failed to set enable_multi_ret_implicit_sret");
    let flags = settings::Flags::new(flag_builder);

    let isa = isa_builder.finish(flags).expect("Failed to create ISA");
    let pointer_type = isa.pointer_type();
    let call_conv = CallConv::triple_default(&triple);

    // Create JIT module and register the native function
    let mut jit_builder =
        JITBuilder::with_isa(isa.clone(), cranelift_module::default_libcall_names());
    jit_builder.symbol("test_structreturn", test_structreturn as *const u8);
    let mut jit_module = JITModule::new(jit_builder);

    // Define external function signature: () -> i32, i32, i32
    // With enable_multi_ret_implicit_sret, Cranelift will automatically:
    // - On ARM64: return via registers (x0, x1) if they fit
    // - On RISC-V32: use StructReturn if they don't fit in registers
    // This matches what Rust does automatically!
    let mut ext_sig = Signature::new(call_conv);
    ext_sig.returns.push(AbiParam::new(types::I32));
    ext_sig.returns.push(AbiParam::new(types::I32));
    ext_sig.returns.push(AbiParam::new(types::I32));

    let ext_func_id = jit_module
        .declare_function("test_structreturn", Linkage::Import, &ext_sig)
        .unwrap();

    // Define main function: () -> i32
    let mut main_sig = Signature::new(call_conv);
    main_sig.returns.push(AbiParam::new(types::I32));

    let main_func_id = jit_module
        .declare_function("main", Linkage::Export, &main_sig)
        .unwrap();

    // Build main function body
    let mut ctx = jit_module.make_context();
    ctx.func.signature = main_sig.clone();
    ctx.func.name = cranelift_codegen::ir::UserFuncName::user(0, 0);

    // Create entry block (no parameters since main takes no args)
    let entry_block = ctx.func.dfg.make_block();
    ctx.func.layout.append_block(entry_block);

    // Set up function builder
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    let mut builder_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);
    builder.switch_to_block(entry_block);
    builder.seal_block(entry_block);

    // Convert FuncId to FuncRef for the call (must be done before using builder)
    let ext_func_ref = jit_module.declare_func_in_func(ext_func_id, &mut builder.func);

    // Check if Cranelift automatically added StructReturn (depends on platform)
    let ext_func_data = &builder.func.dfg.ext_funcs[ext_func_ref];
    let sig_ref = ext_func_data.signature;
    let uses_struct_return = builder.func.dfg.signatures[sig_ref]
        .params
        .iter()
        .any(|p| p.purpose == ArgumentPurpose::StructReturn);

    let (val0, val1, val2) = if uses_struct_return {
        // StructReturn path: allocate buffer and call
        let buffer_slot =
            builder
                .func
                .create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                    cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                    12, // 3 * 4 bytes
                    4,  // 4-byte alignment
                ));
        let buffer_ptr = builder.ins().stack_addr(pointer_type, buffer_slot, 0);

        builder.ins().call(ext_func_ref, &[buffer_ptr]);

        // Load values from buffer
        let v0 = builder.ins().load(
            types::I32,
            cranelift_codegen::ir::MemFlags::trusted(),
            buffer_ptr,
            0,
        );
        let v1 = builder.ins().load(
            types::I32,
            cranelift_codegen::ir::MemFlags::trusted(),
            buffer_ptr,
            4,
        );
        let v2 = builder.ins().load(
            types::I32,
            cranelift_codegen::ir::MemFlags::trusted(),
            buffer_ptr,
            8,
        );
        (v0, v1, v2)
    } else {
        // Register return path: extract from call results
        // On ARM64 with enable_multi_ret_implicit_sret, Cranelift returns 3 i32 values directly
        // (not packed in I64 - it uses multiple return registers)
        let call_result = builder.ins().call(ext_func_ref, &[]);
        let results = builder.inst_results(call_result);

        // Results are already i32 values (one per return register)
        let v0 = results[0];
        let v1 = results[1];
        let v2 = results[2];

        (v0, v1, v2)
    };

    // Sum the three values
    let sum1 = builder.ins().iadd(val0, val1);
    let sum2 = builder.ins().iadd(sum1, val2);

    // Return the sum
    builder.ins().return_(&[sum2]);
    builder.finalize();

    // Print CLIF IR before compilation
    println!("\n=== CLIF IR ===");
    use cranelift_codegen::write_function;
    let mut clif_buf = String::new();
    write_function(&mut clif_buf, &ctx.func).unwrap();
    println!("{}", clif_buf);

    // Enable disassembly for assembly output
    ctx.set_disasm(true);

    // Compile and finalize
    jit_module
        .define_function(main_func_id, &mut ctx)
        .expect("Failed to define main function");

    // Print VCode and assembly
    if let Some(compiled_code) = ctx.compiled_code() {
        if let Some(ref vcode) = compiled_code.vcode {
            println!("\n=== VCode ===");
            println!("{}", vcode);
        }

        // Try to generate disassembly
        let disasm = {
            let isa = jit_module.isa();
            if let Ok(cs) = isa.to_capstone() {
                if let Ok(disasm_str) = compiled_code.disassemble(Some(&ctx.func.params), &cs) {
                    Some(disasm_str)
                } else {
                    compiled_code.vcode.clone()
                }
            } else {
                compiled_code.vcode.clone()
            }
        };

        if let Some(ref disasm) = disasm {
            println!("\n=== Assembly ===");
            println!("{}", disasm);
        }
    }

    jit_module.finalize_definitions().unwrap();

    // Call the compiled function
    let main_ptr = jit_module.get_finalized_function(main_func_id);

    unsafe {
        let main_fn: extern "C" fn() -> i32 = std::mem::transmute(main_ptr);
        let result = main_fn();
        assert_eq!(result, 6, "Expected sum of [1, 2, 3] = 6");
    }
}

// Native Rust function that returns a struct
// Rust's ABI: returns via registers on ARM64, StructReturn on RISC-V32
extern "C" fn test_structreturn() -> [i32; 3] {
    [1, 2, 3]
}
