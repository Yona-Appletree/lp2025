# Questions for LP Library Float-to-Fixed32 Conversion Plan

## Context

We implemented LP library functions (`lp_simplex1`, `lp_simplex2`, `lp_simplex3`) but missed a critical architectural detail:

- **LP library functions are semantically float functions** - they operate on float types in GLSL
- **Eventually we'll have float implementations** - when native float support is added
- **Currently we only support fixed32** - so we have `__lp_fixed32_lp_simplex3` implementations
- **The compiler should generate float calls** - which the fixed32 transform then converts

This matches the pattern used for `sin`/`cos`:
- Codegen generates TestCase calls to `"sinf"` or `"__lp_sin"` (float semantics)
- Fixed32 transform converts `"sinf"` → `__lp_fixed32_sin` via `map_testcase_to_builtin()`

Currently, `lp_lib` functions bypass this pattern:
- Codegen directly calls builtins via `get_builtin_func_ref()` 
- This skips the float → fixed32 conversion step

## Questions

### Q1: Naming Convention for Float LP Library Functions

**Context**: We need to decide what the "float" version of these functions should be called in the compiler's intermediate representation.

**Question**: What should the TestCase/function name be for float LP library functions?

**Options**:
- Option A: `__lp_float_lp_simplex3` (explicit float prefix)
- Option B: `__lp_simplex3` (no float prefix, just the base name)
- Option C: `lp_simplex3` (user-facing name, no prefix)
- Option D: Something else?

**Suggested Answer**: Option B (`__lp_simplex3`) - matches the pattern used for `sin`/`cos` where `"sinf"` maps to `"__lp_sin"` in the mapping table. The float nature is implicit since we're in float codegen before the transform.

**ANSWERED**: Option B (`__lp_simplex3`) - confirmed by user. This matches `LpLibFn::Simplex3.symbol_name()` and the existing mapping table entries.

### Q2: Codegen Strategy for LP Library Functions

**Context**: Currently `emit_lp_lib_fn_call()` directly gets builtin function references. We need to change it to emit TestCase calls instead.

**Question**: How should codegen emit LP library function calls?

**Options**:
- Option A: Use `LpLibFn::symbol_name()` to get `"__lp_simplex3"`, emit TestCase call similar to `get_math_libcall()` pattern
- Option B: Create new helper similar to `get_math_libcall()` but for LP library functions
- Option C: Something else?

**Suggested Answer**: Option A - Use `LpLibFn::symbol_name()` to get TestCase name, emit TestCase call using same pattern as `get_math_libcall()`. Need to handle different argument counts and vector argument flattening.

**ANSWERED**: Option A - confirmed by user. Use `LpLibFn::symbol_name()` and emit TestCase calls following the `get_math_libcall()` pattern.

### Q3: BuiltinId Mapping Strategy

**Context**: The registry has been regenerated. `BuiltinId::LpSimplex3.name()` should now return `"__lp_fixed32_lp_simplex3"` (the actual function name). The mapping table maps `"__lp_simplex3"` (semantic/float name from `LpLibFn::symbol_name()`) → `BuiltinId::LpSimplex3`.

**Question**: How should the transform convert TestCase calls?

**Suggested Answer**: The transform should:
1. Map `"__lp_simplex3"` → `BuiltinId::LpSimplex3` via `map_testcase_to_builtin()`
2. Use `BuiltinId::LpSimplex3.name()` to get `"__lp_fixed32_lp_simplex3"`
3. Look up `"__lp_fixed32_lp_simplex3"` in `func_id_map` to get the FuncId
4. Create call to that function

This matches how `sin` works: `"sinf"` → `BuiltinId::Fixed32Sin` → `"__lp_fixed32_sin"`.

**Note**: The generator should be driven by `LpLibFn` enum, not prefix matching. It should:
- Read `LpLibFn` enum to know what LP library functions exist
- Match discovered function names to `LpLibFn::symbol_name()` values
- Use `LpLibFn::builtin_id()` to determine the `BuiltinId` variant
- Generate registry entries based on actual function names (which may be `__lp_fixed32_lp_simplex3`)

