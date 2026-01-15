//! Server-side transport trait
//!
//! Defines the interface for server-side transport implementations.
//! Messages are consumed (moved) on send, and receive is non-blocking.

use crate::transport::Message;
use lp_model::TransportError;

/// Trait for server-side transport implementations
///
/// This trait provides a simple polling-based interface for sending and receiving
/// messages. Messages are consumed (moved) on send, and receive is non-blocking
/// (returns `None` if no message is available).
///
/// Separate from `ClientTransport` for clarity, even though the interface is
/// identical. This allows for different implementations or future extensions
/// specific to server-side use cases.
///
/// # Examples
///
/// ```rust,no_run
/// use lp_shared::transport::{ServerTransport, Message, TransportError};
///
/// struct MyTransport;
///
/// impl ServerTransport for MyTransport {
///     fn send(&mut self, msg: Message) -> Result<(), TransportError> {
///         // Send message
///         Ok(())
///     }
///
///     fn receive(&mut self) -> Result<Option<Message>, TransportError> {
///         // Receive message (non-blocking)
///         Ok(None)
///     }
/// }
/// ```
pub trait ServerTransport {
    /// Send a message (consumes the message)
    ///
    /// # Arguments
    ///
    /// * `msg` - The message to send (consumed/moved)
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the message was sent successfully
    /// * `Err(TransportError)` if sending failed
    fn send(&mut self, msg: Message) -> Result<(), TransportError>;

    /// Receive a message (non-blocking)
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Message))` if a message is available
    /// * `Ok(None)` if no message is available (non-blocking)
    /// * `Err(TransportError)` if receiving failed
    fn receive(&mut self) -> Result<Option<Message>, TransportError>;
}
