use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use crystal_client_config::ClientConfig;
use std::path::PathBuf;

#[derive(Resource)]
struct LauncherState {
    server_addr: String,
    account: String,
    password: String,
    character_index: String,
    status: String,
}

impl Default for LauncherState {
    fn default() -> Self {
        let cfg = ClientConfig::load_or_default(ClientConfig::default_path()).unwrap_or_default();
        Self {
            server_addr: cfg.network.server_addr.clone(),
            account: cfg.account.unwrap_or_default(),
            password: String::new(),
            character_index: cfg.character_index.map(|v| v.to_string()).unwrap_or_default(),
            status: String::new(),
        }
    }
}

#[derive(Component)]
struct InputServerBtn;
#[derive(Component)]
struct InputAccountBtn;
#[derive(Component)]
struct InputPasswordBtn;
#[derive(Component)]
struct InputCharIndexBtn;

#[derive(Component)]
struct InputServerText;
#[derive(Component)]
struct InputAccountText;
#[derive(Component)]
struct InputPasswordText;
#[derive(Component)]
struct InputCharIndexText;
#[derive(Component)]
struct StatusText;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: "../Client/Resources".to_string(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Crystal Launcher".to_string(),
                        resolution: (800.0, 520.0).into(),
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .init_resource::<LauncherState>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_focus_click,
                handle_text_input,
                update_ui,
                handle_buttons,
                update_button_skins,
            ),
        )
        .run();
}

#[derive(Resource, Clone)]
struct LauncherAssets {
    launch_base: Handle<Image>,
    launch_hover: Handle<Image>,
    launch_pressed: Handle<Image>,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, state: Res<LauncherState>) {
    commands.spawn(Camera2dBundle::default());

    let assets = LauncherAssets {
        launch_base: asset_server.load("Launch_Base.png"),
        launch_hover: asset_server.load("Launch_Hover.png"),
        launch_pressed: asset_server.load("Launch_Pressed.png"),
    };
    let launch_base = assets.launch_base.clone();
    commands.insert_resource(assets);

    let font_size = 18.0;

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(10.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgb(0.08, 0.08, 0.09)),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Crystal Launcher",
                TextStyle {
                    font_size: 28.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));

            spawn_input_row(
                parent,
                "Server",
                &state.server_addr,
                InputServerBtn,
                InputServerText,
                font_size,
            );
            spawn_input_row(
                parent,
                "Account",
                &state.account,
                InputAccountBtn,
                InputAccountText,
                font_size,
            );
            spawn_input_row(
                parent,
                "Password",
                &mask_password(&state.password),
                InputPasswordBtn,
                InputPasswordText,
                font_size,
            );
            spawn_input_row(
                parent,
                "Character Index",
                &state.character_index,
                InputCharIndexBtn,
                InputCharIndexText,
                font_size,
            );

            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(10.0),
                        margin: UiRect::top(Val::Px(10.0)),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    spawn_button(parent, "Save Config", LauncherButton::Save);
                    spawn_launch_button(parent, LauncherButton::Launch, launch_base.clone());
                    spawn_button(parent, "Open Logs", LauncherButton::OpenLogsDir);
                });

            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(10.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    spawn_button(parent, "Open key_events.log", LauncherButton::OpenKeyLog);
                    spawn_button(
                        parent,
                        "Open unhandled_packets.log",
                        LauncherButton::OpenUnhandledLog,
                    );
                });

            parent.spawn((
                TextBundle::from_section(
                    if state.status.is_empty() {
                        ""
                    } else {
                        &state.status
                    },
                    TextStyle {
                        font_size,
                        color: Color::srgb(0.9, 0.9, 0.9),
                        ..default()
                    },
                ),
                StatusText,
            ));

            parent.spawn(TextBundle::from_section(
                "Note: password is NOT saved. It is passed to client only at launch time.",
                TextStyle {
                    font_size: 14.0,
                    color: Color::srgb(0.7, 0.7, 0.7),
                    ..default()
                },
            ));
        });

    commands.insert_resource(FocusedField::default());
}

fn mask_password(pwd: &str) -> String {
    "*".repeat(pwd.chars().count())
}

