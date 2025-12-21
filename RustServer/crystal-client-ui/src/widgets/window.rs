//! Window Widget
//!
//! 可拖动、可关闭的窗口（对标 MirDialog）。

use bevy::prelude::*;

/// 窗口组件
#[derive(Component)]
pub struct MirWindow {
    /// 窗口标题
    pub title: String,
    /// 是否可拖动
    pub draggable: bool,
    /// 是否可关闭
    pub closable: bool,
}

/// 窗口拖动状态
#[derive(Component, Default)]
pub struct WindowDragState {
    /// 是否正在拖动
    pub is_dragging: bool,
    /// 拖动起始位置
    pub drag_start: Option<Vec2>,
}

/// 窗口拖动系统
pub fn handle_window_drag(
    mut query: Query<(&MirWindow, &mut WindowDragState, &mut Style), With<MirWindow>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut cursor_moved: EventReader<CursorMoved>,
) {
    // TODO: 实现窗口拖动逻辑
    // - 检测鼠标按下在窗口标题栏
    // - 更新窗口位置
    // - 处理拖动结束
}

