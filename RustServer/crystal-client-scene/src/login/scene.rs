//! 登录场景根

use bevy::prelude::*;
use super::ui::setup_login_ui;
use super::logic::LoginSceneState;

/// 登录场景根组件
#[derive(Component)]
pub struct LoginSceneRoot;

/// 初始化登录场景
pub fn setup_login_scene(
    mut commands: Commands,
) {
    // 创建场景根
    commands.spawn(LoginSceneRoot);
    
    // 初始化登录场景状态
    commands.insert_resource(LoginSceneState::default());
    
    // 创建登录界面 UI
    setup_login_ui(&mut commands);
}