#[derive(Resource, Default)]
struct FocusedField {
    field: Option<FieldKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FieldKind {
    Server,
    Account,
    Password,
    CharIndex,
}

#[derive(Component)]
struct LauncherBtn(LauncherButton);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LauncherButton {
    Save,
    Launch,
    OpenLogsDir,
    OpenKeyLog,
    OpenUnhandledLog,
}

#[derive(Component)]
struct LaunchButtonImage;

fn spawn_launch_button(parent: &mut ChildBuilder, kind: LauncherButton, base: Handle<Image>) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(140.0),
                    height: Val::Px(44.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                ..default()
            },
            LauncherBtn(kind),
        ))
        .with_children(|p| {
            p.spawn((
                ImageBundle {
                    style: Style {
                        width: Val::Px(140.0),
                        height: Val::Px(44.0),
                        ..default()
                    },
                    image: UiImage { texture: base, ..default() },
                    ..default()
                },
                LaunchButtonImage,
            ));
        });
}

fn update_button_skins(
    assets: Res<LauncherAssets>,
    mut q: Query<(&Interaction, &LauncherBtn, &Children), (Changed<Interaction>, With<Button>)>,
    mut img_q: Query<&mut UiImage, With<LaunchButtonImage>>,
) {
    for (interaction, btn, children) in q.iter_mut() {
        if btn.0 != LauncherButton::Launch {
            continue;
        }

        let handle = match interaction {
            Interaction::Pressed => assets.launch_pressed.clone(),
            Interaction::Hovered => assets.launch_hover.clone(),
            Interaction::None => assets.launch_base.clone(),
        };

        for &c in children.iter() {
            if let Ok(mut ui_img) = img_q.get_mut(c) {
                ui_img.texture = handle.clone();
            }
        }
    }
}

fn logs_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("logs")
}

fn key_log_path() -> PathBuf {
    logs_dir().join("key_events.log")
}

fn unhandled_log_path() -> PathBuf {
    logs_dir().join("unhandled_packets.log")
}

fn open_path_default(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Use Windows shell to open with default association.
        // cmd /C start "" "<path>"
        std::process::Command::new("cmd")
            .args([
                "/C",
                "start",
                "",
                path.to_string_lossy().as_ref(),
            ])
            .spawn()
            .map(|_| ())
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
    }
}

fn spawn_input_row<TBtn: Component, TText: Component>(
    parent: &mut ChildBuilder,
    label: &str,
    value: &str,
    btn_marker: TBtn,
    text_marker: TText,
    font_size: f32,
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                format!("{label}:"),
                TextStyle {
                    font_size,
                    color: Color::srgb(0.9, 0.9, 0.9),
                    ..default()
                },
            ));

            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(520.0),
                            height: Val::Px(32.0),
                            padding: UiRect::horizontal(Val::Px(8.0)),
                            justify_content: JustifyContent::FlexStart,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgb(0.15, 0.15, 0.17)),
                        ..default()
                    },
                    btn_marker,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        TextBundle::from_section(
                            value,
                            TextStyle {
                                font_size,
                                color: Color::srgb(0.95, 0.95, 0.95),
                                ..default()
                            },
                        ),
                        text_marker,
                    ));
                });
        });
}

fn spawn_button(parent: &mut ChildBuilder, label: &str, kind: LauncherButton) {
    parent.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(160.0),
                height: Val::Px(34.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: BackgroundColor(Color::srgb(0.2, 0.2, 0.22)),
            ..default()
        },
        LauncherBtn(kind),
    ))
    .with_children(|p| {
        p.spawn(TextBundle::from_section(
            label,
            TextStyle {
                font_size: 16.0,
                color: Color::WHITE,
                ..default()
            },
        ));
    });
}

