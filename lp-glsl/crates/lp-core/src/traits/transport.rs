//! Message transport abstraction trait

use crate::error::Error;

/// Platform-agnostic message transport trait
///
/// This trait abstracts sending and receiving JSON messages, independent of the
/// underlying transport mechanism (serial port, stdio, HTTP, UDP, etc.).
pub trait Transport {
    /// Send a JSON message string
    ///
    /// The message should be a complete JSON string. The implementation is responsible
    /// for adding any framing (e.g., newline termination) if needed.
    fn send_message(&mut self, message: &str) -> Result<(), Error>;

    /// Receive a JSON message string
    ///
    /// This is a blocking call that reads until a complete message is received.
    /// The implementation should handle buffering and message framing (e.g., reading
    /// until a newline is encountered).
    fn receive_message(&mut self) -> Result<alloc::string::String, Error>;
}
