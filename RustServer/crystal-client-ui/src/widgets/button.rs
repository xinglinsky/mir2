//! Button Widget
//!
//! 三态按钮（base/hover/pressed/disabled）。

use bevy::prelude::*;
use crate::theme::mir2_skin::ButtonSkin;

/// 按钮状态
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonState {
    /// 正常状态
    Base,
    /// 悬停状态
    Hover,
    /// 按下状态
    Pressed,
    /// 禁用状态
    Disabled,
}

/// 按钮组件
#[derive(Component)]
pub struct MirButton {
    /// 按钮皮肤
    pub skin: ButtonSkin,
    /// 当前状态
    pub state: ButtonState,
    /// 是否启用
    pub enabled: bool,
}

impl Default for MirButton {
    fn default() -> Self {
        Self {
            skin: ButtonSkin {
                base: Handle::default(),
                hover: Handle::default(),
                pressed: Handle::default(),
            },
            state: ButtonState::Base,
            enabled: true,
        }
    }
}

/// 按钮点击事件
#[derive(Event)]
pub struct ButtonClicked {
    /// 按钮实体
    pub entity: Entity,
}

/// 更新按钮状态系统
pub fn update_button_state(
    mut query: Query<(Entity, &mut MirButton, &mut UiImage, &Interaction), Changed<Interaction>>,
    mut click_events: EventWriter<ButtonClicked>,
) {
    for (entity, mut button, mut ui_image, interaction) in query.iter_mut() {
        if !button.enabled {
            button.state = ButtonState::Disabled;
            ui_image.texture = button.skin.base.clone();
            continue;
        }
        
        match *interaction {
            Interaction::Pressed => {
                button.state = ButtonState::Pressed;
                ui_image.texture = button.skin.pressed.clone();
            }
            Interaction::Hovered => {
                if button.state != ButtonState::Pressed {
                    button.state = ButtonState::Hover;
                    ui_image.texture = button.skin.hover.clone();
                }
            }
            Interaction::None => {
                // 如果之前是按下状态，现在变为 None，说明按钮被点击
                if button.state == ButtonState::Pressed {
                    click_events.send(ButtonClicked { entity });
                }
                button.state = ButtonState::Base;
                ui_image.texture = button.skin.base.clone();
            }
        }
    }
}

/// 创建按钮实体
pub fn spawn_button(
    commands: &mut Commands,
    position: (f32, f32),
    size: (f32, f32),
    skin: ButtonSkin,
) -> Entity {
    commands
        .spawn((
            MirButton {
                skin: skin.clone(),
                state: ButtonState::Base,
                enabled: true,
            },
            Button,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(position.0),
                    top: Val::Px(position.1),
                    width: Val::Px(size.0),
                    height: Val::Px(size.1),
                    ..default()
                },
                ..default()
            },
            UiImage::new(skin.base.clone()),
        ))
        .id()
}
