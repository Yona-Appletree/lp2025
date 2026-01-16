//! Main UI state and egui App implementation

use crate::commands::dev::async_client::AsyncLpClient;
use eframe::egui;
use lp_engine_client::project::ClientProjectView;
use lp_model::{NodeHandle, project::handle::ProjectHandle};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

/// Debug UI application state
pub struct DebugUiState {
    /// Project view (shared between sync and UI)
    project_view: Arc<Mutex<ClientProjectView>>,
    /// Project handle
    project_handle: ProjectHandle,
    /// Async client for syncing
    /// Note: Sync will be handled via a channel-based approach
    /// For now, we store it but don't use it directly
    _async_client: AsyncLpClient,
    /// Nodes we're tracking detail for
    tracked_nodes: BTreeSet<NodeHandle>,
    /// "All detail" checkbox state
    all_detail: bool,
    /// Whether a sync is currently in progress
    sync_in_progress: bool,
    /// GLSL code cache (keyed by node handle)
    glsl_cache: BTreeMap<NodeHandle, String>,
}

impl DebugUiState {
    /// Create new debug UI state
    pub fn new(
        project_view: Arc<Mutex<ClientProjectView>>,
        project_handle: ProjectHandle,
        async_client: AsyncLpClient,
        _runtime_handle: tokio::runtime::Handle,
    ) -> Self {
        // TODO: Set up sync mechanism
        // The challenge is that ClientProjectView is not Send, so we can't easily
        // spawn a task that holds a lock on it. We'll need to use a LocalSet
        // or restructure the sync to not hold the lock across await.
        //
        // For Phase 6, we'll implement basic structure. The actual sync mechanism
        // will be refined in later phases once we have the UI working.

        Self {
            project_view,
            project_handle,
            _async_client: async_client,
            tracked_nodes: BTreeSet::new(),
            all_detail: false,
            sync_in_progress: false,
            glsl_cache: BTreeMap::new(),
        }
    }

    /// Handle sync logic
    ///
    /// Checks if sync is in progress, starts new sync if not, and handles completion.
    /// TODO: Implement proper async sync handling
    fn handle_sync(&mut self) {
        // Update view's detail_tracking to match tracked_nodes
        {
            let mut view = self.project_view.lock().unwrap();
            view.detail_tracking.clear();
            view.detail_tracking
                .extend(self.tracked_nodes.iter().copied());
        }

        // TODO: Implement actual sync
        // For now, this is a placeholder
    }
}

impl eframe::App for DebugUiState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle sync
        self.handle_sync();

        // Request repaint to keep loop running
        ctx.request_repaint_after(std::time::Duration::from_millis(16)); // ~60 FPS

        // Render UI (placeholder for now)
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Debug UI");
            ui.label("UI implementation in progress...");
            ui.label(format!("Tracked nodes: {}", self.tracked_nodes.len()));
        });
    }
}
