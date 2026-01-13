use crate::nodes::{NodeHandle, NodeKind, NodeConfig};
use crate::path::LpPath;
use crate::project::FrameId;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Node specifier for API requests
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiNodeSpecifier {
    /// No nodes
    None,
    /// All nodes
    All,
    /// Specific handles
    ByHandles(Vec<NodeHandle>),
}

/// Project request from client
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectRequest {
    /// Get changes since a frame
    GetChanges {
        /// Last frame client synced
        since_frame: FrameId,
        /// Which nodes need full state
        detail_specifier: ApiNodeSpecifier,
    },
}

/// Project response from server
/// 
/// Note: Cannot implement Clone because NodeDetail contains trait object
#[derive(Debug)]
pub enum ProjectResponse {
    /// Changes response
    GetChanges {
        /// Current frame ID
        current_frame: FrameId,
        /// All current node handles (for pruning removed nodes)
        node_handles: Vec<NodeHandle>,
        /// Changed nodes since since_frame
        node_changes: Vec<NodeChange>,
        /// Full detail for requested nodes
        node_details: BTreeMap<NodeHandle, NodeDetail>,
    },
}

/// Node change notification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeChange {
    /// New node created
    Created {
        handle: NodeHandle,
        path: LpPath,
        kind: NodeKind,
    },
    /// Config updated
    ConfigUpdated {
        handle: NodeHandle,
        config_ver: FrameId,
    },
    /// State updated
    StateUpdated {
        handle: NodeHandle,
        state_ver: FrameId,
    },
    /// Status changed
    StatusChanged {
        handle: NodeHandle,
        status: NodeStatus,
    },
    /// Node removed
    Removed {
        handle: NodeHandle,
    },
}

/// Node status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeStatus {
    /// Created but not yet initialized
    Created,
    /// Error initializing the node
    InitError(String),
    /// Node is running normally
    Ok,
    /// Node is running, but something is wrong
    Warn(String),
    /// Node cannot run
    Error(String),
}

/// Node detail - full config + state + status
/// 
/// Note: Cannot implement Clone/PartialEq/Eq because config is a trait object
#[derive(Debug)]
pub struct NodeDetail {
    pub path: LpPath,
    pub config: Box<dyn NodeConfig>, // todo!() - will need serialization later
    pub state: NodeState, // External state only
    pub status: NodeStatus,
}

/// Node state - external state (shared with clients)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeState {
    Texture(crate::nodes::texture::TextureState),
    Shader(crate::nodes::shader::ShaderState),
    Output(crate::nodes::output::OutputState),
    Fixture(crate::nodes::fixture::FixtureState),
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_api_node_specifier() {
        let spec = ApiNodeSpecifier::None;
        assert_eq!(spec, ApiNodeSpecifier::None);
        
        let spec = ApiNodeSpecifier::All;
        assert_eq!(spec, ApiNodeSpecifier::All);
        
        let spec = ApiNodeSpecifier::ByHandles(vec![NodeHandle::new(1), NodeHandle::new(2)]);
        match spec {
            ApiNodeSpecifier::ByHandles(handles) => {
                assert_eq!(handles.len(), 2);
            }
            _ => panic!("Expected ByHandles"),
        }
    }

    #[test]
    fn test_project_request() {
        let request = ProjectRequest::GetChanges {
            since_frame: FrameId::default(),
            detail_specifier: ApiNodeSpecifier::All,
        };
        match request {
            ProjectRequest::GetChanges { since_frame, detail_specifier } => {
                assert_eq!(since_frame, FrameId::default());
                assert_eq!(detail_specifier, ApiNodeSpecifier::All);
            }
        }
    }

    #[test]
    fn test_node_status() {
        let status = NodeStatus::Created;
        assert_eq!(status, NodeStatus::Created);
        
        let status = NodeStatus::InitError("test error".to_string());
        match status {
            NodeStatus::InitError(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Expected InitError"),
        }
    }

    #[test]
    fn test_node_state() {
        let state = NodeState::Texture(crate::nodes::texture::TextureState {
            texture_data: vec![0, 1, 2, 3],
        });
        match state {
            NodeState::Texture(tex_state) => {
                assert_eq!(tex_state.texture_data.len(), 4);
            }
            _ => panic!("Expected Texture state"),
        }
    }
}

