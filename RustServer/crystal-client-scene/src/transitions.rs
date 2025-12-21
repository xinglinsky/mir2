//! 场景切换逻辑

use bevy::prelude::*;
use super::state::SceneState;
use super::login::scene::setup_login_scene;
use super::select::scene::setup_select_scene;
use super::game::scene::setup_game_scene;

/// 场景切换事件
#[derive(Event)]
pub struct SceneTransition {
    /// 目标场景
    pub target: SceneState,
}

/// 处理场景切换系统
pub fn handle_scene_transitions(
    mut transition_events: EventReader<SceneTransition>,
    mut next_state: ResMut<NextState<SceneState>>,
) {
    for event in transition_events.read() {
        next_state.set(event.target);
    }
}

/// 场景进入系统（on_enter）
///
/// 在场景状态改变时调用，用于初始化场景
pub fn on_scene_enter(
    mut commands: Commands,
    state: Res<State<SceneState>>,
    mut transition_events: EventWriter<SceneTransition>,
) {
    match state.get() {
        SceneState::Launcher => {
            // TODO: 初始化启动器场景
        }
        SceneState::Login => {
            setup_login_scene(&mut commands);
        }
        SceneState::Select => {
            setup_select_scene(&mut commands);
        }
        SceneState::Game => {
            setup_game_scene(&mut commands);
        }
    }
}

/// 场景退出系统（on_exit）
///
/// 在场景状态改变前调用，用于清理场景
pub fn on_scene_exit(
    mut commands: Commands,
    state: Res<State<SceneState>>,
    query: Query<Entity, (With<super::login::scene::LoginSceneRoot>, Or<(With<super::select::scene::SelectSceneRoot>, With<super::game::scene::GameSceneRoot>)>)>,
) {
    match state.get() {
        SceneState::Launcher => {
            // TODO: 清理启动器场景
        }
        SceneState::Login => {
            // 清理登录场景实体
            for entity in query.iter() {
                commands.entity(entity).despawn_recursive();
            }
        }
        SceneState::Select => {
            // 清理选角场景实体
            for entity in query.iter() {
                commands.entity(entity).despawn_recursive();
            }
        }
        SceneState::Game => {
            // 清理游戏场景实体
            for entity in query.iter() {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}
