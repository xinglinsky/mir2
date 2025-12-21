//! App 构建器

use bevy::prelude::*;
use super::plugins::ClientPlugins;
use super::resources::RuntimeConfig;
use super::startup::startup_systems;
use super::shutdown::shutdown_systems;

/// 构建基础客户端 App
///
/// 返回一个配置了默认插件和基础系统的 App。
/// 调用者可以在此基础上添加自己的插件、资源和系统。
///
/// # Returns
///
/// 配置好的 Bevy App
pub fn build_app() -> App {
    let mut app = App::new();

    // 添加默认插件（窗口、渲染、输入等）
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Crystal M2".to_string(),
            resolution: (1024.0, 768.0).into(),
            resizable: false,
            ..default()
        }),
        ..default()
    }));

    // 添加客户端插件
    app.add_plugins(ClientPlugins::default());

    // 添加启动系统
    app.add_systems(Startup, startup_systems);

    // 添加关闭系统
    app.add_systems(Update, shutdown_systems);

    app
}

/// 构建带运行时配置的客户端 App
///
/// # Arguments
///
/// * `cfg` - 运行时配置
///
/// # Returns
///
/// 配置好的 Bevy App
pub fn build_app_with_config(cfg: RuntimeConfig) -> App {
    let mut app = build_app();
    app.insert_resource(cfg);
    app
}

