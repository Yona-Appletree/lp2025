//! File sync utilities
//!
//! Provides functions for syncing individual file changes to the server.

use anyhow::{Context, Result};
use lp_shared::fs::{fs_event::ChangeType, fs_event::FsChange, LpFs};
use std::sync::Arc;

use crate::client::LpClient;

/// Sync a file change to the server
///
/// Reads the file from local filesystem (if needed) and writes/deletes it on the server.
///
/// # Arguments
///
/// * `client` - Async client for communicating with server
/// * `change` - File change event to sync
/// * `project_uid` - Project UID for server-side path
/// * `local_fs` - Local filesystem for reading files (wrapped in Arc for Send + Sync)
///
/// # Returns
///
/// * `Ok(())` if the change was synced successfully
/// * `Err` if syncing failed
pub async fn sync_file_change(
    client: &Arc<LpClient>,
    change: &FsChange,
    project_uid: &str,
    _project_dir: &std::path::Path,
    local_fs: &Arc<dyn LpFs + Send + Sync>,
) -> Result<()> {
    // Build server path: projects/{project_uid}/{file_path}
    // Remove leading '/' from change.path for server path
    let server_path = if change.path.starts_with('/') {
        format!("projects/{}/{}", project_uid, &change.path[1..])
    } else {
        format!("projects/{}/{}", project_uid, change.path)
    };

    match change.change_type {
        ChangeType::Create | ChangeType::Modify => {
            // Read file from local filesystem
            let data = local_fs.read_file(&change.path).map_err(|e| {
                anyhow::anyhow!("Failed to read file {}: {}", change.path, e)
            })?;

            // Write file to server
            client
                .fs_write(&server_path, data)
                .await
                .with_context(|| format!("Failed to write file to server: {}", server_path))?;
        }
        ChangeType::Delete => {
            // Delete file from server
            client
                .fs_delete_file(&server_path)
                .await
                .with_context(|| format!("Failed to delete file on server: {}", server_path))?;
        }
    }

    Ok(())
}
