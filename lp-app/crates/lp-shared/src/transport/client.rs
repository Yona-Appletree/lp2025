//! Client-side transport trait
//!
//! Defines the interface for client-side transport implementations.
//! Messages are consumed (moved) on send, and receive is non-blocking.
//!
//! The transport handles serialization/deserialization internally.

use lp_model::{ClientMessage, ServerMessage, TransportError};

/// Trait for client-side transport implementations
///
/// This trait provides a simple polling-based interface for sending and receiving
/// messages. Messages are consumed (moved) on send, and receive is non-blocking
/// (returns `None` if no message is available).
///
/// The transport handles serialization/deserialization internally.
///
/// # Examples
///
/// ```rust,no_run
/// use lp_shared::transport::ClientTransport;
/// use lp_model::{ClientMessage, ServerMessage, TransportError};
///
/// struct MyTransport;
///
/// impl ClientTransport for MyTransport {
///     fn send(&mut self, msg: ClientMessage) -> Result<(), TransportError> {
///         // Send message (transport handles serialization)
///         Ok(())
///     }
///
///     fn receive(&mut self) -> Result<Option<ServerMessage>, TransportError> {
///         // Receive message (transport handles deserialization)
///         Ok(None)
///     }
/// }
/// ```
pub trait ClientTransport {
    /// Send a client message (consumes the message)
    ///
    /// The transport handles serialization internally.
    ///
    /// # Arguments
    ///
    /// * `msg` - The client message to send (consumed/moved)
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the message was sent successfully
    /// * `Err(TransportError)` if sending failed
    fn send(&mut self, msg: ClientMessage) -> Result<(), TransportError>;

    /// Receive a server message (non-blocking)
    ///
    /// The transport handles deserialization internally.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(ServerMessage))` if a message is available
    /// * `Ok(None)` if no message is available (non-blocking)
    /// * `Err(TransportError)` if receiving failed
    fn receive(&mut self) -> Result<Option<ServerMessage>, TransportError>;
}
