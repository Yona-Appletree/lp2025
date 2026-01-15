//! End-to-end filesystem sync tests
//!
//! Tests client-server filesystem synchronization using in-memory transport.
//! Verifies that all filesystem operations work correctly with request/response
//! correlation and JSON serialization.
//!
//! Uses the tick-based API for both client and server, allowing synchronous
//! message processing in tests.

extern crate alloc;

use alloc::{boxed::Box, rc::Rc};
use core::cell::RefCell;
use lp_client::{ClientError, LocalMemoryTransport, LpClient};
use lp_model::Message;
use lp_server::LpServer;
use lp_shared::fs::{LpFsMemory, LpFsMemoryShared};
use lp_shared::output::MemoryOutputProvider;
use lp_shared::transport::{ClientTransport, ServerTransport};

/// Set up server and client with memory transport
///
/// Returns `(server, client, client_transport, server_transport)` for
/// synchronous message processing in tests.
fn setup_server_and_client(
    fs: LpFsMemory,
) -> (
    LpServer,
    LpClient,
    LocalMemoryTransport,
    LocalMemoryTransport,
) {
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
    // Client sends through client_transport -> goes to client_to_server queue
    // Server receives from server_transport -> reads from client_to_server queue
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
    // Server sends responses through server_transport -> goes to server_to_client queue
    // Client receives from client_transport -> reads from server_to_client queue
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

                // Outgoing messages from client would be sent here if needed
                // For now, client only processes incoming responses
            }
            Ok(None) => break,
            Err(e) => {
                return Err(ClientError::Transport(e));
            }
        }
    }

    Ok(())
}

