//! Async client wrapper
//!
//! TODO: Will be recreated in client/ directory in phase 5
//! This is a temporary stub to allow compilation.

use anyhow::Error;
use lp_model::project::api::SerializableProjectResponse;

/// Async client wrapper around LpClient
///
/// TODO: Will be recreated in client/async_client.rs in phase 5
#[allow(dead_code)]
pub struct AsyncLpClient {
    // Stub - will be properly implemented in phase 5
}

impl AsyncLpClient {
    /// Create a new async client
    ///
    /// TODO: Will be properly implemented in phase 5
    #[allow(dead_code)]
    pub fn new(
        _transport: std::sync::Arc<
            tokio::sync::Mutex<Box<dyn lp_shared::transport::ClientTransport + Send>>,
        >,
    ) -> Self {
        Self {}
    }

    /// Project sync internal method
    ///
    /// TODO: Will be properly implemented in phase 5
    #[allow(dead_code)]
    pub async fn project_sync_internal(
        &mut self,
        _handle: lp_model::project::handle::ProjectHandle,
        _since_frame: lp_model::project::FrameId,
        _detail_specifier: lp_model::project::api::ApiNodeSpecifier,
    ) -> Result<SerializableProjectResponse, Error> {
        todo!("Will be implemented in phase 5")
    }
}

/// Convert SerializableProjectResponse to project response
///
/// TODO: Will be properly implemented in phase 5
#[allow(dead_code)]
pub fn serializable_response_to_project_response(
    _response: SerializableProjectResponse,
) -> Result<lp_model::project::api::ProjectResponse, Error> {
    todo!("Will be implemented in phase 5")
}
