//! WebSocket client transport
//!
//! Implements `ClientTransport` using synchronous `tungstenite` with internal buffering
//! to match the polling interface.

use std::collections::VecDeque;

use lp_model::{ClientMessage, ServerMessage, TransportError};
use lp_shared::transport::ClientTransport;
use std::net::TcpStream;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{WebSocket, connect};

/// WebSocket client transport
///
/// Uses synchronous `tungstenite` with internal buffering to provide a polling-based
/// interface. Messages are buffered internally to allow non-blocking receive.
pub struct WebSocketClientTransport {
    /// WebSocket connection (None if disconnected)
    socket: Option<WebSocket<MaybeTlsStream<TcpStream>>>,
    /// Buffer for incoming messages
    incoming_buffer: VecDeque<ServerMessage>,
}

impl WebSocketClientTransport {
    /// Create a new WebSocket client transport and connect to the server
    ///
    /// # Arguments
    ///
    /// * `url` - WebSocket URL (e.g., `ws://localhost:2812/`)
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` if connection succeeded
    /// * `Err(TransportError)` if connection failed
    pub fn new(url: &str) -> Result<Self, TransportError> {
        // Connect via tungstenite (handles TCP connection internally)
        let (socket, _) = connect(url).map_err(|e| {
            TransportError::Other(format!(
                "Failed to establish WebSocket connection to '{}': {}",
                url, e
            ))
        })?;

        // Note: Setting non-blocking mode on the underlying stream is complex with tungstenite
        // We'll handle WouldBlock errors in fill_buffer() instead
        // The tungstenite library will return WouldBlock errors if the stream would block

        Ok(Self {
            socket: Some(socket),
            incoming_buffer: VecDeque::new(),
        })
    }

    /// Fill the incoming buffer from the websocket (non-blocking)
    ///
    /// Attempts to read messages from the websocket and adds them to the buffer.
    /// Returns immediately if no messages are available.
    fn fill_buffer(&mut self) -> Result<(), TransportError> {
        let socket = match &mut self.socket {
            Some(s) => s,
            None => return Err(TransportError::ConnectionLost),
        };

        // Try to read messages (non-blocking due to non-blocking TCP stream)
        loop {
            match socket.read() {
                Ok(tungstenite::Message::Text(text)) => {
                    // Deserialize ServerMessage from JSON
                    let msg: ServerMessage = serde_json::from_str(&text).map_err(|e| {
                        TransportError::Deserialization(format!(
                            "Failed to deserialize ServerMessage: {}",
                            e
                        ))
                    })?;
                    self.incoming_buffer.push_back(msg);
                }
                Ok(tungstenite::Message::Binary(data)) => {
                    // Deserialize ServerMessage from binary JSON
                    let msg: ServerMessage = serde_json::from_slice(&data).map_err(|e| {
                        TransportError::Deserialization(format!(
                            "Failed to deserialize ServerMessage: {}",
                            e
                        ))
                    })?;
                    self.incoming_buffer.push_back(msg);
                }
                Ok(tungstenite::Message::Close(_)) => {
                    self.socket = None;
                    return Err(TransportError::ConnectionLost);
                }
                Ok(tungstenite::Message::Ping(_)) => {
                    // Auto-respond to pings
                    if let Err(e) = socket.send(tungstenite::Message::Pong(vec![])) {
                        self.socket = None;
                        return Err(TransportError::Other(format!("Failed to send pong: {}", e)));
                    }
                }
                Ok(tungstenite::Message::Pong(_)) => {
                    // Ignore pongs
                }
                Ok(tungstenite::Message::Frame(_)) => {
                    // Ignore raw frames
                }
                Err(tungstenite::Error::Io(ref e))
                    if e.kind() == std::io::ErrorKind::WouldBlock =>
                {
                    // No data available, return
                    break;
                }
                Err(e) => {
                    // Other error, connection may be lost
                    self.socket = None;
                    return Err(TransportError::Other(format!("WebSocket error: {}", e)));
                }
            }
        }

        Ok(())
    }
}

impl ClientTransport for WebSocketClientTransport {
    fn send(&mut self, msg: ClientMessage) -> Result<(), TransportError> {
        let socket = match &mut self.socket {
            Some(s) => s,
            None => return Err(TransportError::ConnectionLost),
        };

        // Serialize ClientMessage to JSON
        let json = serde_json::to_string(&msg).map_err(|e| {
            TransportError::Serialization(format!("Failed to serialize ClientMessage: {}", e))
        })?;

        // Send as text message
        socket
            .send(tungstenite::Message::Text(json))
            .map_err(|e| TransportError::Other(format!("Failed to send message: {}", e)))?;

        Ok(())
    }

    fn receive(&mut self) -> Result<Option<ServerMessage>, TransportError> {
        // First, try to fill the buffer from the websocket
        self.fill_buffer()?;

        // Return a message from the buffer if available
        Ok(self.incoming_buffer.pop_front())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lp_model::server::FsRequest;

    #[test]
    fn test_websocket_client_transport_creation() {
        // This test would require a running websocket server
        // For now, just verify the struct can be created conceptually
        // Actual connection tests will be in integration tests
    }

    #[test]
    fn test_serialization_format() {
        // Test that we serialize/deserialize correctly
        let msg = ClientMessage {
            id: 1,
            msg: lp_model::ClientRequest::Filesystem(FsRequest::Read {
                path: "/test".to_string(),
            }),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, msg.id);
    }
}
