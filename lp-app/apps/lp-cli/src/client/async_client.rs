//! Async client wrapper around LpClient
//!
//! Provides async methods for filesystem and project operations by wrapping
//! `LpClient` (for request ID generation and message creation) and `AsyncClientTransport`
//! (for async request/response correlation).

use anyhow::{Error, Result};
use lp_client::LpClient;
use lp_model::{
    project::{
        api::{ApiNodeSpecifier, SerializableProjectResponse},
        handle::ProjectHandle,
        FrameId,
    },
    server::{AvailableProject, LoadedProject},
    ClientMessage, Message, ServerMessage, ServerResponse,
};
use std::sync::{Arc, Mutex};

use crate::client::async_transport::AsyncClientTransport;

/// Async client wrapper around LpClient
///
/// Provides typed async methods for filesystem and project operations.
/// Uses `LpClient` for request ID generation and message creation, then
/// uses `AsyncClientTransport` for async request/response correlation.
///
/// # Example
///
/// ```no_run
/// use lp_cli::client::{client_connect, async_client::AsyncLpClient, async_transport::AsyncClientTransport, specifier::HostSpecifier};
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let transport = client_connect(HostSpecifier::Local)?;
/// let async_transport = Arc::new(AsyncClientTransport::new(transport));
/// let mut client = AsyncLpClient::new(async_transport);
///
/// let data = client.fs_read("/project.json").await?;
/// # Ok(())
/// # }
/// ```
pub struct AsyncLpClient {
    /// Async transport for sending requests and receiving responses
    transport: Arc<AsyncClientTransport>,
    /// LpClient for request ID generation and message creation
    client: Mutex<LpClient>,
}

impl AsyncLpClient {
    /// Create a new async client
    ///
    /// # Arguments
    ///
    /// * `transport` - The async client transport (wrapped in Arc for sharing)
    ///
    /// # Returns
    ///
    /// * `Self` - The async client
    pub fn new(transport: Arc<AsyncClientTransport>) -> Self {
        Self {
            transport,
            client: Mutex::new(LpClient::new()),
        }
    }

    /// Extract ClientMessage from Message::Client
    fn extract_client_message(message: Message) -> Result<ClientMessage> {
        match message {
            Message::Client(msg) => Ok(msg),
            Message::Server(_) => Err(Error::msg("Expected ClientMessage, got ServerMessage")),
        }
    }

    /// Extract ServerResponse from ServerMessage
    fn extract_server_response(message: ServerMessage) -> ServerResponse {
        message.msg
    }

