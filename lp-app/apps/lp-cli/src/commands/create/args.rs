use std::path::PathBuf;

#[allow(dead_code)] // Fields will be used in phase 6
pub struct CreateArgs {
    pub dir: PathBuf,
    pub name: Option<String>,
    pub uid: Option<String>,
}
