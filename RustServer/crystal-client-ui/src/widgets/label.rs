//! Label Widget
//!
//! 简单的文本标签组件。

use bevy::prelude::*;
use crate::theme::colors::UiColors;
use crate::theme::metrics::UiMetrics;

/// 创建标签实体
pub fn spawn_label(
    commands: &mut Commands,
    text: impl Into<String>,
    position: (f32, f32),
    font_size: Option<f32>,
    color: Option<Color>,
) -> Entity {
    commands
        .spawn((
            TextBundle::from_section(
                text,
                TextStyle {
                    font_size: font_size.unwrap_or(UiMetrics::DEFAULT_FONT_SIZE),
                    color: color.unwrap_or(UiColors::WHITE),
                    ..default()
                },
            )
            .with_style(Style {
                position_type: PositionType::Absolute,
                left: Val::Px(position.0),
                top: Val::Px(position.1),
                ..default()
            }),
        ))
        .id()
}

