//! Message parsing and handling utilities

use alloc::{format, string::String};
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::protocol::command::Command;

/// Message trait for extensibility (future CRC, framing, etc.)
pub trait Message {
    /// Serialize message to JSON string
    fn to_json(&self) -> Result<String, Error>
    where
        Self: Serialize,
    {
        serde_json::to_string(self).map_err(|e| Error::Serialization(format!("{}", e)))
    }

    /// Deserialize message from JSON string
    fn from_json(json: &str) -> Result<Self, Error>
    where
        Self: for<'de> Deserialize<'de>,
    {
        serde_json::from_str(json).map_err(|e| Error::Serialization(format!("{}", e)))
    }
}

impl Message for Command {}
impl Message for crate::protocol::command::Response {}

/// Parse a command from a JSON string (reads until `\n` if needed)
pub fn parse_command(json: &str) -> Result<Command, Error> {
    // Remove trailing newline if present
    let json = json.trim_end_matches('\n');
    Command::from_json(json)
}

/// Serialize a command to JSON string with newline
pub fn serialize_command(command: &Command) -> Result<String, Error> {
    let mut json = command.to_json()?;
    json.push('\n');
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::command::{Command, LogLevel};

    #[test]
    fn test_parse_command() {
        let json = r#"{"$type":"GetProject"}"#;
        let command = parse_command(json).unwrap();
        assert!(matches!(command, Command::GetProject));

        // Test with newline
        let json = r#"{"$type":"Log","level":"info","message":"Test"}
"#;
        let command = parse_command(json).unwrap();
        assert!(matches!(
            command,
            Command::Log {
                level: LogLevel::Info,
                ..
            }
        ));
    }

    #[test]
    fn test_serialize_command() {
        let command = Command::GetProject;
        let json = serialize_command(&command).unwrap();
        assert!(json.ends_with('\n'));
        assert!(json.contains("\"$type\":\"GetProject\""));
    }
}
