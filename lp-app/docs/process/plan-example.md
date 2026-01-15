# Plan Example

This document shows examples of both plan structures: single file and separate files.

## Example 1: Single File Plan

For simpler plans with 3-5 phases, use a single `00-plan.md` file:

```markdown
# Plan: Project Commands

## Overview

Implement project management commands in the client-server system, enabling clients to load,
unload, and interact with projects running on the server.

See `00-design.md` for architecture details.

## Success Criteria

- Client can send `LoadProject` request and receive handle
- Client can send `UnloadProject` request and project is unloaded
- Client can list available projects
- Client can list loaded projects with handles
- Client can send `GetChanges` request and receive serialized response
- All tests pass
- Code compiles without warnings

## Phases

1. Extend message types for project commands
2. Implement project loading and unloading
3. Implement project request routing
4. Add client API methods
5. Add integration tests
6. Cleanup and finalization

---

## Phase 1: Extend Message Types for Project Commands

### Description

Add project management variants to `ClientRequest` enum and enable `ProjectRequest` variant in
`ServerResponse` enum. This establishes the message protocol for project commands.

### Implementation Notes

- Update `lp-model/src/message.rs` to add project variants to `ClientRequest`
- Update `lp-model/src/server/api.rs` to enable `ProjectRequest` variant in `ServerResponse`
- Ensure serialization works correctly for all new variants

### Success Criteria

- [ ] `ClientRequest` has all project management variants
- [ ] `ServerResponse` has enabled `ProjectRequest` variant
- [ ] All variants serialize/deserialize correctly
- [ ] Tests pass
- [ ] Code compiles without warnings

### Style Notes

- Place helper utility functions **at the bottom** of files
- Place more abstract things, entry points, and tests **first**
- Run `cargo +nightly fmt` on all changes before committing
- Keep language professional and restrained
- Avoid overly optimistic language like "comprehensive", "fully production ready"
- Avoid emoticons
- Code is never done, never perfect, never fully ready, never fully complete

---

## Phase 2: Implement Project Loading and Unloading

### Description

Implement project loading and unloading functionality in `ProjectManager`. Projects are loaded
from the filesystem and initialized automatically. Handles are generated sequentially.

### Implementation Notes

- Update `ProjectManager::load_project()` to generate handles and auto-initialize
- Update `ProjectManager::unload_project()` to remove projects by handle
- Update `ProjectManager::get_project()` and `get_project_mut()` to use handles
- Implement handle generation (sequential IDs starting from 1)

### Success Criteria

- [ ] `ProjectManager::load_project()` generates handles and initializes projects
- [ ] `ProjectManager::unload_project()` removes projects by handle
- [ ] Projects are auto-initialized on load
- [ ] Handle generation is sequential and deterministic
- [ ] Tests pass
- [ ] Code compiles without warnings

### Style Notes

- Place helper utility functions **at the bottom** of files
- Place more abstract things, entry points, and tests **first**
- Run `cargo +nightly fmt` on all changes before committing
- Keep language professional and restrained
- Avoid overly optimistic language like "comprehensive", "fully production ready"
- Avoid emoticons
- Code is never done, never perfect, never fully ready, never fully complete

---

## Phase 3: Implement Project Request Routing

### Description

Implement message handlers for project requests. Route `LoadProject`, `UnloadProject`, and
`ProjectRequest` messages to appropriate handlers.

### Implementation Notes

- Create handler functions in `lp-server/src/handlers.rs`
- Implement `handle_load_project()`, `handle_unload_project()`, `handle_project_request()`
- Update `handle_client_message()` to route project requests
- Implement `GetChanges` request handling

### Success Criteria

- [ ] All project request handlers are implemented
- [ ] `handle_client_message()` routes project requests correctly
- [ ] `GetChanges` requests return serialized responses
- [ ] Error handling is correct for all cases
- [ ] Tests pass
- [ ] Code compiles without warnings

### Style Notes

- Place helper utility functions **at the bottom** of files
- Place more abstract things, entry points, and tests **first**
- Run `cargo +nightly fmt` on all changes before committing
- Keep language professional and restrained
- Avoid overly optimistic language like "comprehensive", "fully production ready"
- Avoid emoticons
- Code is never done, never perfect, never fully ready, never fully complete

---

## Phase 4: Add Client API Methods

### Description

Add client API methods for project management. Methods send requests and extract responses,
handling request/response correlation.

### Implementation Notes

- Add project management methods to `lp-client/src/client.rs`
- Implement response extractor methods
- Handle request/response correlation
- Ensure blocking behavior for synchronous operations

### Success Criteria

- [ ] Client has methods for all project operations
- [ ] Request/response correlation works correctly
- [ ] Methods return appropriate types
- [ ] Error handling is correct
- [ ] Tests pass
- [ ] Code compiles without warnings

### Style Notes

- Place helper utility functions **at the bottom** of files
- Place more abstract things, entry points, and tests **first**
- Run `cargo +nightly fmt` on all changes before committing
- Keep language professional and restrained
- Avoid overly optimistic language like "comprehensive", "fully production ready"
- Avoid emoticons
- Code is never done, never perfect, never fully ready, never fully complete

---

## Phase 5: Add Integration Tests

### Description

Add integration tests for project commands. Test the full flow from client to server and verify
project lifecycle operations.

### Implementation Notes

- Create `lp-client/tests/project_sync.rs`
- Add helper functions for test setup
- Test project loading, unloading, listing, and GetChanges
- Verify serialization round-trips

### Success Criteria

- [ ] Integration tests cover all project operations
- [ ] Tests verify project lifecycle
- [ ] Tests verify serialization
- [ ] All tests pass
- [ ] Code compiles without warnings

### Style Notes

- Place helper utility functions **at the bottom** of files
- Place more abstract things, entry points, and tests **first**
- Run `cargo +nightly fmt` on all changes before committing
- Keep language professional and restrained
- Avoid overly optimistic language like "comprehensive", "fully production ready"
- Avoid emoticons
- Code is never done, never perfect, never fully ready, never fully complete

---

## Phase 6: Cleanup and Finalization

### Description

Remove temporary code, fix warnings, ensure all tests pass, and format code.

### Implementation Notes

- Remove any temporary code, TODOs, debug prints
- Fix all warnings
- Ensure all tests pass
- Run `cargo +nightly fmt` on entire workspace
- Review code for clarity and consistency

### Success Criteria

- [ ] No temporary code or TODOs remain
- [ ] All warnings are fixed
- [ ] All tests pass
- [ ] Code is formatted with `cargo +nightly fmt`
- [ ] Code is clean and readable

### Style Notes

- Place helper utility functions **at the bottom** of files
- Place more abstract things, entry points, and tests **first**
- Run `cargo +nightly fmt` on all changes before committing
- Keep language professional and restrained
- Avoid overly optimistic language like "comprehensive", "fully production ready"
- Avoid emoticons
- Code is never done, never perfect, never fully ready, never fully complete
```