fn handle_buttons(
    mut interaction_q: Query<(&Interaction, &LauncherBtn), (Changed<Interaction>, With<Button>)>,
    mut state: ResMut<LauncherState>,
) {
    for (interaction, btn) in interaction_q.iter_mut() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match btn.0 {
            LauncherButton::Save => {
                let mut cfg = ClientConfig::default();
                cfg.network.server_addr = state.server_addr.trim().to_string();
                cfg.account = if state.account.trim().is_empty() {
                    None
                } else {
                    Some(state.account.trim().to_string())
                };
                cfg.character_index = state.character_index.trim().parse::<i32>().ok();

                match cfg.save(ClientConfig::default_path()) {
                    Ok(_) => state.status = "Saved config/client.toml".to_string(),
                    Err(e) => state.status = format!("Save failed: {e}"),
                }
            }
            LauncherButton::Launch => {
                let mut cfg = ClientConfig::default();
                cfg.network.server_addr = state.server_addr.trim().to_string();
                cfg.account = if state.account.trim().is_empty() {
                    None
                } else {
                    Some(state.account.trim().to_string())
                };
                cfg.character_index = state.character_index.trim().parse::<i32>().ok();
                let _ = cfg
                .save(ClientConfig::default_path());

                let exe = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                    .map(|dir| dir.join("crystal-client-bin.exe"));

                let Some(exe) = exe else {
                    state.status = "Launch failed: cannot locate client exe".to_string();
                    continue;
                };

                let mut cmd = std::process::Command::new(exe);
                cmd.arg("--config")
                    .arg(ClientConfig::default_path());

                if !state.account.trim().is_empty() {
                    cmd.arg("--account").arg(state.account.trim());
                }
                if !state.password.is_empty() {
                    cmd.arg("--password").arg(&state.password);
                }

                if let Ok(idx) = state.character_index.trim().parse::<i32>() {
                    cmd.arg("--start").arg(idx.to_string());
                }

                match cmd.spawn() {
                    Ok(_) => state.status = "Launched client".to_string(),
                    Err(e) => state.status = format!("Launch failed: {e}"),
                }
            }
            LauncherButton::OpenLogsDir => match open_path_default(&logs_dir()) {
                Ok(_) => state.status = "Opened logs directory".to_string(),
                Err(e) => state.status = format!("Open logs failed: {e}"),
            },
            LauncherButton::OpenKeyLog => match open_path_default(&key_log_path()) {
                Ok(_) => state.status = "Opened key_events.log".to_string(),
                Err(e) => state.status = format!("Open key_events.log failed: {e}"),
            },
            LauncherButton::OpenUnhandledLog => match open_path_default(&unhandled_log_path()) {
                Ok(_) => state.status = "Opened unhandled_packets.log".to_string(),
                Err(e) => state.status = format!("Open unhandled_packets.log failed: {e}"),
            },
        }
    }
}

fn handle_focus_click(
    mut focused: ResMut<FocusedField>,
    q: Query<
        (
            &Interaction,
            Option<&InputServerBtn>,
            Option<&InputAccountBtn>,
            Option<&InputPasswordBtn>,
            Option<&InputCharIndexBtn>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, server, account, password, idx) in q.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if server.is_some() {
            focused.field = Some(FieldKind::Server);
        } else if account.is_some() {
            focused.field = Some(FieldKind::Account);
        } else if password.is_some() {
            focused.field = Some(FieldKind::Password);
        } else if idx.is_some() {
            focused.field = Some(FieldKind::CharIndex);
        }
    }
}

fn handle_text_input(
    mut ev_keys: EventReader<KeyboardInput>,
    focused: Res<FocusedField>,
    mut state: ResMut<LauncherState>,
) {
    let Some(field) = focused.field else {
        return;
    };

    for ev in ev_keys.read() {
        if !ev.state.is_pressed() {
            continue;
        }

        match ev.key_code {
            KeyCode::Backspace => match field {
                FieldKind::Server => {
                    state.server_addr.pop();
                }
                FieldKind::Account => {
                    state.account.pop();
                }
                FieldKind::Password => {
                    state.password.pop();
                }
                FieldKind::CharIndex => {
                    state.character_index.pop();
                }
            },
            KeyCode::Space => match field {
                FieldKind::Server => state.server_addr.push(' '),
                FieldKind::Account => state.account.push(' '),
                FieldKind::Password => state.password.push(' '),
                FieldKind::CharIndex => state.character_index.push(' '),
            },
            _ => {
                if let Key::Character(s) = &ev.logical_key {
                    if s.chars().all(|c| c.is_control()) {
                        continue;
                    }
                    match field {
                        FieldKind::Server => state.server_addr.push_str(s.as_str()),
                        FieldKind::Account => state.account.push_str(s.as_str()),
                        FieldKind::Password => state.password.push_str(s.as_str()),
                        FieldKind::CharIndex => state.character_index.push_str(s.as_str()),
                    }
                }
            }
        }
    }
}

fn update_ui(
    state: Res<LauncherState>,
    mut sets: ParamSet<(
        Query<&mut Text, With<InputServerText>>,
        Query<&mut Text, With<InputAccountText>>,
        Query<&mut Text, With<InputPasswordText>>,
        Query<&mut Text, With<InputCharIndexText>>,
        Query<&mut Text, With<StatusText>>,
    )>,
) {
    if let Ok(mut t) = sets.p0().get_single_mut() {
        t.sections[0].value = state.server_addr.clone();
    }
    if let Ok(mut t) = sets.p1().get_single_mut() {
        t.sections[0].value = state.account.clone();
    }
    if let Ok(mut t) = sets.p2().get_single_mut() {
        t.sections[0].value = mask_password(&state.password);
    }
    if let Ok(mut t) = sets.p3().get_single_mut() {
        t.sections[0].value = state.character_index.clone();
    }
    if let Ok(mut t) = sets.p4().get_single_mut() {
        t.sections[0].value = state.status.clone();
    }
}