    /// Read a file from the server filesystem
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file (relative to server root)
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` if the file was read successfully
    /// * `Err` if reading failed or transport error occurred
    pub async fn fs_read(&self, path: &str) -> Result<Vec<u8>> {
        // Create request using LpClient
        let (message, request_id) = {
            let mut client = self.client.lock().unwrap();
            client.fs_read(path.to_string())
        };

        // Extract ClientMessage
        let client_msg = Self::extract_client_message(message)?;

        // Send request and wait for response
        let server_msg = self
            .transport
            .send_request(client_msg)
            .await
            .map_err(|e| Error::msg(format!("Transport error: {}", e)))?;

        // Extract ServerResponse
        let response = Self::extract_server_response(server_msg);

        // Extract result using LpClient
        let mut client = self.client.lock().unwrap();
        client
            .extract_read_response(request_id, response)
            .map_err(|e| Error::msg(format!("Client error: {}", e)))
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
    /// * `Ok(())` if the file was written successfully
    /// * `Err` if writing failed or transport error occurred
    pub async fn fs_write(&self, path: &str, data: Vec<u8>) -> Result<()> {
        // Create request using LpClient
        let (message, request_id) = {
            let mut client = self.client.lock().unwrap();
            client.fs_write(path.to_string(), data)
        };

        // Extract ClientMessage
        let client_msg = Self::extract_client_message(message)?;

        // Send request and wait for response
        let server_msg = self
            .transport
            .send_request(client_msg)
            .await
            .map_err(|e| Error::msg(format!("Transport error: {}", e)))?;

        // Extract ServerResponse
        let response = Self::extract_server_response(server_msg);

        // Extract result using LpClient
        let mut client = self.client.lock().unwrap();
        client
            .extract_write_response(request_id, response)
            .map_err(|e| Error::msg(format!("Client error: {}", e)))
    }

    /// Delete a file from the server filesystem
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file (relative to server root)
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the file was deleted successfully
    /// * `Err` if deletion failed or transport error occurred
    pub async fn fs_delete_file(&self, path: &str) -> Result<()> {
        // Create request using LpClient
        let (message, request_id) = {
            let mut client = self.client.lock().unwrap();
            client.fs_delete_file(path.to_string())
        };

        // Extract ClientMessage
        let client_msg = Self::extract_client_message(message)?;

        // Send request and wait for response
        let server_msg = self
            .transport
            .send_request(client_msg)
            .await
            .map_err(|e| Error::msg(format!("Transport error: {}", e)))?;

        // Extract ServerResponse
        let response = Self::extract_server_response(server_msg);

        // Extract result using LpClient
        let mut client = self.client.lock().unwrap();
        client
            .extract_delete_file_response(request_id, response)
            .map_err(|e| Error::msg(format!("Client error: {}", e)))
    }

    /// List directory contents from the server filesystem
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the directory (relative to server root)
    /// * `recursive` - Whether to list recursively
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - List of file/directory paths
    /// * `Err` if listing failed or transport error occurred
    pub async fn fs_list_dir(&self, path: &str, recursive: bool) -> Result<Vec<String>> {
        // Create request using LpClient
        let (message, request_id) = {
            let mut client = self.client.lock().unwrap();
            client.fs_list_dir(path.to_string(), recursive)
        };

        // Extract ClientMessage
        let client_msg = Self::extract_client_message(message)?;

        // Send request and wait for response
        let server_msg = self
            .transport
            .send_request(client_msg)
            .await
            .map_err(|e| Error::msg(format!("Transport error: {}", e)))?;

        // Extract ServerResponse
        let response = Self::extract_server_response(server_msg);

        // Extract result using LpClient
        let mut client = self.client.lock().unwrap();
        client
            .extract_list_dir_response(request_id, response)
            .map_err(|e| Error::msg(format!("Client error: {}", e)))
    }

    /// Load a project on the server
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the project file (relative to server root)
    ///
    /// # Returns
    ///
    /// * `Ok(ProjectHandle)` if the project was loaded successfully
    /// * `Err` if loading failed or transport error occurred
    pub async fn project_load(&self, path: &str) -> Result<ProjectHandle> {
        // Create request using LpClient
        let (message, request_id) = {
            let mut client = self.client.lock().unwrap();
            client.project_load(path.to_string())
        };

        // Extract ClientMessage
        let client_msg = Self::extract_client_message(message)?;

        // Send request and wait for response
        let server_msg = self
            .transport
            .send_request(client_msg)
            .await
            .map_err(|e| Error::msg(format!("Transport error: {}", e)))?;

        // Extract ServerResponse
        let response = Self::extract_server_response(server_msg);

        // Extract result using LpClient
        let mut client = self.client.lock().unwrap();
        client
            .extract_load_project_response(request_id, response)
            .map_err(|e| Error::msg(format!("Client error: {}", e)))
    }

    /// Unload a project on the server
    ///
    /// # Arguments
    ///
    /// * `handle` - Project handle to unload
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the project was unloaded successfully
    /// * `Err` if unloading failed or transport error occurred
    pub async fn project_unload(&self, handle: ProjectHandle) -> Result<()> {
        // Create request using LpClient
        let (message, request_id) = {
            let mut client = self.client.lock().unwrap();
            client.project_unload(handle)
        };

        // Extract ClientMessage
        let client_msg = Self::extract_client_message(message)?;

        // Send request and wait for response
        let server_msg = self
            .transport
            .send_request(client_msg)
            .await
            .map_err(|e| Error::msg(format!("Transport error: {}", e)))?;

        // Extract ServerResponse
        let response = Self::extract_server_response(server_msg);

        // Extract result using LpClient
        let mut client = self.client.lock().unwrap();
        client
            .extract_unload_project_response(request_id, response)
            .map_err(|e| Error::msg(format!("Client error: {}", e)))
    }

    /// Get project changes since a specific frame
    ///
    /// # Arguments
    ///
    /// * `handle` - Project handle
    /// * `since_frame` - Frame ID to get changes since (None for all changes)
    /// * `detail_specifier` - Which nodes to include in the response
    ///
    /// # Returns
    ///
    /// * `Ok(SerializableProjectResponse)` if the request succeeded
    /// * `Err` if the request failed or transport error occurred
    pub async fn project_sync_internal(
        &self,
        handle: ProjectHandle,
        since_frame: Option<FrameId>,
        detail_specifier: ApiNodeSpecifier,
    ) -> Result<SerializableProjectResponse> {
        // Use FrameId::default() if since_frame is None (get all changes)
        let since_frame = since_frame.unwrap_or_default();

        // Create request using LpClient
        let (message, request_id) = {
            let mut client = self.client.lock().unwrap();
            client.project_get_changes(handle, since_frame, detail_specifier)
        };

        // Extract ClientMessage
        let client_msg = Self::extract_client_message(message)?;

        // Send request and wait for response
        let server_msg = self
            .transport
            .send_request(client_msg)
            .await
            .map_err(|e| Error::msg(format!("Transport error: {}", e)))?;

        // Extract ServerResponse
        let response = Self::extract_server_response(server_msg);

        // Extract result using LpClient
        let mut client = self.client.lock().unwrap();
        client
            .extract_get_changes_response(request_id, response)
            .map_err(|e| Error::msg(format!("Client error: {}", e)))
    }

    /// List available projects on the server filesystem
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<AvailableProject>)` - List of available projects
    /// * `Err` if listing failed or transport error occurred
    pub async fn project_list_available(&self) -> Result<Vec<AvailableProject>> {
        // Create request using LpClient
        let (message, request_id) = {
            let mut client = self.client.lock().unwrap();
            client.project_list_available()
        };

        // Extract ClientMessage
        let client_msg = Self::extract_client_message(message)?;

        // Send request and wait for response
        let server_msg = self
            .transport
            .send_request(client_msg)
            .await
            .map_err(|e| Error::msg(format!("Transport error: {}", e)))?;

        // Extract ServerResponse
        let response = Self::extract_server_response(server_msg);

        // Extract result using LpClient
        let mut client = self.client.lock().unwrap();
        client
            .extract_list_available_projects_response(request_id, response)
            .map_err(|e| Error::msg(format!("Client error: {}", e)))
    }

