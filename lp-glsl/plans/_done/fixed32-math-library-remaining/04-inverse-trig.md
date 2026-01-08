# Phase 4: Inverse Trig Functions

## Goal

Implement inverse trigonometric functions (asin, acos, atan, atan2) using libfixmath's implementations.

## Tasks

### 4.1 Port libfixmath Atan2 Implementation

In `lp-builtins/src/fixed32/atan2.rs`:
- Port libfixmath's atan2 implementation (2-arg function)
- Uses polynomial approximation with range reduction
- Export as `#[unsafe(no_mangle)] pub extern "C" fn __lp_fixed32_atan2(y: i32, x: i32) -> i32`
- Signature: (i32, i32) -> i32

### 4.2 Port libfixmath Atan Implementation

In `lp-builtins/src/fixed32/atan.rs`:
- Implement as `atan2(x, 1)` (libfixmath does this)
- Export as `#[unsafe(no_mangle)] pub extern "C" fn __lp_fixed32_atan(x: i32) -> i32`

### 4.3 Port libfixmath Asin Implementation

In `lp-builtins/src/fixed32/asin.rs`:
- Port libfixmath's asin: uses sqrt and atan
- Formula: `asin(x) = atan(x / sqrt(1 - x²))` for |x| <= 1
- Export as `#[unsafe(no_mangle)] pub extern "C" fn __lp_fixed32_asin(x: i32) -> i32`

### 4.4 Port libfixmath Acos Implementation

In `lp-builtins/src/fixed32/acos.rs`:
- Implement as `π/2 - asin(x)` (libfixmath does this)
- Export as `#[unsafe(no_mangle)] pub extern "C" fn __lp_fixed32_acos(x: i32) -> i32`

### 4.5 Add to Module

In `lp-builtins/src/fixed32/mod.rs`:
- Add `mod atan2;`, `mod atan;`, `mod asin;`, `mod acos;`
- Export all functions

### 4.6 Update Builtins App

In `lp-builtins-app/src/main.rs`:
- Add references to all inverse trig functions

### 4.7 Add to Registry

In `lp-glsl-compiler/src/backend/builtins/registry.rs`:
- Add `Fixed32Atan2`, `Fixed32Atan`, `Fixed32Asin`, `Fixed32Acos` to `BuiltinId` enum
- Add signatures: atan2 is (i32, i32) -> i32, others are (i32) -> i32
- Add to all registry functions

### 4.8 Extend Transform for 2-Arg Functions

In `lp-glsl-compiler/src/backend/transform/fixed32/converters/math.rs`:
- Modify `map_testcase_to_builtin()` to return `(BuiltinId, usize)` where usize is argument count
- Add mappings: `"atan2f"`/`"__lp_atan2"` -> `(Fixed32Atan2, 2)`
- Add mappings for 1-arg functions: `"atanf"`, `"asinf"`, `"acosf"` and `"__lp_atan"`, `"__lp_asin"`, `"__lp_acos"`

In `lp-glsl-compiler/src/backend/transform/fixed32/converters/calls.rs`:
- Modify conversion logic to handle both 1-arg and 2-arg functions
- Check argument count matches expected count
- Extract and map correct number of arguments

### 4.9 Add Tests

- Add tests for each function using `test_fixed32_function_relative()` helper
- Source test cases from libfixmath test suite
- Use 0.01 tolerance initially

## Success Criteria

- All inverse trig functions compile and are exported
- Functions are referenced in builtins app
- Transform successfully converts both 1-arg and 2-arg function calls
- Tests pass with 0.01 tolerance
- `builtins/phases/03-inverse-trig.glsl` passes
- All code compiles without warnings

