mod net;
mod router;
mod state;
mod ui;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;

use crate::app_config::RuntimeConfig;

use net::send_keep_alive;
use router::pump_net_events;
use state::ChatState;
use ui::{handle_chat_input, setup, update_chat_ui, update_fps_text};

pub(crate) fn run_runtime_chat(runtime: RuntimeConfig) {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .insert_resource(runtime)
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
