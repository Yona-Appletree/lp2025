//! Main server struct for processing messages and managing projects

extern crate alloc;

use crate::error::ServerError;
use crate::handlers;
use crate::project_manager::ProjectManager;
use alloc::{boxed::Box, rc::Rc, string::{String, ToString}, vec::Vec};
use core::cell::RefCell;
use lp_model::Message;
use lp_shared::fs::LpFs;
use lp_shared::output::OutputProvider;

/// Main server struct for processing client-server messages
///
/// Uses a tick-based API similar to game engines, processing incoming messages
/// and returning responses. The server manages projects and handles filesystem
/// operations on its base filesystem.
pub struct LpServer {
    /// Output provider (shared, mutable) for projects
    output_provider: Rc<RefCell<dyn OutputProvider>>,
    /// Project manager for handling multiple projects
    project_manager: ProjectManager,
    /// Base filesystem (server root, projects in `projects/` subdirectory)
    base_fs: Box<dyn LpFs>,
}

impl LpServer {
    /// Create a new LpServer instance
    ///
    /// # Arguments
    ///
    /// * `output_provider` - Shared output provider for projects (Rc<RefCell> for no_std compatibility)
    /// * `base_fs` - Base filesystem (server root, projects stored in `projects_base_dir` subdirectory)
    /// * `projects_base_dir` - Base directory for projects (e.g., "projects/")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// extern crate alloc;
    /// use lp_server::LpServer;
    /// use lp_shared::fs::LpFsStd;
    /// use lp_shared::output::MemoryOutputProvider;
    /// use alloc::{boxed::Box, rc::Rc, string::ToString};
    /// use core::cell::RefCell;
    ///
    /// let output_provider = Rc::new(RefCell::new(MemoryOutputProvider::new()));
    /// let base_fs = Box::new(LpFsStd::new("/path/to/server/root".into()));
    /// let server = LpServer::new(output_provider, base_fs, "projects/".to_string());
    /// ```
    pub fn new(
        output_provider: Rc<RefCell<dyn OutputProvider>>,
        base_fs: Box<dyn LpFs>,
        projects_base_dir: String,
    ) -> Self {
        let project_manager = ProjectManager::new(projects_base_dir);
        Self {
            output_provider,
            project_manager,
            base_fs,
        }
    }

    /// Process incoming messages and return responses
    ///
    /// This is the main entry point for processing client messages. It handles
    /// filesystem operations and project management requests.
    ///
    /// # Arguments
    ///
    /// * `delta_ms` - Time delta in milliseconds (for future use with project updates)
    /// * `incoming` - Vector of incoming messages from clients
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Message>)` - Vector of response messages (all `Message::Server` variants)
    /// * `Err(ServerError)` - If processing failed
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// extern crate alloc;
    /// use lp_model::Message;
    /// use lp_server::LpServer;
    /// use lp_shared::fs::LpFsMemory;
    /// use lp_shared::output::MemoryOutputProvider;
    /// use alloc::{boxed::Box, rc::Rc, vec::Vec};
    /// use core::cell::RefCell;
    ///
    /// let output_provider = Rc::new(RefCell::new(MemoryOutputProvider::new()));
    /// let base_fs = Box::new(LpFsMemory::new());
    /// let mut server = LpServer::new(output_provider, base_fs, "projects/".to_string());
    /// let incoming = vec![/* messages */];
    /// let responses = server.tick(16, incoming).unwrap();
    /// ```
    pub fn tick(
        &mut self,
        _delta_ms: u32,
        incoming: Vec<Message>,
    ) -> Result<Vec<Message>, ServerError> {
        let mut responses = Vec::new();

        for message in incoming {
            match message {
                Message::Client(client_msg) => {
                    // Process client message and generate response
                    let response = handlers::handle_client_message(
                        &mut self.project_manager,
                        &mut *self.base_fs,
                        &self.output_provider,
                        client_msg,
                    )?;
                    responses.push(Message::Server(response));
                }
                Message::Server(_) => {
                    // Server messages shouldn't be sent to the server
                    // Log or ignore
                    return Err(ServerError::Core(
                        "Received server message on server side".to_string(),
                    ));
                }
            }
        }

        Ok(responses)
    }

    /// Get a reference to the base filesystem
    pub fn base_fs(&self) -> &dyn LpFs {
        &*self.base_fs
    }

    /// Get a mutable reference to the project manager
    pub fn project_manager_mut(&mut self) -> &mut ProjectManager {
        &mut self.project_manager
    }
}
