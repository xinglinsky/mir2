//! 通用对话框基类

use bevy::prelude::*;

/// 对话框根组件
#[derive(Component)]
pub struct DialogRoot {
    /// 对话框类型
    pub dialog_type: DialogType,
    /// 是否可见
    pub visible: bool,
}

/// 对话框类型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogType {
    Inventory,
    Character,
    ChatOptions,
    Guild,
    Npc,
    Main,
}

/// 对话框基类 Trait
pub trait Dialog {
    /// 显示对话框
    fn show(&mut self);
    
    /// 隐藏对话框
    fn hide(&mut self);
    
    /// 是否可见
    fn is_visible(&self) -> bool;
}

