use std::path::PathBuf;

pub struct DevArgs {
    pub host: Option<String>,
    pub dir: Option<PathBuf>,
    pub push: bool,
}
