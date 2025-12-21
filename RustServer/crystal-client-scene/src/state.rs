//! 场景状态定义

use bevy::prelude::*;

/// 场景状态
///
/// 表示客户端当前所在的场景
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SceneState {
    /// 启动器场景（可选）
    #[default]
    Launcher,
    
    /// 登录场景
    Login,
    
    /// 选角场景
    Select,
    
    /// 游戏场景
    Game,
}

