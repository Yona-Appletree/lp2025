//! Application-level runtime management

pub mod file_change;
pub mod lp_app;
pub mod platform;

pub use crate::api::messages::{MsgIn, MsgOut};
pub use file_change::{ChangeType, FileChange};
pub use lp_app::LpApp;
pub use platform::Platform;
