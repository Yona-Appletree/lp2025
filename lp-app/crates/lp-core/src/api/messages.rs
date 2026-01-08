//! Message types for LpApp communication

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Incoming messages to LpApp
///
/// Essentially the same as `Command` but renamed for clarity in the app context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum MsgIn {
    ListNodes {},

    /// Start receiving updates for the specified node.
    WatchNode {
        id: String,
    },

    /// Stop receiving updates for the specified node.
    UnwatchNode {
        id: String,
    },
}

/// Outgoing messages from LpApp
///
/// Responses and status updates sent back to firmware.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum MsgOut {
    NodeList { nodes: Vec<String> },
}
