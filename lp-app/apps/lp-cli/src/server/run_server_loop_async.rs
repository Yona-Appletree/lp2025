use lp_model::{Message, TransportError};
use lp_server::LpServer;
use lp_shared::transport::ServerTransport;
use std::time::Duration;

/// Run the server main loop asynchronously
///
/// Processes incoming messages from clients and routes responses back.
/// This is the async version that works with tokio runtime.
///
/// # Arguments
///
/// * `server` - The LpServer instance
/// * `transport` - The server transport (handles connections)
///
/// # Returns
///
/// * `Ok(())` if the loop completes successfully
/// * `Err` if there's an unrecoverable error
pub async fn run_server_loop_async<T: ServerTransport>(
    mut server: LpServer,
    mut transport: T,
) -> anyhow::Result<()> {
    // Main server loop
    loop {
        // Collect incoming messages from all connections
        let mut incoming_messages = Vec::new();

        // Poll transport for messages (non-blocking)
        loop {
            match transport.receive() {
                Ok(Some(client_msg)) => {
                    // Wrap in Message envelope
                    incoming_messages.push(Message::Client(client_msg));
                }
                Ok(None) => {
                    // No more messages available
                    break;
                }
                Err(e) => {
                    // Connection lost is expected when client disconnects - exit gracefully
                    if matches!(e, TransportError::ConnectionLost) {
                        return Ok(());
                    }
                    // Other transport errors - log and continue
                    eprintln!("Transport error: {}", e);
                    break;
                }
            }
        }

        // Process messages if any
        if !incoming_messages.is_empty() {
            match server.tick(16, incoming_messages) {
                Ok(responses) => {
                    // Send responses back via transport
                    for response in responses {
                        if let Message::Server(server_msg) = response {
                            if let Err(e) = transport.send(server_msg) {
                                eprintln!("Failed to send response: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Server error: {}", e);
                    // Continue running despite errors
                }
            }
        }

        // Async sleep to avoid busy-waiting
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
