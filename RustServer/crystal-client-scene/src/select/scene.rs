//! 选角场景根

use bevy::prelude::*;

/// 选角场景根组件
#[derive(Component)]
pub struct SelectSceneRoot;

/// 初始化选角场景
pub fn setup_select_scene(
    mut commands: Commands,
) {
    commands.spawn(SelectSceneRoot);
    
    // TODO: 调用 ui::setup_select_ui 创建选角界面
}

