//! Async wrapper for LpClient
//!
//! Provides async/await interface for LpClient operations with proper timeout handling.
//! The underlying LpClient remains synchronous for WebAssembly compatibility.

use anyhow::{Context, Result};
use lp_client::LpClient;
use lp_engine_client::project::ClientProjectView;
use lp_model::{
    Message,
    project::{api::ProjectResponse, handle::ProjectHandle},
    server::ServerResponse,
};
use lp_shared::transport::ClientTransport;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;

/// Default timeout for request/response operations (5 seconds)
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Async wrapper for LpClient
///
/// Provides async/await interface for LpClient operations while keeping
/// the underlying LpClient synchronous for WebAssembly compatibility.
/// Handles message polling, timeout, and proper async/await semantics.
pub struct AsyncLpClient {
    /// The synchronous LpClient instance
    client: LpClient,
    /// The transport for sending/receiving messages (shared via Arc<Mutex<>>)
    transport: Arc<Mutex<Box<dyn ClientTransport + Send>>>,
}

impl AsyncLpClient {
    /// Create a new AsyncLpClient
    ///
    /// # Arguments
    ///
    /// * `transport` - The client transport (shared via Arc<Mutex<>>, must be Send for cross-thread use)
    pub fn new(transport: Arc<Mutex<Box<dyn ClientTransport + Send>>>) -> Self {
        Self {
            client: LpClient::new(),
            transport,
        }
    }

