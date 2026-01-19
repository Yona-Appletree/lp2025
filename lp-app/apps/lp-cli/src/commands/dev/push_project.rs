//! Push project files to server
//!
//! Provides async function to push local project files to the server.

use anyhow::{Context, Result};
use lp_shared::fs::LpFs;

use crate::client::LpClient;

/// Push project files from local filesystem to server
///
/// Recursively reads all files from the local project directory and writes
/// them to the server filesystem.
///
/// # Arguments
///
/// * `client` - Async client for communicating with server
/// * `local_fs` - Local filesystem (project root)
/// * `project_uid` - Project UID (used for server-side project path)
///
/// # Returns
///
/// * `Ok(())` if all files were pushed successfully
/// * `Err` if any file operation failed
pub async fn push_project_async(
    client: &LpClient,
    local_fs: &dyn LpFs,
    project_uid: &str,
) -> Result<()> {
    // List all files recursively in the project directory
    let files = local_fs.list_dir("/", true).map_err(|e| {
        anyhow::anyhow!("Failed to list project files: {}", e)
    })?;

    // Push each file to the server
    for file_path in files {
        // Read file from local filesystem
        let data = local_fs.read_file(&file_path).map_err(|e| {
            anyhow::anyhow!("Failed to read file {}: {}", file_path, e)
        })?;

        // Build server path: projects/{project_uid}/{file_path}
        // Remove leading '/' from file_path for server path
        let server_path = if file_path.starts_with('/') {
            format!("projects/{}/{}", project_uid, &file_path[1..])
        } else {
            format!("projects/{}/{}", project_uid, file_path)
        };

        // Write file to server
        client
            .fs_write(&server_path, data)
            .await
            .with_context(|| format!("Failed to write file to server: {}", server_path))?;
    }

    Ok(())
}
