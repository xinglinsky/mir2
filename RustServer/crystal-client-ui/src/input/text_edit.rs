//! 文本编辑组件
//!
//! 提供文本输入、过滤、光标、选择等功能。

use bevy::prelude::*;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input::keyboard::ReceivedCharacter;
use crate::input::focus::{Focusable, FocusState};

/// 文本输入框组件
#[derive(Component)]
pub struct TextInput {
    /// 当前文本内容
    pub text: String,
    /// 光标位置（字符索引）
    pub cursor_position: usize,
    /// 选择起始位置（如果存在选择）
    pub selection_start: Option<usize>,
    /// 最大长度限制
    pub max_length: Option<usize>,
    /// 是否只读
    pub read_only: bool,
    /// 是否显示光标
    pub show_caret: bool,
    /// 光标闪烁计时器
    pub caret_timer: Timer,
}

impl Default for TextInput {
    fn default() -> Self {
        Self {
            text: String::new(),
            cursor_position: 0,
            selection_start: None,
            max_length: None,
            read_only: false,
            show_caret: true,
            caret_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
        }
    }
}

/// 文本输入框更新系统
pub fn update_text_input(
    mut query: Query<(Entity, &mut TextInput, &mut Text), With<Focusable>>,
    focus_state: Res<FocusState>,
    mut keyboard_input: EventReader<KeyboardInput>,
    mut char_input: EventReader<ReceivedCharacter>,
    time: Res<Time>,
) {
    // 只处理当前聚焦的输入框
    let Some(focused) = focus_state.focused else {
        return;
    };
    
    let Ok((entity, mut text_input, mut text)) = query.get_mut(focused) else {
        return;
    };
    
    if text_input.read_only {
        return;
    }
    
    // 更新光标闪烁
    text_input.caret_timer.tick(time.delta());
    if text_input.caret_timer.just_finished() {
        text_input.show_caret = !text_input.show_caret;
    }
    
    // 处理键盘输入（按键）
    for event in keyboard_input.read() {
        if !event.state.is_pressed() {
            continue;
        }
        
        match event.key_code {
            Some(KeyCode::Backspace) => {
                if text_input.cursor_position > 0 {
                    let mut chars: Vec<char> = text_input.text.chars().collect();
                    chars.remove(text_input.cursor_position - 1);
                    text_input.text = chars.into_iter().collect();
                    text_input.cursor_position -= 1;
                }
            }
            Some(KeyCode::Delete) => {
                if text_input.cursor_position < text_input.text.chars().count() {
                    let mut chars: Vec<char> = text_input.text.chars().collect();
                    chars.remove(text_input.cursor_position);
                    text_input.text = chars.into_iter().collect();
                }
            }
            Some(KeyCode::ArrowLeft) => {
                if text_input.cursor_position > 0 {
                    text_input.cursor_position -= 1;
                }
            }
            Some(KeyCode::ArrowRight) => {
                if text_input.cursor_position < text_input.text.chars().count() {
                    text_input.cursor_position += 1;
                }
            }
            Some(KeyCode::Home) => {
                text_input.cursor_position = 0;
            }
            Some(KeyCode::End) => {
                text_input.cursor_position = text_input.text.chars().count();
            }
            _ => {}
        }
        
        // 更新文本显示
        update_text_display(&mut text, &text_input);
    }
    
    // 处理字符输入
    for event in char_input.read() {
        let char = event.char;
        
        // 检查长度限制
        if let Some(max_len) = text_input.max_length {
            if text_input.text.chars().count() >= max_len {
                continue;
            }
        }
        
        // 过滤字符（只允许字母和数字）
        if char.is_ascii_alphanumeric() {
            let mut chars: Vec<char> = text_input.text.chars().collect();
            chars.insert(text_input.cursor_position, char);
            text_input.text = chars.into_iter().collect();
            text_input.cursor_position += 1;
            
            // 更新文本显示
            update_text_display(&mut text, &text_input);
        }
    }
}

/// 更新文本显示（包含光标）
fn update_text_display(text: &mut Text, text_input: &TextInput) {
    let mut display_text = text_input.text.clone();
    
    // 插入光标
    if text_input.show_caret {
        let cursor_pos = text_input.cursor_position.min(display_text.chars().count());
        let mut chars: Vec<char> = display_text.chars().collect();
        chars.insert(cursor_pos, '|');
        display_text = chars.into_iter().collect();
    }
    
    text.sections[0].value = display_text;
}

/// 创建文本输入框实体
pub fn spawn_text_input(
    commands: &mut Commands,
    position: (f32, f32),
    width: f32,
    max_length: Option<usize>,
) -> Entity {
    commands
        .spawn((
            TextInput {
                max_length,
                ..default()
            },
            Focusable,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(position.0),
                    top: Val::Px(position.1),
                    width: Val::Px(width),
                    height: Val::Px(24.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: BackgroundColor(crate::theme::colors::UiColors::TEXTBOX_BG),
                border_color: BorderColor(crate::theme::colors::UiColors::TEXTBOX_BORDER),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "",
                TextStyle {
                    font_size: crate::theme::metrics::UiMetrics::DEFAULT_FONT_SIZE,
                    color: crate::theme::colors::UiColors::TEXTBOX_TEXT,
                    ..default()
                },
            ));
        })
        .id()
}
