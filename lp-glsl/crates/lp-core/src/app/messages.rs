//! Message types for LpApp communication

use alloc::string::String;
use serde::{Deserialize, Serialize};

use crate::project::config::ProjectConfig;
use crate::protocol::LogLevel;

/// Incoming messages to LpApp
///
/// Essentially the same as `Command` but renamed for clarity in the app context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum MsgIn {
    #[serde(rename = "UpdateProject")]
    UpdateProject { project: ProjectConfig },
    #[serde(rename = "GetProject")]
    GetProject,
    #[serde(rename = "Log")]
    Log { level: LogLevel, message: String },
}

/// Outgoing messages from LpApp
///
/// Responses and status updates sent back to firmware.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum MsgOut {
    #[serde(rename = "Project")]
    Project { project: ProjectConfig },
    // Future: status updates, errors, etc.
}

impl From<crate::protocol::Command> for MsgIn {
    fn from(cmd: crate::protocol::Command) -> Self {
        match cmd {
            crate::protocol::Command::UpdateProject { project } => MsgIn::UpdateProject { project },
            crate::protocol::Command::GetProject => MsgIn::GetProject,
            crate::protocol::Command::Log { level, message } => MsgIn::Log { level, message },
        }
    }
}

impl From<MsgIn> for crate::protocol::Command {
    fn from(msg: MsgIn) -> Self {
        match msg {
            MsgIn::UpdateProject { project } => crate::protocol::Command::UpdateProject { project },
            MsgIn::GetProject => crate::protocol::Command::GetProject,
            MsgIn::Log { level, message } => crate::protocol::Command::Log { level, message },
        }
    }
}
