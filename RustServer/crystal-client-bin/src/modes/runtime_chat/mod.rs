mod net;
mod router;
mod state;
mod ui;

use bevy::prelude::*;
use crystal_client_app::build_app;
use crystal_client_app::RuntimeConfig as AppRuntimeConfig;

use crate::app_config::RuntimeConfig as BinRuntimeConfig;

use net::send_keep_alive;
use router::pump_net_events;
use state::ChatState;
use ui::{handle_chat_input, setup, update_chat_ui, update_fps_text};

pub(crate) fn run_runtime_chat(runtime: BinRuntimeConfig) {
    let app_runtime = AppRuntimeConfig {
        server_addr: runtime.server_addr.clone(),
        account: runtime.account.clone(),
        password: runtime.password.clone(),
        start: runtime.start,
        config_path: runtime.config_path.clone(),
    };
    
    // 使用 crystal-client-app 的基础 App 构建器
    let mut app = build_app();
    
    // 添加模式特定的资源
    app.insert_resource(app_runtime)
        .init_resource::<ChatState>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                pump_net_events,
                send_keep_alive,
                handle_chat_input,
                update_chat_ui,
                update_fps_text,
            ),
        )
        .run();
}
