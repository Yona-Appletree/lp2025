//! Transport traits for client-server communication
//!
//! This module defines traits for pluggable transport implementations.
//! Messages are consumed (moved) on send, and receive is non-blocking.

pub mod client;
pub mod server;

pub use client::ClientTransport;
pub use server::ServerTransport;
// Re-export TransportError from lp-model for convenience
pub use lp_model::TransportError;

/// Transport-level message wrapper
///
/// This is a transport-layer abstraction that wraps serialized protocol messages.
/// Protocol messages (`lp_model::Message`) are serialized (typically to JSON) and
/// wrapped in this type for transport.
///
/// Different transport implementations may use different serialization formats
/// (JSON, binary, etc.), but they all use this wrapper type.
#[derive(Debug, Clone)]
pub struct Message {
    /// Serialized message payload (typically JSON bytes of `lp_model::Message`)
    pub payload: alloc::vec::Vec<u8>,
}

impl Message {
    /// Create a new message with the given payload
    pub fn new(payload: alloc::vec::Vec<u8>) -> Self {
        Self { payload }
    }
}
