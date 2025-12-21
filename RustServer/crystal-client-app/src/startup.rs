//! 启动系统

use bevy::prelude::*;

/// 启动系统集合
///
/// 在应用启动时执行的系统，用于：
/// - 加载配置
/// - 预热资产
/// - 初始化全局资源
pub fn startup_systems(mut commands: Commands) {
    // 初始化相机（2D 相机，用于 UI 渲染）
    commands.spawn(Camera2dBundle::default());
    
    // TODO: 后续实现
    // - 加载客户端配置（从文件或环境变量）
    // - 预热常用资产（Lib 文件、字体等）
    // - 初始化全局资源（Time、Diagnostics 等）
}

