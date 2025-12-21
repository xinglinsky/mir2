//! 角色信息对话框

use bevy::prelude::*;
use super::common::*;

/// 角色信息对话框组件
#[derive(Component)]
pub struct CharacterDialog;

/// 创建角色信息对话框
pub fn spawn_character_dialog(
    commands: &mut Commands,
) -> Entity {
    // TODO: 使用 crystal-client-ui 创建角色信息对话框
    commands.spawn((
        DialogRoot {
            dialog_type: DialogType::Character,
            visible: true,
        },
        CharacterDialog,
    )).id()
}

