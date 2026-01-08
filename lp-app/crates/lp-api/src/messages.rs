//! Client/server protocol message types

extern crate alloc;

use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

/// Log level for log messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum ClientMsg {
    /// Read a file from the project filesystem
    FsRead { path: String },

    /// Write a file to the project filesystem
    FsWrite { path: String, data: Vec<u8> },

    /// Check if a file exists
    FsExists { path: String },

    /// List directory contents
    FsListDir { path: String },

    /// Start syncing a project
    SyncStart { project_name: String },

    /// Sync a file (client -> server)
    SyncFile { path: String, data: Vec<u8> },

    /// Get texture data for debugging
    GetTextureData { texture_id: String },

    /// Get node status for debugging
    GetNodeStatus { node_id: String, node_type: String },

    /// Get output state for debugging
    GetOutputState { output_id: String },

    /// Subscribe to log messages
    SubscribeLogs { level: LogLevel },
}

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum ServerMsg {
    /// Response to FsRead
    FsReadResponse { path: String, data: Vec<u8> },

    /// Response to FsExists
    FsExistsResponse { path: String, exists: bool },

    /// Response to FsListDir
    FsListDirResponse { path: String, entries: Vec<String> },

    /// Sync a file (server -> client)
    SyncFile { path: String, data: Vec<u8> },

    /// Sync complete
    SyncComplete,

    /// Texture data response
    TextureData {
        texture_id: String,
        data: Vec<u8>,
        width: u32,
        height: u32,
        format: String,
    },

    /// Node status response
    NodeStatus { node_id: String, status: String },

    /// Output state response
    OutputState { output_id: String, pixels: Vec<u8> },

    /// Log message
    Log { level: LogLevel, message: String },

    /// Error response
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_msg_serialization() {
        let msg = ClientMsg::FsRead {
            path: "/project.json".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("FsRead"));
        assert!(json.contains("/project.json"));

        let deserialized: ClientMsg = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientMsg::FsRead { path } => assert_eq!(path, "/project.json"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_server_msg_serialization() {
        let msg = ServerMsg::Log {
            level: LogLevel::Info,
            message: "Test message".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Log"));
        assert!(json.contains("Test message"));

        let deserialized: ServerMsg = serde_json::from_str(&json).unwrap();
        match deserialized {
            ServerMsg::Log { level, message } => {
                assert_eq!(level, LogLevel::Info);
                assert_eq!(message, "Test message");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_log_level_serialization() {
        let level = LogLevel::Warn;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"warn\"");

        let deserialized: LogLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, LogLevel::Warn);
    }
}