#[test]
fn test_filesystem_sync() {
    // Set up filesystem with initial files
    let mut fs = LpFsMemory::new();
    fs.write_file_mut("/projects/test/file1.txt", b"content1")
        .unwrap();
    fs.write_file_mut("/projects/test/nested/file2.txt", b"content2")
        .unwrap();

    // Create server and client
    let (mut server, mut client, mut client_transport, mut server_transport) =
        setup_server_and_client(fs);

    // Test read
    let (request_msg, request_id) = client.fs_read("/projects/test/file1.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let content = client.extract_read_response(request_id, response).unwrap();
    assert_eq!(content, b"content1");

    // Test write
    let (request_msg, request_id) = client.fs_write(
        "/projects/test/new.txt".to_string(),
        b"new content".to_vec(),
    );
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    client.extract_write_response(request_id, response).unwrap();

    // Verify write by reading
    let (request_msg, request_id) = client.fs_read("/projects/test/new.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let content = client.extract_read_response(request_id, response).unwrap();
    assert_eq!(content, b"new content");

    // Test list (non-recursive)
    let (request_msg, request_id) = client.fs_list_dir("/projects/test".to_string(), false);
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let entries = client
        .extract_list_dir_response(request_id, response)
        .unwrap();
    assert!(entries.contains(&"/projects/test/file1.txt".to_string()));
    assert!(entries.contains(&"/projects/test/new.txt".to_string()));
    assert!(entries.contains(&"/projects/test/nested".to_string()));

    // Test list (recursive)
    let (request_msg, request_id) = client.fs_list_dir("/projects/test".to_string(), true);
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let entries = client
        .extract_list_dir_response(request_id, response)
        .unwrap();
    assert!(entries.contains(&"/projects/test/nested/file2.txt".to_string()));

    // Test delete file
    let (request_msg, request_id) = client.fs_delete_file("/projects/test/new.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    client
        .extract_delete_file_response(request_id, response)
        .unwrap();

    // Test delete directory (recursive)
    let (request_msg, request_id) = client.fs_delete_dir("/projects/test/nested".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    client
        .extract_delete_dir_response(request_id, response)
        .unwrap();

    // Verify deletions
    let (request_msg, request_id) = client.fs_list_dir("/projects/test".to_string(), true);
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let entries = client
        .extract_list_dir_response(request_id, response)
        .unwrap();
    assert!(!entries.contains(&"/projects/test/new.txt".to_string()));
    assert!(!entries.contains(&"/projects/test/nested".to_string()));
}

#[test]
fn test_fs_read() {
    let mut fs = LpFsMemory::new();
    fs.write_file_mut("/test.txt", b"hello world").unwrap();

    let (mut server, mut client, mut client_transport, mut server_transport) =
        setup_server_and_client(fs);
    let (request_msg, request_id) = client.fs_read("/test.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let content = client.extract_read_response(request_id, response).unwrap();
    assert_eq!(content, b"hello world");
}

#[test]
fn test_fs_write() {
    let fs = LpFsMemory::new();
    let (mut server, mut client, mut client_transport, mut server_transport) =
        setup_server_and_client(fs);

    let (request_msg, request_id) =
        client.fs_write("/test.txt".to_string(), b"written content".to_vec());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    client.extract_write_response(request_id, response).unwrap();

    let (request_msg, request_id) = client.fs_read("/test.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let content = client.extract_read_response(request_id, response).unwrap();
    assert_eq!(content, b"written content");
}

#[test]
fn test_fs_delete_file() {
    let mut fs = LpFsMemory::new();
    fs.write_file_mut("/test.txt", b"content").unwrap();

    let (mut server, mut client, mut client_transport, mut server_transport) =
        setup_server_and_client(fs);

    // Verify file exists
    let (request_msg, request_id) = client.fs_read("/test.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let _content = client.extract_read_response(request_id, response).unwrap();

    // Delete file
    let (request_msg, request_id) = client.fs_delete_file("/test.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    client
        .extract_delete_file_response(request_id, response)
        .unwrap();

    // Verify file doesn't exist
    let (request_msg, request_id) = client.fs_read("/test.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let result = client.extract_read_response(request_id, response);
    assert!(result.is_err(), "File should not exist");
}

#[test]
fn test_fs_delete_dir() {
    let mut fs = LpFsMemory::new();
    fs.write_file_mut("/dir/file1.txt", b"content1").unwrap();
    fs.write_file_mut("/dir/nested/file2.txt", b"content2")
        .unwrap();

    let (mut server, mut client, mut client_transport, mut server_transport) =
        setup_server_and_client(fs);

    // Verify files exist
    let (request_msg, request_id) = client.fs_read("/dir/file1.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let _content = client.extract_read_response(request_id, response).unwrap();

    let (request_msg, request_id) = client.fs_read("/dir/nested/file2.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let _content = client.extract_read_response(request_id, response).unwrap();

    // Delete directory (recursive)
    let (request_msg, request_id) = client.fs_delete_dir("/dir".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    client
        .extract_delete_dir_response(request_id, response)
        .unwrap();

    // Verify files are gone
    let (request_msg, request_id) = client.fs_read("/dir/file1.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let result1 = client.extract_read_response(request_id, response);
    assert!(result1.is_err(), "File should not exist");

    let (request_msg, request_id) = client.fs_read("/dir/nested/file2.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let result2 = client.extract_read_response(request_id, response);
    assert!(result2.is_err(), "File should not exist");
}

#[test]
fn test_fs_list_dir_non_recursive() {
    let mut fs = LpFsMemory::new();
    fs.write_file_mut("/dir/file1.txt", b"content1").unwrap();
    fs.write_file_mut("/dir/file2.txt", b"content2").unwrap();
    fs.write_file_mut("/dir/nested/file3.txt", b"content3")
        .unwrap();

    let (mut server, mut client, mut client_transport, mut server_transport) =
        setup_server_and_client(fs);

    let (request_msg, request_id) = client.fs_list_dir("/dir".to_string(), false);
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let entries = client
        .extract_list_dir_response(request_id, response)
        .unwrap();

    // Should contain immediate children only
    assert!(entries.contains(&"/dir/file1.txt".to_string()));
    assert!(entries.contains(&"/dir/file2.txt".to_string()));
    assert!(entries.contains(&"/dir/nested".to_string()));
    // Should NOT contain nested file
    assert!(!entries.contains(&"/dir/nested/file3.txt".to_string()));
}

#[test]
fn test_fs_list_dir_recursive() {
    let mut fs = LpFsMemory::new();
    fs.write_file_mut("/dir/file1.txt", b"content1").unwrap();
    fs.write_file_mut("/dir/nested/file2.txt", b"content2")
        .unwrap();
    fs.write_file_mut("/dir/nested/deep/file3.txt", b"content3")
        .unwrap();

    let (mut server, mut client, mut client_transport, mut server_transport) =
        setup_server_and_client(fs);

    let (request_msg, request_id) = client.fs_list_dir("/dir".to_string(), true);
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let entries = client
        .extract_list_dir_response(request_id, response)
        .unwrap();

    // Should contain all files recursively
    assert!(entries.contains(&"/dir/file1.txt".to_string()));
    assert!(entries.contains(&"/dir/nested/file2.txt".to_string()));
    assert!(entries.contains(&"/dir/nested/deep/file3.txt".to_string()));
}

#[test]
fn test_fs_read_not_found() {
    let fs = LpFsMemory::new();
    let (mut server, mut client, mut client_transport, mut server_transport) =
        setup_server_and_client(fs);

    let (request_msg, request_id) = client.fs_read("/nonexistent.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let result = client.extract_read_response(request_id, response);

    assert!(result.is_err(), "Reading non-existent file should fail");
    match result {
        Err(ClientError::Protocol { message }) => {
            assert!(message.contains("not found") || message.contains("NotFound"));
        }
        _ => panic!("Expected Protocol error"),
    }
}

#[test]
fn test_fs_delete_not_found() {
    let fs = LpFsMemory::new();
    let (mut server, mut client, mut client_transport, mut server_transport) =
        setup_server_and_client(fs);

    let (request_msg, request_id) = client.fs_delete_file("/nonexistent.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let result = client.extract_delete_file_response(request_id, response);

    assert!(result.is_err(), "Deleting non-existent file should fail");
}

#[test]
fn test_fs_delete_root() {
    let fs = LpFsMemory::new();
    let (mut server, mut client, mut client_transport, mut server_transport) =
        setup_server_and_client(fs);

    // Attempting to delete root should fail
    let (request_msg, request_id) = client.fs_delete_dir("/".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();
    let response = client.get_response(request_id).unwrap();
    let result = client.extract_delete_dir_response(request_id, response);

    assert!(result.is_err(), "Deleting root directory should fail");
    match result {
        Err(ClientError::Protocol { message }) => {
            assert!(
                message.contains("root") || message.contains("Cannot delete"),
                "Error message should mention root: {}",
                message
            );
        }
        _ => panic!("Expected Protocol error"),
    }
}

#[test]
fn test_multiple_requests() {
    let mut fs = LpFsMemory::new();
    fs.write_file_mut("/file1.txt", b"content1").unwrap();
    fs.write_file_mut("/file2.txt", b"content2").unwrap();
    fs.write_file_mut("/file3.txt", b"content3").unwrap();

    let (mut server, mut client, mut client_transport, mut server_transport) =
        setup_server_and_client(fs);

    // Send multiple read requests
    let (request_msg, request_id1) = client.fs_read("/file1.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();

    let (request_msg, request_id2) = client.fs_read("/file2.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();

    let (request_msg, request_id3) = client.fs_read("/file3.txt".to_string());
    send_client_message(&mut client_transport, request_msg).unwrap();

    // Process all requests
    process_messages(
        &mut client,
        &mut server,
        &mut client_transport,
        &mut server_transport,
    )
    .unwrap();

    // Verify all responses match
    let response1 = client.get_response(request_id1).unwrap();
    let content1 = client
        .extract_read_response(request_id1, response1)
        .unwrap();
    assert_eq!(content1, b"content1");

    let response2 = client.get_response(request_id2).unwrap();
    let content2 = client
        .extract_read_response(request_id2, response2)
        .unwrap();
    assert_eq!(content2, b"content2");

    let response3 = client.get_response(request_id3).unwrap();
    let content3 = client
        .extract_read_response(request_id3, response3)
        .unwrap();
    assert_eq!(content3, b"content3");
}
