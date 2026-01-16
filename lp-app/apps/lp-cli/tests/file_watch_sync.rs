//! Unit tests for file watching and syncing functionality
//!
//! Tests file change detection and syncing using memory filesystem to simulate
//! local file changes and verify they are synced to the server.

extern crate alloc;

use alloc::{boxed::Box, rc::Rc};
use core::cell::RefCell;
use lp_server::LpServer;
use lp_shared::fs::{LpFs, LpFsMemory, fs_event::ChangeType};
use lp_shared::output::MemoryOutputProvider;
use lp_shared::transport::{ClientTransport, ServerTransport};
use std::sync::Arc;
use tokio::sync::Mutex;

use lp_cli::client::local::{AsyncLocalServerTransport, create_local_transport_pair};
use lp_cli::dev::async_client::AsyncLpClient;
use lp_cli::dev::sync::sync_file_change;

/// Set up test environment with server, client, and memory filesystems
///
/// Returns:
/// - Server instance
/// - AsyncLpClient instance
/// - Client filesystem (simulating local project directory)
/// - Server filesystem (server's base filesystem)
fn setup_test_environment() -> (
    LpServer,
    AsyncLpClient,
    LpFsMemory,
    LpFsMemory,
    AsyncLocalServerTransport, // server_transport
) {
    // Create transport pair (using async-safe transports)
    let (client_transport, server_transport) = create_local_transport_pair();

    // Create server with memory filesystem
    let output_provider = Rc::new(RefCell::new(MemoryOutputProvider::new()));
    let server_fs = LpFsMemory::new();
    // Create a new instance for the server (LpFsMemory doesn't implement Clone)
    let server_fs_for_server = LpFsMemory::new();
    let server = LpServer::new(
        output_provider,
        Box::new(server_fs_for_server),
        "projects".to_string(),
    );

    // Create client filesystem (simulating local project directory)
    let client_fs = LpFsMemory::new();

    // Create async client with shared transport
    // Note: We need to wrap the transport, but LocalTransport is not Clone
    // For testing, we'll create a wrapper that implements the trait
    let shared_transport: Arc<Mutex<Box<dyn ClientTransport + Send>>> =
        Arc::new(Mutex::new(Box::new(client_transport)));
    let async_client = AsyncLpClient::new(shared_transport);

    (server, async_client, client_fs, server_fs, server_transport)
}

/// Process messages on server side only (for testing async client)
fn process_messages_server_only(
    server_transport: &mut AsyncLocalServerTransport,
    server: &mut LpServer,
) -> Result<(), String> {
    // Process client -> server messages
    loop {
        match ServerTransport::receive(server_transport) {
            Ok(Some(client_msg)) => {
                use lp_model::Message;
                let message = Message::Client(client_msg);

                let responses = server
                    .tick(0, vec![message])
                    .map_err(|e| format!("Server error: {}", e))?;

                for response in responses {
                    if let Message::Server(server_msg) = response {
                        ServerTransport::send(server_transport, server_msg)
                            .map_err(|e| format!("Failed to send server message: {}", e))?;
                    }
                }
            }
            Ok(None) => break,
            Err(e) => return Err(format!("Transport error: {}", e)),
        }
    }

    Ok(())
}

/// Process messages between client and server (synchronous helper for tests)
fn process_messages(
    _client_transport: &mut AsyncLocalServerTransport,
    server_transport: &mut AsyncLocalServerTransport,
    server: &mut LpServer,
) -> Result<(), String> {
    process_messages_server_only(server_transport, server)
}

/// Simulate a file change by modifying the client filesystem
///
/// This simulates what the file watcher would detect.
fn simulate_file_change(
    fs: &mut LpFsMemory,
    path: &str,
    change_type: ChangeType,
    content: Option<&[u8]>,
) {
    match change_type {
        ChangeType::Create | ChangeType::Modify => {
            if let Some(data) = content {
                fs.write_file(path, data).unwrap();
            }
        }
        ChangeType::Delete => {
            fs.delete_file(path).unwrap();
        }
    }
}

/// Verify that a file exists on the server filesystem with expected content
fn verify_server_file(
    server_fs: &LpFsMemory,
    expected_path: &str,
    expected_content: &[u8],
) -> Result<(), String> {
    let content = server_fs
        .read_file(expected_path)
        .map_err(|e| format!("File {} not found on server: {}", expected_path, e))?;

    if content != expected_content {
        return Err(format!(
            "File {} content mismatch. Expected {:?}, got {:?}",
            expected_path, expected_content, content
        ));
    }

    Ok(())
}

/// Verify that a file does not exist on the server filesystem
fn verify_server_file_not_exists(server_fs: &LpFsMemory, path: &str) -> Result<(), String> {
    match server_fs.read_file(path) {
        Ok(_) => Err(format!("File {} should not exist on server", path)),
        Err(_) => Ok(()),
    }
}

