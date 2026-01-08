//! Toy language JIT demonstration - TRUE runtime compilation!
//!
//! This demonstrates Cranelift compiling toy language source code at runtime
//! in a no_std RISC-V environment using the full toy language JIT pipeline.

extern crate alloc;

use alloc::vec::Vec;
use cranelift_codegen::isa::riscv32::isa_builder;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{
    Context,
    ir::{AbiParam, InstBuilder, types},
};
use cranelift_control::ControlPlane;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use hashbrown::HashMap;
use target_lexicon::Triple;

/// Compile a toy language function to RISC-V machine code.
fn compile_toy_function(
    params: Vec<alloc::string::String>,
    the_return: alloc::string::String,
    stmts: Vec<lp_toy_lang::frontend::Expr>,
    isa: &dyn cranelift_codegen::isa::TargetIsa,
) -> Result<Vec<u8>, alloc::string::String> {
    use lp_toy_lang::frontend::Expr;

    println!("  Creating Context...");
    println!(
        "    Context struct size: {} bytes",
        core::mem::size_of::<Context>()
    );
    let mut ctx = Context::new();
    let int = types::I32;

    // Setup function signature
    for _ in &params {
        ctx.func.signature.params.push(AbiParam::new(int));
    }
    ctx.func.signature.returns.push(AbiParam::new(int));

    println!("  Building function IR...");

    // Check available memory before we start
    unsafe extern "C" {
        static __heap_start: u8;
        static __heap_end: u8;
        static __stack_start: u8;
    }
    let heap_size = unsafe {
        core::ptr::addr_of!(__heap_end) as usize - core::ptr::addr_of!(__heap_start) as usize
    };
    println!(
        "  - Heap size configured: {} bytes ({} KB)",
        heap_size,
        heap_size / 1024
    );

    // Reset memory stats before compilation
    runtime_embive::reset_memory_stats();
    let (current_before, peak_before, _, count_before) = runtime_embive::get_memory_usage();
    println!(
        "  - Memory before IR build: {} bytes current, {} bytes peak, {} allocs",
        current_before, peak_before, count_before
    );

    // Build function
    println!("  - Creating builder context...");
    let mut builder_context = FunctionBuilderContext::new();
    println!("  - Creating function builder...");
    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_context);
    println!("  - Creating entry block...");
    let entry_block = builder.create_block();
    println!("  - Setting up entry block...");
    builder.append_block_params_for_function_params(entry_block);
    builder.switch_to_block(entry_block);
    builder.seal_block(entry_block);
    println!("  ✓ Entry block ready");

    // Declare variables
    println!("  - Declaring variables...");
    let mut variables = HashMap::new();
    for (i, name) in params.iter().enumerate() {
        let val = builder.block_params(entry_block)[i];
        let var = builder.declare_var(int);
        variables.insert(name.clone(), var);
        builder.def_var(var, val);
        println!("    - Param {}: {}", i, name);
    }

    let return_var = builder.declare_var(int);
    let zero = builder.ins().iconst(int, 0);
    variables.insert(the_return.clone(), return_var);
    builder.def_var(return_var, zero);
    println!("  ✓ Variables declared");

    // Translate statements
    println!("  - Translating {} statements...", stmts.len());
    for (i, expr) in stmts.iter().enumerate() {
        println!("    - Statement {}: {:?}", i, expr);
        translate_expr(&mut builder, &variables, expr);
    }
    println!("  ✓ Statements translated");

    // Return
    println!("  - Adding return...");
    let return_value = builder.use_var(return_var);
    builder.ins().return_(&[return_value]);

    println!("  - Finalizing IR...");
    builder.finalize();
    println!("  ✓ IR finalized");

    let (current_after_ir, peak_after_ir, _, count_after_ir) = runtime_embive::get_memory_usage();
    println!(
        "  - Memory after IR build: {} bytes current, {} bytes peak, {} allocs",
        current_after_ir, peak_after_ir, count_after_ir
    );

    // Compile - this is where it might fail
    println!("  ================================================");
    println!("  Starting Cranelift backend compilation...");
    println!(
        "  Memory stats: {} bytes in use, {} peak",
        current_after_ir, peak_after_ir
    );
    println!("  ================================================");

    // Add detailed tracing for the compile call
    println!("  [TRACE] About to call ctx.compile()");
    println!("  [TRACE] ISA: {}", isa.name());
    println!(
        "  [TRACE] Function has {} blocks, {} instructions",
        ctx.func.dfg.num_blocks(),
        ctx.func.dfg.num_insts()
    );

    // Check memory state right before compile
    let (mem_before, peak_before, total_before, count_before) = runtime_embive::get_memory_usage();
    println!(
        "  [PRE-COMPILE] Memory: {} bytes used, {} peak, {} allocs",
        mem_before, peak_before, count_before
    );

    // Try a test allocation to verify allocator is working
    println!("  [PRE-COMPILE] Testing 48-byte allocation...");
    let test_buf: alloc::vec::Vec<u8> = alloc::vec![0u8; 48];
    println!("  [PRE-COMPILE] ✓ 48-byte test allocation succeeded!");
    drop(test_buf);

    // Check stack pointer before compilation
    let sp: usize;
    unsafe {
        core::arch::asm!("mv {}, sp", out(reg) sp);
    }
    println!("  [DEBUG] Current stack pointer: 0x{:x}", sp);

    let stack_start = unsafe { core::ptr::addr_of!(__stack_start) as usize };
    let heap_start = unsafe { core::ptr::addr_of!(__heap_start) as usize };
    let heap_end = unsafe { core::ptr::addr_of!(__heap_end) as usize };

    println!("  [DEBUG] __stack_start: 0x{:x}", stack_start);
    println!("  [DEBUG] __heap_start: 0x{:x}", heap_start);
    println!("  [DEBUG] __heap_end: 0x{:x}", heap_end);
    println!(
        "  [DEBUG] Stack space available: {} bytes",
        stack_start - sp
    );

    // Try to compile with error handling
    println!("  [TRACE] Creating ControlPlane...");
    let mut ctrl_plane = ControlPlane::default();
    println!("  [TRACE] Entering ctx.compile()...");
    let code_info = match ctx.compile(isa, &mut ctrl_plane) {
        Ok(info) => {
            println!("  [TRACE] Compile succeeded!");
            info
        }
        Err(e) => {
            println!("  [TRACE] Compile returned error: {:?}", e);
            return Err(alloc::format!("Codegen failed: {:?}", e));
        }
    };
    println!("  [TRACE] After compile, code_info obtained");

    let (current_final, peak_final, total_allocated, count_final) =
        runtime_embive::get_memory_usage();
    println!("  ================================================");
    println!("  ✓✓✓ CODEGEN COMPLETE! ✓✓✓");
    println!("  Memory statistics:");
    println!(
        "    - Current usage: {} bytes ({} KB)",
        current_final,
        current_final / 1024
    );
    println!(
        "    - Peak usage: {} bytes ({} KB)",
        peak_final,
        peak_final / 1024
    );
    println!(
        "    - Total allocated: {} bytes ({} KB)",
        total_allocated,
        total_allocated / 1024
    );
    println!("    - Allocation count: {}", count_final);
    println!("  ================================================");

    Ok(code_info.buffer.data().to_vec())
}

