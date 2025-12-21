//! 布局锚点系统
//!
//! 提供 C# Point/Size 到 Bevy absolute layout 的辅助工具。

use bevy::prelude::*;

/// 2D 点（对应 C# Point）
#[derive(Clone, Copy, Debug, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// 2D 尺寸（对应 C# Size）
#[derive(Clone, Copy, Debug, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// 从 Point 和 Size 创建 Bevy Style
pub fn create_style_from_point_size(position: Point, size: Size) -> Style {
    Style {
        position_type: PositionType::Absolute,
        left: Val::Px(position.x),
        top: Val::Px(position.y),
        width: Val::Px(size.width),
        height: Val::Px(size.height),
        ..default()
    }
}

/// 从 C# 风格的坐标创建 Bevy Style
///
/// # 参数
/// - `x`, `y`: 位置坐标
/// - `width`, `height`: 尺寸
pub fn create_style(x: f32, y: f32, width: f32, height: f32) -> Style {
    create_style_from_point_size(Point::new(x, y), Size::new(width, height))
}

