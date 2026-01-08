//! Filesystem watcher for detecting file changes

use lp_engine::app::{ChangeType, FileChange};
use lp_engine::error::Error;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc;

/// Filesystem watcher for a project directory
pub struct FileWatcher {
    #[allow(dead_code)] // Watcher must be kept alive to continue watching
    watcher: RecommendedWatcher,
    root_path: PathBuf,
    receiver: Mutex<mpsc::Receiver<Result<Event, notify::Error>>>,
}

impl FileWatcher {
    /// Create a new file watcher for the given project root directory
    pub fn watch_project(root_path: PathBuf) -> Result<Self, Error> {
        // Create a channel for receiving events
        let (tx, rx) = mpsc::channel();

        // Create the watcher
        let mut watcher = RecommendedWatcher::new(
            move |event: Result<Event, notify::Error>| {
                // Send event to receiver (ignore send errors - receiver might be dropped)
                let _ = tx.send(event);
            },
            notify::Config::default(),
        )
        .map_err(|e| Error::Filesystem(format!("Failed to create filesystem watcher: {}", e)))?;

        // Watch the root directory recursively
        watcher
            .watch(&root_path, RecursiveMode::Recursive)
            .map_err(|e| {
                Error::Filesystem(format!("Failed to watch directory {:?}: {}", root_path, e))
            })?;

        Ok(Self {
            watcher,
            root_path,
            receiver: Mutex::new(rx),
        })
    }

    /// Get all file changes since the last call
    ///
    /// Returns a vector of FileChange events with paths relative to project root.
    pub fn get_changes(&self) -> Vec<FileChange> {
        let receiver = self.receiver.lock().unwrap();
        let mut changes = Vec::new();

        // Collect all available events (non-blocking)
        while let Ok(event_result) = receiver.try_recv() {
            match event_result {
                Ok(event) => {
                    // Convert notify event to FileChange format
                    if let Some(file_changes) = self.convert_event(&event) {
                        changes.extend(file_changes);
                    }
                }
                Err(e) => {
                    log::error!("Filesystem watcher error: {}", e);
                }
            }
        }

        changes
    }

    /// Convert a notify Event to FileChange format
    ///
    /// Returns None if the event should be ignored (e.g., temporary files).
    fn convert_event(&self, event: &Event) -> Option<Vec<FileChange>> {
        let mut changes = Vec::new();

        // Determine change type from event kind
        let change_type = match event.kind {
            EventKind::Create(_) => ChangeType::Create,
            EventKind::Modify(_) => ChangeType::Modify,
            EventKind::Remove(_) => ChangeType::Delete,
            EventKind::Any | EventKind::Other => {
                // Ignore other event types
                return None;
            }
            EventKind::Access(_) => {
                // Ignore access events (we only care about modifications)
                return None;
            }
        };

        // Process each path in the event
        for path in &event.paths {
            // Convert absolute path to relative path from project root
            if let Some(relative_path) = self.path_to_relative(path) {
                // Filter out temporary files and other irrelevant files
                if self.should_include_path(&relative_path) {
                    changes.push(FileChange {
                        path: relative_path,
                        change_type,
                    });
                }
            }
        }

        if changes.is_empty() {
            None
        } else {
            Some(changes)
        }
    }

    /// Convert an absolute path to a relative path from project root
    ///
    /// Returns the path with a leading slash (e.g., "/project.json").
    fn path_to_relative(&self, absolute_path: &Path) -> Option<String> {
        // Strip the root path prefix
        let relative_path = absolute_path.strip_prefix(&self.root_path).ok()?;

        // Convert to string with leading slash
        let path_str = format!("/{}", relative_path.to_string_lossy().replace('\\', "/"));

        Some(path_str)
    }

    /// Check if a path should be included in file changes
    ///
    /// Filters out temporary files, editor backups, etc.
    fn should_include_path(&self, path: &str) -> bool {
        // Filter out common temporary/backup files
        let excluded_patterns = [
            "~",         // Editor backup files
            ".swp",      // Vim swap files
            ".tmp",      // Temporary files
            ".bak",      // Backup files
            ".DS_Store", // macOS metadata
            "Thumbs.db", // Windows thumbnail cache
        ];

        // Check if path contains any excluded pattern
        for pattern in &excluded_patterns {
            if path.contains(pattern) {
                return false;
            }
        }

        true
    }
}
