# RISC-V32IMAC Feature Matrix

This document tracks the implementation status of RISC-V32IMAC instruction set architecture (ISA) features in the `lp-riscv-tools` emulator.

## Instruction Set Extensions

- **RV32I**: Base Integer Instruction Set - ✅ Fully Supported
- **M**: Integer Multiplication and Division Extension - ✅ Fully Supported
- **A**: Atomic Instructions Extension - ✅ Fully Supported
- **C**: Compressed Instructions Extension - ✅ Fully Supported

## Instruction Groups

### RV32I Base Instructions

#### Arithmetic Instructions
- ✅ ADD, SUB
- ✅ ADDI
- ✅ LUI, AUIPC

#### Logical Instructions
- ✅ AND, OR, XOR
- ✅ ANDI, ORI, XORI

#### Shift Instructions
- ✅ SLL, SRL, SRA
- ✅ SLLI, SRLI, SRAI

#### Comparison Instructions
- ✅ SLT, SLTU
- ✅ SLTI, SLTIU

#### Load/Store Instructions
- ✅ LB, LH, LW
- ✅ LBU, LHU
- ✅ SB, SH, SW

#### Control Flow Instructions
- ✅ JAL, JALR
- ✅ BEQ, BNE, BLT, BGE, BLTU, BGEU

#### System Instructions
- ✅ ECALL
- ✅ EBREAK
- ✅ FENCE
- ✅ **FENCE.I** - Instruction cache synchronization (added for JIT support)

### M Extension (Integer Multiplication and Division)

- ✅ MUL
- ✅ MULH (signed × signed)
- ✅ MULHSU (signed × unsigned)
- ✅ MULHU (unsigned × unsigned)
- ✅ DIV (signed division)
- ✅ DIVU (unsigned division)
- ✅ REM (signed remainder)
- ✅ REMU (unsigned remainder)

**Edge Cases:**
- ✅ Division by zero returns -1 (signed) or max value (unsigned) per RISC-V spec
- ✅ Overflow cases handled correctly

### A Extension (Atomic Instructions)

- ✅ LR.W (Load Reserved Word)
- ✅ SC.W (Store Conditional Word)
- ✅ AMOSWAP.W (Atomic Swap Word)
- ✅ AMOADD.W (Atomic Add Word)
- ✅ AMOXOR.W (Atomic XOR Word)
- ✅ AMOAND.W (Atomic AND Word)
- ✅ AMOOR.W (Atomic OR Word)

**Note:** In the single-threaded emulator, atomic instructions are implemented as simple read-modify-write operations. LR.W/SC.W always succeed.

### C Extension (Compressed Instructions)

All standard RVC instructions are supported:

#### Register Operations
- ✅ C.ADDI, C.LI, C.LUI
- ✅ C.MV, C.ADD, C.SUB
- ✅ C.AND, C.OR, C.XOR
- ✅ C.ANDI
- ✅ C.SLLI, C.SRLI, C.SRAI

#### Load/Store
- ✅ C.LW, C.SW
- ✅ C.LWSP, C.SWSP
- ✅ C.ADDI4SPN, C.ADDI16SP

#### Control Flow
- ✅ C.J, C.JR, C.JALR
- ✅ C.JAL (RV32 only)
- ✅ C.BEQZ, C.BNEZ

#### System
- ✅ C.NOP
- ✅ C.EBREAK

## Test Coverage

### Unit Tests
- `crates/lp-riscv-tools/tests/instruction_tests.rs` - Individual instruction tests
- `crates/lp-riscv-tools/tests/jit_test_compatibility.rs` - JIT compilation pipeline tests

### Integration Tests
- `crates/lp-riscv-tools/tests/riscv_nostd_test.rs` - Full-stack no_std program tests

## Known Limitations

1. **Single-threaded execution**: Atomic instructions don't provide true atomicity guarantees (no other threads to contend with)

2. **Instruction cache**: FENCE.I is implemented as a no-op since the emulator doesn't have separate instruction/data caches

3. **Memory model**: FENCE is a no-op since there's no memory ordering to enforce in single-threaded execution

4. **Privileged instructions**: Only user-mode instructions are supported (no supervisor/machine mode)

## FENCE.I Support

**Status:** ✅ Fully Supported

FENCE.I (instruction cache synchronization) is now fully supported for JIT compilation scenarios. The instruction:
- Decodes correctly (opcode 0x0f, funct3=0x1, imm=0x001)
- Encodes correctly (0x0000100f)
- Executes as a no-op (appropriate for emulator without separate I-cache)

This enables JIT-compiled code to work correctly in the emulator, matching the behavior of native hardware.

## References

- RISC-V ISA Specification: https://riscv.org/technical/specifications/
- RISC-V ISA Documentation: `/Users/yona/dev/photomancer/riscv-isadoc`





