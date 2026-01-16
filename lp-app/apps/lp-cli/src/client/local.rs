//! Async local transport for in-memory communication
//!
//! Provides tokio channel-based transport for communication between server and client
//! running in the same process. Uses unbounded channels for simplicity.

use lp_model::{ClientMessage, ServerMessage, TransportError};
use lp_shared::transport::{ClientTransport, ServerTransport};
use tokio::sync::mpsc;

/// Async local client transport
///
/// Uses tokio channels to send client messages and receive server messages.
/// Provides non-blocking receive via `try_recv()`.
pub struct AsyncLocalClientTransport {
    /// Sender for client messages (client -> server)
    client_tx: mpsc::UnboundedSender<ClientMessage>,
    /// Receiver for server messages (server -> client)
    client_rx: mpsc::UnboundedReceiver<ServerMessage>,
}

impl AsyncLocalClientTransport {
    /// Create a new async local client transport
    ///
    /// # Arguments
    ///
    /// * `client_tx` - Sender for client messages
    /// * `client_rx` - Receiver for server messages
    pub fn new(
        client_tx: mpsc::UnboundedSender<ClientMessage>,
        client_rx: mpsc::UnboundedReceiver<ServerMessage>,
    ) -> Self {
        Self {
            client_tx,
            client_rx,
        }
    }
}

impl ClientTransport for AsyncLocalClientTransport {
    fn send(&mut self, msg: ClientMessage) -> Result<(), TransportError> {
        self.client_tx
            .send(msg)
            .map_err(|_| TransportError::ConnectionLost)
    }

    fn receive(&mut self) -> Result<Option<ServerMessage>, TransportError> {
        match self.client_rx.try_recv() {
            Ok(msg) => Ok(Some(msg)),
            Err(mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(mpsc::error::TryRecvError::Disconnected) => Err(TransportError::ConnectionLost),
        }
    }
}

/// Async local server transport
///
/// Uses tokio channels to send server messages and receive client messages.
/// Provides non-blocking receive via `try_recv()`.
pub struct AsyncLocalServerTransport {
    /// Sender for server messages (server -> client)
    server_tx: mpsc::UnboundedSender<ServerMessage>,
    /// Receiver for client messages (client -> server)
    server_rx: mpsc::UnboundedReceiver<ClientMessage>,
}

impl AsyncLocalServerTransport {
    /// Create a new async local server transport
    ///
    /// # Arguments
    ///
    /// * `server_tx` - Sender for server messages
    /// * `server_rx` - Receiver for client messages
    pub fn new(
        server_tx: mpsc::UnboundedSender<ServerMessage>,
        server_rx: mpsc::UnboundedReceiver<ClientMessage>,
    ) -> Self {
        Self {
            server_tx,
            server_rx,
        }
    }
}

impl ServerTransport for AsyncLocalServerTransport {
    fn send(&mut self, msg: ServerMessage) -> Result<(), TransportError> {
        self.server_tx
            .send(msg)
            .map_err(|_| TransportError::ConnectionLost)
    }

    fn receive(&mut self) -> Result<Option<ClientMessage>, TransportError> {
        match self.server_rx.try_recv() {
            Ok(msg) => Ok(Some(msg)),
            Err(mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(mpsc::error::TryRecvError::Disconnected) => Err(TransportError::ConnectionLost),
        }
    }
}

/// Create a pair of connected local transports
///
/// Returns a client transport and server transport that are connected via
/// tokio channels. Messages sent via the client transport will be received
/// by the server transport, and vice versa.
pub fn create_local_transport_pair() -> (AsyncLocalClientTransport, AsyncLocalServerTransport) {
    let (client_tx, server_rx) = mpsc::unbounded_channel();
    let (server_tx, client_rx) = mpsc::unbounded_channel();

    (
        AsyncLocalClientTransport::new(client_tx, client_rx),
        AsyncLocalServerTransport::new(server_tx, server_rx),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use lp_model::{ClientRequest, ServerResponse};

    #[test]
    fn test_create_transport_pair() {
        let (_client_transport, _server_transport) = create_local_transport_pair();
        // Just verify they can be created
    }

    #[test]
    fn test_client_send_receive() {
        let (mut client_transport, mut server_transport) = create_local_transport_pair();

        // Send message from client
        let client_msg = ClientMessage {
            id: 1,
            msg: ClientRequest::ListAvailableProjects,
        };
        client_transport.send(client_msg.clone()).unwrap();

        // Receive on server
        let received = server_transport.receive().unwrap();
        assert!(received.is_some());
        let received_msg = received.unwrap();
        assert_eq!(received_msg.id, 1);

        // Send response from server
        let server_msg = ServerMessage {
            id: 1,
            msg: ServerResponse::ListAvailableProjects { projects: vec![] },
        };
        server_transport.send(server_msg).unwrap();

        // Receive on client
        let received = client_transport.receive().unwrap();
        assert!(received.is_some());
        let received_msg = received.unwrap();
        assert_eq!(received_msg.id, 1);
    }

    #[test]
    fn test_non_blocking_receive() {
        let (mut client_transport, _server_transport) = create_local_transport_pair();

        // Receive when empty should return None
        let result = client_transport.receive().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_server_send_receive() {
        let (mut client_transport, mut server_transport) = create_local_transport_pair();

        // Send message from server
        let server_msg = ServerMessage {
            id: 2,
            msg: ServerResponse::ListAvailableProjects { projects: vec![] },
        };
        server_transport.send(server_msg).unwrap();

        // Receive on client
        let received = client_transport.receive().unwrap();
        assert!(received.is_some());
        let received_msg = received.unwrap();
        assert_eq!(received_msg.id, 2);
    }

    #[test]
    fn test_multiple_messages() {
        let (mut client_transport, mut server_transport) = create_local_transport_pair();

        // Send multiple messages from client
        for i in 1..=5 {
            let client_msg = ClientMessage {
                id: i,
                msg: ClientRequest::ListAvailableProjects,
            };
            client_transport.send(client_msg).unwrap();
        }

        // Receive all messages on server
        for i in 1..=5 {
            let received = server_transport.receive().unwrap();
            assert!(received.is_some());
            assert_eq!(received.unwrap().id, i);
        }

        // Should be empty now
        let result = server_transport.receive().unwrap();
        assert!(result.is_none());
    }
}
