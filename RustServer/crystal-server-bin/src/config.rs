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
    #[serde(default)]
    pub log_filter: Option<String>,
}

pub fn load_server_config<P: AsRef<Path>>(path: P) -> io::Result<ServerConfig> {
    let text = fs::read_to_string(path.as_ref())?;
    let cfg: ServerConfig = toml::from_str(&text).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("failed to parse server config: {e}"))
    })?;
    Ok(cfg)
}
