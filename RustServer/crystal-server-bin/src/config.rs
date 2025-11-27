use std::fs;
use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub listen_addr: SocketAddr,
    pub accounts_db_path: PathBuf,
    pub server_mirdb_path: PathBuf,
    pub maps_path: PathBuf,
    #[serde(default = "default_drops_path")]
    pub drops_path: PathBuf,
    #[serde(default = "default_recipes_path")]
    pub recipes_path: PathBuf,
    #[serde(default = "default_spawn_multiplier")]
    pub spawn_multiplier: u16,
    #[serde(default = "default_respawn_base_spawn_rate_minutes")]
    pub respawn_base_spawn_rate_minutes: u8,
    #[serde(default = "default_drop_rate")]
    pub drop_rate: f32,
    #[serde(default = "default_teleport_to_npc_cost")]
    pub teleport_to_npc_cost: i32,
    #[serde(default = "default_game_master_effect")]
    pub game_master_effect: bool,
    #[serde(default = "default_safe_zone_border")]
    pub safe_zone_border: bool,
    #[serde(default = "default_safe_zone_healing")]
    pub safe_zone_healing: bool,
    #[serde(default)]
    pub log_filter: Option<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    10_000
}

fn default_spawn_multiplier() -> u16 {
    1
}

fn default_respawn_base_spawn_rate_minutes() -> u8 {
    20
}

fn default_drops_path() -> PathBuf {
    PathBuf::from("./Envir/Drops")
}

fn default_drop_rate() -> f32 {
    1.0
}

fn default_teleport_to_npc_cost() -> i32 {
    3000
}

fn default_recipes_path() -> PathBuf {
    PathBuf::from("./Envir/Recipe")
}

fn default_game_master_effect() -> bool {
    true
}

fn default_safe_zone_border() -> bool {
    true
}

fn default_safe_zone_healing() -> bool {
    false
}

pub fn load_server_config<P: AsRef<Path>>(path: P) -> io::Result<ServerConfig> {
    let text = fs::read_to_string(path.as_ref())?;
    let cfg: ServerConfig = toml::from_str(&text).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("failed to parse server config: {e}"))
    })?;
    Ok(cfg)
}
