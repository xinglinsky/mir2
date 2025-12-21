//! 选角场景逻辑

use bevy::prelude::*;
use crystal_shared_proto::select::SelectInfo;

/// 选角场景状态
#[derive(Resource)]
pub struct SelectSceneState {
    /// 角色列表
    pub characters: Vec<SelectInfo>,
    /// 当前选中的角色索引
    pub selected_index: Option<i32>,
}

impl Default for SelectSceneState {
    fn default() -> Self {
        Self {
            characters: Vec::new(),
            selected_index: None,
        }
    }
}

/// 选择角色
pub fn select_character(
    state: &mut SelectSceneState,
    index: i32,
) -> bool {
    if index >= 0 && (index as usize) < state.characters.len() {
        state.selected_index = Some(index);
        true
    } else {
        false
    }
}

