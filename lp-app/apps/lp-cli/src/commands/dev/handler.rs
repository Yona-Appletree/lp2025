//! Dev command handler
//!
//! Orchestrates the dev command: connects to server, syncs project, and runs file watching and UI.

use anyhow::{Context, Result};
use lp_model::project::ProjectConfig;
use lp_shared::fs::{LpFs, LpFsStd};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;

use crate::client::{
    LpClient, client_connect, specifier::HostSpecifier,
};
use crate::commands::dev::{fs_loop, pull_project_async, push_project_async};
use crate::debug_ui::DebugUiState;

use super::args::DevArgs;

/// Validate that a local project exists and extract project info
///
/// # Arguments
///
/// * `project_dir` - Path to the project directory
///
/// # Returns
///
/// * `Ok((project_uid, project_name))` if project is valid
/// * `Err` if project.json is missing or invalid
fn validate_local_project(project_dir: &PathBuf) -> Result<(String, String)> {
    // Create filesystem for reading project.json
    let fs = LpFsStd::new(project_dir.clone());

    // Read and parse project.json
    let data = fs.read_file("/project.json").map_err(|e| {
        anyhow::anyhow!(
            "Failed to read project.json from: {}\n\
             Error: {}\n\
             Make sure you're in a project directory or specify the project directory with --dir",
            project_dir.display(),
            e
        )
    })?;

    let config: ProjectConfig = serde_json::from_slice(&data).with_context(|| {
        format!(
            "Failed to parse project.json from: {}",
            project_dir.display()
        )
    })?;

    Ok((config.uid.clone(), config.name.clone()))
}

/// Handle the dev command
///
/// Connects to server, syncs project files, and runs file watching and UI loops.
pub fn handle_dev(args: DevArgs) -> Result<()> {
    // Validate local project
    let (project_uid, _project_name) = validate_local_project(&args.dir)?;

    // Parse host specifier
    let host_spec = if let Some(Some(host)) = &args.push_host {
        // Push to specified host
        HostSpecifier::parse(host)?
    } else if args.push_host.is_some() {
        // Push to local in-memory server
        HostSpecifier::Local
    } else {
        // No push specified - use local for now (could be changed to require explicit host)
        HostSpecifier::Local
    };

    // Create tokio runtime
    let runtime = tokio::runtime::Runtime::new()?;

    // Run async handler
    runtime.block_on(async { handle_dev_async(args, project_uid, host_spec).await })
}

/// Async handler for dev command
async fn handle_dev_async(
    args: DevArgs,
    project_uid: String,
    host_spec: HostSpecifier,
) -> Result<()> {
    // Connect to server
    let transport = client_connect(host_spec).context("Failed to connect to server")?;

    // Wrap transport in Arc<Mutex> for sharing
    let shared_transport = Arc::new(tokio::sync::Mutex::new(transport));

    // Create LpClient with shared transport
    let client = Arc::new(LpClient::new_shared(Arc::clone(&shared_transport)));

    // Create local filesystem
    let local_fs: Arc<dyn LpFs> = Arc::new(LpFsStd::new(args.dir.clone()));

    // Run initial tasks: push or pull project if needed
    if args.push_host.is_some() {
        // Push project to server
        push_project_async(&client, &*local_fs, &project_uid)
            .await
            .context("Failed to push project to server")?;
    } else {
        // Pull project from server (if it exists)
        // Note: This might fail if project doesn't exist on server, which is OK
        if let Err(e) = pull_project_async(&client, &*local_fs, &project_uid).await {
            eprintln!("Warning: Failed to pull project from server: {}", e);
            eprintln!("Continuing with local project files...");
        }
    }

    // Load project on server
    let project_path = format!("projects/{}", project_uid);
    let project_handle = client
        .project_load(&project_path)
        .await
        .context("Failed to load project on server")?;

    // Spawn fs_loop task
    let fs_loop_handle = {
        let transport = Arc::clone(&shared_transport);
        let project_dir = args.dir.clone();
        let project_uid = project_uid.clone();
        // Create a new filesystem instance for the fs_loop (LpFsStd doesn't implement Clone)
        let local_fs_for_loop: Arc<dyn LpFs + Send + Sync> =
            Arc::new(LpFsStd::new(args.dir.clone()));
        tokio::spawn(async move {
            if let Err(e) = fs_loop(transport, project_dir, project_uid, local_fs_for_loop).await {
                eprintln!("fs_loop error: {}", e);
            }
        })
    };

    // Run UI or wait for shutdown
    if args.headless {
        // Headless mode: wait for Ctrl+C
        println!("Running in headless mode. Press Ctrl+C to exit.");
        signal::ctrl_c().await?;
        println!("Shutting down...");
    } else {
        // Run UI
        let project_view = Arc::new(std::sync::Mutex::new(
            lp_engine_client::project::ClientProjectView::new(),
        ));

        // Create a new LpClient for the UI (shares the same transport)
        let ui_client = LpClient::new_shared(Arc::clone(&shared_transport));

        let ui_state = DebugUiState::new(
            project_view,
            project_handle,
            ui_client,
            tokio::runtime::Handle::current(),
        );

        // Run UI (blocking)
        let options = eframe::NativeOptions::default();
        if let Err(e) = eframe::run_native(
            "LightPlayer Dev UI",
            options,
            Box::new(|_cc| Box::new(ui_state)),
        ) {
            eprintln!("UI error: {}", e);
        }
    }

    // Close transport explicitly
    // Note: We can't easily close transport here since it's in an Arc
    // The transport will be closed when all references are dropped
    // Abort the fs_loop task
    fs_loop_handle.abort();
    let _ = fs_loop_handle.await;

    Ok(())
}
