//! Crystal Client 配置管理
//!
//! 本 crate 提供客户端配置的加载、保存和管理功能。

mod model;
mod io;
mod paths;

pub use model::*;
pub use io::{load_or_default, save};
pub use paths::*;

// 导出 paths 模块供其他 crate 使用
pub mod paths_module {
    pub use super::paths::*;
}

// 向后兼容：保留旧的简化接口
impl ClientConfig {
    /// 获取默认配置文件路径
    pub fn default_path() -> std::path::PathBuf {
        paths::default_config_path()
    }

    /// 加载配置或使用默认值
    pub fn load_or_default(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        io::load_or_default(path)
    }

    /// 保存配置
    pub fn save(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        io::save(self, path)
    }
}
