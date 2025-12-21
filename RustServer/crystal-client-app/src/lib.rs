//! Crystal Client App - Bevy App 组装与初始化
//!
//! 本 crate 负责构建和配置 Bevy App，包括：
//! - App 构建器
//! - 插件系统
//! - 全局资源管理
//! - 启动/关闭系统

mod app_builder;
mod plugins;
mod resources;
mod startup;
mod shutdown;

pub use app_builder::{build_app, build_app_with_config};
pub use plugins::ClientPlugins;
pub use resources::{LoginUiConfig, LibTestConfig, RuntimeConfig};
pub use startup::startup_systems;
pub use shutdown::shutdown_systems;

/// 运行客户端应用
///
/// # Arguments
///
/// * `cfg` - 运行时配置
pub fn run(cfg: RuntimeConfig) {
    let mut app = build_app_with_config(cfg);
    app.run();
}

