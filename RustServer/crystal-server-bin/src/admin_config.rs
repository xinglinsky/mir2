use std::fs;
use std::io;
use std::net::SocketAddr;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AdminConfig {
    pub admin_listen_addr: SocketAddr,
    pub admin_token: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub admin_trusted_ip: Option<String>,
}

pub fn load_admin_config<P: AsRef<Path>>(path: P) -> io::Result<AdminConfig> {
    let text = fs::read_to_string(path.as_ref())?;
    let cfg: AdminConfig = toml::from_str(&text).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("failed to parse admin config: {e}"))
    })?;
    Ok(cfg)
}
