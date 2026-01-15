//! Client-side transport trait
//!
//! Defines the interface for client-side transport implementations.
//! Messages are consumed (moved) on send, and receive is non-blocking.

use crate::transport::Message;
use lp_model::TransportError;

/// Trait for client-side transport implementations
///
/// This trait provides a simple polling-based interface for sending and receiving
/// messages. Messages are consumed (moved) on send, and receive is non-blocking
/// (returns `None` if no message is available).
///
/// # Examples
///
/// ```rust,no_run
/// use lp_shared::transport::{ClientTransport, Message, TransportError};
///
/// struct MyTransport;
///
/// impl ClientTransport for MyTransport {
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
pub trait ClientTransport {
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
