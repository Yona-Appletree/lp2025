# TODO Analysis - Next Implementation Steps

## Analysis Summary

Found **33 `todo!()` items** across the codebase, categorized as:

- **Critical Path (Runtime Functionality)**: 20 items
  - Runtime contexts (8): Node resolution, filesystem access, texture/output access
  - Node runtimes (8): Init/render for texture, shader, output, fixture
  - State extraction (4): Get actual state from runtimes for sync API

- **Infrastructure**: 4 items
  - Config serialization (3): Proper cloning/serialization
  - Output flushing (1): Flush outputs after rendering

- **Nice-to-Have**: 9 items
  - Model improvements (4): Structured types for fixture config
  - Client view (2): Update config/state from details
  - Other (3): Node cleanup, etc.

---

## 5 Suggested Next Steps

### 1. Implement Runtime Contexts (NodeInitContext & RenderContext)
**Priority**: 🔴 HIGHEST - Blocks fixture initialization and rendering

**What to do**:
- Implement `InitContext::resolve_output()` and `resolve_texture()` - resolve `NodeSpecifier` to handles by looking up nodes in `ProjectRuntime`
- Implement `InitContext::get_node_fs()` - return filesystem from context
- Implement `RenderContextImpl::get_texture()` - look up texture runtime, trigger lazy render if needed, return pixel data
- Implement `RenderContextImpl::get_output()` - look up output runtime, return mutable buffer slice

**Why first**: Fixture nodes need these to resolve dependencies and access texture/output data.

---

### 2. Implement Texture Runtime (init + render)
**Priority**: 🔴 HIGH - Foundation for rendering pipeline

**What to do**:
- `TextureRuntime::init()` - load texture config, allocate buffer, load from file if needed
- `TextureRuntime::render()` - generate/update texture data (for now, simple patterns or file loading)
- Store state in `TextureRuntime` (buffer, dimensions, format)
- Update `state_ver` when texture changes

**Why second**: Fixtures depend on textures; shaders render to textures.

---

### 3. Implement Fixture Runtime (init + render)
**Priority**: 🔴 HIGH - Core rendering logic

**What to do**:
- `FixtureRuntime::init()` - resolve output/texture specifiers using `NodeInitContext`, store handles
- `FixtureRuntime::render()` - sample texture using `RenderContext::get_texture()`, transform coordinates, write to output using `RenderContext::get_output()`
- Store resolved handles and mapping state in `FixtureRuntime`

**Why third**: This ties textures and outputs together - completes the basic rendering loop.

---

### 4. Implement State Extraction for Sync API
**Priority**: 🟡 MEDIUM - Needed for client sync

**What to do**:
- Add methods to runtimes to get state (e.g., `TextureRuntime::get_state()`)
- Update `get_changes()` to call these methods instead of returning empty placeholders
- Update `state_ver` when state changes during render

**Why fourth**: Enables clients to see actual runtime state, not placeholders.

---

### 5. Implement Output Runtime (init + render + flushing)
**Priority**: 🟡 MEDIUM - Needed for actual hardware output

**What to do**:
- `OutputRuntime::init()` - setup GPIO pins, allocate channel buffers
- `OutputRuntime::render()` - copy data to output buffers
- Implement output flushing in `ProjectRuntime::render()` - flush outputs with `state_ver == frame_id`
- Store channel data state in `OutputRuntime`

**Why fifth**: Completes the pipeline from texture → fixture → output → hardware.

---

## Recommended Order

1. **Runtime Contexts** (unblocks everything)
2. **Texture Runtime** (foundation)
3. **Fixture Runtime** (core logic)
4. **State Extraction** (sync)
5. **Output Runtime** (completes pipeline)

This order:
- ✅ Unblocks dependencies incrementally
- ✅ Keeps system working at each step
- ✅ Enables end-to-end rendering after step 3
- ✅ Adds sync and hardware output in steps 4-5
