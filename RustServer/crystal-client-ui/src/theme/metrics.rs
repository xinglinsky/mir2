//! UI 尺寸度量
//!
//! 定义 UI 组件的默认尺寸、间距、字号等。

/// UI 尺寸度量
pub struct UiMetrics;

impl UiMetrics {
    /// 默认字体大小
    pub const DEFAULT_FONT_SIZE: f32 = 16.0;
    
    /// 小字体大小
    pub const SMALL_FONT_SIZE: f32 = 12.0;
    
    /// 大字体大小
    pub const LARGE_FONT_SIZE: f32 = 20.0;
    
    /// 默认内边距
    pub const DEFAULT_PADDING: f32 = 8.0;
    
    /// 小内边距
    pub const SMALL_PADDING: f32 = 4.0;
    
    /// 大内边距
    pub const LARGE_PADDING: f32 = 16.0;
    
    /// 文本输入框默认高度
    pub const TEXTBOX_HEIGHT: f32 = 24.0;
    
    /// 按钮默认高度
    pub const BUTTON_HEIGHT: f32 = 32.0;
    
    /// 按钮默认宽度
    pub const BUTTON_WIDTH: f32 = 100.0;
    
    /// 文本输入框边框宽度
    pub const TEXTBOX_BORDER_WIDTH: f32 = 1.0;
}

