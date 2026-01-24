# Design: LP Library Float-to-Fixed32 Conversion

## Overview

Fix LP library functions (`lp_simplex1/2/3`, `lp_hash`) to follow the correct float→fixed32 conversion pattern. Currently, codegen directly calls builtins, bypassing the transform. Functions should emit TestCase calls that the fixed32 transform converts, matching the pattern used for `sin`/`cos`.

## File Structure

```
lp-glsl/crates/lp-glsl-compiler/src/
├── frontend/
│   ├── semantic/
│   │   └── lp_lib_fns.rs              # UPDATE: Add needs_fixed32_mapping() and fixed32_name() methods
│   └── codegen/
│       └── lp_lib_fns.rs              # UPDATE: Change emit_lp_lib_fn_call() to emit TestCase calls
│
├── backend/
│   ├── builtins/
│   │   └── registry.rs                # UPDATE: Regenerate with correct BuiltinId names (LpSimplex3, not Fixed32LpSimplex3)
│   └── transform/
│       └── fixed32/
│           └── converters/
│               ├── calls.rs           # VERIFY: Already handles TestCase→builtin conversion correctly
│               └── math.rs            # VERIFY: map_testcase_to_builtin() already has correct mappings

lp-glsl/apps/lp-builtin-gen/
└── src/
    └── main.rs                        # UPDATE: Use LpLibFn enum as source of truth instead of prefix matching
```

## Types Summary

### LpLibFn Extensions (`frontend/semantic/lp_lib_fns.rs`)

```
LpLibFn - # UPDATE: Add new methods
├── needs_fixed32_mapping(&self) -> bool - # NEW: Returns true for simplex functions, false for hash
└── fixed32_name(&self) -> Option<&'static str> - # NEW: Returns Some("__lp_fixed32_lp_simplex3") or None

Existing methods (unchanged):
├── symbol_name(&self) -> &'static str - # Returns "__lp_simplex3" (TestCase name)
├── builtin_id(&self) -> BuiltinId - # Returns BuiltinId::LpSimplex3
└── user_name(&self) -> &'static str - # Returns "lp_simplex3"
```

### Codegen Changes (`frontend/codegen/lp_lib_fns.rs`)

```
emit_lp_lib_fn_call() - # UPDATE: Change implementation
├── OLD: Direct builtin call via get_builtin_func_ref()
└── NEW: Emit TestCase call using LpLibFn::symbol_name()
    ├── Check if needs_fixed32_mapping() -> true
    ├── Get TestCase name from symbol_name() (e.g., "__lp_simplex3")
    ├── Flatten vector arguments
    ├── Emit TestCase call (similar to get_math_libcall() pattern)
    └── Let fixed32 transform handle conversion
```

### Generator Changes (`apps/lp-builtin-gen/src/main.rs`)

```
discover_builtins() - # UPDATE: Use LpLibFn enum as source of truth
├── OLD: Prefix matching (__lp_fixed32_, __lp_hash_, __lp_simplex)
└── NEW: Read LpLibFn enum
    ├── For each LpLibFn variant:
    │   ├── Use LpLibFn::builtin_id() to get BuiltinId name (e.g., LpSimplex3)
    │   ├── Use LpLibFn::fixed32_name() or symbol_name() to find actual function
    │   └── Match discovered function names to expected names
    └── Generate registry with correct BuiltinId names

extract_builtin() - # UPDATE: Match functions to LpLibFn variants
└── Match function names to LpLibFn::fixed32_name() or symbol_name() values
```

### Registry Changes (`backend/builtins/registry.rs`)

```
BuiltinId - # UPDATE: Regenerate with correct names
├── OLD: Fixed32LpSimplex1, Fixed32LpSimplex2, Fixed32LpSimplex3
└── NEW: LpSimplex1, LpSimplex2, LpSimplex3 (matches LpLibFn::builtin_id())

BuiltinId::LpSimplex3.name() - # UPDATE: Return actual function name
└── Returns "__lp_fixed32_lp_simplex3" (actual implementation)
```

## Architecture Flow

### Current (Incorrect) Flow

```
1. GLSL: lp_simplex3(vec3(1.0, 2.0, 3.0), 123u)
2. Codegen: emit_lp_lib_fn_call() → get_builtin_func_ref(BuiltinId::LpSimplex3)
3. Direct call to __lp_fixed32_lp_simplex3 (bypasses transform)
```

### New (Correct) Flow

```
1. GLSL: lp_simplex3(vec3(1.0, 2.0, 3.0), 123u)
2. Codegen: emit_lp_lib_fn_call()
   ├── Check needs_fixed32_mapping() → true
   ├── Get symbol_name() → "__lp_simplex3"
   ├── Flatten vec3 → (i32, i32, i32, u32)
   └── Emit TestCase call to "__lp_simplex3"
3. Fixed32 Transform: convert_call()
   ├── Detect TestCase call to "__lp_simplex3"
   ├── map_testcase_to_builtin("__lp_simplex3") → BuiltinId::LpSimplex3
   ├── BuiltinId::LpSimplex3.name() → "__lp_fixed32_lp_simplex3"
   ├── Look up "__lp_fixed32_lp_simplex3" in func_id_map
   └── Create call to __lp_fixed32_lp_simplex3
4. Runtime: Calls __lp_fixed32_lp_simplex3
```