## Example 2: Separate Files Plan

For complex plans with many phases, use separate files:

### 00-overview.md

```markdown
# Overview: Project Commands

## Goal

Implement project management commands in the client-server system, enabling clients to load,
unload, and interact with projects running on the server.

## Scope

- **Message Protocol**: Extend `ClientRequest` and `ServerResponse` with project variants
- **Project Management**: Implement loading, unloading, and listing of projects
- **Request Routing**: Route project requests to appropriate handlers
- **Client API**: Add client methods for project operations
- **Serialization**: Implement serializable wrappers for project responses
- **Testing**: Add integration tests for project commands

## Out of Scope (for now)

- Project creation via API (projects must exist on filesystem)
- Project configuration changes
- Multiple clients managing the same project

## Success Criteria

- Client can send `LoadProject` request and receive handle
- Client can send `UnloadProject` request and project is unloaded
- Client can list available projects
- Client can list loaded projects with handles
- Client can send `GetChanges` request and receive serialized response
- Integration tests create project on client, sync to server, load it, verify it's running
- All tests pass
- Code compiles without warnings

## Dependencies

- `00-design.md` - Architecture design document
- Filesystem sync foundation (already implemented)
- Project runtime (already implemented)

## Phases

1. Extend message types for project commands
2. Implement project loading and unloading
3. Implement project request routing
4. Add client API methods
5. Add integration tests
6. Cleanup and finalization
```

