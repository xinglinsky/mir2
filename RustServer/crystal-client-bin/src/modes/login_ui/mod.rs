mod net;
mod select_ui;
mod state;
mod ui;

use bevy::prelude::*;
use crystal_client_app::{LoginUiConfig, startup_systems, shutdown_systems};

use crate::app_config::LoginUiConfig as BinLoginUiConfig;

use net::{login_ui_begin_login, login_ui_pump_net};
use state::{CaretBlink, LoginUiNetState, LoginUiStage, LoginUiStartLogin, LoginUiState};
use select_ui::{
    login_ui_enter_select, login_ui_select_animate_character_display, login_ui_select_handle_buttons,
    login_ui_select_handle_keypress, login_ui_select_update_interface,
};
use ui::{
    login_ui_animate_background, login_ui_enter_in_game,
    login_ui_enter_login,
    login_ui_handle_buttons, login_ui_handle_focus_click, login_ui_handle_text_input,
    login_ui_setup, login_ui_update_button_skins,
    login_ui_update_caret, login_ui_update_in_game_status, login_ui_update_input_frames,
    login_ui_update_ok_enabled, login_ui_update_text,
};

pub(crate) fn run_login_ui(cfg: BinLoginUiConfig) {
    let server_addr = cfg.server_addr.clone();
    let app_cfg = LoginUiConfig {
        data_dir: cfg.data_dir.clone(),
        server_addr: cfg.server_addr.clone(),
    };
    
    // login_ui 模式需要特定的窗口配置，所以直接构建 App
    let mut app = App::new();
    
    // 添加默认插件，使用 login_ui 特定的窗口配置
    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Crystal M2".to_string(),
                resolution: (1024.0, 768.0).into(),
                resizable: false,
                ..default()
            }),
            ..default()
        }),
    );
    
    // 添加客户端插件（性能诊断等）
    app.add_plugins(crystal_client_app::ClientPlugins::default());
    
    // 添加启动和关闭系统（来自 crystal-client-app）
    app.add_systems(Startup, startup_systems);
    app.add_systems(Update, shutdown_systems);
    
    // 添加模式特定的资源
    app.insert_resource(app_cfg)
        .insert_resource(LoginUiState {
            account: String::new(),
            password: String::new(),
            focus: state::LoginField::Account,
            bg_frame: 0,
            bg_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
        })
        .insert_resource(LoginUiStage::Login)
        .insert_resource(LoginUiNetState {
            server_addr,
            net: None,
            connected: false,
            just_connected: false,
            sent_version: false,
            version_checked: false,
            pending_login: None,
            logged_in: false,
            characters: Vec::new(),
            selected_character_index: None,
            pending_start_game: None,
            start_game_ok: false,
            map_index: None,
            map_file_name: None,
            user_name: None,
            user_location_x: None,
            user_location_y: None,
            last_error: None,
        })
        .insert_resource(CaretBlink {
            timer: Timer::from_seconds(0.5, TimerMode::Repeating),
            visible: true,
        })
        .add_event::<LoginUiStartLogin>()
        .add_systems(Startup, login_ui_setup)
        .add_systems(
            Update,
            (
                login_ui_animate_background,
                login_ui_handle_focus_click,
                login_ui_handle_text_input,
                login_ui_update_input_frames,
                login_ui_update_ok_enabled,
                login_ui_update_text,
                login_ui_update_caret,
                login_ui_update_button_skins,
                login_ui_handle_buttons,
                login_ui_begin_login,
                login_ui_pump_net,
                login_ui_select_handle_buttons,
                login_ui_select_handle_keypress,
                login_ui_enter_select,
                login_ui_select_update_interface,
                login_ui_select_animate_character_display,
                login_ui_enter_login,
                login_ui_enter_in_game,
                login_ui_update_in_game_status,
            ),
        )
        .run();
}
