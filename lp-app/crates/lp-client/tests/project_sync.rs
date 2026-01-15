//! End-to-end project management tests
//!
//! Tests client-server project management operations using in-memory transport.
//! Verifies that projects can be loaded, unloaded, listed, and queried correctly.
//!
//! Uses the tick-based API for both client and server, allowing synchronous
//! message processing in tests.

extern crate alloc;

use alloc::{boxed::Box, rc::Rc, string::String};
use core::cell::RefCell;
use lp_client::{ClientError, LpClient, LocalMemoryTransport};
use lp_model::{
    project::{
        api::{ApiNodeSpecifier, SerializableProjectResponse},
        handle::ProjectHandle,
        FrameId,
    },
    Message,
};
use lp_server::LpServer;
use lp_shared::fs::{LpFs, LpFsMemory, LpFsMemoryShared};
use lp_shared::output::MemoryOutputProvider;
use lp_shared::transport::{ClientTransport, ServerTransport};

#[test]
fn test_project_load_unload() {
    let mut client_fs = LpFsMemory::new();
    create_test_project_on_client(&mut client_fs);

    let client_fs_shared = LpFsMemoryShared::new(client_fs);
    let server_fs = LpFsMemory::new();
    let (mut server, mut client, mut client_transport, mut server_transport) =
        setup_server_and_client(server_fs);

    // Sync project to server
    sync_project_to_server(
        &mut client,
        &mut client_transport,
        &mut server_transport,
        &mut server,
        "test",
        &client_fs_shared,
    )
    .unwrap();

    // Load project
    let handle = load_project_on_server(
        &mut client,
        &mut client_transport,
        &mut server_transport,
        &mut server,
        "projects/test",
    )
    .unwrap();

    // Verify project is loaded
    assert!(
        verify_project_loaded(&server, handle),
        "Project should be loaded"
    );

    // Unload project
    let (unload_msg, unload_id) = client.project_unload(handle);
    send_client_message(&mut client_transport, unload_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();

    let unload_response = client.get_response(unload_id).unwrap();
    client
        .extract_unload_project_response(unload_id, unload_response)
        .unwrap();

    // Verify project is unloaded
    assert!(
        !verify_project_loaded(&server, handle),
        "Project should be unloaded"
    );
}

#[test]
fn test_project_list_operations() {
    let mut client_fs = LpFsMemory::new();
    create_test_project_on_client(&mut client_fs);

    let client_fs_shared = LpFsMemoryShared::new(client_fs);
    let server_fs = LpFsMemory::new();
    let (mut server, mut client, mut client_transport, mut server_transport) =
        setup_server_and_client(server_fs);

    // Sync project to server
    sync_project_to_server(
        &mut client,
        &mut client_transport,
        &mut server_transport,
        &mut server,
        "test",
        &client_fs_shared,
    )
    .unwrap();

    // List available projects (should include our project)
    let (list_msg, list_id) = client.project_list_available();
    send_client_message(&mut client_transport, list_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();

    let list_response = client.get_response(list_id).unwrap();
    let available = client
        .extract_list_available_projects_response(list_id, list_response)
        .unwrap();
    assert!(
        !available.is_empty(),
        "Should have at least one available project"
    );

    // Load project
    let handle = load_project_on_server(
        &mut client,
        &mut client_transport,
        &mut server_transport,
        &mut server,
        "projects/test",
    )
    .unwrap();

    // List loaded projects (should include our project)
    let (loaded_msg, loaded_id) = client.project_list_loaded();
    send_client_message(&mut client_transport, loaded_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();

    let loaded_response = client.get_response(loaded_id).unwrap();
    let loaded = client
        .extract_list_loaded_projects_response(loaded_id, loaded_response)
        .unwrap();
    assert_eq!(loaded.len(), 1, "Should have one loaded project");
    assert_eq!(
        loaded[0].handle, handle,
        "Loaded project should match handle"
    );
}

#[test]
fn test_project_lifecycle() {
    let mut client_fs = LpFsMemory::new();
    create_test_project_on_client(&mut client_fs);

    let client_fs_shared = LpFsMemoryShared::new(client_fs);
    let server_fs = LpFsMemory::new();
    let (mut server, mut client, mut client_transport, mut server_transport) =
        setup_server_and_client(server_fs);

    // Sync project to server
    sync_project_to_server(
        &mut client,
        &mut client_transport,
        &mut server_transport,
        &mut server,
        "test",
        &client_fs_shared,
    )
    .unwrap();

    // Load project
    let handle = load_project_on_server(
        &mut client,
        &mut client_transport,
        &mut server_transport,
        &mut server,
        "projects/test",
    )
    .unwrap();

    // Verify project is running
    verify_project_running(&mut server, handle).unwrap();

    // Unload project
    let (unload_msg, unload_id) = client.project_unload(handle);
    send_client_message(&mut client_transport, unload_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();

    let unload_response = client.get_response(unload_id).unwrap();
    client
        .extract_unload_project_response(unload_id, unload_response)
        .unwrap();
}

#[test]
fn test_project_get_changes() {
    let mut client_fs = LpFsMemory::new();
    create_test_project_on_client(&mut client_fs);

    let client_fs_shared = LpFsMemoryShared::new(client_fs);
    let server_fs = LpFsMemory::new();
    let (mut server, mut client, mut client_transport, mut server_transport) =
        setup_server_and_client(server_fs);

    // Sync project to server
    sync_project_to_server(
        &mut client,
        &mut client_transport,
        &mut server_transport,
        &mut server,
        "test",
        &client_fs_shared,
    )
    .unwrap();

    // Load project
    let handle = load_project_on_server(
        &mut client,
        &mut client_transport,
        &mut server_transport,
        &mut server,
        "projects/test",
    )
    .unwrap();

    // Advance the project a few frames
    {
        let project = server
            .project_manager_mut()
            .get_project_mut(handle)
            .unwrap();
        project.runtime_mut().tick(4).unwrap();
        project.runtime_mut().tick(4).unwrap();
    }

    // Get changes
    let (changes_msg, changes_id) =
        client.project_get_changes(handle, FrameId::default(), ApiNodeSpecifier::All);
    send_client_message(&mut client_transport, changes_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();

    let changes_response = client.get_response(changes_id).unwrap();
    let changes = client
        .extract_get_changes_response(changes_id, changes_response)
        .unwrap();

    match changes {
        SerializableProjectResponse::GetChanges {
            current_frame,
            node_handles,
            node_changes: _,
            node_details: _,
        } => {
            assert!(current_frame.as_i64() > 0, "Should have advanced frames");
            assert!(!node_handles.is_empty(), "Should have nodes");
        }
    }
}

/// Set up server and client with memory transport
///
/// Returns `(server, client, client_transport, server_transport)` for
/// synchronous message processing in tests.
fn setup_server_and_client(
    fs: LpFsMemory,
) -> (LpServer, LpClient, LocalMemoryTransport, LocalMemoryTransport) {
    // Create transport pair
    let (client_transport, server_transport) = LocalMemoryTransport::new_pair();

    // Create server with shared filesystem (allows mutation through immutable trait)
    let output_provider = Rc::new(RefCell::new(MemoryOutputProvider::new()));
    let shared_fs = LpFsMemoryShared::new(fs);
    let server = LpServer::new(output_provider, Box::new(shared_fs), "projects".to_string());

    // Create client
    let client = LpClient::new();

    (server, client, client_transport, server_transport)
}

/// Extract ClientMessage from Message envelope and send via transport
fn send_client_message(
    transport: &mut LocalMemoryTransport,
    msg: Message,
) -> Result<(), ClientError> {
    let client_msg = match msg {
        Message::Client(msg) => msg,
        Message::Server(_) => {
            return Err(ClientError::Protocol {
                message: "Expected Client message".to_string(),
            });
        }
    };
    ClientTransport::send(transport, client_msg).map_err(ClientError::from)
}

/// Process messages synchronously between client and server
///
/// This bridges messages through the transport, processing them on both
/// client and server using their tick() methods.
fn process_messages(
    client: &mut LpClient,
    server: &mut LpServer,
    client_transport: &mut LocalMemoryTransport,
    server_transport: &mut LocalMemoryTransport,
) -> Result<(), ClientError> {
    // Process client -> server messages
    loop {
        match ServerTransport::receive(&mut *server_transport) {
            Ok(Some(client_msg)) => {
                // Transport handles deserialization, we get ClientMessage directly
                // Wrap in Message envelope for server.tick()
                let message = Message::Client(client_msg);

                // Process on server
                let responses = server
                    .tick(0, vec![message])
                    .map_err(|e| ClientError::Other {
                        message: format!("Server error: {}", e),
                    })?;

                // Send responses back through server transport
                for response in responses {
                    // Extract ServerMessage from Message envelope
                    match response {
                        Message::Server(server_msg) => {
                            ServerTransport::send(&mut *server_transport, server_msg)
                                .map_err(ClientError::from)?;
                        }
                        Message::Client(_) => {
                            return Err(ClientError::Protocol {
                                message: "Server received client message".to_string(),
                            });
                        }
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                return Err(ClientError::Transport(e));
            }
        }
    }

    // Process server -> client messages
    loop {
        match ClientTransport::receive(&mut *client_transport) {
            Ok(Some(server_msg)) => {
                // Transport handles deserialization, we get ServerMessage directly
                // Wrap in Message envelope for client.tick()
                let message = Message::Server(server_msg);

                // Process on client
                let _outgoing = client.tick(vec![message]).map_err(|e| ClientError::Other {
                    message: format!("Client error: {}", e),
                })?;
            }
            Ok(None) => break,
            Err(e) => {
                return Err(ClientError::Transport(e));
            }
        }
    }

    Ok(())
}

/// Create a test project on the client filesystem
///
/// Uses ProjectBuilder to create a simple project with texture, shader, output, and fixture.
/// Returns the project name.
fn create_test_project_on_client(fs: &mut LpFsMemory) -> String {
    let mut builder = ProjectBuilder::new(fs);
    let texture_path = builder.texture_basic();
    builder.shader_basic(&texture_path);
    let output_path = builder.output_basic();
    builder.fixture_basic(&output_path, &texture_path);
    builder.build();
    String::from("test")
}

/// Sync project files from client to server
///
/// Reads all files from the client filesystem and writes them to the server
/// via filesystem operations. ProjectBuilder creates files at project root,
/// so we list from "/" and sync them to "/projects/{project_name}/".
fn sync_project_to_server(
    client: &mut LpClient,
    client_transport: &mut MemoryTransport,
    server_transport: &mut MemoryTransport,
    server: &mut LpServer,
    project_name: &str,
    client_fs: &LpFsMemoryShared,
) -> Result<(), ClientError> {
    // List all files in the project recursively (from client FS root)
    let entries = client_fs
        .get_mut()
        .list_dir("/", true)
        .map_err(|e| ClientError::Other {
            message: format!("Failed to list client files: {}", e),
        })?;

    // Write each file to server (adjusting path to include projects/{name}/ prefix)
    for entry in entries {
        if entry.ends_with(".json") || entry.ends_with(".glsl") {
            // Read file from client filesystem directly
            let content =
                client_fs
                    .get_mut()
                    .read_file(&entry)
                    .map_err(|e| ClientError::Other {
                        message: format!("Failed to read client file {}: {}", entry, e),
                    })?;

            // Write file to server (add projects/{name}/ prefix)
            let server_path = if entry.starts_with('/') {
                format!("/projects/{}{}", project_name, entry)
            } else {
                format!("/projects/{}/{}", project_name, entry)
            };

            let (write_msg, write_id) = client.fs_write(server_path.clone(), content);
            send_client_message(&mut *client_transport, write_msg).unwrap();
            process_messages(client, server, client_transport, server_transport)?;

            let write_response = client.get_response(write_id).unwrap();
            client
                .extract_write_response(write_id, write_response)
                .unwrap();
        }
    }

    Ok(())
}

/// Load a project on the server
///
/// Sends LoadProject request and processes messages, returning the handle.
fn load_project_on_server(
    client: &mut LpClient,
    client_transport: &mut MemoryTransport,
    server_transport: &mut MemoryTransport,
    server: &mut LpServer,
    project_path: &str,
) -> Result<ProjectHandle, ClientError> {
    let (request_msg, request_id) = client.project_load(project_path.to_string());
    send_client_message(&mut *client_transport, request_msg).unwrap();
    process_messages(client, server, client_transport, server_transport)?;

    let response = client.get_response(request_id).unwrap();
    client.extract_load_project_response(request_id, response)
}

/// Verify a project is loaded on the server
fn verify_project_loaded(server: &LpServer, handle: ProjectHandle) -> bool {
    server.project_manager().get_project(handle).is_some()
}

/// Verify a project is running (can tick)
fn verify_project_running(server: &mut LpServer, handle: ProjectHandle) -> Result<(), String> {
    let project = server
        .project_manager_mut()
        .get_project_mut(handle)
        .ok_or_else(|| format!("Project {} not found", handle.id()))?;

    // Try to tick the project runtime
    project
        .runtime_mut()
        .tick(4)
        .map_err(|e| format!("Failed to tick project: {}", e))?;

    Ok(())
}
