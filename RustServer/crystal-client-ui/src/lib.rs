//! Crystal Client UI Framework
//!
//! 提供可复用的 UI 组件、主题系统、输入管理和布局工具。

pub mod plugin;
pub mod theme;
pub mod input;
pub mod widgets;
pub mod layout;
pub mod debug;

pub use plugin::UiPlugin;
pub use theme::*;
pub use input::*;
pub use widgets::*;
pub use layout::*;
pub use debug::*;