// TODO: Will be re-enabled in phase 6 when sync_file_change is properly implemented
#[tokio::test]
#[ignore]
async fn test_sync_file_create() {
    let (mut server, mut async_client, mut client_fs, server_fs, mut server_transport) =
        setup_test_environment();

    let project_uid = "test-project";
    let project_dir = std::path::Path::new("/tmp"); // Dummy path for test

    // Simulate creating a new file on client
    let file_path = "/src/test.glsl";
    let file_content = b"void main() { }";
    simulate_file_change(
        &mut client_fs,
        file_path,
        ChangeType::Create,
        Some(file_content),
    );

    // Get the change from client filesystem
    let changes = client_fs.get_changes();
    assert_eq!(changes.len(), 1);
    let change = &changes[0];
    assert_eq!(change.path, file_path);
    assert_eq!(change.change_type, ChangeType::Create);

    // Sync the change to server
    sync_file_change(
        &mut async_client,
        change,
        project_uid,
        project_dir,
        &client_fs,
    )
    .await
    .unwrap();

    // Process messages to ensure server receives the write request
    // Note: We can't access client_transport directly, but the async client handles it internally
    // For testing, we'll process server-side messages manually
    process_messages_server_only(&mut server_transport, &mut server).unwrap();

    // Give async client time to process response
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify file was synced to server
    let server_path = format!("/projects/{}{}", project_uid, file_path);
    verify_server_file(&server_fs, &server_path, file_content).unwrap();
}

// TODO: Will be re-enabled in phase 6 when sync_file_change is properly implemented
#[tokio::test]
#[ignore]
async fn test_sync_file_modify() {
    let (mut server, mut async_client, mut client_fs, server_fs, mut server_transport) =
        setup_test_environment();

    let project_uid = "test-project";
    let project_dir = std::path::Path::new("/tmp");

    // Create initial file
    let file_path = "/src/test.glsl";
    let initial_content = b"void main() { }";
    simulate_file_change(
        &mut client_fs,
        file_path,
        ChangeType::Create,
        Some(initial_content),
    );

    // Reset changes tracking
    client_fs.reset_changes();

    // Modify the file
    let modified_content = b"void main() { gl_FragColor = vec4(1.0); }";
    simulate_file_change(
        &mut client_fs,
        file_path,
        ChangeType::Modify,
        Some(modified_content),
    );

    // Get the change
    let changes = client_fs.get_changes();
    assert_eq!(changes.len(), 1);
    let change = &changes[0];
    assert_eq!(change.change_type, ChangeType::Modify);

    // Sync the change to server
    sync_file_change(
        &mut async_client,
        change,
        project_uid,
        project_dir,
        &client_fs,
    )
    .await
    .unwrap();

    // Process messages
    process_messages_server_only(&mut server_transport, &mut server).unwrap();

    // Give async client time to process response
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify file was updated on server
    let server_path = format!("/projects/{}{}", project_uid, file_path);
    verify_server_file(&server_fs, &server_path, modified_content).unwrap();
}

// TODO: Will be re-enabled in phase 6 when sync_file_change is properly implemented
#[tokio::test]
#[ignore]
async fn test_sync_multiple_changes() {
    let (mut server, mut async_client, mut client_fs, server_fs, mut server_transport) =
        setup_test_environment();

    let project_uid = "test-project";
    let project_dir = std::path::Path::new("/tmp");

    // Create multiple files
    let file1_path = "/src/file1.glsl";
    let file1_content = b"file1 content";
    simulate_file_change(
        &mut client_fs,
        file1_path,
        ChangeType::Create,
        Some(file1_content),
    );

    let file2_path = "/src/file2.glsl";
    let file2_content = b"file2 content";
    simulate_file_change(
        &mut client_fs,
        file2_path,
        ChangeType::Create,
        Some(file2_content),
    );

    // Get all changes
    let changes = client_fs.get_changes();
    assert_eq!(changes.len(), 2);

    // Sync all changes (one at a time for testing)
    for change in &changes {
        sync_file_change(
            &mut async_client,
            &change,
            project_uid,
            project_dir,
            &client_fs,
        )
        .await
        .unwrap();
    }

    // Process messages
    process_messages_server_only(&mut server_transport, &mut server).unwrap();

    // Give async client time to process responses
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Verify both files were synced
    let server_path1 = format!("/projects/{}{}", project_uid, file1_path);
    let server_path2 = format!("/projects/{}{}", project_uid, file2_path);
    verify_server_file(&server_fs, &server_path1, file1_content).unwrap();
    verify_server_file(&server_fs, &server_path2, file2_content).unwrap();
}

// TODO: Will be re-enabled in phase 6 when sync_file_change is properly implemented
#[tokio::test]
#[ignore]
async fn test_sync_file_delete() {
    let (mut server, mut async_client, mut client_fs, server_fs, mut server_transport) =
        setup_test_environment();

    let project_uid = "test-project";
    let project_dir = std::path::Path::new("/tmp");

    // First create a file and sync it
    let file_path = "/src/test.glsl";
    let file_content = b"void main() { }";
    simulate_file_change(
        &mut client_fs,
        file_path,
        ChangeType::Create,
        Some(file_content),
    );

    let changes = client_fs.get_changes();
    sync_file_change(
        &mut async_client,
        &changes[0],
        project_uid,
        project_dir,
        &client_fs,
    )
    .await
    .unwrap();

    process_messages_server_only(&mut server_transport, &mut server).unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify file exists on server
    let server_path = format!("/projects/{}{}", project_uid, file_path);
    verify_server_file(&server_fs, &server_path, file_content).unwrap();

    // Reset changes and delete the file
    client_fs.reset_changes();
    simulate_file_change(&mut client_fs, file_path, ChangeType::Delete, None);

    // Get delete change
    let changes = client_fs.get_changes();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].change_type, ChangeType::Delete);

    // Sync delete (should log warning but not crash)
    sync_file_change(
        &mut async_client,
        &changes[0],
        project_uid,
        project_dir,
        &client_fs,
    )
    .await
    .unwrap();

    // Note: Delete sync is not yet implemented, so file will still exist on server
    // This test verifies that delete events don't crash the sync function
}
