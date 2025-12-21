use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crystal_client_net::NetClient;
use crystal_shared_proto::login::CChat;

use crate::app_config::RuntimeConfig;

use super::state::{
    log_key_event, ChatInputText, ChatLogText, ChatState, ChatStatusText, FpsText,
};

pub(super) fn setup(
    mut commands: Commands,
    runtime: Res<RuntimeConfig>,
    mut chat: ResMut<ChatState>,
    existing_cams: Query<Entity, With<Camera>>,
) {
    if existing_cams.is_empty() {
        commands.spawn(Camera2dBundle::default());
    }

    let chat = &mut *chat;

    chat.lines.push("[net] connecting...".to_string());
    match NetClient::connect(&runtime.server_addr) {
        Ok(net) => {
            chat.net = Some(net);
            chat.connected = false;
            chat.sent_version = false;
            chat.version_checked = false;
            chat.logged_in = false;
            chat.pending_start_game = None;
            chat.keepalive_timer.reset();

            log_key_event(
                chat,
                format!(
                    "[runtime] config={} server={} ",
                    runtime.config_path, runtime.server_addr
                ),
            );

            if let (Some(account), Some(password)) = (runtime.account.clone(), runtime.password.clone()) {
                chat.pending_login = Some((account, password));
                log_key_event(chat, "[runtime] auto login queued".to_string());
            }
            if let Some(idx) = runtime.start {
                chat.pending_start_game = Some(idx);
                log_key_event(chat, format!("[runtime] auto start queued idx={idx}"));
            }
        }
        Err(e) => {
            chat.lines.push(format!("[net] connect error: {e}"));
            chat.last_error = Some(format!("{e}"));
            chat.net = None;
        }
    }

    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                top: Val::Px(8.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "FPS: --",
                    TextStyle {
                        font_size: 18.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ),
                FpsText,
            ));
        });

    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                bottom: Val::Px(8.0),
                width: Val::Px(520.0),
                height: Val::Px(220.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "[chat]",
                    TextStyle {
                        font_size: 16.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ),
                ChatStatusText,
            ));

            parent.spawn((
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font_size: 16.0,
                        color: Color::WHITE,
                        ..default()
                    },
                )
                .with_style(Style {
                    flex_grow: 1.0,
                    ..default()
                }),
                ChatLogText,
            ));

            parent.spawn((
                TextBundle::from_section(
                    "> ",
                    TextStyle {
                        font_size: 16.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ),
                ChatInputText,
            ));
        });
}

pub(super) fn update_fps_text(diagnostics: Res<DiagnosticsStore>, mut query: Query<&mut Text, With<FpsText>>) {
    let mut text = match query.get_single_mut() {
        Ok(v) => v,
        Err(_) => return,
    };

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    text.sections[0].value = format!("FPS: {:.0}", fps);
}

pub(super) fn handle_chat_input(mut chat: ResMut<ChatState>, mut ev_keys: EventReader<KeyboardInput>) {
    let chat = &mut *chat;

    for ev in ev_keys.read() {
        if !ev.state.is_pressed() {
            continue;
        }
        match ev.key_code {
            KeyCode::Backspace => {
                chat.input.pop();
            }
            KeyCode::Space => {
                chat.input.push(' ');
            }
            KeyCode::Enter => {
                let msg = chat.input.trim().to_string();
                if msg.is_empty() {
                    chat.input.clear();
                    continue;
                }

                if let Some(rest) = msg.strip_prefix("/login") {
                    let parts: Vec<&str> = rest.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let account_id = parts[0].to_string();
                        let password = parts[1].to_string();
                        chat.pending_login = Some((account_id, password));
                        chat.lines.push("[login] queued".to_string());
                    } else {
                        chat.lines
                            .push("[login] usage: /login <account> <password>".to_string());
                    }
                } else if let Some(rest) = msg.strip_prefix("/start") {
                    let parts: Vec<&str> = rest.split_whitespace().collect();
                    if parts.len() >= 1 {
                        match parts[0].parse::<i32>() {
                            Ok(idx) => {
                                chat.pending_start_game = Some(idx);
                                chat.lines.push("[start_game] queued".to_string());
                            }
                            Err(_) => {
                                chat.lines
                                    .push("[start_game] usage: /start <character_index>".to_string());
                            }
                        }
                    } else {
                        chat.lines
                            .push("[start_game] usage: /start <character_index>".to_string());
                    }
                } else {
                    chat.lines.push(format!("[me] {msg}"));
                    if let Some(net) = chat.net.as_ref() {
                        let pkt = CChat {
                            message: msg,
                            linked_items: Vec::new(),
                        }
                        .encode();

                        match pkt {
                            Ok(pkt) => {
                                let _ = net.send_raw(pkt);
                            }
                            Err(e) => {
                                chat.lines.push(format!("[chat] encode error: {e}"));
                            }
                        }
                    }
                }

                chat.input.clear();
            }
            _ => {
                if let Key::Character(s) = &ev.logical_key {
                    if s.chars().all(|c| c.is_control()) {
                        continue;
                    }
                    chat.input.push_str(s.as_str());
                }
            }
        }
    }
}

pub(super) fn update_chat_ui(
    chat: Res<ChatState>,
    mut sets: ParamSet<(
        Query<&mut Text, With<ChatLogText>>,
        Query<&mut Text, With<ChatInputText>>,
        Query<&mut Text, With<ChatStatusText>>,
    )>,
) {
    let chat = &*chat;

    if let Ok(mut t) = sets.p1().get_single_mut() {
        t.sections[0].value = format!("> {}", chat.input);
    }

    if let Ok(mut t) = sets.p2().get_single_mut() {
        let status = if chat.net.is_none() {
            "[net] disabled"
        } else if chat.connected {
            "[net] connected"
        } else {
            "[net] connecting/disconnected"
        };
        t.sections[0].value = status.to_string();
    }

    if let Ok(mut t) = sets.p0().get_single_mut() {
        let start = chat.lines.len().saturating_sub(12);
        t.sections[0].value = chat.lines[start..].join("\n");
    }
}
