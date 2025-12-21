//! 配置文件的加载和保存

use crate::model::ClientConfig;
use std::fs;
use std::io;
use std::path::Path;

/// 加载配置或使用默认值
///
/// # Arguments
///
/// * `path` - 配置文件路径
///
/// # Returns
///
/// 加载的配置，如果文件不存在则返回默认配置
pub fn load_or_default(path: impl AsRef<Path>) -> io::Result<ClientConfig> {
    let path = path.as_ref();
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(ClientConfig::default()),
        Err(e) => return Err(e),
    };

    let s = String::from_utf8(bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    toml::from_str(&s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// 保存配置到文件
///
/// # Arguments
///
/// * `config` - 要保存的配置
/// * `path` - 配置文件路径
pub fn save(config: &ClientConfig, path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let s = toml::to_string_pretty(config)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, s)
}

