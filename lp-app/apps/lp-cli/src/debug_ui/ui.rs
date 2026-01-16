//! Main UI state and egui App implementation

use crate::commands::dev::async_client::{
    AsyncLpClient, serializable_response_to_project_response,
};
use crate::debug_ui::panels;
use eframe::egui;
use lp_engine_client::project::ClientProjectView;
use lp_model::{NodeHandle, project::handle::ProjectHandle};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::oneshot;

/// Debug UI application state
pub struct DebugUiState {
    /// Project view (shared between sync and UI)
    project_view: Arc<Mutex<ClientProjectView>>,
    /// Project handle
    project_handle: ProjectHandle,
    /// Async client for syncing (shared via Arc<Mutex<>>)
    async_client: Arc<tokio::sync::Mutex<AsyncLpClient>>,
    /// Nodes we're tracking detail for
    tracked_nodes: BTreeSet<NodeHandle>,
    /// "All detail" checkbox state
    all_detail: bool,
    /// Whether a sync is currently in progress
    sync_in_progress: bool,
    /// Pending sync result receiver (if sync is in progress)
    /// Contains SerializableProjectResponse which can be sent across threads
    pending_sync: Option<
        oneshot::Receiver<
            Result<lp_model::project::api::SerializableProjectResponse, anyhow::Error>,
        >,
    >,
    /// GLSL code cache (keyed by node handle)
    glsl_cache: BTreeMap<NodeHandle, String>,
    /// Currently selected node handle (for detail display)
    selected_node: Option<NodeHandle>,
    /// Tokio runtime handle for spawning async tasks
    runtime_handle: tokio::runtime::Handle,
    /// Last frame time for FPS calculation
    last_frame_time: Option<Instant>,
    /// Frame count
    frame_count: u64,
    /// FPS history (last 60 frames)
    fps_history: Vec<f32>,
}

impl DebugUiState {
    /// Create new debug UI state
    pub fn new(
        project_view: Arc<Mutex<ClientProjectView>>,
        project_handle: ProjectHandle,
        async_client: AsyncLpClient,
        runtime_handle: tokio::runtime::Handle,
    ) -> Self {
        Self {
            project_view,
            project_handle,
            async_client: Arc::new(tokio::sync::Mutex::new(async_client)),
            tracked_nodes: BTreeSet::new(),
            all_detail: false,
            sync_in_progress: false,
            pending_sync: None,
            glsl_cache: BTreeMap::new(),
            selected_node: None,
            runtime_handle,
            last_frame_time: None,
            frame_count: 0,
            fps_history: Vec::new(),
        }
    }

    /// Handle sync logic
    ///
    /// Checks if sync is in progress, starts new sync if not, and handles completion.
    /// Uses a channel-based approach to avoid holding locks across await points.
    fn handle_sync(&mut self) {
        // Check if previous sync completed
        if let Some(mut receiver) = self.pending_sync.take() {
            match receiver.try_recv() {
                Ok(Ok(serializable_response)) => {
                    // Sync completed successfully - convert and apply changes in UI thread
                    match serializable_response_to_project_response(serializable_response) {
                        Ok(project_response) => {
                            let mut view = self.project_view.lock().unwrap();
                            match view.apply_changes(&project_response) {
                                Ok(()) => {
                                    self.sync_in_progress = false;
                                }
                                Err(e) => {
                                    eprintln!("Failed to apply changes: {}", e);
                                    self.sync_in_progress = false;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to convert response: {}", e);
                            self.sync_in_progress = false;
                        }
                    }
                }
                Ok(Err(e)) => {
                    // Sync failed
                    eprintln!("Sync error: {}", e);
                    self.sync_in_progress = false;
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                    // Still in progress, put receiver back
                    self.pending_sync = Some(receiver);
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    // Channel closed (shouldn't happen)
                    self.sync_in_progress = false;
                }
            }
        }

        // Start new sync if not in progress
        if !self.sync_in_progress {
            // Update view's detail_tracking to match tracked_nodes and get sync parameters
            let (since_frame, detail_specifier) = {
                let mut view = self.project_view.lock().unwrap();
                let is_initial_sync = view.nodes.is_empty();

                view.detail_tracking.clear();
                view.detail_tracking
                    .extend(self.tracked_nodes.iter().copied());

                let since_frame = view.frame_id;
                // For initial sync (empty view), request all nodes to populate the list
                // Otherwise use normal detail_specifier
                let detail_specifier = if is_initial_sync {
                    lp_model::project::api::ApiNodeSpecifier::All
                } else {
                    view.detail_specifier()
                };
                (since_frame, detail_specifier)
            };

            // Spawn async task to do sync (without holding view lock)
            let client = Arc::clone(&self.async_client);
            let handle = self.project_handle;
            let runtime_handle = self.runtime_handle.clone();

            let (tx, rx) = oneshot::channel();
            self.pending_sync = Some(rx);
            self.sync_in_progress = true;

            runtime_handle.spawn(async move {
                // Do async sync call (no view lock held)
                let result = {
                    let mut client_guard = client.lock().await;
                    client_guard
                        .project_sync_internal(handle, since_frame, detail_specifier)
                        .await
                };

                // Send result back to UI thread (SerializableProjectResponse is Send)
                match result {
                    Ok(serializable_response) => {
                        // Send the serializable response back - UI thread will convert and apply it
                        let _ = tx.send(Ok(serializable_response));
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e));
                    }
                }
            });
        }
    }
}