### Q4: Hash Function Handling

**Context**: Hash functions (`lp_hash`) are not float-specific - they operate on integers. They're correctly in `builtins/shared/`.

**Question**: Should hash functions follow the same pattern, or stay as-is?

**Suggested Answer**: `LpLibFn` should be the source of truth for determining:
1. Whether a function needs fixed32 mapping (simplex functions do, hash functions don't)
2. What the fixed32 implementation name is (e.g., `__lp_fixed32_lp_simplex3` for simplex functions)

Hash functions don't need conversion, so they can stay as direct builtin calls. But `LpLibFn` should have methods to determine this programmatically.

**ANSWERED**: Option A - Keep hash functions as direct builtin calls, but extend `LpLibFn` with methods to determine if a function needs fixed32 mapping and what the mapped name is. This keeps the source of truth in `LpLibFn`.

**Implementation Note**: Add methods to `LpLibFn`:
- `fixed32_name(&self) -> Option<&'static str>` - returns `Some("__lp_fixed32_lp_simplex3")` for simplex functions, `None` for hash functions (single source of truth)
- `needs_fixed32_mapping(&self) -> bool` - delegates to `fixed32_name().is_some()` to keep a single source of truth

This allows codegen to check if a function needs TestCase conversion or can be called directly.

### Q5: Generator Fix

**Context**: The generator currently uses prefix matching to discover functions. It should use `LpLibFn` as the source of truth.

**Question**: How should the generator discover and map functions?

**Suggested Answer**: Generator should:
1. Read `LpLibFn` enum to know what functions exist
2. For each variant, look for matching function name (e.g., `__lp_fixed32_lp_simplex3` for simplex, `__lp_hash_*` for hash)
3. Use `LpLibFn::builtin_id()` to determine `BuiltinId` variant name (e.g., `LpSimplex3`, not `Fixed32LpSimplex3`)
4. Map `BuiltinId::LpSimplex3.name()` to actual function name found

**ANSWERED**: Confirmed by user. Generator should use `LpLibFn` enum as source of truth and match discovered functions to expected names.

### Q6: Backward Compatibility

**Context**: The current implementation works, but uses the wrong pattern. Changing it might break existing code.

**Question**: Do we need to maintain backward compatibility, or can we change the pattern?

**Suggested Answer**: Since this is a recent implementation and the pattern is wrong, we should fix it now. No need for backward compatibility - the correct pattern is more important.

**ANSWERED**: No backward compatibility needed - fix the pattern now.

### Q6: Float Implementation Placeholder

**Context**: We don't have float implementations yet, but the architecture should assume they'll exist.

**Question**: Should we create placeholder float implementations, or just assume they'll be added later?

**Suggested Answer**: No placeholder needed. The architecture should support float implementations, but we don't need to implement them now. When float support is added, we'll add `__lp_float_lp_simplex3` (or similar) implementations, and the codegen will generate calls to those instead of the fixed32 versions.

Actually, wait - if we're generating TestCase calls to `__lp_simplex3`, and there's no float implementation yet, what happens? The transform will convert `__lp_simplex3` → `__lp_fixed32_lp_simplex3`, which exists. So the current flow is:
- Codegen: `lp_simplex3(...)` → TestCase call to `__lp_simplex3`
- Transform: `__lp_simplex3` → `__lp_fixed32_lp_simplex3` (via mapping)
- Runtime: Calls `__lp_fixed32_lp_simplex3`

When float support is added:
- Codegen: `lp_simplex3(...)` → TestCase call to `__lp_simplex3` (same)
- Transform: If float target, keep as `__lp_float_lp_simplex3` (or similar)
- Runtime: Calls `__lp_float_lp_simplex3`

So the TestCase name `__lp_simplex3` represents the "semantic" function, and the transform decides which implementation to use based on target.

## Notes

### Current State Analysis

1. **Registry Mismatch**:
   - Registry has `Fixed32LpSimplex1/2/3` (generated from prefix matching)
   - `lp_lib_fns.rs` expects `LpSimplex1/2/3` (semantic names)
   - Generator needs to use `LpLibFn` enum as source of truth

2. **Codegen** (`lp_lib_fns.rs`):
   - Currently calls `get_builtin_func_ref(builtin_id)` directly
   - This bypasses the TestCase → fixed32 conversion pattern
   - Should emit TestCase calls using `LpLibFn::symbol_name()` (e.g., `"__lp_simplex3"`)

3. **Transform** (`calls.rs`):
   - `map_testcase_to_builtin()` should map `"__lp_simplex3"` → `BuiltinId::LpSimplex3`
   - `BuiltinId::LpSimplex3.name()` should return `"__lp_fixed32_lp_simplex3"` (actual function)
   - Transform looks up `"__lp_fixed32_lp_simplex3"` in `func_id_map`

4. **Hash Functions**:
   - In `builtins/shared/` (correct - not float-specific)
   - Use `__lp_hash_*` naming (no `fixed32` prefix)
   - Should continue to work, but verify codegen path

### Architecture

**Source of Truth**: `LpLibFn` enum in `lp_lib_fns.rs` defines:
- Semantic function names (`lp_simplex3`)
- TestCase names (`__lp_simplex3`) via `symbol_name()`
- BuiltinId mapping (`LpSimplex3`) via `builtin_id()`

**Generator Should**:
- Read `LpLibFn` enum to know what functions exist
- Match discovered function names to `LpLibFn::symbol_name()` values
- Generate `BuiltinId` variants matching `LpLibFn::builtin_id()` (e.g., `LpSimplex3`, not `Fixed32LpSimplex3`)
- Map `BuiltinId::LpSimplex3.name()` to actual function name (`__lp_fixed32_lp_simplex3`)

**Flow**:
1. Codegen: `lp_simplex3(...)` → TestCase call to `LpLibFn::Simplex3.symbol_name()` = `"__lp_simplex3"`
2. Transform: `"__lp_simplex3"` → `BuiltinId::LpSimplex3` (via `map_testcase_to_builtin()`)
3. Transform: `BuiltinId::LpSimplex3.name()` → `"__lp_fixed32_lp_simplex3"`
4. Transform: Look up `"__lp_fixed32_lp_simplex3"` in `func_id_map` → create call
5. Runtime: Calls `__lp_fixed32_lp_simplex3`

### Implementation Strategy

1. **Fix Generator** - Update `lp-builtin-gen` to use `LpLibFn` enum as source of truth:
   - Read `LpLibFn` enum from `lp_lib_fns.rs` 
   - Match discovered functions to `LpLibFn::symbol_name()` values
   - Use `LpLibFn::builtin_id()` to determine `BuiltinId` variant
   - Generate registry with actual function names (e.g., `__lp_fixed32_lp_simplex3`)

2. **Fix Codegen** - Change `emit_lp_lib_fn_call()` to emit TestCase calls:
   - Use `LpLibFn::symbol_name()` to get TestCase name (e.g., `"__lp_simplex3"`)
   - Emit TestCase call instead of direct builtin call
   - Let fixed32 transform handle conversion

3. **Verify Transform** - Ensure `map_testcase_to_builtin()` correctly maps:
   - `"__lp_simplex3"` → `BuiltinId::LpSimplex3`
   - Transform uses `BuiltinId::LpSimplex3.name()` → `"__lp_fixed32_lp_simplex3"`
   - Looks up `"__lp_fixed32_lp_simplex3"` in `func_id_map`

4. **Test** - Verify end-to-end flow:
   - Codegen emits TestCase call to `"__lp_simplex3"`
   - Transform converts to `__lp_fixed32_lp_simplex3` call
   - Runtime calls correct function

5. **Hash Functions** - Verify hash functions still work (they're not float-specific, so may not need conversion)
