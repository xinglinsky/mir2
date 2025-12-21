//! 客户端配置模型定义

use serde::{Deserialize, Serialize};

/// 客户端配置根结构
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientConfig {
    /// 网络配置
    #[serde(default)]
    pub network: NetworkConfig,
    
    /// 图形配置
    #[serde(default)]
    pub graphics: GraphicsConfig,
    
    /// 音频配置
    #[serde(default)]
    pub audio: AudioConfig,
    
    /// 游戏配置
    #[serde(default)]
    pub game: GameConfig,
    
    /// 聊天配置
    #[serde(default)]
    pub chat: ChatConfig,
    
    /// 过滤器配置
    #[serde(default)]
    pub filter: FilterConfig,
    
    /// 日志配置
    #[serde(default)]
    pub logs: LogsConfig,
    
    /// 账号信息（可选，用于自动登录）
    pub account: Option<String>,
    
    /// 角色索引（可选，用于自动选择角色）
    pub character_index: Option<i32>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            graphics: GraphicsConfig::default(),
            audio: AudioConfig::default(),
            game: GameConfig::default(),
            chat: ChatConfig::default(),
            filter: FilterConfig::default(),
            logs: LogsConfig::default(),
            account: None,
            character_index: None,
        }
    }
}

/// 网络配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// 服务器地址（IP:Port 格式）
    #[serde(default = "default_server_addr")]
    pub server_addr: String,
    
    /// 是否使用配置文件中的服务器地址
    #[serde(default = "default_false")]
    pub use_config: bool,
    
    /// IP 地址（当 use_config 为 true 时使用）
    #[serde(default = "default_ip")]
    pub ip_address: String,
    
    /// 端口（当 use_config 为 true 时使用）
    #[serde(default = "default_port")]
    pub port: u16,
    
    /// 连接超时（毫秒）
    #[serde(default = "default_timeout")]
    pub timeout_ms: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            server_addr: default_server_addr(),
            use_config: false,
            ip_address: default_ip(),
            port: default_port(),
            timeout_ms: default_timeout(),
        }
    }
}

/// 图形配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphicsConfig {
    /// 屏幕宽度
    #[serde(default = "default_screen_width")]
    pub screen_width: u32,
    
    /// 屏幕高度
    #[serde(default = "default_screen_height")]
    pub screen_height: u32,
    
    /// 是否全屏
    #[serde(default = "default_true")]
    pub fullscreen: bool,
    
    /// 是否无边框窗口
    #[serde(default = "default_true")]
    pub borderless: bool,
    
    /// 是否置顶
    #[serde(default = "default_true")]
    pub top_most: bool,
    
    /// 是否限制鼠标在窗口内
    #[serde(default = "default_false")]
    pub mouse_clip: bool,
    
    /// 字体名称
    #[serde(default = "default_font_name")]
    pub font_name: String,
    
    /// 字体大小
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    
    /// 是否使用鼠标光标
    #[serde(default = "default_true")]
    pub use_mouse_cursors: bool,
    
    /// 是否限制 FPS
    #[serde(default = "default_true")]
    pub fps_cap: bool,
    
    /// 最大 FPS
    #[serde(default = "default_max_fps")]
    pub max_fps: u32,
    
    /// 分辨率（基准值，用于缩放）
    #[serde(default = "default_resolution")]
    pub resolution: u32,
    
    /// 是否调试模式
    #[serde(default = "default_false")]
    pub debug_mode: bool,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            screen_width: default_screen_width(),
            screen_height: default_screen_height(),
            fullscreen: true,
            borderless: true,
            top_most: true,
            mouse_clip: false,
            font_name: default_font_name(),
            font_size: default_font_size(),
            use_mouse_cursors: true,
            fps_cap: true,
            max_fps: default_max_fps(),
            resolution: default_resolution(),
            debug_mode: false,
        }
    }
}

/// 音频配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioConfig {
    /// 音量（0-100）
    #[serde(default = "default_volume")]
    pub volume: u8,
    
    /// 音乐音量（0-100）
    #[serde(default = "default_volume")]
    pub music_volume: u8,
    
    /// 音效重叠数
    #[serde(default = "default_sound_overlap")]
    pub sound_overlap: u32,
    
    /// 音频清理间隔（分钟）
    #[serde(default = "default_sound_clean_minutes")]
    pub sound_clean_minutes: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            volume: default_volume(),
            music_volume: default_volume(),
            sound_overlap: default_sound_overlap(),
            sound_clean_minutes: default_sound_clean_minutes(),
        }
    }
}

