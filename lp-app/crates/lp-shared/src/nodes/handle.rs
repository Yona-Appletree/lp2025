pub struct NodeHandle(pub i32);

/// Runtime identifier for a node.
impl NodeHandle {
    pub const NONE: Self = NodeHandle(-1);
    pub fn new(handle: i32) -> Self {
        NodeHandle(handle)
    }
}
