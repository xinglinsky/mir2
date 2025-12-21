//! Crystal Client 资产管理系统
//!
//! 本 crate 负责客户端资源的加载、缓存和管理，包括：
//! - Lib 文件集合管理
//! - Sprite 缓存
//! - 字体加载
//! - 地图资源索引
//! - 音频清单

mod paths;
mod libset;
mod sprites;

pub use paths::*;
pub use libset::*;
pub use sprites::*;

// 导出 libset 模块供其他 crate 使用
pub mod libset_module {
    pub use super::libset::*;
}

