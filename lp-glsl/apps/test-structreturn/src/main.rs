//! Direct CLIF test for StructReturn JIT execution
//! Compare assembly of native Rust vs JIT-compiled StructReturn functions

use cranelift_codegen::ir::{AbiParam, ArgumentPurpose, InstBuilder, MemFlags};
use cranelift_codegen::isa::{lookup as isa_lookup, CallConv};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use lp_jit_util::call_structreturn;
use std::fs;
use target_lexicon::Triple;

/// Native Rust function that mimics StructReturn calling convention
/// This is what we expect the JIT function to match
#[no_mangle]
pub extern "C" fn native_structreturn_vec3(buffer: *mut f32) {
    unsafe {
        *buffer.add(0) = 1.0;
        *buffer.add(1) = 2.0;
        *buffer.add(2) = 3.0;
    }
}

/// Test StructReturn for a specific ISA by directly building CLIF
fn test_structreturn_clif(
    triple: Triple,
    isa_name: &str,
    expected_values: &[f32],
) -> Result<(), String> {
    println!("\n=== Testing StructReturn CLIF on {} ===", isa_name);

    // Create ISA
    let mut flag_builder = settings::builder();
    flag_builder.set("use_colocated_libcalls", "false").unwrap();
    flag_builder.set("opt_level", "none").unwrap();

    let isa_builder = isa_lookup(triple.clone())
        .map_err(|e| format!("Failed to lookup ISA for {}: {:?}", isa_name, e))?;

    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .map_err(|e| format!("Failed to create ISA for {}: {:?}", isa_name, e))?;

    let jit_builder = JITBuilder::with_isa(isa.clone(), cranelift_module::default_libcall_names());
    let mut module = JITModule::new(jit_builder);

    // Get calling convention for this triple
    let call_conv = CallConv::triple_default(&triple);
    let pointer_type = module.isa().pointer_type();

    println!("Calling convention: {:?}", call_conv);
    println!("Pointer type: {:?}", pointer_type);

    // Create function signature with StructReturn
    let mut sig = cranelift_codegen::ir::Signature::new(call_conv);

    // Add StructReturn parameter FIRST (critical for ABI)
    sig.params.push(AbiParam::special(
        pointer_type,
        ArgumentPurpose::StructReturn,
    ));
    // Returns void
    sig.returns.clear();

    println!("Signature: {:?}", sig);

    let func_id = module
        .declare_function("test_vec", Linkage::Export, &sig)
        .map_err(|e| format!("Failed to declare function: {:?}", e))?;

    // Build the function
    let mut ctx = module.make_context();
    ctx.func.signature = sig;

    // Get StructReturn parameter index
    let struct_ret_index = ctx
        .func
        .signature
        .special_param_index(ArgumentPurpose::StructReturn)
        .ok_or_else(|| "StructReturn parameter not found".to_string())?;

    println!("StructReturn parameter index: {}", struct_ret_index);

    let mut builder_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);

    let entry = builder.create_block();
    builder.append_block_params_for_function_params(entry);
    builder.switch_to_block(entry);
    builder.seal_block(entry);

    // Get StructReturn pointer from block params
    let ret_ptr = builder.block_params(entry)[struct_ret_index];

    // Write values to the return buffer
    for (i, &val) in expected_values.iter().enumerate() {
        let const_val = builder.ins().f32const(val);
        let offset = (i * 4) as i32; // 4 bytes per f32
        builder
            .ins()
            .store(MemFlags::trusted(), const_val, ret_ptr, offset);
    }

    builder.ins().return_(&[]);
    builder.finalize();

    println!("Function CLIF:\n{}", &ctx.func);

    // Compile and save the code buffer before finalizing
    println!("Compiling function...");
    module
        .define_function(func_id, &mut ctx)
        .map_err(|e| format!("Failed to define function: {:?}", e))?;

    // Get the compiled code buffer for disassembly
    let mut ctx2 = module.make_context();
    ctx2.func = ctx.func.clone();
    let compiled = ctx2
        .compile(module.isa(), &mut Default::default())
        .map_err(|e| format!("Failed to compile for disassembly: {:?}", e))?;
    let code_buffer = compiled.code_buffer();

    // Save JIT code to file for disassembly
    fs::write("/tmp/jit_code.bin", code_buffer)
        .map_err(|e| format!("Failed to write JIT code: {}", e))?;
    println!(
        "Saved JIT code ({} bytes) to /tmp/jit_code.bin",
        code_buffer.len()
    );

    println!("Finalizing definitions...");
    module
        .finalize_definitions()
        .map_err(|e| format!("Failed to finalize definitions: {:?}", e))?;

    // Get function pointer
    println!("Getting function pointer...");
    let code_ptr = module.get_finalized_function(func_id);
    println!("Function pointer: {:p}", code_ptr);

    // Test native function first
    println!("\n--- Testing native Rust function ---");
    let mut buffer_native = vec![0.0f32; expected_values.len()];
    native_structreturn_vec3(buffer_native.as_mut_ptr());
    println!("Native result: {:?}", buffer_native);
    if buffer_native == expected_values {
        println!("✅ Native function works!");
    } else {
        return Err(format!(
            "Native function failed: expected {:?}, got {:?}",
            expected_values, buffer_native
        ));
    }

    // Call the JIT function using the correct calling convention
    println!("\n--- Testing JIT function ---");
    let buffer_size = expected_values.len();
    let mut buffer = vec![0.0f32; buffer_size];
    let buffer_ptr = buffer.as_mut_ptr();

    println!("Buffer ptr: {:p}, size: {}", buffer_ptr, buffer_size);
    println!("Buffer alignment: {}", std::mem::align_of::<f32>());

    // Use the utility function to handle platform-specific calling conventions
    println!("Using calling convention: {:?}", call_conv);
    unsafe {
        call_structreturn(code_ptr, buffer_ptr, buffer_size, call_conv, pointer_type)
            .map_err(|e| format!("StructReturn call failed: {}", e))?;
    }

    println!("Function returned");
    println!("Result: {:?}", buffer);
    println!("Expected: {:?}", expected_values);

    // Verify
    if buffer == expected_values {
        println!("✅ SUCCESS: StructReturn works correctly on {}!", isa_name);
        Ok(())
    } else {
        Err(format!(
            "❌ FAILED on {}: Expected {:?}, got {:?}",
            isa_name, expected_values, buffer
        ))
    }
}

/// Test on native ISA
fn test_native() -> Result<(), String> {
    let triple = Triple::host();
    let isa_name = format!("native ({})", triple);
    test_structreturn_clif(triple, &isa_name, &[1.0, 2.0, 3.0])
}

fn main() {
    println!("Testing StructReturn with direct CLIF JIT execution...");
    println!("Comparing native Rust vs JIT-compiled assembly\n");

    match test_native() {
        Ok(()) => {
            println!("\n🎉 StructReturn JIT test passed!");
            println!("\nTo examine assembly:");
            println!(
                "  otool -tv target/release/test-structreturn | grep -A 20 native_structreturn"
            );
            println!("  otool -tv /tmp/jit_code.bin");
        }
        Err(e) => {
            println!("\n❌ StructReturn JIT test failed: {}", e);
            println!("\nTo examine assembly:");
            println!(
                "  otool -tv target/release/test-structreturn | grep -A 20 native_structreturn"
            );
            println!("  otool -tv /tmp/jit_code.bin");
            std::process::exit(1);
        }
    }
}
