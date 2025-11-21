use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct WorldConfig {
    pub map_path: PathBuf,
    pub spawn_multiplier: u16,
    pub respawn_base_spawn_rate_minutes: u8,
}

impl WorldConfig {
    pub fn new<P: AsRef<Path>>(map_path: P, spawn_multiplier: u16, respawn_base_spawn_rate_minutes: u8) -> Self {
        Self {
            map_path: map_path.as_ref().to_path_buf(),
            spawn_multiplier,
            respawn_base_spawn_rate_minutes,
        }
    }
}
