//! List Widget
//!
//! 列表组件，支持滚动和选择。

use bevy::prelude::*;

/// 列表组件
#[derive(Component)]
pub struct MirList {
    /// 列表项数量
    pub item_count: usize,
    /// 当前选中的索引
    pub selected_index: Option<usize>,
    /// 是否支持多选
    pub multi_select: bool,
}

/// 列表项组件
#[derive(Component)]
pub struct ListItem {
    /// 列表项索引
    pub index: usize,
    /// 是否选中
    pub selected: bool,
}

