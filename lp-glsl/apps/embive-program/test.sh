#!/bin/bash
# Test script for embive-program
# This builds the embive-program and verifies it can be created successfully

set -e

echo "=== Embive Program Test ==="
echo

# Find workspace root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Workspace: $WORKSPACE_ROOT"
echo

# Build the program
echo "Building embive-program for riscv32imac-unknown-none-elf..."
cd "$WORKSPACE_ROOT"
cargo build --package embive-program --target riscv32imac-unknown-none-elf --release

# Check that the binary was created
BINARY_PATH="$WORKSPACE_ROOT/target/riscv32imac-unknown-none-elf/release/embive-program"
if [ ! -f "$BINARY_PATH" ]; then
    echo "ERROR: Binary not found at $BINARY_PATH"
    exit 1
fi

echo
echo "✅ Build successful!"
echo "Binary: $BINARY_PATH"
echo "Size: $(ls -lh "$BINARY_PATH" | awk '{print $5}')"
echo

# Check that it's an ELF file
if file "$BINARY_PATH" | grep -q "ELF 32-bit LSB executable, UCB RISC-V"; then
    echo "✅ Binary is a valid RISC-V 32-bit ELF executable"
else
    echo "WARNING: Binary format check failed"
    file "$BINARY_PATH"
fi

echo
echo "=== Test Complete ===" 
echo
echo "To run the program in embive VM, you'll need to:"
echo "1. Load the ELF file"
echo "2. Transpile it to embive bytecode"  
echo "3. Execute it in the embive interpreter"
echo
echo "The program should output steps showing compilation and execution of add(5,3)"
echo "and report a result of 8."