fn translate_expr(
    builder: &mut FunctionBuilder,
    variables: &HashMap<alloc::string::String, Variable>,
    expr: &lp_toy_lang::frontend::Expr,
) -> cranelift_codegen::ir::Value {
    use lp_toy_lang::frontend::Expr;

    match expr {
        Expr::Literal(lit) => {
            let imm: i32 = lit.parse().unwrap_or(0);
            builder.ins().iconst(types::I32, i64::from(imm))
        }
        Expr::Identifier(name) => {
            let var = variables.get(name).unwrap();
            builder.use_var(*var)
        }
        Expr::Assign(name, expr) => {
            let value = translate_expr(builder, variables, expr);
            let var = variables.get(name).unwrap();
            builder.def_var(*var, value);
            value
        }
        Expr::Add(lhs, rhs) => {
            let l = translate_expr(builder, variables, lhs);
            let r = translate_expr(builder, variables, rhs);
            builder.ins().iadd(l, r)
        }
        Expr::Sub(lhs, rhs) => {
            let l = translate_expr(builder, variables, lhs);
            let r = translate_expr(builder, variables, rhs);
            builder.ins().isub(l, r)
        }
        Expr::Mul(lhs, rhs) => {
            let l = translate_expr(builder, variables, lhs);
            let r = translate_expr(builder, variables, rhs);
            builder.ins().imul(l, r)
        }
        _ => builder.ins().iconst(types::I32, 0),
    }
}