    /// List loaded projects on the server
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<LoadedProject>)` - List of loaded projects
    /// * `Err` if listing failed or transport error occurred
    pub async fn project_list_loaded(&self) -> Result<Vec<LoadedProject>> {
        // Create request using LpClient
        let (message, request_id) = {
            let mut client = self.client.lock().unwrap();
            client.project_list_loaded()
        };

        // Extract ClientMessage
        let client_msg = Self::extract_client_message(message)?;

        // Send request and wait for response
        let server_msg = self
            .transport
            .send_request(client_msg)
            .await
            .map_err(|e| Error::msg(format!("Transport error: {}", e)))?;

        // Extract ServerResponse
        let response = Self::extract_server_response(server_msg);

        // Extract result using LpClient
        let mut client = self.client.lock().unwrap();
        client
            .extract_list_loaded_projects_response(request_id, response)
            .map_err(|e| Error::msg(format!("Client error: {}", e)))
    }
}

/// Convert SerializableProjectResponse to project response
///
/// This is a helper function for converting the serializable response
/// to the engine client's ProjectResponse type (which is actually `lp_model::project::api::ProjectResponse`).
pub fn serializable_response_to_project_response(
    response: SerializableProjectResponse,
) -> Result<lp_model::project::api::ProjectResponse> {
    // TODO: Implement conversion from SerializableProjectResponse to ProjectResponse
    // This requires converting SerializableNodeDetail back to NodeDetail with Box<dyn NodeConfig>
    // For now, this is a placeholder
    Err(Error::msg("serializable_response_to_project_response not yet implemented - needs conversion from SerializableNodeDetail to NodeDetail"))
}