    /// Wait for a response to a specific request
    ///
    /// Polls the transport for messages, processes them via LpClient::tick(),
    /// and waits for the specified request ID to receive a response.
    /// Uses timeout to prevent indefinite blocking.
    ///
    /// # Arguments
    ///
    /// * `request_id` - The request ID to wait for
    ///
    /// # Returns
    ///
    /// * `Ok(ServerResponse)` - The response when received
    /// * `Err` - If timeout occurs or transport error
    async fn wait_for_response(&mut self, request_id: u64) -> Result<ServerResponse> {
        timeout(DEFAULT_TIMEOUT, async {
            loop {
                // Process all available messages
                let mut incoming_messages = Vec::new();
                loop {
                    let mut transport = self.transport.lock().await;
                    match transport.receive() {
                        Ok(Some(server_msg)) => {
                            incoming_messages.push(Message::Server(server_msg));
                        }
                        Ok(None) => {
                            // No more messages available
                            break;
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("Transport error: {}", e));
                        }
                    }
                }

                // Process messages if any
                if !incoming_messages.is_empty() {
                    self.client
                        .tick(incoming_messages)
                        .map_err(|e| anyhow::anyhow!("Client error: {}", e))?;
                }

                // Check if we got the response
                if let Some(response) = self.client.get_response(request_id) {
                    return Ok(response);
                }

                // Yield to allow other tasks to run
                tokio::task::yield_now().await;
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .map_err(|_| anyhow::anyhow!("Timeout waiting for response to request {}", request_id))?
    }

    /// Read a file from the server filesystem
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file (relative to server root)
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - File contents
    /// * `Err` - If read failed or timeout occurred
    #[allow(dead_code)]
    pub async fn fs_read(&mut self, path: String) -> Result<Vec<u8>> {
        // Create read request
        let (read_msg, read_id) = self.client.fs_read(path.clone());

        // Extract ClientMessage and send
        let client_msg = match read_msg {
            Message::Client(msg) => msg,
            Message::Server(_) => {
                return Err(anyhow::anyhow!("Expected Client message"));
            }
        };

        self.transport
            .lock()
            .await
            .send(client_msg)
            .map_err(|e| anyhow::anyhow!("Failed to send read request for {}: {}", path, e))?;

        // Wait for response
        let response = self
            .wait_for_response(read_id)
            .await
            .with_context(|| format!("Failed to read file {}", path))?;

        // Extract response
        self.client
            .extract_read_response(read_id, response)
            .map_err(|e| anyhow::anyhow!("Server error reading {}: {}", path, e))
    }

    /// Write a file to the server filesystem
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file (relative to server root)
    /// * `data` - File contents to write
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If write succeeded
    /// * `Err` - If write failed or timeout occurred
    pub async fn fs_write(&mut self, path: String, data: Vec<u8>) -> Result<()> {
        // Create write request
        let (write_msg, write_id) = self.client.fs_write(path.clone(), data);

        // Extract ClientMessage and send
        let client_msg = match write_msg {
            Message::Client(msg) => msg,
            Message::Server(_) => {
                return Err(anyhow::anyhow!("Expected Client message"));
            }
        };

        self.transport
            .lock()
            .await
            .send(client_msg)
            .map_err(|e| anyhow::anyhow!("Failed to send write request for {}: {}", path, e))?;

        // Wait for response
        let response = self
            .wait_for_response(write_id)
            .await
            .with_context(|| format!("Failed to write file {}", path))?;

        // Extract response
        self.client
            .extract_write_response(write_id, response)
            .map_err(|e| anyhow::anyhow!("Server error writing {}: {}", path, e))
    }

    /// Load a project on the server
    ///
    /// # Arguments
    ///
    /// * `path` - Project path (project UID or path to project directory)
    ///
    /// # Returns
    ///
    /// * `Ok(ProjectHandle)` - Project handle if load succeeded
    /// * `Err` - If load failed or timeout occurred
    pub async fn project_load(&mut self, path: String) -> Result<ProjectHandle> {
        // Create load request
        let (load_msg, load_id) = self.client.project_load(path.clone());

        // Extract ClientMessage and send
        let client_msg = match load_msg {
            Message::Client(msg) => msg,
            Message::Server(_) => {
                return Err(anyhow::anyhow!("Expected Client message"));
            }
        };

        self.transport.lock().await.send(client_msg).map_err(|e| {
            anyhow::anyhow!("Failed to send load project request for {}: {}", path, e)
        })?;

        // Wait for response
        let response = self
            .wait_for_response(load_id)
            .await
            .with_context(|| format!("Failed to load project {}", path))?;

        // Extract response
        self.client
            .extract_load_project_response(load_id, response)
            .map_err(|e| anyhow::anyhow!("Server error loading project {}: {}", path, e))
    }

    /// Sync project view with server (internal, returns serializable response)
    ///
    /// Sends GetChanges request and returns SerializableProjectResponse.
    /// Returns when sync completes or timeout occurs.
    ///
    /// Note: This method does NOT hold a lock on `view` across await points.
    /// Caller should lock view, read needed data, unlock, call this method,
    /// then lock again to apply changes.
    ///
    /// # Arguments
    ///
    /// * `handle` - Project handle
    /// * `since_frame` - Frame ID to sync from
    /// * `detail_specifier` - Specifier for which nodes to get detail for
    ///
    /// # Returns
    ///
    /// * `Ok(SerializableProjectResponse)` - The serializable project response if sync succeeded
    /// * `Err` - If sync failed or timeout occurred
    pub async fn project_sync_internal(
        &mut self,
        handle: ProjectHandle,
        since_frame: lp_model::FrameId,
        detail_specifier: lp_model::project::api::ApiNodeSpecifier,
    ) -> Result<lp_model::project::api::SerializableProjectResponse> {
        // Create get changes request
        let (get_changes_msg, get_changes_id) =
            self.client
                .project_get_changes(handle, since_frame, detail_specifier);

        // Extract ClientMessage and send
        let client_msg = match get_changes_msg {
            Message::Client(msg) => msg,
            Message::Server(_) => {
                return Err(anyhow::anyhow!("Expected Client message"));
            }
        };

        self.transport
            .lock()
            .await
            .send(client_msg)
            .map_err(|e| anyhow::anyhow!("Failed to send get changes request: {}", e))?;

        // Wait for response
        let response = self
            .wait_for_response(get_changes_id)
            .await
            .with_context(|| "Failed to get project changes")?;

        // Extract response (already SerializableProjectResponse)
        let serializable_response = self
            .client
            .extract_get_changes_response(get_changes_id, response)
            .map_err(|e| anyhow::anyhow!("Server error getting changes: {}", e))?;

        Ok(serializable_response)
    }
}

/// Convert SerializableProjectResponse to ProjectResponse
///
/// This conversion is needed because ClientProjectView::apply_changes expects ProjectResponse,
/// but the client receives SerializableProjectResponse over the wire.
pub(crate) fn serializable_response_to_project_response(
    serializable: lp_model::project::api::SerializableProjectResponse,
) -> Result<ProjectResponse, String> {
    match serializable {
        lp_model::project::api::SerializableProjectResponse::GetChanges {
            current_frame,
            node_handles,
            node_changes,
            node_details,
        } => {
            // Convert SerializableNodeDetail to NodeDetail
            use std::collections::BTreeMap;
            let mut details_map = BTreeMap::new();
            for (handle, serializable_detail) in node_details {
                let detail = match serializable_detail {
                    lp_model::project::api::SerializableNodeDetail::Texture {
                        path,
                        config,
                        state,
                        status,
                    } => lp_model::project::api::NodeDetail {
                        path,
                        config: Box::new(config),
                        state,
                        status,
                    },
                    lp_model::project::api::SerializableNodeDetail::Shader {
                        path,
                        config,
                        state,
                        status,
                    } => lp_model::project::api::NodeDetail {
                        path,
                        config: Box::new(config),
                        state,
                        status,
                    },
                    lp_model::project::api::SerializableNodeDetail::Output {
                        path,
                        config,
                        state,
                        status,
                    } => lp_model::project::api::NodeDetail {
                        path,
                        config: Box::new(config),
                        state,
                        status,
                    },
                    lp_model::project::api::SerializableNodeDetail::Fixture {
                        path,
                        config,
                        state,
                        status,
                    } => lp_model::project::api::NodeDetail {
                        path,
                        config: Box::new(config),
                        state,
                        status,
                    },
                };
                details_map.insert(handle, detail);
            }

            Ok(ProjectResponse::GetChanges {
                current_frame,
                node_handles,
                node_changes,
                node_details: details_map,
            })
        }
    }
}
