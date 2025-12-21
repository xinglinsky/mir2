//! UI 颜色定义
//!
//! 定义游戏中使用的标准颜色。

use bevy::prelude::Color;

/// 标准 UI 颜色
pub struct UiColors;

impl UiColors {
    /// 白色
    pub const WHITE: Color = Color::WHITE;
    
    /// 黑色
    pub const BLACK: Color = Color::BLACK;
    
    /// 文本输入框背景色（黑色）
    pub const TEXTBOX_BG: Color = Color::BLACK;
    
    /// 文本输入框边框色（白色）
    pub const TEXTBOX_BORDER: Color = Color::WHITE;
    
    /// 文本输入框文本色（白色）
    pub const TEXTBOX_TEXT: Color = Color::WHITE;
    
    /// 按钮禁用状态颜色（灰色）
    pub const BUTTON_DISABLED: Color = Color::srgb(0.5, 0.5, 0.5);
    
    /// 错误文本颜色（红色）
    pub const ERROR_TEXT: Color = Color::srgb(1.0, 0.0, 0.0);
    
    /// 成功文本颜色（绿色）
    pub const SUCCESS_TEXT: Color = Color::srgb(0.0, 1.0, 0.0);
}

