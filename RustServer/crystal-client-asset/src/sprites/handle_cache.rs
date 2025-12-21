//! Sprite Handle 缓存

use bevy::prelude::*;
use std::collections::HashMap;

/// Sprite Handle 缓存键
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpriteKey {
    pub lib_id: String,
    pub index: usize,
}

/// Sprite Handle 缓存
///
/// 缓存 Lib 图片到 Bevy Image Handle 的映射
pub struct SpriteHandleCache {
    cache: HashMap<SpriteKey, Handle<Image>>,
}

impl SpriteHandleCache {
    /// 创建新的缓存
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// 获取缓存的 Handle
    ///
    /// # Arguments
    ///
    /// * `lib_id` - Lib 文件标识
    /// * `index` - 图片索引
    ///
    /// # Returns
    ///
    /// 如果缓存中存在则返回 `Some(&Handle<Image>)`，否则返回 `None`
    pub fn get(&self, lib_id: &str, index: usize) -> Option<&Handle<Image>> {
        let key = SpriteKey {
            lib_id: lib_id.to_string(),
            index,
        };
        self.cache.get(&key)
    }

    /// 插入缓存的 Handle
    ///
    /// # Arguments
    ///
    /// * `lib_id` - Lib 文件标识
    /// * `index` - 图片索引
    /// * `handle` - Bevy Image Handle
    pub fn insert(&mut self, lib_id: &str, index: usize, handle: Handle<Image>) {
        let key = SpriteKey {
            lib_id: lib_id.to_string(),
            index,
        };
        self.cache.insert(key, handle);
    }

    /// 清除所有缓存
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

impl Default for SpriteHandleCache {
    fn default() -> Self {
        Self::new()
    }
}

