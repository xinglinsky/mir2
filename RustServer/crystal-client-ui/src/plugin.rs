use bevy::prelude::*;
use bevy::input::keyboard::ReceivedCharacter;
use crate::input::focus::{FocusState, FocusEvent};
use crate::input::text_edit::update_text_input;
use crate::widgets::button::{update_button_state, ButtonClicked};
use crate::debug::overlay::update_debug_overlay;

/// UI 框架插件
///
/// 提供 UI 组件、主题系统、输入管理和布局工具。
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // 初始化资源
        app.init_resource::<crate::theme::UiTheme>();
        app.init_resource::<FocusState>();
        
        // 注册事件
        app.add_event::<FocusEvent>();
        app.add_event::<ButtonClicked>();
        app.add_event::<ReceivedCharacter>();
        
        // 添加系统
        app.add_systems(
            Update,
            (
                update_button_state,
                update_text_input,
                update_debug_overlay,
                // TODO: 添加其他 UI 系统
            ),
        );
    }
}

