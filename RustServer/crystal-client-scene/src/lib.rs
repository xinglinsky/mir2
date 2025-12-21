//! Crystal Client 场景系统
//!
//! 本 crate 负责场景状态管理和场景切换，包括：
//! - 场景状态机
//! - 场景切换逻辑
//! - 登录/选角/游戏场景
//! - 对话框系统

mod plugin;
mod state;
mod transitions;

pub use plugin::ScenePlugin;
pub use state::SceneState;

// 场景模块
pub mod login;
pub mod select;
pub mod game;
pub mod dialogs;

