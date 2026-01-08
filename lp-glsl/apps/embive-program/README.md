# Embive Toy Language JIT Demo

This application demonstrates running the toy language compiler in a `no_std` environment on `riscv32` inside the embive VM.

## Overview

The demo shows:

1. Pre-compiling a simple toy language program (`fn add(a, b) -> result { result = a + b }`) to RISC-V machine code using Cranelift at build time
2. Embedding the compiled ELF file in the binary
3. At runtime (inside embive VM), transpiling the ELF to embive bytecode
4. Executing the transpiled code as a function pointer
5. Verifying the result (should return 8 for `add(5, 3)`)

## Building

```bash
cargo build --package embive-program --target riscv32imac-unknown-none-elf --release
```

The binary will be located at:

```
target/riscv32imac-unknown-none-elf/release/embive-program
```

## Testing

### Build Test

A shell script is provided to build and verify the binary:

```bash
./apps/embive-program/test.sh
```

This script:

- Builds the embive-program for riscv32
- Verifies the binary was created successfully
- Checks that it's a valid RISC-V ELF executable

### Integration Test

An integration test is available that builds the program and runs it in the embive VM:

```
apps/embive-program/tests/integration_test.rs
```

Note: Due to the architecture-specific assembly in `runtime-embive`, this test has dependency resolution challenges when building for the host. For full end-to-end testing, the binary should be run in an actual embive VM environment.

## Architecture

### Build-time Compilation (`build.rs`)

- Uses `lp-riscv-shared` to compile the toy language program to RISC-V machine code
- Generates a simple ELF file containing the compiled code
- Embeds the ELF as a constant array in the generated code

### Runtime Execution (`src/jit_test.rs`)

- Loads the pre-compiled ELF from the constant
- Uses `embive` transpiler to convert RISC-V ELF to embive bytecode
- Casts the bytecode to a function pointer
- Executes the function and verifies the result

### Components

- **`runtime-embive`**: Provides no_std runtime support (entry point, allocator, syscalls, panic handler)
- **`lp-riscv-shared`**: Compiles toy language to RISC-V ELF using Cranelift
- **`lp-toy-lang`**: Toy language parser and frontend
- **`embive`**: RISC-V to embive bytecode transpiler (external crate)

## Key Technical Choices

1. **Pre-compilation**: Since cranelift requires `std` for compilation, we compile at build time (which has `std`) rather than runtime (which is `no_std`)

2. **Custom ELF Generator**: cranelift-object doesn't support riscv32 as a target, so we manually generate a minimal ELF file structure

3. **Simple Function**: The demo uses a simple add function to keep the generated code small and easy to verify

## Limitations

- Only simple toy language programs without function calls are supported
- The generated ELF is minimal and may not work with all ELF loaders
- Designed specifically for embive VM execution
