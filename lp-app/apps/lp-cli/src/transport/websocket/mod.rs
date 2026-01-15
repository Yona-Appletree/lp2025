//! WebSocket transport implementations
//!
//! Provides WebSocket-based transport for both client and server.

pub mod client;
pub mod server;

#[allow(dead_code)] // Will be used in phase 8
pub use client::WebSocketClientTransport;
