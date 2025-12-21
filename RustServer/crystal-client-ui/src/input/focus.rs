//! 输入焦点管理
//!
//! 类似 WinForms 的 ActiveControl，管理 UI 组件的焦点状态。

use bevy::prelude::*;

/// 可聚焦组件标记
#[derive(Component)]
pub struct Focusable;

/// 当前聚焦的实体
#[derive(Resource, Default)]
pub struct FocusState {
    /// 当前聚焦的实体
    pub focused: Option<Entity>,
}

/// 焦点管理事件
#[derive(Event)]
pub enum FocusEvent {
    /// 获得焦点
    Focused(Entity),
    /// 失去焦点
    Unfocused(Entity),
}

/// 设置焦点系统
pub fn set_focus(
    mut focus_state: ResMut<FocusState>,
    mut focus_events: EventWriter<FocusEvent>,
    target: Entity,
    query: Query<Entity, With<Focusable>>,
) {
    // 如果目标实体可聚焦
    if query.get(target).is_ok() {
        // 取消之前的焦点
        if let Some(old_focused) = focus_state.focused {
            if old_focused != target {
                focus_events.send(FocusEvent::Unfocused(old_focused));
            }
        }
        
        // 设置新焦点
        focus_state.focused = Some(target);
        focus_events.send(FocusEvent::Focused(target));
    }
}

/// 清除焦点系统
pub fn clear_focus(
    mut focus_state: ResMut<FocusState>,
    mut focus_events: EventWriter<FocusEvent>,
) {
    if let Some(focused) = focus_state.focused {
        focus_events.send(FocusEvent::Unfocused(focused));
        focus_state.focused = None;
    }
}

