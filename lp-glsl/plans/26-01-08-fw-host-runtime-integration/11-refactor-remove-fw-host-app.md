# Phase 11: Refactor - Remove FwHostApp, use LpApp directly

## Goal

Remove `FwHostApp` wrapper and have `main.rs` use `LpApp` directly. Move `create_default_project` to `LpApp`.

## Decisions

1. **Message Handling**: ✅ **DECIDED**: Remove `Command` and `Transport` - use `MsgIn` directly

   - `main.rs` reads messages (if needed) and passes `MsgIn` directly to `LpApp::tick()`
   - No transport abstraction needed in fw-host

2. **LED Visualization**: How should the UI access LED output data?

   - The runtime creates `HostLedOutput` instances via `OutputProvider`
   - But visualization needs to read from them
   - Option A: `LpApp` exposes a method to get LED outputs (e.g., `get_led_outputs() -> Vec<&HostLedOutput>`)
   - Option B: Store LED outputs in a shared location (Arc<Mutex<...>>) accessible to both
   - Option C: Something else?

3. **Default Project Creation**: Should `create_default_project` be:

   - Option A: A method on `LpApp` that creates and saves default project if none exists (called automatically in `load_project()` if file doesn't exist)
   - Option B: A separate method that firmware calls explicitly
   - Option C: Something else?

4. **UI Access Methods**: Methods like `project()` and `runtime()` - should these:
   - Option A: Stay on `LpApp` (already exist)
   - Option B: Be removed, access `LpApp` directly
   - Option C: Something else?

## Tasks

1. Move `create_default_project` logic to `LpApp`
2. Update `LpApp::load_project()` to create default project if file doesn't exist
3. Remove `FwHostApp` struct
4. Update `main.rs` to use `LpApp` directly
5. Handle transport/message reading in `main.rs`
6. Fix LED visualization access (get LED outputs from runtime)
7. Update UI code to access `LpApp` methods directly

## Success Criteria

- `FwHostApp` is removed
- `main.rs` uses `LpApp` directly
- Default project creation works automatically
- LED visualization can access LED output data
- Code compiles without errors
- All functionality still works
