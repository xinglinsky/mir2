//! UI 主题系统
//!
//! 提供颜色定义、尺寸度量、按钮皮肤等主题相关功能。

pub mod colors;
pub mod metrics;
pub mod mir2_skin;

use bevy::prelude::*;

pub use colors::*;
pub use metrics::*;
pub use mir2_skin::*;

/// UI 主题资源
#[derive(Resource, Default)]
pub struct UiTheme {
    // TODO: 添加主题相关字段
}