### 01-extend-message-types.md

```markdown
# Phase 1: Extend Message Types for Project Commands

## Goal

Add project management variants to `ClientRequest` enum and enable `ProjectRequest` variant in
`ServerResponse` enum. This establishes the message protocol for project commands.

## Tasks

1. Update `lp-model/src/message.rs`:
   - Add project management variants to `ClientRequest` enum:
     - `LoadProject { path: String }`
     - `UnloadProject { handle: ProjectHandle }`
     - `ProjectRequest { handle: ProjectHandle, request: ProjectRequest }`
     - `ListAvailableProjects`
     - `ListLoadedProjects`
   - Add necessary imports (`ProjectHandle`, `ProjectRequest`)
   - Update serialization tests to include project variants

2. Update `lp-model/src/server/api.rs`:
   - Enable `ProjectRequest` variant in `ServerResponse` enum
   - Add project management response variants:
     - `LoadProject { handle: ProjectHandle }`
     - `UnloadProject`
     - `ListAvailableProjects { projects: Vec<AvailableProject> }`
     - `ListLoadedProjects { projects: Vec<LoadedProject> }`

3. Verify serialization:
   - Test round-trip serialization for each new variant
   - Ensure JSON tag names match expected structure

## Success Criteria

- [ ] `ClientRequest` has all project management variants
- [ ] `ServerResponse` has enabled `ProjectRequest` variant
- [ ] All variants serialize/deserialize correctly
- [ ] Tests pass
- [ ] Code compiles without warnings

## Style Notes

- Place helper utility functions **at the bottom** of files
- Place more abstract things, entry points, and tests **first**
- Run `cargo +nightly fmt` on all changes before committing
- Keep language professional and restrained
- Avoid overly optimistic language like "comprehensive", "fully production ready"
- Avoid emoticons
- Code is never done, never perfect, never fully ready, never fully complete
```

### 02-implement-project-loading.md

```markdown
# Phase 2: Implement Project Loading and Unloading

## Goal

Implement project loading and unloading functionality in `ProjectManager`. Projects are loaded
from the filesystem and initialized automatically. Handles are generated sequentially.

## Tasks

1. Update `lp-server/src/project_manager.rs`:
   - Modify `ProjectManager` struct to use handle-based mapping:
     - `projects: HashMap<ProjectHandle, Project>`
     - `name_to_handle: HashMap<String, ProjectHandle>`
     - `next_handle_id: u32` (starts at 1)
   - Update `load_project()` signature to return `ProjectHandle`
   - Generate new handle (increment `next_handle_id`)
   - Extract project name from path (last component)
   - Create project-scoped filesystem using `chroot()`
   - Auto-initialize project runtime
   - Store handle -> project and name -> handle mappings
   - Update `unload_project()` to take `ProjectHandle` and remove both mappings
   - Update `get_project()` and `get_project_mut()` to use handles
   - Update `list_loaded_projects()` to return `Vec<LoadedProject>` with handles

2. Add helper method:
   - `get_handle_by_name()` for reverse lookup

## Success Criteria

- [ ] `ProjectManager::load_project()` generates handles and initializes projects
- [ ] `ProjectManager::unload_project()` removes projects by handle
- [ ] Projects are auto-initialized on load
- [ ] Handle generation is sequential and deterministic
- [ ] Project filesystem is scoped correctly
- [ ] Tests pass
- [ ] Code compiles without warnings

## Style Notes

- Place helper utility functions **at the bottom** of files
- Place more abstract things, entry points, and tests **first**
- Run `cargo +nightly fmt` on all changes before committing
- Keep language professional and restrained
- Avoid overly optimistic language like "comprehensive", "fully production ready"
- Avoid emoticons
- Code is never done, never perfect, never fully ready, never fully complete
```

