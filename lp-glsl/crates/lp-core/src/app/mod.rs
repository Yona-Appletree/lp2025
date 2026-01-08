//! Application-level runtime management

pub mod file_change;
pub mod lp_app;
pub mod messages;
pub mod platform;

pub use file_change::{ChangeType, FileChange};
pub use lp_app::LpApp;
pub use messages::{MsgIn, MsgOut};
pub use platform::Platform;
