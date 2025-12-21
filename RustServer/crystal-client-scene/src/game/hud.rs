//! 游戏 HUD（Head-Up Display）

use bevy::prelude::*;

/// HUD 根组件
#[derive(Component)]
pub struct GameHudRoot;

/// 设置游戏 HUD
pub fn setup_game_hud(
    mut commands: Commands,
) {
    commands.spawn(GameHudRoot);
    
    // TODO: 创建 HUD 元素
    // - HP/MP 条
    // - 技能栏
    // - 小地图
    // - 聊天窗口
    // - 其他 UI 元素
}

