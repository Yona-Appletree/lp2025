use crate::nodes::handle::NodeHandle;
use crate::nodes::state::NodeState;
use crate::project::frame_id::FrameId;
use alloc::string::String;
use alloc::vec::Vec;
use hashbrown::HashMap;

pub enum ProjectRequest {
    /// Get changes since a specific frame
    GetChanges {
        since_frame: FrameId,

        /// Nodes for which detail should always be returned.
        detail_specifier: NodeSpecifier,
    },
}

pub enum ProjectResponse {
    GetChanges {
        /// Current frame ID
        current_frame: FrameId,

        engine_stats: EngineStats,

        /// Detailed node information
        /// Included for nodes matching detail_specifier
        /// and any nodes created after `since_frame`
        node_detail: HashMap<NodeHandle, NodeDetail>,

        /// List of current node handles
        /// Allows clients to prune removed nodes
        node_handles: Vec<NodeHandle>,
    },
}

pub struct EngineStats {
    frame_ms_avg: f32,
    frame_ms_std_dev: f32,
    memory_max_usage: u64,
    memory_avg_usage: u64,
}

pub struct NodeDetail {
    path: String,
    state: NodeState,
}

pub enum NodeSpecifier {
    None,
    All,
    ByHandles(Vec<i32>),
}
