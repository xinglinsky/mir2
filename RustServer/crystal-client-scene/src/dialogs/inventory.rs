//! 背包对话框

use bevy::prelude::*;
use super::common::*;

/// 背包对话框组件
#[derive(Component)]
pub struct InventoryDialog;

/// 创建背包对话框
pub fn spawn_inventory_dialog(
    commands: &mut Commands,
) -> Entity {
    // TODO: 使用 crystal-client-ui 创建背包对话框
    commands.spawn((
        DialogRoot {
            dialog_type: DialogType::Inventory,
            visible: true,
        },
        InventoryDialog,
    )).id()
}

