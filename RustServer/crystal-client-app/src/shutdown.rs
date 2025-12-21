//! 关闭系统

use bevy::app::AppExit;
use bevy::prelude::*;

/// 关闭系统集合
///
/// 在应用关闭时执行的系统，用于：
/// - 优雅退出
/// - 日志 flush
/// - 资源清理
pub fn shutdown_systems(
    mut exit: EventWriter<AppExit>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // ESC 键退出
    if keyboard_input.just_pressed(KeyCode::Escape) {
        exit.send(AppExit::Success);
    }
    
    // TODO: 后续实现
    // - 保存配置（如果配置被修改）
    // - 关闭网络连接（通过事件通知网络系统）
    // - 清理资源
    // - 日志 flush（通过 tracing 子系统）
}

