//! Dev command handler
//!
//! Orchestrates the dev command execution.

use anyhow::Result;
use std::path::PathBuf;

use super::args::DevArgs;
use super::push::{load_project, push_project, validate_local_project};
use crate::messages;
use crate::server::{create_server, run_server_loop_async};
use crate::transport::HostSpecifier;
use crate::transport::WebSocketClientTransport;
use crate::transport::local::create_local_transport_pair;
use lp_client::LpClient;
use lp_shared::fs::LpFsStd;
use lp_shared::transport::ClientTransport;
use tokio::runtime::Runtime;
use tokio::task::LocalSet;

/// Handle the dev command
///
/// Validates local project, connects to server, and pushes project files.
/// Supports both in-memory server (when host is None) and WebSocket server.
pub fn handle_dev(args: DevArgs) -> Result<()> {
    // Determine project directory (default to current directory)
    let project_dir = args
        .dir
        .as_ref()
        .map(|d| d.clone())
        .unwrap_or_else(|| PathBuf::from("."));

    // Validate local project
    let (project_uid, project_name) = validate_local_project(&project_dir)?;

    // Parse host specifier
    let host_spec = HostSpecifier::parse_optional(args.host.as_deref())
        .map_err(|e| anyhow::anyhow!("Invalid host specifier: {}", e))?;

    // Handle based on host specifier
    match host_spec {
        HostSpecifier::Local => handle_dev_local(args, project_dir, project_uid, project_name),
        HostSpecifier::WebSocket { url } => {
            handle_dev_websocket(args, project_dir, project_uid, project_name, &url)
        }
        HostSpecifier::Serial { .. } => {
            anyhow::bail!("Serial transport not yet implemented");
        }
    }
}

/// Handle dev command with local in-memory server
fn handle_dev_local(
    args: DevArgs,
    project_dir: PathBuf,
    project_uid: String,
    project_name: String,
) -> Result<()> {
    // Create tokio runtime
    let runtime = Runtime::new()?;

    // Run async code
    runtime.block_on(async {
        // Create LocalSet for spawn_local (needed because LpServer is not Send)
        let local_set = LocalSet::new();

        local_set
            .run_until(async {
                // Create in-memory server
                let (server, _base_fs) = create_server(None, true, None)
                    .map_err(|e| anyhow::anyhow!("Failed to create server: {}", e))?;

                // Create local transport pair
                let (mut client_transport, server_transport) = create_local_transport_pair();

                // Spawn server task (using spawn_local because LpServer is not Send)
                tokio::task::spawn_local(run_server_loop_async(server, server_transport));

                // Give server a moment to start
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;

                // Create client
                let mut client = LpClient::new();

                // Create local filesystem view of project directory
                let local_fs = LpFsStd::new(project_dir.clone());

                // Push project if requested
                if args.push {
                    println!(
                        "Pushing project '{}' (uid: {}) to server...",
                        project_name, project_uid
                    );

                    push_project(&mut client, &mut client_transport, &local_fs, &project_uid)
                        .map_err(|e| {
                            anyhow::anyhow!("Failed to push project '{}': {}", project_name, e)
                        })?;

                    println!("Project files pushed successfully");
                }

                // Load project on server
                println!("Loading project on server...");
                let handle = load_project(&mut client, &mut client_transport, &project_uid)
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to load project '{}': {}", project_name, e)
                    })?;

                messages::print_success(
                    &format!(
                        "Project '{}' (uid: {}) loaded successfully",
                        project_name, project_uid
                    ),
                    &[
                        &format!("Project handle: {:?}", handle),
                        "Project is now running on the server",
                        "Press Ctrl+C to stop",
                    ],
                );

                // TODO: Enter client loop (will be implemented in Phase 6)
                // For now, just wait a bit to show it's working
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                Ok(())
            })
            .await
    })
}

/// Handle dev command with WebSocket server
fn handle_dev_websocket(
    args: DevArgs,
    project_dir: PathBuf,
    project_uid: String,
    project_name: String,
    url: &str,
) -> Result<()> {
    // Create WebSocket transport
    let mut transport: Box<dyn ClientTransport> = Box::new(
        WebSocketClientTransport::new(url)
            .map_err(|e| anyhow::anyhow!("Failed to connect to {}: {}", url, e))?,
    );

    // Create client
    let mut client = LpClient::new();

    // Create local filesystem view of project directory
    let local_fs = LpFsStd::new(project_dir.clone());

    // Push project if requested
    if args.push {
        println!(
            "Pushing project '{}' (uid: {}) to server...",
            project_name, project_uid
        );

        push_project(&mut client, transport.as_mut(), &local_fs, &project_uid)
            .map_err(|e| anyhow::anyhow!("Failed to push project '{}': {}", project_name, e))?;

        println!("Project files pushed successfully");
    }

    // Load project on server
    println!("Loading project on server...");
    let handle = load_project(&mut client, transport.as_mut(), &project_uid)
        .map_err(|e| anyhow::anyhow!("Failed to load project '{}': {}", project_name, e))?;

    messages::print_success(
        &format!(
            "Project '{}' (uid: {}) loaded successfully",
            project_name, project_uid
        ),
        &[
            &format!("Project handle: {:?}", handle),
            "Project is now running on the server",
            "Press Ctrl+C to stop",
        ],
    );

    // TODO: Enter client loop (will be implemented in Phase 6)
    // For now, just return
    Ok(())
}
