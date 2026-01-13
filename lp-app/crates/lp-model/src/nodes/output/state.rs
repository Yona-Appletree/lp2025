use alloc::vec::Vec;

/// Output node state - runtime values
#[derive(Debug, Clone)]
pub struct OutputState {
    /// Channel data buffer
    pub channel_data: Vec<u8>,
}
