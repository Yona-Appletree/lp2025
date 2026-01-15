### 1. Title and Overview (Executive Summary)

Start with a clear title and a high-level overview section that summarizes:

- What the design accomplishes
- Key goals and scope
- How it fits into the larger system

Example:

```markdown
# Design: Project Commands

## Overview

Add support for project management commands in the client-server system:

- Load projects from the server filesystem
- Send messages to loaded projects (GetChanges)
- Close/unload projects
- List available projects (on disk) and loaded projects (in memory)

This builds on the filesystem sync foundation and enables clients to manage and interact with projects running on the server.
```

### 2. File Structure Summary

Include a file tree showing all relevant files with annotations indicating what changes:

- `# NEW:` for new files
- `# MODIFY:` for modified files
- `# REMOVE:` for deleted files
- `# (no changes)` for files that remain unchanged but are relevant context

Example:

```
lp-model/src/
├── message.rs                 # MODIFY: Add project variants to ClientRequest
├── server/
│   ├── api.rs                # MODIFY: Enable ProjectRequest variant in ServerResponse
│   └── fs_api.rs             # (no changes)
└── project/
    ├── api.rs                # MODIFY: Add SerializableNodeDetail, SerializableProjectResponse
    └── handle.rs             # (no changes)
```

### 3. Type Tree Summary

Provide a summary of types, functions, and their changes organized by file/module:

Example:

````
### lp-model/src/message.rs
- `pub enum ClientRequest` - **MODIFY**: Add project management variants:
  ```rust
  pub enum ClientRequest {
      Filesystem(FsRequest),
      LoadProject { path: String },
      UnloadProject { handle: ProjectHandle },
      ProjectRequest { handle: ProjectHandle, request: ProjectRequest },
      ListAvailableProjects,
      ListLoadedProjects,
  }
````

### lp-model/src/server/api.rs

- `pub enum ServerResponse` - **MODIFY**: Enable `ProjectRequest` variant:
  ```rust
  pub enum ServerResponse {
      Filesystem(FsResponse),
      LoadProject { handle: ProjectHandle },
      UnloadProject,
      ProjectRequest { response: SerializableProjectResponse },
      ListAvailableProjects { projects: Vec<AvailableProject> },
      ListLoadedProjects { projects: Vec<LoadedProject> },
  }
  ```

```

### 4. Process Flow (ASCII Art)

Include ASCII art diagrams showing the flow of operations, data flow, or interaction patterns:

Example:
```

## Process Flow

### Project Loading Flow

```
Client                    Server                    ProjectManager
  |                         |                              |
  |-- LoadProject(path) -->|                              |
  |                         |-- load_project(name) ------->|
  |                         |                              |-- Create Project
  |                         |                              |-- Initialize Runtime
  |                         |<-- handle -------------------|
  |<-- LoadProject{handle}--|                              |
  |                         |                              |
```

### Message Routing

```
ClientRequest
    |
    +-- Filesystem(FsRequest) -----> handle_fs_request()
    |
    +-- LoadProject { path } -------> handle_load_project()
    |
    +-- ProjectRequest { handle, request } --> handle_project_request()
                                              |
                                              +-- GetChanges --> project.get_changes()
```

````

### 5. Detailed Sections

After the summaries, include detailed sections covering:

#### Design Decisions
Document key architectural choices and their rationale:

```markdown
## Design Decisions

### 1. Client Request Structure
**Decision**: Direct mapping - `ClientRequest` has same variants as `ServerRequest`
- Simpler structure, no extra nesting
- Consistent with filesystem requests
- Easier to route on server side

### 2. Project Handle Generation
**Decision**: Sequential IDs starting from 1, incrementing on each load (no reuse)
- Simple and deterministic (good for tests)
- No need to track freed handles
- Handles are opaque identifiers
````

#### Implementation Details

Provide specific implementation notes, algorithms, or patterns:

```markdown
## Implementation Notes

### Path Extraction Logic

For `LoadProject` path extraction:

1. If path starts with `projects_base_dir` prefix, strip it
2. Extract last component as project name
3. Examples:
   - `projects_base_dir = "projects"`, path = `"projects/my-project"` -> name = `"my-project"`
   - `projects_base_dir = "projects"`, path = `"my-project"` -> name = `"my-project"`
```

#### Error Handling

Document error cases and how they're handled:

```markdown
## Error Handling

- `LoadProject`: Returns `ServerError::ProjectNotFound` if project doesn't exist on filesystem
- `LoadProject`: Returns `ServerError::Core` if project initialization fails
- `UnloadProject`: Returns `ServerError::ProjectNotFound` if handle doesn't exist
```

#### Testing Strategy

Outline testing approach:

```markdown
## Testing Strategy

### Integration Tests (`lp-client/tests/project_sync.rs`)

Helper functions:

- `create_test_project_on_client(fs: &mut dyn LpFs) -> String`
- `load_project_on_server(...) -> ProjectHandle`
- `verify_project_loaded(server: &LpServer, handle: ProjectHandle) -> bool`

Test cases:

1. `test_project_load_unload()` - Load project, verify loaded, unload, verify unloaded
2. `test_project_get_changes()` - Load project, send GetChanges request, verify response
```

#### Success Criteria

List measurable success criteria:

```markdown
## Success Criteria

- [ ] Client can send `LoadProject` request and receive handle
- [ ] Client can send `UnloadProject` request and project is unloaded
- [ ] Client can list available projects
- [ ] All tests pass
- [ ] Code compiles without warnings
```

### 6. Notes Section (Optional)

Any additional notes, future considerations, or known limitations:

```markdown
## Notes

- `SerializableNodeDetail` conversion requires downcasting `Box<dyn NodeConfig>` to concrete types
- Project auto-initialization ensures projects are ready to use immediately after loading
- Handle generation is sequential and deterministic for easier testing
```

### Structure Summary

The complete structure should be:

1. **Title and Overview** - Executive summary
2. **File Structure** - Code block with file tree
3. **Type Tree** - Code blocks with type summaries organized by module
4. **Process Flow** - ASCII art diagrams (if applicable)
5. **Design Decisions** - Key architectural choices
6. **Implementation Notes** - Specific implementation details
7. **Error Handling** - Error cases and handling
8. **Testing Strategy** - Testing approach
9. **Success Criteria** - Measurable goals
10. **Notes** - Additional context (optional)
