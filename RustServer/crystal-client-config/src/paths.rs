//! 路径管理
//!
//! 统一管理客户端相关的路径配置，包括数据目录、地图目录、音频目录等。

use std::path::{Path, PathBuf};

/// 获取默认配置文件路径
pub fn default_config_path() -> PathBuf {
    PathBuf::from("config/client.toml")
}

/// 数据目录路径
pub fn data_path(base: Option<&Path>) -> PathBuf {
    base.map(|p| p.join("Data"))
        .unwrap_or_else(|| PathBuf::from("./Data"))
}

/// 地图目录路径
pub fn map_path(base: Option<&Path>) -> PathBuf {
    base.map(|p| p.join("Map"))
        .unwrap_or_else(|| PathBuf::from("./Map"))
}

/// 音频目录路径
pub fn sound_path(base: Option<&Path>) -> PathBuf {
    base.map(|p| p.join("Sound"))
        .unwrap_or_else(|| PathBuf::from("./Sound"))
}

/// 额外数据目录路径
pub fn extra_data_path(base: Option<&Path>) -> PathBuf {
    data_path(base).join("Extra")
}

/// Shader 目录路径
pub fn shaders_path(base: Option<&Path>) -> PathBuf {
    data_path(base).join("Shaders")
}

/// 怪物资源目录路径
pub fn monster_path(base: Option<&Path>) -> PathBuf {
    data_path(base).join("Monster")
}

/// NPC 资源目录路径
pub fn npc_path(base: Option<&Path>) -> PathBuf {
    data_path(base).join("NPC")
}

/// 用户数据目录路径
pub fn user_data_path(base: Option<&Path>) -> PathBuf {
    data_path(base).join("UserData")
}

/// 解析相对路径或绝对路径
///
/// # Arguments
///
/// * `path` - 路径字符串（可以是相对路径或绝对路径）
/// * `base` - 基础路径（用于解析相对路径）
///
/// # Returns
///
/// 解析后的路径
pub fn resolve_path(path: &str, base: Option<&Path>) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        base.map(|b| b.join(&p))
            .unwrap_or_else(|| p)
    }
}

