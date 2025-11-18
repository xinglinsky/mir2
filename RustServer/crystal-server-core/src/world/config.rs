use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct WorldConfig {
    pub map_path: PathBuf,
}

impl WorldConfig {
    pub fn new<P: AsRef<Path>>(map_path: P) -> Self {
        Self {
            map_path: map_path.as_ref().to_path_buf(),
        }
    }
}
