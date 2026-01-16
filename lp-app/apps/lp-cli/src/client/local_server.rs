//! Local server transport
//!
//! Encapsulates an in-memory server running on a separate thread and provides
//! a client transport interface for communicating with it.

use anyhow::Result;
use lp_shared::transport::ClientTransport;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use crate::client::local::create_local_transport_pair;
use crate::server::{create_server, run_server_loop_async};

/// Local server transport that manages an in-memory server thread
///
/// This struct encapsulates the lifecycle of a server running on a separate thread.
/// It provides access to a client transport for communicating with the server.
pub struct LocalServerTransport {
    /// Handle to the server thread (None after close())
    server_handle: Option<JoinHandle<()>>,
    /// Client transport for communicating with the server
    client_transport: Box<dyn ClientTransport + Send>,
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
            client_transport: Box::new(client_transport),
            closed,
        })
    }

    /// Get a reference to the client transport
    ///
    /// Returns a reference to the client transport that can be used to communicate
    /// with the server running on the separate thread.
    pub fn client_transport(&self) -> &dyn ClientTransport {
        &*self.client_transport
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
        let _ = self.client_transport.close();

        // Wait for server thread to finish
        if let Some(handle) = self.server_handle.take() {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("Server thread panicked"))?;
        }

        Ok(())
    }
}

impl Drop for LocalServerTransport {
    fn drop(&mut self) {
        // If not already closed, try to close (best-effort)
        if !self.closed.load(Ordering::Relaxed) {
            // Close the transport (signals server to shut down)
            let _ = self.client_transport.close();
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

    #[test]
    fn test_local_server_transport_creation() {
        let mut transport = LocalServerTransport::new().unwrap();
        // Verify it was created
        let _client = transport.client_transport();
        // Cleanup
        transport.close().unwrap();
    }

    #[test]
    fn test_client_transport_works() {
        let mut transport = LocalServerTransport::new().unwrap();

        // Get reference to client transport
        // Since client_transport() returns &dyn ClientTransport, we can't get &mut
        // This test verifies the transport can be accessed
        let _client = transport.client_transport();

        // Close the transport
        transport.close().unwrap();
    }

    #[test]
    fn test_close_stops_server() {
        let mut transport = LocalServerTransport::new().unwrap();
        // Close should wait for server thread
        transport.close().unwrap();
        // If we get here, close succeeded
    }
}
