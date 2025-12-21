//! TextBox Widget
//!
//! 黑底文本框，带边框和光标。

use bevy::prelude::*;
use crate::input::text_edit::TextInput;
use crate::input::focus::Focusable;
use crate::theme::colors::UiColors;
use crate::theme::metrics::UiMetrics;

/// 创建文本框实体
pub fn spawn_textbox(
    commands: &mut Commands,
    position: (f32, f32),
    width: f32,
    height: Option<f32>,
) -> Entity {
    let height = height.unwrap_or(UiMetrics::TEXTBOX_HEIGHT);
    
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(position.0),
                    top: Val::Px(position.1),
                    width: Val::Px(width),
                    height: Val::Px(height),
                    border: UiRect::all(Val::Px(UiMetrics::TEXTBOX_BORDER_WIDTH)),
                    ..default()
                },
                background_color: BackgroundColor(UiColors::TEXTBOX_BG),
                border_color: BorderColor(UiColors::TEXTBOX_BORDER),
                ..default()
            },
            TextInput::default(),
            Focusable,
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "",
                TextStyle {
                    font_size: UiMetrics::DEFAULT_FONT_SIZE,
                    color: UiColors::TEXTBOX_TEXT,
                    ..default()
                },
            ));
        })
        .id()
}