### Hash Functions Flow (Unchanged)

```
1. GLSL: lp_hash(42u, 123u)
2. Codegen: emit_lp_lib_fn_call()
   ├── Check needs_fixed32_mapping() → false
   └── Direct builtin call to __lp_hash_1 (no TestCase conversion needed)
3. Runtime: Calls __lp_hash_1
```

## Design Decisions

### 1. LpLibFn as Source of Truth

`LpLibFn` enum is the single source of truth for:
- Which functions exist
- Whether they need fixed32 mapping
- What their TestCase names are
- What their BuiltinId variants are
- What their fixed32 implementation names are

This ensures consistency across codegen, transform, and generator.

### 2. TestCase Names Represent Semantic Functions

TestCase names like `"__lp_simplex3"` represent the semantic function (float version), not the implementation. The transform decides which implementation to use based on the target:
- Fixed32 target: `"__lp_simplex3"` → `__lp_fixed32_lp_simplex3`
- Float target (future): `"__lp_simplex3"` → `__lp_float_lp_simplex3` (or similar)

### 3. Hash Functions Don't Need Conversion

Hash functions operate on integers (`u32`), not floats, so they don't need float→fixed32 conversion. They can be called directly as builtins. The `needs_fixed32_mapping()` method returns `false` for hash functions.

### 4. Generator Uses LpLibFn Enum

The generator reads `LpLibFn` enum to know what functions should exist, then matches discovered function names to expected names. This ensures the registry matches what `lp_lib_fns.rs` expects.

### 5. Consistent with sin/cos Pattern

This matches the existing pattern for `sin`/`cos`:
- Codegen emits TestCase call to `"sinf"` or `"__lp_sin"`
- Transform converts to `__lp_fixed32_sin` via `map_testcase_to_builtin()`
- Same pattern applies to LP library functions

## Implementation Details

### LpLibFn Method Implementations

```rust
impl LpLibFn {
    /// Get the fixed32 implementation name, if this function needs mapping
    pub fn fixed32_name(&self) -> Option<&'static str> {
        match self {
            LpLibFn::Simplex1 => Some("__lp_fixed32_lp_simplex1"),
            LpLibFn::Simplex2 => Some("__lp_fixed32_lp_simplex2"),
            LpLibFn::Simplex3 => Some("__lp_fixed32_lp_simplex3"),
            _ => None, // Hash functions don't have fixed32 versions
        }
    }

    /// Check if this function needs fixed32 mapping (float→fixed32 conversion)
    /// Delegates to fixed32_name() to keep a single source of truth
    pub fn needs_fixed32_mapping(&self) -> bool {
        self.fixed32_name().is_some()
    }
}
```

### Codegen TestCase Call Pattern

```rust
// Similar to get_math_libcall() but for LP library functions
fn get_lp_lib_testcase_call(&mut self, lp_fn: LpLibFn, arg_count: usize) -> Result<FuncRef, GlslError> {
    let testcase_name = lp_fn.symbol_name(); // e.g., "__lp_simplex3"
    
    // Create signature based on argument count
    let mut sig = Signature::new(CallConv::SystemV);
    for _ in 0..arg_count {
        sig.params.push(AbiParam::new(types::F32)); // Float before transform
    }
    sig.returns.push(AbiParam::new(types::F32));
    
    // Create TestCase name
    let sig_ref = self.builder.func.import_signature(sig);
    let ext_name = ExternalName::testcase(testcase_name.as_bytes());
    let ext_func = ExtFuncData {
        name: ext_name,
        signature: sig_ref,
        colocated: false,
    };
    Ok(self.builder.func.import_function(ext_func))
}
```

### Generator Function Discovery

```rust
// Pseudo-code for generator changes
fn discover_lp_lib_functions() -> Vec<BuiltinInfo> {
    let mut builtins = Vec::new();
    
    // Read LpLibFn enum variants
    for lp_fn in [LpLibFn::Simplex1, LpLibFn::Simplex2, LpLibFn::Simplex3, ...] {
        // Find matching function name
        let expected_name = lp_fn.fixed32_name().unwrap_or_else(|| lp_fn.symbol_name());
        
        // Discover function with matching name
        if let Some(func) = find_function_by_name(expected_name) {
            builtins.push(BuiltinInfo {
                enum_variant: lp_fn.builtin_id().to_string(), // "LpSimplex3"
                symbol_name: expected_name, // "__lp_fixed32_lp_simplex3"
                function_name: func.name,
                param_count: func.param_count,
            });
        }
    }
    
    builtins
}
```

## Verification Points

1. **Registry**: `BuiltinId::LpSimplex3.name()` returns `"__lp_fixed32_lp_simplex3"`
2. **Mapping**: `map_testcase_to_builtin("__lp_simplex3")` returns `BuiltinId::LpSimplex3`
3. **Codegen**: `emit_lp_lib_fn_call()` emits TestCase calls for simplex functions
4. **Transform**: `convert_call()` correctly converts TestCase calls to builtin calls
5. **Hash**: Hash functions continue to work as direct builtin calls