/// 游戏配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameConfig {
    /// 账号 ID（用于自动登录）
    #[serde(default)]
    pub account_id: String,
    
    /// 密码（用于自动登录）
    #[serde(default)]
    pub password: String,
    
    /// 技能模式
    #[serde(default = "default_false")]
    pub skill_mode: bool,
    
    /// 是否显示技能栏
    #[serde(default = "default_true")]
    pub skill_bar: bool,
    
    /// 是否显示特效
    #[serde(default = "default_true")]
    pub effect: bool,
    
    /// 是否显示升级特效
    #[serde(default = "default_true")]
    pub level_effect: bool,
    
    /// 是否显示掉落物品
    #[serde(default = "default_true")]
    pub drop_view: bool,
    
    /// 是否显示名称
    #[serde(default = "default_true")]
    pub name_view: bool,
    
    /// 是否显示 HP/MP
    #[serde(default = "default_true")]
    pub hp_view: bool,
    
    /// 是否透明聊天窗口
    #[serde(default = "default_false")]
    pub transparent_chat: bool,
    
    /// 是否显示模式
    #[serde(default = "default_false")]
    pub mode_view: bool,
    
    /// 是否显示耐久度窗口
    #[serde(default = "default_false")]
    pub dura_view: bool,
    
    /// 是否显示伤害数字
    #[serde(default = "default_true")]
    pub display_damage: bool,
    
    /// 是否目标死亡后继续显示
    #[serde(default = "default_false")]
    pub target_dead: bool,
    
    /// 是否高亮目标
    #[serde(default = "default_true")]
    pub highlight_target: bool,
    
    /// 是否展开 Buff 窗口
    #[serde(default = "default_true")]
    pub expanded_buff_window: bool,
    
    /// 是否展开英雄 Buff 窗口
    #[serde(default = "default_true")]
    pub expanded_hero_buff_window: bool,
    
    /// 是否显示尸体名称
    #[serde(default = "default_false")]
    pub display_body_name: bool,
    
    /// 是否使用新移动系统
    #[serde(default = "default_false")]
    pub new_move: bool,
    
    /// 技能栏位置（2x2 数组：[[x1, y1], [x2, y2]]）
    #[serde(default = "default_skillbar_location")]
    pub skillbar_location: [[i32; 2]; 2],
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            account_id: String::new(),
            password: String::new(),
            skill_mode: false,
            skill_bar: true,
            effect: true,
            level_effect: true,
            drop_view: true,
            name_view: true,
            hp_view: true,
            transparent_chat: false,
            mode_view: false,
            dura_view: false,
            display_damage: true,
            target_dead: false,
            highlight_target: true,
            expanded_buff_window: true,
            expanded_hero_buff_window: true,
            display_body_name: false,
            new_move: false,
            skillbar_location: default_skillbar_location(),
        }
    }
}

/// 聊天配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatConfig {
    /// 是否显示普通聊天
    #[serde(default = "default_true")]
    pub show_normal_chat: bool,
    
    /// 是否显示喊话聊天
    #[serde(default = "default_true")]
    pub show_yell_chat: bool,
    
    /// 是否显示私聊
    #[serde(default = "default_true")]
    pub show_whisper_chat: bool,
    
    /// 是否显示爱人聊天
    #[serde(default = "default_true")]
    pub show_lover_chat: bool,
    
    /// 是否显示师徒聊天
    #[serde(default = "default_true")]
    pub show_mentor_chat: bool,
    
    /// 是否显示组队聊天
    #[serde(default = "default_true")]
    pub show_group_chat: bool,
    
    /// 是否显示行会聊天
    #[serde(default = "default_true")]
    pub show_guild_chat: bool,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            show_normal_chat: true,
            show_yell_chat: true,
            show_whisper_chat: true,
            show_lover_chat: true,
            show_mentor_chat: true,
            show_group_chat: true,
            show_guild_chat: true,
        }
    }
}

/// 过滤器配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilterConfig {
    /// 是否过滤普通聊天
    #[serde(default = "default_false")]
    pub filter_normal_chat: bool,
    
    /// 是否过滤私聊
    #[serde(default = "default_false")]
    pub filter_whisper_chat: bool,
    
    /// 是否过滤喊话
    #[serde(default = "default_false")]
    pub filter_shout_chat: bool,
    
    /// 是否过滤系统聊天
    #[serde(default = "default_false")]
    pub filter_system_chat: bool,
    
    /// 是否过滤爱人聊天
    #[serde(default = "default_false")]
    pub filter_lover_chat: bool,
    
    /// 是否过滤师徒聊天
    #[serde(default = "default_false")]
    pub filter_mentor_chat: bool,
    
    /// 是否过滤组队聊天
    #[serde(default = "default_false")]
    pub filter_group_chat: bool,
    
    /// 是否过滤行会聊天
    #[serde(default = "default_false")]
    pub filter_guild_chat: bool,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            filter_normal_chat: false,
            filter_whisper_chat: false,
            filter_shout_chat: false,
            filter_system_chat: false,
            filter_lover_chat: false,
            filter_mentor_chat: false,
            filter_group_chat: false,
            filter_guild_chat: false,
        }
    }
}

/// 日志配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogsConfig {
    /// 是否记录错误日志
    #[serde(default = "default_true")]
    pub log_errors: bool,
    
    /// 是否记录聊天日志
    #[serde(default = "default_true")]
    pub log_chat: bool,
    
    /// 剩余错误日志数量
    #[serde(default = "default_remaining_error_logs")]
    pub remaining_error_logs: u32,
}

impl Default for LogsConfig {
    fn default() -> Self {
        Self {
            log_errors: true,
            log_chat: true,
            remaining_error_logs: default_remaining_error_logs(),
        }
    }
}

// 默认值函数

fn default_server_addr() -> String {
    "127.0.0.1:7000".to_string()
}

fn default_ip() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    7000
}

fn default_timeout() -> u32 {
    5000
}

fn default_screen_width() -> u32 {
    1024
}

fn default_screen_height() -> u32 {
    768
}

fn default_font_name() -> String {
    "Arial".to_string()
}

fn default_font_size() -> f32 {
    8.0
}

fn default_max_fps() -> u32 {
    100
}

fn default_resolution() -> u32 {
    1024
}

fn default_volume() -> u8 {
    100
}

fn default_sound_overlap() -> u32 {
    3
}

fn default_sound_clean_minutes() -> u32 {
    5
}

fn default_skillbar_location() -> [[i32; 2]; 2] {
    [[0, 0], [216, 0]]
}

fn default_remaining_error_logs() -> u32 {
    100
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