/// Run the toy language JIT demonstration.
///
/// This performs REAL JIT compilation:
/// 1. Parse toy language source code
/// 2. Compile with the full toy language JIT (using Cranelift)
/// 3. Execute the compiled function
/// All happening at runtime in no_std on RISC-V!
pub fn run_toy_demo() -> i32 {
    println!("");
    println!("=== Toy Language Runtime JIT Demo ===");

    // Check if allocator is working
    println!("Testing allocator...");
    let test_vec = alloc::vec![1, 2, 3, 4, 5];
    println!(
        "  ✓ Allocator working (test allocation succeeded: {} items)",
        test_vec.len()
    );
    drop(test_vec);

    let (current, peak, total, count) = runtime_embive::get_memory_usage();
    println!(
        "  Memory: {} current, {} peak, {} total, {} allocs",
        current, peak, total, count
    );

    let source = r#"
fn add(a, b) -> (result) {
    result = a * b
}
"#;

    println!("Source code:");
    println!("{}", source);
    println!("");

    // Create RISC-V ISA
    println!("Setting up RISC-V compiler...");
    let mut flag_builder = settings::builder();
    flag_builder.set("opt_level", "none").unwrap(); // Try minimal optimization to reduce stack usage
    flag_builder.set("is_pic", "false").unwrap();
    flag_builder.set("enable_verifier", "false").unwrap(); // Disable verifier to reduce work
    let isa_flags = settings::Flags::new(flag_builder);

    let triple = Triple {
        architecture: target_lexicon::Architecture::Riscv32(
            target_lexicon::Riscv32Architecture::Riscv32imac,
        ),
        vendor: target_lexicon::Vendor::Unknown,
        operating_system: target_lexicon::OperatingSystem::None_,
        environment: target_lexicon::Environment::Unknown,
        binary_format: target_lexicon::BinaryFormat::Elf,
    };

    let isa = match isa_builder(triple).finish(isa_flags) {
        Ok(isa) => isa,
        Err(_) => {
            println!("✗ ISA creation failed");
            return -1;
        }
    };
    println!("✓ ISA initialized");
    println!("");

    // COMPILE with Cranelift at runtime in no_std!
    println!("Compiling with Cranelift...");

    println!("Step 1: Parsing source code...");
    use lp_toy_lang::frontend::parser;
    let (_name, params, the_return, stmts) = match parser::function(source) {
        Ok(parsed) => {
            println!("✓ Parsing successful");
            parsed
        }
        Err(e) => {
            println!("✗ Parsing failed: {:?}", e);
            return -1;
        }
    };

    println!("Step 2: Compiling to machine code...");
    println!(
        "  Function has {} parameters, {} statements",
        params.len(),
        stmts.len()
    );

    let machine_code = match compile_toy_function(params, the_return, stmts, &*isa) {
        Ok(code) => {
            println!("✓ Compilation successful: {} bytes", code.len());
            println!("  Code: {:02x?}", &code[0..code.len().min(32)]);
            code
        }
        Err(e) => {
            println!("✗ Compilation failed: {}", e);
            return -1;
        }
    };

    let code_ptr = machine_code.as_ptr();

    // EXECUTE the JIT-compiled function!
    println!("");
    println!("Executing: add(5, 3)");
    type AddFn = extern "C" fn(isize, isize) -> isize;
    let add_fn: AddFn = unsafe { core::mem::transmute(code_ptr) };
    let result = add_fn(5, 3);
    println!("✓ Result: {}", result);
    println!("");

    result as i32
}
