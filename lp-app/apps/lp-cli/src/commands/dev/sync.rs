//! File sync utilities
//!
//! TODO: Will be recreated in phase 6
//! This is a temporary stub to allow compilation.

use anyhow::Result;
use lp_shared::fs::{LpFs, fs_event::FsChange};

/// Sync a file change to the server
///
/// TODO: Will be properly implemented in phase 6
#[allow(dead_code)]
pub async fn sync_file_change(
    _client: &mut crate::commands::dev::async_client::AsyncLpClient,
    _change: FsChange,
    _project_uid: &str,
    _local_fs: &dyn LpFs,
) -> Result<()> {
    todo!("Will be implemented in phase 6")
}
