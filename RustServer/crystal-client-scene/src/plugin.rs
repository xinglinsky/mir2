//! 场景系统插件

use bevy::prelude::*;
use super::state::SceneState;
use super::transitions::{handle_scene_transitions, on_scene_enter, on_scene_exit, SceneTransition};

/// 场景系统插件
///
/// 管理场景状态和场景切换
pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        // 初始化场景状态
        app.init_state::<SceneState>();
        
        // 注册事件
        app.add_event::<SceneTransition>();
        
        // 添加场景切换系统
        app.add_systems(
            Update,
            handle_scene_transitions,
        );
        
        // 添加场景进入/退出系统
        app.add_systems(
            OnEnter(SceneState::Launcher),
            on_scene_enter,
        );
        app.add_systems(
            OnEnter(SceneState::Login),
            on_scene_enter,
        );
        app.add_systems(
            OnEnter(SceneState::Select),
            on_scene_enter,
        );
        app.add_systems(
            OnEnter(SceneState::Game),
            on_scene_enter,
        );
        
        app.add_systems(
            OnExit(SceneState::Launcher),
            on_scene_exit,
        );
        app.add_systems(
            OnExit(SceneState::Login),
            on_scene_exit,
        );
        app.add_systems(
            OnExit(SceneState::Select),
            on_scene_exit,
        );
        app.add_systems(
            OnExit(SceneState::Game),
            on_scene_exit,
        );
    }
}

