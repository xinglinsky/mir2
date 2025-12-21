//! 资产路径管理
//!
//! 根据配置解析各种资源路径（Data/Map/Sound 等）

use crystal_client_config::paths_module as config_paths;
use std::path::PathBuf;

/// 资产路径管理器
///
/// 统一管理客户端资源的路径解析
pub struct AssetPaths {
    base_path: Option<PathBuf>,
}

impl AssetPaths {
    /// 创建新的资产路径管理器
    ///
    /// # Arguments
    ///
    /// * `base_path` - 基础路径（可选，用于解析相对路径）
    pub fn new(base_path: Option<PathBuf>) -> Self {
        Self { base_path }
    }

    /// 获取数据目录路径
    pub fn data_path(&self) -> PathBuf {
        config_paths::data_path(self.base_path.as_deref())
    }

    /// 获取地图目录路径
    pub fn map_path(&self) -> PathBuf {
        config_paths::map_path(self.base_path.as_deref())
    }

    /// 获取音频目录路径
    pub fn sound_path(&self) -> PathBuf {
        config_paths::sound_path(self.base_path.as_deref())
    }

    /// 获取额外数据目录路径
    pub fn extra_data_path(&self) -> PathBuf {
        config_paths::extra_data_path(self.base_path.as_deref())
    }

    /// 获取 Shader 目录路径
    pub fn shaders_path(&self) -> PathBuf {
        config_paths::shaders_path(self.base_path.as_deref())
    }

    /// 获取怪物资源目录路径
    pub fn monster_path(&self) -> PathBuf {
        config_paths::monster_path(self.base_path.as_deref())
    }

    /// 获取 NPC 资源目录路径
    pub fn npc_path(&self) -> PathBuf {
        config_paths::npc_path(self.base_path.as_deref())
    }

    /// 获取用户数据目录路径
    pub fn user_data_path(&self) -> PathBuf {
        config_paths::user_data_path(self.base_path.as_deref())
    }

    /// 解析 Lib 文件路径
    ///
    /// # Arguments
    ///
    /// * `lib_name` - Lib 文件名（如 "ChrSel.Lib"）
    pub fn lib_path(&self, lib_name: &str) -> PathBuf {
        self.data_path().join(lib_name)
    }

    /// 解析地图文件路径
    ///
    /// # Arguments
    ///
    /// * `map_index` - 地图索引（0-400）
    pub fn map_file_path(&self, map_index: u32) -> PathBuf {
        self.map_path().join(format!("Map{:03}.map", map_index))
    }
}