impl eframe::App for DebugUiState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Calculate FPS
        let now = Instant::now();
        let delta_ms = if let Some(last_time) = self.last_frame_time {
            let delta = now.duration_since(last_time);
            delta.as_millis().min(u32::MAX as u128) as u32
        } else {
            0
        };
        self.last_frame_time = Some(now);
        self.frame_count += 1;

        let current_fps = if delta_ms > 0 {
            1000.0 / delta_ms as f32
        } else {
            0.0
        };
        self.fps_history.push(current_fps);
        if self.fps_history.len() > 60 {
            self.fps_history.remove(0);
        }

        // Handle sync
        self.handle_sync();

        // Request repaint to keep loop running
        ctx.request_repaint_after(std::time::Duration::from_millis(16)); // ~60 FPS

        // Get view snapshot for rendering
        let view = self.project_view.lock().unwrap();

        // Status panel (top)
        egui::TopBottomPanel::top("status_panel").show(ctx, |ui| {
            panels::render_status_panel(
                ui,
                self.frame_count,
                self.fps_history.last().copied().unwrap_or(0.0),
                if !self.fps_history.is_empty() {
                    self.fps_history.iter().sum::<f32>() / self.fps_history.len() as f32
                } else {
                    0.0
                },
                self.sync_in_progress,
            );
        });

        // Side panel for nodes list
        egui::SidePanel::left("nodes_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                panels::render_nodes_panel(
                    ui,
                    &view,
                    &mut self.tracked_nodes,
                    &mut self.all_detail,
                );
            });

        // Main panel for node details
        egui::CentralPanel::default().show(ctx, |ui| {
            // Find selected node (first tracked node, or first node if none tracked)
            let node_to_show = if let Some(selected) = self.selected_node {
                view.nodes.get(&selected)
            } else {
                // Show first tracked node, or first node if none tracked
                self.tracked_nodes
                    .iter()
                    .next()
                    .and_then(|handle| view.nodes.get(handle))
                    .or_else(|| view.nodes.values().next())
            };

            if let Some(entry) = node_to_show {
                // Update selected_node
                if let Some(handle) = view
                    .nodes
                    .iter()
                    .find(|(_, e)| std::ptr::eq(*e, entry))
                    .map(|(h, _)| *h)
                {
                    self.selected_node = Some(handle);
                }

                // Render appropriate panel based on node kind
                match &entry.kind {
                    lp_model::NodeKind::Texture => {
                        if let Some(lp_model::project::api::NodeState::Texture(state)) =
                            &entry.state
                        {
                            panels::render_texture_panel(ui, entry, state);
                        } else {
                            ui.label("No texture state available");
                        }
                    }
                    lp_model::NodeKind::Shader => {
                        if let Some(lp_model::project::api::NodeState::Shader(state)) = &entry.state
                        {
                            panels::render_shader_panel(ui, entry, state);
                        } else {
                            ui.label("No shader state available");
                        }
                    }
                    lp_model::NodeKind::Fixture => {
                        if let Some(lp_model::project::api::NodeState::Fixture(state)) =
                            &entry.state
                        {
                            if let Some(handle) = self.selected_node {
                                panels::render_fixture_panel(ui, &view, entry, state, &handle);
                            } else {
                                ui.label("No fixture handle available");
                            }
                        } else {
                            ui.label("No fixture state available");
                        }
                    }
                    lp_model::NodeKind::Output => {
                        if let Some(lp_model::project::api::NodeState::Output(state)) = &entry.state
                        {
                            panels::render_output_panel(ui, entry, state);
                        } else {
                            ui.label("No output state available");
                        }
                    }
                }
            } else {
                ui.label("No nodes available");
            }
        });
    }
}
