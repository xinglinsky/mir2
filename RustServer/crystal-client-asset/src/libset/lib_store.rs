//! Lib 文件存储和缓存

use crate::libset::LibId;
use crate::paths::AssetPaths;
use bevy::prelude::*;
use crystal_lib::{LibFile, LibError};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LibStoreError {
    #[error("lib error: {0}")]
    Lib(#[from] LibError),
    
    #[error("lib not found: {0:?}")]
    NotFound(LibId),
    
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Lib 文件存储
///
/// 管理 Lib 文件的加载和缓存
pub struct LibStore {
    paths: AssetPaths,
    libs: HashMap<LibId, LibFile>,
}

impl LibStore {
    /// 创建新的 Lib 存储
    ///
    /// # Arguments
    ///
    /// * `paths` - 资产路径管理器
    pub fn new(paths: AssetPaths) -> Self {
        Self {
            paths,
            libs: HashMap::new(),
        }
    }

    /// 加载 Lib 文件
    ///
    /// # Arguments
    ///
    /// * `lib_id` - Lib 文件 ID
    ///
    /// # Returns
    ///
    /// 成功返回 `LibFile`，失败返回错误
    pub fn load(&mut self, lib_id: LibId) -> Result<&LibFile, LibStoreError> {
        if self.libs.contains_key(&lib_id) {
            return Ok(self.libs.get(&lib_id).unwrap());
        }

        let path = self.paths.lib_path(lib_id.filename());
        let lib = LibFile::load(&path)?;
        self.libs.insert(lib_id, lib);
        Ok(self.libs.get(&lib_id).unwrap())
    }

    /// 获取已加载的 Lib 文件（不加载）
    ///
    /// # Arguments
    ///
    /// * `lib_id` - Lib 文件 ID
    ///
    /// # Returns
    ///
    /// 如果已加载则返回 `Some(&LibFile)`，否则返回 `None`
    pub fn get(&self, lib_id: &LibId) -> Option<&LibFile> {
        self.libs.get(lib_id)
    }

    /// 检查 Lib 文件是否已加载
    pub fn is_loaded(&self, lib_id: &LibId) -> bool {
        self.libs.contains_key(lib_id)
    }

    /// 清除所有缓存的 Lib 文件
    pub fn clear(&mut self) {
        self.libs.clear();
    }
}

