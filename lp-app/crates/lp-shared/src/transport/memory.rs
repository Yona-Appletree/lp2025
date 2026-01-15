//! Local in-memory transport for testing
//!
//! Provides a simple single-threaded in-memory transport that serializes/deserializes messages
//! to/from JSON to ensure the message protocol works correctly.
//!
//! Uses Rc<RefCell> for single-threaded interior mutability. This transport is explicitly
//! single-threaded and does not require std.

extern crate alloc;

use crate::transport::{ClientTransport, ServerTransport};
use alloc::{collections::VecDeque, format, rc::Rc, string::ToString, vec::Vec};
use core::cell::RefCell;
use lp_model::{ClientMessage, ServerMessage, TransportError};
use serde_json;

/// Shared state for bidirectional communication
struct SharedState {
    /// Queue of serialized client messages (client -> server)
    client_to_server: VecDeque<Vec<u8>>,
    /// Queue of serialized server messages (server -> client)
    server_to_client: VecDeque<Vec<u8>>,
}

/// Local in-memory transport implementation
///
/// Uses shared queues for bidirectional communication. Messages are serialized
/// to/from JSON to ensure the message protocol serializes correctly.
///
/// This transport is explicitly single-threaded and uses Rc<RefCell> for
/// interior mutability. It does not require std.
pub struct LocalMemoryTransport {
    /// Shared state (wrapped in RefCell for interior mutability)
    state: Rc<RefCell<SharedState>>,
    /// Whether this is the client side (true) or server side (false)
    is_client: bool,
}

impl LocalMemoryTransport {
    /// Create a pair of transports (client and server)
    ///
    /// Returns `(client_transport, server_transport)` that can communicate
    /// with each other through in-memory queues.
    pub fn new_pair() -> (Self, Self) {
        let state = Rc::new(RefCell::new(SharedState {
            client_to_server: VecDeque::new(),
            server_to_client: VecDeque::new(),
        }));

        let client = LocalMemoryTransport {
            state: Rc::clone(&state),
            is_client: true,
        };

        let server = LocalMemoryTransport {
            state,
            is_client: false,
        };

        (client, server)
    }
}

impl ClientTransport for LocalMemoryTransport {
    fn send(&mut self, msg: ClientMessage) -> Result<(), TransportError> {
        if !self.is_client {
            return Err(TransportError::Other(
                "Cannot use server transport as client transport".to_string(),
            ));
        }

        // Serialize the message
        let payload = serde_json::to_vec(&msg).map_err(|e| {
            TransportError::Serialization(format!("Failed to serialize ClientMessage: {}", e))
        })?;

        let mut state = self.state.borrow_mut();
        state.client_to_server.push_back(payload);
        Ok(())
    }

    fn receive(&mut self) -> Result<Option<ServerMessage>, TransportError> {
        if !self.is_client {
            return Err(TransportError::Other(
                "Cannot use server transport as client transport".to_string(),
            ));
        }

        let mut state = self.state.borrow_mut();
        Ok(state
            .server_to_client
            .pop_front()
            .map(|payload| {
                serde_json::from_slice(&payload).map_err(|e| {
                    TransportError::Deserialization(format!(
                        "Failed to deserialize ServerMessage: {}",
                        e
                    ))
                })
            })
            .transpose()?)
    }
}

impl ServerTransport for LocalMemoryTransport {
    fn send(&mut self, msg: ServerMessage) -> Result<(), TransportError> {
        if self.is_client {
            return Err(TransportError::Other(
                "Cannot use client transport as server transport".to_string(),
            ));
        }

        // Serialize the message
        let payload = serde_json::to_vec(&msg).map_err(|e| {
            TransportError::Serialization(format!("Failed to serialize ServerMessage: {}", e))
        })?;

        let mut state = self.state.borrow_mut();
        state.server_to_client.push_back(payload);
        Ok(())
    }

    fn receive(&mut self) -> Result<Option<ClientMessage>, TransportError> {
        if self.is_client {
            return Err(TransportError::Other(
                "Cannot use client transport as server transport".to_string(),
            ));
        }

        let mut state = self.state.borrow_mut();
        Ok(state
            .client_to_server
            .pop_front()
            .map(|payload| {
                serde_json::from_slice(&payload).map_err(|e| {
                    TransportError::Deserialization(format!(
                        "Failed to deserialize ClientMessage: {}",
                        e
                    ))
                })
            })
            .transpose()?)
    }
}
