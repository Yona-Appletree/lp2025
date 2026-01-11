//! Runtime types and traits

pub mod contexts;
pub mod frame_time;
pub mod lifecycle;

pub use contexts::{
    FixtureRenderContext, InitContext, OutputRenderContext, ShaderRenderContext,
    TextureRenderContext,
};
pub use frame_time::FrameTime;
pub use lifecycle::NodeLifecycle;

use lp_shared::nodes::handle::NodeHandle;
use lp_shared::project::frame_id::FrameId;

/// Base structure for all node runtimes containing handle, path, and frame tracking
#[derive(Debug)]
pub struct NodeRuntimeBase {
    pub handle: NodeHandle,
    pub path: String,
    pub created_frame: FrameId,
    pub last_config_frame: FrameId,
    pub last_state_frame: FrameId,
}

impl NodeRuntimeBase {
    /// Create a new NodeRuntimeBase with handle, path, and current frame
    pub fn new(handle: NodeHandle, path: String, current_frame: FrameId) -> Self {
        Self {
            handle,
            path,
            created_frame: current_frame,
            last_config_frame: current_frame,
            last_state_frame: current_frame,
        }
    }

    /// Update the last config frame (called when config changes)
    pub fn update_config_frame(&mut self, frame: FrameId) {
        self.last_config_frame = frame;
    }

    /// Update the last state frame (called when state changes)
    pub fn update_state_frame(&mut self, frame: FrameId) {
        self.last_state_frame = frame;
    }
}