### 03-implement-project-routing.md

```markdown
# Phase 3: Implement Project Request Routing

## Goal

Implement message handlers for project requests. Route `LoadProject`, `UnloadProject`, and
`ProjectRequest` messages to appropriate handlers.

## Tasks

1. Update `lp-server/src/handlers.rs`:
   - Modify `handle_client_message()` to route project requests
   - Implement `handle_load_project()`:
     - Extract project name from path
     - Call `ProjectManager::load_project()`
     - Return `ServerResponse::LoadProject { handle }`
   - Implement `handle_unload_project()`:
     - Call `ProjectManager::unload_project(handle)`
     - Return `ServerResponse::UnloadProject`
   - Implement `handle_project_request()`:
     - Get project by handle
     - Match `ProjectRequest` variant:
       - `GetChanges { since_frame, detail_specifier }`:
         - Call `runtime.get_changes(since_frame, detail_specifier)`
         - Convert `ProjectResponse` to `SerializableProjectResponse`
         - Return `ServerResponse::ProjectRequest { response }`
   - Implement `handle_list_available_projects()`:
     - Call `ProjectManager::list_available_projects(base_fs)`
     - Convert to `Vec<AvailableProject>`
     - Return `ServerResponse::ListAvailableProjects { projects }`
   - Implement `handle_list_loaded_projects()`:
     - Call `ProjectManager::list_loaded_projects()`
     - Return `ServerResponse::ListLoadedProjects { projects }`

2. Implement serialization helpers in `lp-model/src/project/api.rs`:
   - Create `SerializableNodeDetail` enum
   - Create `SerializableProjectResponse` enum
   - Implement conversion functions `NodeDetail::to_serializable()` and `ProjectResponse::to_serializable()`

## Success Criteria

- [ ] All project request handlers are implemented
- [ ] `handle_client_message()` routes project requests correctly
- [ ] `GetChanges` requests return serialized responses
- [ ] Serialization conversion works correctly
- [ ] Error handling is correct for all cases
- [ ] Tests pass
- [ ] Code compiles without warnings

## Style Notes

- Place helper utility functions **at the bottom** of files
- Place more abstract things, entry points, and tests **first**
- Run `cargo +nightly fmt` on all changes before committing
- Keep language professional and restrained
- Avoid overly optimistic language like "comprehensive", "fully production ready"
- Avoid emoticons
- Code is never done, never perfect, never fully ready, never fully complete
```

### 04-add-client-api.md

```markdown
# Phase 4: Add Client API Methods

## Goal

Add client API methods for project management. Methods send requests and extract responses,
handling request/response correlation.

## Tasks

1. Update `lp-client/src/client.rs`:
   - Add project management methods:
     - `project_load(&mut self, path: String) -> (Message, u64)`
     - `project_unload(&mut self, handle: ProjectHandle) -> (Message, u64)`
     - `project_get_changes(&mut self, handle: ProjectHandle, since_frame: FrameId, detail_specifier: ApiNodeSpecifier) -> (Message, u64)`
     - `project_list_available(&mut self) -> (Message, u64)`
     - `project_list_loaded(&mut self) -> (Message, u64)`
   - Add response extractor methods:
     - `extract_load_project_response(&mut self, request_id: u64, response: ServerResponse) -> Result<ProjectHandle, ClientError>`
     - `extract_unload_project_response(&mut self, request_id: u64, response: ServerResponse) -> Result<(), ClientError>`
     - `extract_get_changes_response(&mut self, request_id: u64, response: ServerResponse) -> Result<SerializableProjectResponse, ClientError>`
     - `extract_list_available_projects_response(&mut self, request_id: u64, response: ServerResponse) -> Result<Vec<AvailableProject>, ClientError>`
     - `extract_list_loaded_projects_response(&mut self, request_id: u64, response: ServerResponse) -> Result<Vec<LoadedProject>, ClientError>`

## Success Criteria

- [ ] Client has methods for all project operations
- [ ] Request/response correlation works correctly
- [ ] Methods return appropriate types
- [ ] Error handling is correct
- [ ] Tests pass
- [ ] Code compiles without warnings

## Style Notes

- Place helper utility functions **at the bottom** of files
- Place more abstract things, entry points, and tests **first**
- Run `cargo +nightly fmt` on all changes before committing
- Keep language professional and restrained
- Avoid overly optimistic language like "comprehensive", "fully production ready"
- Avoid emoticons
- Code is never done, never perfect, never fully ready, never fully complete
```

