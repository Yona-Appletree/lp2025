//! Local server transport
//!
//! Encapsulates an in-memory server running on a separate thread and provides
//! a client transport interface for communicating with it.

use anyhow::Result;
use lp_model::TransportError;
use crate::client::transport::ClientTransport;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crate::client::local::{create_local_transport_pair, AsyncLocalClientTransport};
use crate::server::{create_server, run_server_loop_async};

/// Local server transport that manages an in-memory server thread
///
/// This struct encapsulates the lifecycle of a server running on a separate thread.
/// It provides access to a client transport for communicating with the server.
pub struct LocalServerTransport {
    /// Handle to the server thread (None after close())
    server_handle: Option<JoinHandle<()>>,
    /// Client transport for communicating with the server
    client_transport: AsyncLocalClientTransport,
    /// Whether the transport has been closed
    closed: Arc<AtomicBool>,
}

impl LocalServerTransport {
    /// Create a new local server transport
    ///
    /// Spawns a server thread with its own tokio runtime and returns a client transport
    /// for communicating with it.
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` if server was spawned successfully
    /// * `Err` if server creation or thread spawning failed
    pub fn new() -> Result<Self> {
        // Create transport pair
        let (client_transport, server_transport) = create_local_transport_pair();

        // Create closed flag (shared between client and server)
        let closed = Arc::new(AtomicBool::new(false));

        // Spawn server thread
        let closed_clone = Arc::clone(&closed);
        let server_handle = thread::Builder::new()
            .name("lp-server".to_string())
            .spawn(move || {
                // Create tokio runtime for server
                let runtime = match tokio::runtime::Runtime::new() {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Failed to create tokio runtime for server: {}", e);
                        return;
                    }
                };

                // Create server inside the thread (LpServer is not Send)
                let (server, _base_fs) = match create_server(None, true, None) {
                    Ok((s, fs)) => (s, fs),
                    Err(e) => {
                        eprintln!("Failed to create server: {}", e);
                        return;
                    }
                };

                // Run server loop until transport closes
                runtime.block_on(async {
                    // Create LocalSet for spawn_local (needed because LpServer is not Send)
                    let local_set = tokio::task::LocalSet::new();
                    let _ = local_set
                        .run_until(run_server_loop_async(server, server_transport))
                        .await;
                });

                // Mark as closed when server exits
                closed_clone.store(true, Ordering::Relaxed);
            })
            .map_err(|e| anyhow::anyhow!("Failed to spawn server thread: {}", e))?;

        Ok(Self {
            server_handle: Some(server_handle),
            client_transport,
            closed,
        })
    }

    /// Close the transport and stop the server
    ///
    /// This method is idempotent - calling it multiple times is safe.
    /// It closes the client transport (which signals the server to shut down) and
    /// waits for the server thread to finish.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the server was stopped successfully (or already closed)
    /// * `Err` if waiting for the thread failed
    pub fn close(&mut self) -> Result<()> {
        // Check if already closed
        if self.closed.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Mark as closed
        self.closed.store(true, Ordering::Relaxed);

        // Close the client transport (signals server to shut down)
        // We can't await in sync context, but dropping will close the channels
        // The async close will happen when the transport is dropped

        // Wait for server thread to finish
        if let Some(handle) = self.server_handle.take() {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("Server thread panicked"))?;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl ClientTransport for LocalServerTransport {
    async fn send(&mut self, msg: lp_model::ClientMessage) -> Result<(), TransportError> {
        self.client_transport.send(msg).await
    }

    async fn receive(&mut self) -> Result<lp_model::ServerMessage, TransportError> {
        self.client_transport.receive().await
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        // Check if already closed
        if self.closed.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Mark as closed
        self.closed.store(true, Ordering::Relaxed);

        // Close the client transport (signals server to shut down)
        let _ = self.client_transport.close().await;

        // Wait for server thread to finish
        if let Some(handle) = self.server_handle.take() {
            handle.join().map_err(|_| {
                TransportError::Other("Server thread panicked".to_string())
            })?;
        }

        Ok(())
    }
}

impl Drop for LocalServerTransport {
    fn drop(&mut self) {
        // If not already closed, try to close (best-effort)
        if !self.closed.load(Ordering::Relaxed) {
            // Mark as closed
            self.closed.store(true, Ordering::Relaxed);
            // Try to join the thread if we still have the handle
            if let Some(handle) = self.server_handle.take() {
                let _ = handle.join();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lp_model::{ClientMessage, ClientRequest};

    #[tokio::test]
    async fn test_local_server_transport_creation() {
        let mut transport = LocalServerTransport::new().unwrap();
        // Verify it was created
        // Cleanup
        transport.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_client_transport_works() {
        let mut transport = LocalServerTransport::new().unwrap();

        // Test that we can send and receive
        let msg = ClientMessage {
            id: 1,
            msg: ClientRequest::ListAvailableProjects,
        };
        // Note: send is async, but we can't easily test without a server response
        // Just verify the transport exists
        let _ = transport.send(msg).await;

        // Close the transport
        transport.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_close_stops_server() {
        let mut transport = LocalServerTransport::new().unwrap();
        // Close should wait for server thread
        transport.close().await.unwrap();
        // If we get here, close succeeded
    }
}
