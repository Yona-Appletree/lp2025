//! Command protocol types

use alloc::string::String;
use serde::{Deserialize, Serialize};

use crate::project::config::ProjectConfig;

/// Command types for JSON message protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum Command {
    #[serde(rename = "UpdateProject")]
    UpdateProject { project: ProjectConfig },
    #[serde(rename = "GetProject")]
    GetProject,
    #[serde(rename = "Log")]
    Log { level: LogLevel, message: String },
}

/// Log level for log messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

/// Response types (for future use)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum Response {
    #[serde(rename = "Ok")]
    Ok,
    #[serde(rename = "Error")]
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::config::ProjectConfig;
    use alloc::string::ToString;

    #[test]
    fn test_command_serialize() {
        let config = ProjectConfig {
            uid: "UID12345".to_string(),
            name: "Test".to_string(),
        };

        let command = Command::UpdateProject { project: config };
        let json = serde_json::to_string(&command).unwrap();
        assert!(json.contains("\"$type\":\"UpdateProject\""));
        assert!(json.contains("\"uid\":\"UID12345\""));

        let get_command = Command::GetProject;
        let json = serde_json::to_string(&get_command).unwrap();
        assert!(json.contains("\"$type\":\"GetProject\""));

        let log_command = Command::Log {
            level: LogLevel::Warn,
            message: "Test message".to_string(),
        };
        let json = serde_json::to_string(&log_command).unwrap();
        assert!(json.contains("\"$type\":\"Log\""));
        assert!(json.contains("\"level\":\"warn\""));
        assert!(json.contains("\"message\":\"Test message\""));
    }

    #[test]
    fn test_command_deserialize() {
        let json = r#"{"$type":"GetProject"}"#;
        let command: Command = serde_json::from_str(json).unwrap();
        assert!(matches!(command, Command::GetProject));

        let json = r#"{"$type":"Log","level":"error","message":"Test"}"#;
        let command: Command = serde_json::from_str(json).unwrap();
        assert!(matches!(
            command,
            Command::Log {
                level: LogLevel::Error,
                ..
            }
        ));
    }
}