### 05-add-integration-tests.md

```markdown
# Phase 5: Add Integration Tests

## Goal

Add integration tests for project commands. Test the full flow from client to server and verify
project lifecycle operations.

## Tasks

1. Create `lp-client/tests/project_sync.rs`:
   - Add helper functions:
     - `create_test_project_on_client(fs: &mut dyn LpFs) -> String`
     - `sync_project_to_server(...)` - Write all project files to server via FS operations
     - `load_project_on_server(...) -> ProjectHandle` - Send LoadProject request, process messages, return handle
     - `verify_project_loaded(server: &LpServer, handle: ProjectHandle) -> bool`
     - `verify_project_running(server: &mut LpServer, handle: ProjectHandle) -> Result<(), Error>`
   - Add test cases:
     - `test_project_load_unload()` - Load project, verify loaded, unload, verify unloaded
     - `test_project_list_operations()` - List available, load, list loaded
     - `test_project_lifecycle()` - Create on client, sync to server, load, verify running, unload
     - `test_project_get_changes()` - Load project, send GetChanges request, verify response
     - `test_project_serialization()` - Verify SerializableNodeDetail and SerializableProjectResponse round-trip

## Success Criteria

- [ ] Integration tests cover all project operations
- [ ] Tests verify project lifecycle
- [ ] Tests verify serialization
- [ ] All tests pass
- [ ] Code compiles without warnings

## Style Notes

- Place helper utility functions **at the bottom** of files
- Place more abstract things, entry points, and tests **first**
- Run `cargo +nightly fmt` on all changes before committing
- Keep language professional and restrained
- Avoid overly optimistic language like "comprehensive", "fully production ready"
- Avoid emoticons
- Code is never done, never perfect, never fully ready, never fully complete
```

### 06-cleanup.md

```markdown
# Phase 6: Cleanup and Finalization

## Goal

Remove temporary code, fix warnings, ensure all tests pass, and format code.

## Tasks

1. Review code for temporary code, TODOs, debug prints
2. Fix all warnings
3. Run all tests and ensure they pass
4. Run `cargo +nightly fmt` on entire workspace
5. Review code for clarity and consistency
6. Move plan directory to `plans/_done/`

## Success Criteria

- [ ] No temporary code or TODOs remain
- [ ] All warnings are fixed
- [ ] All tests pass
- [ ] Code is formatted with `cargo +nightly fmt`
- [ ] Code is clean and readable
- [ ] Plan directory moved to `_done/`

## Style Notes

- Place helper utility functions **at the bottom** of files
- Place more abstract things, entry points, and tests **first**
- Run `cargo +nightly fmt` on all changes before committing
- Keep language professional and restrained
- Avoid overly optimistic language like "comprehensive", "fully production ready"
- Avoid emoticons
- Code is never done, never perfect, never fully ready, never fully complete
```

## Key Points

1. **Style Notes**: Every phase file must include the style notes section to keep them in context
2. **Success Criteria**: Each phase should have specific, measurable success criteria
3. **Implementation Notes**: Include relevant implementation details and references to design
4. **Professional Tone**: Use measured, factual language without overly optimistic claims
5. **Code Organization**: Helper functions at bottom, abstract/entry points/tests first
6. **Formatting**: Always run `cargo +nightly fmt` before committing
