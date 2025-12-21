//! 游戏场景根

use bevy::prelude::*;

/// 游戏场景根组件
#[derive(Component)]
pub struct GameSceneRoot;

/// 初始化游戏场景
pub fn setup_game_scene(
    mut commands: Commands,
) {
    commands.spawn(GameSceneRoot);
    
    // TODO: 调用 hud::setup_game_hud 创建 HUD
}

