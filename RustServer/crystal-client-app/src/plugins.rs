//! 客户端插件系统

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;

/// 客户端插件集合
///
/// 统一管理所有客户端相关的 Bevy 插件
#[derive(Default)]
pub struct ClientPlugins;

impl PluginGroup for ClientPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        bevy::app::PluginGroupBuilder::start::<Self>()
            // 性能诊断插件
            .add(FrameTimeDiagnosticsPlugin::default())
            // UI 插件
            .add(crystal_client_ui::UiPlugin)
            // 场景插件
            .add(crystal_client_scene::ScenePlugin)
            // TODO: 后续添加其他插件：
            // - NetPlugin (网络插件)
            // - RenderPlugin (渲染插件)
            // - AudioPlugin (音频插件)
    }
}

