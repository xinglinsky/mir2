use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::app::AppExit;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use clap::Parser;
use crystal_client_config::ClientConfig;
use crystal_client_net::{NetClient, NetEvent};
use crystal_lib::LibFile;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use crystal_shared_proto::login::{
    CChat, CClientVersion, CKeepAlive, CLogin, CStartGame, SClientVersion, SLogin, SLoginBanned,
    SConnected, SKeepAlive, SStartGame, SStartGameBanned, SStartGameDelay, ServerPacketId,
};
use crystal_shared_proto::item::{SNewItemInfo, SNewRecipeInfo};
use crystal_shared_proto::mail::SReceiveMail;
use crystal_shared_proto::map::{SMapChanged, SMapInformation};
use crystal_shared_proto::npc::{SDefaultNpc, SNpcResponse, SNpcUpdate};
use crystal_shared_proto::quest::{SCompleteQuest, SNewQuestInfo};
use crystal_shared_proto::select::{SelectInfo, SLoginSuccess};
use crystal_shared_proto::scene::{
    SAddBuff, SChat, SObjectChat, SObjectColourChanged, SObjectHealth, SObjectHero, SObjectMonster,
    SObjectNpc, SObjectPlayer, SObjectRemove, SObjectRun, SObjectTeleportIn, SObjectTurn, SObjectWalk,
    SSwitchGroup,
};
use crystal_shared_proto::shop::SGameShopInfo;
use crystal_shared_proto::social::{SFriendUpdate, SLoverUpdate, SMentorUpdate};
use crystal_shared_proto::guild::SGuildBuffList;
use crystal_shared_proto::stats::SBaseStatsInfo;
use crystal_shared_proto::user::{
    SFishingUpdate, SHealthChanged, SChangeAMode, SChangePMode, SInTrapRock, STimeOfDay,
    SUserInformation, SUserLocation,
};

#[derive(Parser, Debug, Clone)]
#[command(name = "crystal-client-bin")]
struct Cli {
    #[arg(long)]
    config: Option<String>,
    #[arg(long)]
    server: Option<String>,
    #[arg(long)]
    account: Option<String>,
    #[arg(long)]
    password: Option<String>,
    #[arg(long)]
    start: Option<i32>,

    #[arg(long)]
    login_ui: bool,
    #[arg(long)]
    data_dir: Option<String>,

    #[arg(long)]
    lib_test: bool,
    #[arg(long)]
    lib_path: Option<String>,
    #[arg(long, default_value_t = 1084)]
    lib_index: usize,
}

fn login_ui_enter_in_game(
    mut commands: Commands,
    stage: Res<LoginUiStage>,
    root_q: Query<Entity, With<LoginUiRoot>>,
    login_root_q: Query<Entity, With<LoginUiLoginRoot>>,
    select_root_q: Query<Entity, With<LoginUiSelectRoot>>,
    status_q: Query<Entity, With<InGameStatusText>>,
) {
    if !stage.is_changed() {
        return;
    }
    if *stage != LoginUiStage::InGame {
        return;
    }

    for e in login_root_q.iter() {
        commands.entity(e).despawn_recursive();
    }
    for e in select_root_q.iter() {
        commands.entity(e).despawn_recursive();
    }
    for e in status_q.iter() {
        commands.entity(e).despawn_recursive();
    }

    let Ok(ui_root) = root_q.get_single() else {
        return;
    };

    commands.entity(ui_root).with_children(|root| {
        root.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                top: Val::Px(8.0),
                width: Val::Px(520.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            ..default()
        })
        .with_children(|p| {
            p.spawn((
                TextBundle::from_section(
                    "[ingame] waiting...",
                    TextStyle {
                        font_size: 16.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ),
                InGameStatusText,
            ));
        });
    });
}

fn login_ui_update_in_game_status(
    stage: Res<LoginUiStage>,
    net_state: Res<LoginUiNetState>,
    mut q: Query<&mut Text, With<InGameStatusText>>,
) {
    if *stage != LoginUiStage::InGame {
        return;
    }

    let Ok(mut text) = q.get_single_mut() else {
        return;
    };

    let mut s = String::new();
    s.push_str("[ingame]\n");
    s.push_str(&format!("connected={} version_ok={} logged_in={} start_ok={}\n", net_state.connected, net_state.version_checked, net_state.logged_in, net_state.start_game_ok));
    s.push_str(&format!("map={:?} {:?}\n", net_state.map_index, net_state.map_file_name));
    s.push_str(&format!("user={:?} loc=({:?},{:?})\n", net_state.user_name, net_state.user_location_x, net_state.user_location_y));
    if let Some(e) = &net_state.last_error {
        s.push_str(&format!("error={e}\n"));
    }

    text.sections[0].value = s;
}

fn login_ui_update_input_frames(
    state: Res<LoginUiState>,
    mut q: Query<(&LoginInputFrame, &mut BorderColor)>,
) {
    for (field, mut border) in q.iter_mut() {
        let (text, valid) = match field.0 {
            LoginField::Account => (state.account.as_str(), is_valid_account(state.account.as_str())),
            LoginField::Password => (
                state.password.as_str(),
                is_valid_password(state.password.as_str()),
            ),
        };

        if text.is_empty() {
            border.0 = Color::rgba(0.0, 0.0, 0.0, 0.0);
        } else if valid {
            border.0 = Color::srgb(0.0, 1.0, 0.0);
        } else {
            border.0 = Color::srgb(1.0, 0.0, 0.0);
        }
    }
}

#[derive(Resource, Clone)]
struct RuntimeConfig {
    server_addr: String,
    account: Option<String>,
    password: Option<String>,
    start: Option<i32>,
    config_path: String,
}

#[derive(Resource, Clone)]
struct LibTestConfig {
    lib_path: PathBuf,
    lib_index: usize,
}

#[derive(Resource, Clone)]
struct LoginUiConfig {
    data_dir: PathBuf,
    server_addr: String,
}

pub fn main() {
    let cli = Cli::parse();

    if cli.login_ui {
        let cfg_path: PathBuf = cli
            .config
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(ClientConfig::default_path);
        let cfg = ClientConfig::load_or_default(&cfg_path).unwrap_or_default();
        let server_addr = cli
            .server
            .clone()
            .unwrap_or_else(|| cfg.server_addr.clone());
        let data_dir = cli
            .data_dir
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("../Build/Client/Debug/Data"));
        run_login_ui(LoginUiConfig {
            data_dir,
            server_addr,
        });
        return;
    }

    if cli.lib_test {
        let lib_path = cli
            .lib_path
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("../Build/Client/Debug/Data/Prguse.Lib"));
        run_lib_test(LibTestConfig {
            lib_path,
            lib_index: cli.lib_index,
        });
        return;
    }

    let cfg_path: PathBuf = cli
        .config
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(ClientConfig::default_path);

    let cfg = ClientConfig::load_or_default(&cfg_path).unwrap_or_default();
    let server_addr = cli
        .server
        .clone()
        .unwrap_or_else(|| cfg.server_addr.clone());
    let account = cli.account.clone().or(cfg.account.clone());
    let start = cli.start.or(cfg.character_index);

    let runtime = RuntimeConfig {
        server_addr,
        account,
        password: cli.password.clone(),
        start,
        config_path: cfg_path.to_string_lossy().to_string(),
    };

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

fn run_lib_test(cfg: LibTestConfig) {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(cfg)
        .add_systems(Startup, lib_test_setup)
        .run();
}

fn lib_test_setup(
    mut commands: Commands,
    cfg: Res<LibTestConfig>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.spawn(Camera2dBundle::default());

    let lib = match LibFile::load(&cfg.lib_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[lib-test] load failed: {e}");
            return;
        }
    };

    let img = match lib.get_image(cfg.lib_index) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[lib-test] get_image idx={} failed: {e}", cfg.lib_index);
            return;
        }
    };

    let rgba = img.pixels_rgba();
    let size = Extent3d {
        width: img.width as u32,
        height: img.height as u32,
        depth_or_array_layers: 1,
    };

    let mut bevy_img = Image::new(
        size,
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    bevy_img.sampler = bevy::render::texture::ImageSampler::nearest();

    let handle = images.add(bevy_img);
    commands.spawn(SpriteBundle {
        texture: handle,
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoginField {
    Account,
    Password,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Resource)]
enum LoginUiStage {
    Login,
    Select,
    InGame,
}

#[derive(Resource)]
struct LoginUiNetState {
    server_addr: String,
    net: Option<NetClient>,
    connected: bool,
    just_connected: bool,
    sent_version: bool,
    version_checked: bool,
    pending_login: Option<(String, String)>,
    logged_in: bool,
    characters: Vec<SelectInfo>,
    selected_character_index: Option<i32>,
    pending_start_game: Option<i32>,
    start_game_ok: bool,
    map_index: Option<i32>,
    map_file_name: Option<String>,
    user_name: Option<String>,
    user_location_x: Option<i32>,
    user_location_y: Option<i32>,
    last_error: Option<String>,
}

#[derive(Event)]
struct LoginUiStartLogin;

#[derive(Resource)]
struct LoginUiState {
    account: String,
    password: String,
    focus: LoginField,
    bg_frame: usize,
    bg_timer: Timer,
}

#[derive(Resource)]
struct LoginUiAssets {
    bg_frames: Vec<Handle<Image>>,
    dialog_bg: Handle<Image>,
    title_login: Handle<Image>,
    label_id: Handle<Image>,
    label_pass: Handle<Image>,
    btn_ok: ButtonTriplet,
    btn_new: ButtonTriplet,
    btn_change_pass: ButtonTriplet,
    btn_safe: ButtonTriplet,
    btn_cancel: ButtonTriplet,
}

#[derive(Clone)]
struct ButtonTriplet {
    base: Handle<Image>,
    hover: Handle<Image>,
    pressed: Handle<Image>,
}

#[derive(Component)]
struct LoginBackground;

#[derive(Component)]
struct LoginUiRoot;

#[derive(Component)]
struct LoginUiLoginRoot;

#[derive(Component)]
struct LoginUiSelectRoot;

#[derive(Component)]
struct SelectCharButton(i32);

#[derive(Component)]
struct SelectStartButton;

#[derive(Component)]
struct InGameStatusText;

#[derive(Component)]
struct LoginText(LoginField);

#[derive(Component)]
struct LoginInputArea(LoginField);

#[derive(Component)]
struct LoginInputFrame(LoginField);

#[derive(Component)]
struct LoginCaret;

#[derive(Component)]
struct LoginOkButton;

#[derive(Component)]
struct ButtonEnabled(bool);

#[derive(Resource)]
struct CaretBlink {
    timer: Timer,
    visible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoginButton {
    Ok,
    Account,
    Pass,
    ViewKey,
    Close,
}

#[derive(Component)]
struct LoginButtonKind(LoginButton);

#[derive(Component)]
struct ButtonSkin {
    base: Handle<Image>,
    hover: Handle<Image>,
    pressed: Handle<Image>,
}

#[derive(Component)]
struct ButtonImage;

fn run_login_ui(cfg: LoginUiConfig) {
    let server_addr = cfg.server_addr.clone();
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Crystal M2".to_string(),
                    resolution: (1024.0, 768.0).into(),
                    resizable: false,
                    ..default()
                }),
                ..default()
            }),
        )
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .insert_resource(cfg)
        .insert_resource(LoginUiState {
            account: String::new(),
            password: String::new(),
            focus: LoginField::Account,
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
                login_ui_enter_select,
                login_ui_enter_in_game,
                login_ui_update_in_game_status,
            ),
        )
        .run();
}

fn is_valid_account(s: &str) -> bool {
    // Globals.cs: MinAccountIDLength=3, MaxAccountIDLength=15
    let len = s.chars().count();
    if len < 3 || len > 15 {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric())
}

fn is_valid_password(s: &str) -> bool {
    // Globals.cs: MinPasswordLength=5, MaxPasswordLength=15
    let len = s.chars().count();
    if len < 5 || len > 15 {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric())
}

fn lib_to_image_handle(
    lib: &LibFile,
    index: usize,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>, crystal_lib::LibError> {
    let img = lib.get_image(index)?;
    let rgba = img.pixels_rgba();
    let size = Extent3d {
        width: img.width as u32,
        height: img.height as u32,
        depth_or_array_layers: 1,
    };

    let mut bevy_img = Image::new(
        size,
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    bevy_img.sampler = bevy::render::texture::ImageSampler::nearest();
    Ok(images.add(bevy_img))
}

fn lib_to_sprite(
    lib: &LibFile,
    index: usize,
    images: &mut Assets<Image>,
) -> Result<(Handle<Image>, u32, u32), crystal_lib::LibError> {
    let img = lib.get_image(index)?;
    let w = img.width as u32;
    let h = img.height as u32;
    let rgba = img.pixels_rgba();
    let size = Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: 1,
    };

    let mut bevy_img = Image::new(
        size,
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    bevy_img.sampler = bevy::render::texture::ImageSampler::nearest();
    Ok((images.add(bevy_img), w, h))
}

fn login_ui_setup(
    mut commands: Commands,
    cfg: Res<LoginUiConfig>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.spawn(Camera2dBundle::default());

    let chrsel_path = cfg.data_dir.join("ChrSel.Lib");
    let prguse_path = cfg.data_dir.join("Prguse.Lib");
    let title_path = cfg.data_dir.join("Title.Lib");

    let chrsel = match LibFile::load(&chrsel_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[login-ui] load ChrSel failed: {e}");
            return;
        }
    };
    let prguse = match LibFile::load(&prguse_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[login-ui] load Prguse failed: {e}");
            return;
        }
    };
    let title = match LibFile::load(&title_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[login-ui] load Title failed: {e}");
            return;
        }
    };

    let mut bg_frames = Vec::new();
    for i in 0..19 {
        match lib_to_image_handle(&chrsel, i, &mut images) {
            Ok(h) => bg_frames.push(h),
            Err(e) => {
                eprintln!("[login-ui] load ChrSel idx={i} failed: {e}");
                return;
            }
        }
    }

    let (dialog_bg, dialog_w, dialog_h) = match lib_to_sprite(&prguse, 1084, &mut images) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[login-ui] load Prguse idx=1084 failed: {e}");
            return;
        }
    };

    let (title_login, title_login_w, _title_login_h) = lib_to_sprite(&title, 30, &mut images).unwrap();
    let (label_id, _label_id_w, _label_id_h) = lib_to_sprite(&title, 31, &mut images).unwrap();
    let (label_pass, _label_pass_w, _label_pass_h) = lib_to_sprite(&title, 32, &mut images).unwrap();

    let btn_ok = ButtonTriplet {
        base: lib_to_image_handle(&title, 320, &mut images).unwrap(),
        hover: lib_to_image_handle(&title, 321, &mut images).unwrap(),
        pressed: lib_to_image_handle(&title, 322, &mut images).unwrap(),
    };
    let btn_new = ButtonTriplet {
        base: lib_to_image_handle(&title, 323, &mut images).unwrap(),
        hover: lib_to_image_handle(&title, 324, &mut images).unwrap(),
        pressed: lib_to_image_handle(&title, 325, &mut images).unwrap(),
    };
    let btn_change_pass = ButtonTriplet {
        base: lib_to_image_handle(&title, 326, &mut images).unwrap(),
        hover: lib_to_image_handle(&title, 327, &mut images).unwrap(),
        pressed: lib_to_image_handle(&title, 328, &mut images).unwrap(),
    };
    let btn_safe = ButtonTriplet {
        base: lib_to_image_handle(&title, 332, &mut images).unwrap(),
        hover: lib_to_image_handle(&title, 333, &mut images).unwrap(),
        pressed: lib_to_image_handle(&title, 334, &mut images).unwrap(),
    };
    let btn_cancel = ButtonTriplet {
        base: lib_to_image_handle(&title, 329, &mut images).unwrap(),
        hover: lib_to_image_handle(&title, 330, &mut images).unwrap(),
        pressed: lib_to_image_handle(&title, 331, &mut images).unwrap(),
    };

    let bg0 = bg_frames[0].clone();
    let dialog_bg_ui = dialog_bg.clone();
    let title_login_ui = title_login.clone();
    let label_id_ui = label_id.clone();
    let label_pass_ui = label_pass.clone();
    let btn_ok_ui = btn_ok.clone();
    let btn_new_ui = btn_new.clone();
    let btn_change_pass_ui = btn_change_pass.clone();
    let btn_safe_ui = btn_safe.clone();
    let btn_cancel_ui = btn_cancel.clone();

    commands.insert_resource(LoginUiAssets {
        bg_frames,
        dialog_bg,
        title_login,
        label_id,
        label_pass,
        btn_ok,
        btn_new,
        btn_change_pass,
        btn_safe,
        btn_cancel,
    });

    let ui_root = commands
        .spawn((
            NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        },
            LoginUiRoot,
        ))
        .id();

    commands.entity(ui_root).with_children(|root| {
        // Fullscreen background
        root.spawn((
            ImageBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                image: UiImage::new(bg0.clone()),
                ..default()
            },
            LoginBackground,
        ));

        // Dialog (absolute images inside)
        root.spawn((
            NodeBundle {
            style: Style {
                width: Val::Px(dialog_w as f32),
                height: Val::Px(dialog_h as f32),
                position_type: PositionType::Relative,
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        },
            LoginUiLoginRoot,
        ))
        .with_children(|dlg| {
            // Dialog background image
            dlg.spawn(ImageBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(dialog_w as f32),
                    height: Val::Px(dialog_h as f32),
                    ..default()
                },
                image: UiImage::new(dialog_bg_ui.clone()),
                ..default()
            });

            // Title and labels
            let title_x = ((dialog_w as i32 - title_login_w as i32) / 2).max(0) as f32;
            dlg.spawn(ImageBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(title_x),
                    top: Val::Px(12.0),
                    ..default()
                },
                image: UiImage::new(title_login_ui.clone()),
                ..default()
            });

            dlg.spawn(ImageBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(52.0),
                    top: Val::Px(83.0),
                    ..default()
                },
                image: UiImage::new(label_id_ui.clone()),
                ..default()
            });

            dlg.spawn(ImageBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(43.0),
                    top: Val::Px(105.0),
                    ..default()
                },
                image: UiImage::new(label_pass_ui.clone()),
                ..default()
            });

            // Input boxes (C# MirTextBox is black background; border shows only when non-empty)
            spawn_input_box(dlg, LoginField::Account, 85.0, 85.0, 136.0, 15.0);
            spawn_input_box(dlg, LoginField::Password, 85.0, 108.0, 136.0, 15.0);

            // Focus areas (transparent buttons over input boxes)
            spawn_focus_area(dlg, LoginField::Account, 85.0, 85.0, 136.0, 15.0);
            spawn_focus_area(dlg, LoginField::Password, 85.0, 108.0, 136.0, 15.0);

            // Input text overlays
            dlg.spawn((
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font_size: 16.0,
                        color: Color::WHITE,
                        ..default()
                    },
                )
                .with_style(Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(85.0),
                    top: Val::Px(85.0),
                    ..default()
                }),
                LoginText(LoginField::Account),
            ));
            dlg.spawn((
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font_size: 16.0,
                        color: Color::WHITE,
                        ..default()
                    },
                )
                .with_style(Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(85.0),
                    top: Val::Px(108.0),
                    ..default()
                }),
                LoginText(LoginField::Password),
            ));

            dlg.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(85.0),
                        top: Val::Px(85.0),
                        width: Val::Px(1.0),
                        height: Val::Px(15.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::WHITE),
                    ..default()
                },
                LoginCaret,
            ));

            // Buttons with image children for skinning
            spawn_login_ok_button(dlg, &btn_ok_ui, 227.0, 81.0, 42.0, 42.0);
            spawn_login_button(dlg, LoginButton::Account, &btn_new_ui, 60.0, 163.0, 90.0, 22.0);
            spawn_login_button(
                dlg,
                LoginButton::Pass,
                &btn_change_pass_ui,
                166.0,
                163.0,
                90.0,
                22.0,
            );

            spawn_login_button(dlg, LoginButton::ViewKey, &btn_safe_ui, 60.0, 189.0, 90.0, 22.0);
            spawn_login_button(dlg, LoginButton::Close, &btn_cancel_ui, 288.0, 4.0, 32.0, 32.0);
        });
    });
}

fn login_ui_begin_login(
    mut ev: EventReader<LoginUiStartLogin>,
    state: Res<LoginUiState>,
    mut net_state: ResMut<LoginUiNetState>,
) {
    if ev.read().next().is_none() {
        return;
    }

    if !is_valid_account(&state.account) || !is_valid_password(&state.password) {
        return;
    }

    net_state.pending_login = Some((state.account.clone(), state.password.clone()));

    if net_state.net.is_some() {
        return;
    }

    match NetClient::connect(&net_state.server_addr) {
        Ok(net) => {
            net_state.net = Some(net);
            net_state.connected = false;
            net_state.just_connected = false;
            net_state.sent_version = false;
            net_state.version_checked = false;
            net_state.logged_in = false;
            net_state.characters.clear();
            net_state.selected_character_index = None;
            net_state.pending_start_game = None;
            net_state.start_game_ok = false;
            net_state.map_index = None;
            net_state.map_file_name = None;
            net_state.user_name = None;
            net_state.user_location_x = None;
            net_state.user_location_y = None;
            net_state.last_error = None;
        }
        Err(e) => {
            net_state.last_error = Some(format!("connect error: {e}"));
        }
    }
}

fn login_ui_pump_net(mut stage: ResMut<LoginUiStage>, mut net_state: ResMut<LoginUiNetState>) {
    let mut events = Vec::new();
    if let Some(net) = net_state.net.as_mut() {
        while let Some(ev) = net.try_recv() {
            events.push(ev);
        }
    } else {
        return;
    }

    net_state.just_connected = false;

    for ev in events {
        match ev {
            NetEvent::Connected => {
                net_state.connected = true;
                net_state.just_connected = true;
            }
            NetEvent::Disconnected => {
                net_state.connected = false;
                net_state.sent_version = false;
                net_state.version_checked = false;
                net_state.logged_in = false;
                net_state.characters.clear();
                net_state.selected_character_index = None;
                net_state.pending_start_game = None;
                net_state.start_game_ok = false;
                net_state.map_index = None;
                net_state.map_file_name = None;
                net_state.user_name = None;
                net_state.user_location_x = None;
                net_state.user_location_y = None;
            }
            NetEvent::Error(e) => {
                net_state.last_error = Some(e);
            }
            NetEvent::Packet(pkt) => {
                if pkt.id == ServerPacketId::ClientVersion as i16 {
                    if let Ok(msg) = SClientVersion::decode(&pkt.payload) {
                        net_state.version_checked = msg.result == 1;
                    }
                } else if pkt.id == ServerPacketId::Login as i16 {
                    if let Ok(msg) = SLogin::decode(&pkt.payload) {
                        if msg.result != 1 {
                            net_state.last_error = Some(format!("login failed result={}", msg.result));
                        }
                    }
                } else if pkt.id == ServerPacketId::LoginBanned as i16 {
                    if let Ok(msg) = SLoginBanned::decode(&pkt.payload) {
                        net_state.last_error = Some(format!("banned: {}", msg.reason));
                    }
                } else if pkt.id == ServerPacketId::LoginSuccess as i16 {
                    if let Ok(msg) = SLoginSuccess::decode(&pkt.payload) {
                        net_state.logged_in = true;
                        net_state.characters = msg.characters;
                        net_state.selected_character_index = net_state.characters.first().map(|c| c.index);
                        *stage = LoginUiStage::Select;
                    }
                } else if pkt.id == ServerPacketId::StartGame as i16 {
                    if let Ok(_msg) = SStartGame::decode(&pkt.payload) {
                        net_state.start_game_ok = true;
                        *stage = LoginUiStage::InGame;
                    }
                } else if pkt.id == ServerPacketId::StartGameDelay as i16 {
                    if let Ok(msg) = SStartGameDelay::decode(&pkt.payload) {
                        net_state.last_error = Some(format!("start game delay: {}", msg.milliseconds));
                    }
                } else if pkt.id == ServerPacketId::StartGameBanned as i16 {
                    if let Ok(msg) = SStartGameBanned::decode(&pkt.payload) {
                        net_state.last_error = Some(format!("start game banned: {}", msg.reason));
                    }
                } else if pkt.id == ServerPacketId::MapInformation as i16 {
                    if let Ok(msg) = SMapInformation::decode(&pkt.payload) {
                        net_state.map_index = Some(msg.map_index);
                        net_state.map_file_name = Some(msg.file_name);
                        *stage = LoginUiStage::InGame;
                    }
                } else if pkt.id == ServerPacketId::MapChanged as i16 {
                    if let Ok(msg) = SMapChanged::decode(&pkt.payload) {
                        net_state.map_index = Some(msg.map_index);
                        net_state.map_file_name = Some(msg.file_name);
                        *stage = LoginUiStage::InGame;
                    }
                } else if pkt.id == ServerPacketId::UserInformation as i16 {
                    if let Ok(msg) = SUserInformation::decode(&pkt.payload) {
                        net_state.user_name = Some(msg.name);
                        *stage = LoginUiStage::InGame;
                    }
                } else if pkt.id == ServerPacketId::UserLocation as i16 {
                    if let Ok(msg) = SUserLocation::decode(&pkt.payload) {
                        net_state.user_location_x = Some(msg.location_x);
                        net_state.user_location_y = Some(msg.location_y);
                        *stage = LoginUiStage::InGame;
                    }
                }
            }
        }
    }

    if net_state.just_connected && !net_state.sent_version {
        if let Some(net) = net_state.net.as_ref() {
            if let Ok(pkt) = (CClientVersion {
                version_hash: Vec::new(),
            })
            .encode()
            {
                let _ = net.send_raw(pkt);
                net_state.sent_version = true;
            }
        }
    }

    if net_state.connected && net_state.version_checked {
        if let Some((account_id, password)) = net_state.pending_login.take() {
            if let Some(net) = net_state.net.as_ref() {
                match (CLogin { account_id, password }).encode() {
                    Ok(pkt) => {
                        let _ = net.send_raw(pkt);
                    }
                    Err(e) => {
                        net_state.last_error = Some(format!("CLogin encode error: {e}"));
                    }
                }
            }
        }
    }

    if net_state.connected && net_state.logged_in {
        if let Some(character_index) = net_state.pending_start_game.take() {
            if let Some(net) = net_state.net.as_ref() {
                match (CStartGame { character_index }).encode() {
                    Ok(pkt) => {
                        let _ = net.send_raw(pkt);
                    }
                    Err(e) => {
                        net_state.last_error = Some(format!("CStartGame encode error: {e}"));
                    }
                }
            }
        }
    }
}

fn login_ui_enter_select(
    mut commands: Commands,
    stage: Res<LoginUiStage>,
    net_state: Res<LoginUiNetState>,
    root_q: Query<Entity, With<LoginUiRoot>>,
    login_root_q: Query<Entity, With<LoginUiLoginRoot>>,
    select_root_q: Query<Entity, With<LoginUiSelectRoot>>,
) {
    if !stage.is_changed() {
        return;
    }
    if *stage != LoginUiStage::Select {
        return;
    }

    for e in login_root_q.iter() {
        commands.entity(e).despawn_recursive();
    }
    for e in select_root_q.iter() {
        commands.entity(e).despawn_recursive();
    }

    let Ok(ui_root) = root_q.get_single() else {
        return;
    };

    commands.entity(ui_root).with_children(|root| {
        root.spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(420.0),
                    height: Val::Px(320.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Stretch,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
                ..default()
            },
            LoginUiSelectRoot,
        ))
        .with_children(|panel| {
            panel.spawn(TextBundle::from_section(
                "Select Character",
                TextStyle {
                    font_size: 22.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));

            panel
                .spawn(NodeBundle {
                    style: Style {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::NONE),
                    ..default()
                })
                .with_children(|list| {
                    for ch in net_state.characters.iter() {
                        list.spawn((
                            ButtonBundle {
                                style: Style {
                                    height: Val::Px(28.0),
                                    padding: UiRect::horizontal(Val::Px(8.0)),
                                    justify_content: JustifyContent::FlexStart,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
                                ..default()
                            },
                            SelectCharButton(ch.index),
                        ))
                        .with_children(|b| {
                            b.spawn(TextBundle::from_section(
                                format!("{}  Lv{}", ch.name, ch.level),
                                TextStyle {
                                    font_size: 18.0,
                                    color: Color::WHITE,
                                    ..default()
                                },
                            ));
                        });
                    }
                });

            panel
                .spawn((
                    ButtonBundle {
                        style: Style {
                            height: Val::Px(34.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                        ..default()
                    },
                    SelectStartButton,
                ))
                .with_children(|b| {
                    b.spawn(TextBundle::from_section(
                        "Start",
                        TextStyle {
                            font_size: 18.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ));
                });
        });
    });
}

fn login_ui_select_handle_buttons(
    mut net_state: ResMut<LoginUiNetState>,
    q_chars: Query<(&Interaction, &SelectCharButton), (Changed<Interaction>, With<Button>)>,
    q_start: Query<&Interaction, (Changed<Interaction>, With<Button>, With<SelectStartButton>)>,
) {
    for (interaction, btn) in q_chars.iter() {
        if *interaction == Interaction::Pressed {
            net_state.selected_character_index = Some(btn.0);
        }
    }

    for interaction in q_start.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(idx) = net_state.selected_character_index {
                net_state.pending_start_game = Some(idx);
            }
        }
    }
}

fn spawn_focus_area(parent: &mut ChildBuilder, field: LoginField, x: f32, y: f32, w: f32, h: f32) {
    parent.spawn((
        ButtonBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(x),
                top: Val::Px(y),
                width: Val::Px(w),
                height: Val::Px(h),
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        },
        LoginInputArea(field),
    ));
}

fn spawn_input_box(parent: &mut ChildBuilder, field: LoginField, x: f32, y: f32, w: f32, h: f32) {
    parent.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(x),
                top: Val::Px(y),
                width: Val::Px(w),
                height: Val::Px(h),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::BLACK),
            border_color: BorderColor(Color::rgba(0.0, 0.0, 0.0, 0.0)),
            ..default()
        },
        LoginInputFrame(field),
    ));
}

fn spawn_login_ok_button(parent: &mut ChildBuilder, skin: &ButtonTriplet, x: f32, y: f32, w: f32, h: f32) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(x),
                    top: Val::Px(y),
                    width: Val::Px(w),
                    height: Val::Px(h),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                ..default()
            },
            LoginButtonKind(LoginButton::Ok),
            LoginOkButton,
            ButtonEnabled(false),
            ButtonSkin {
                base: skin.base.clone(),
                hover: skin.hover.clone(),
                pressed: skin.pressed.clone(),
            },
        ))
        .with_children(|b| {
            b.spawn((
                ImageBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    image: UiImage::new(skin.base.clone()),
                    ..default()
                },
                ButtonImage,
            ));
        });
}

fn spawn_login_button(
    parent: &mut ChildBuilder,
    kind: LoginButton,
    skin: &ButtonTriplet,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(x),
                    top: Val::Px(y),
                    width: Val::Px(w),
                    height: Val::Px(h),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                ..default()
            },
            LoginButtonKind(kind),
            ButtonSkin {
                base: skin.base.clone(),
                hover: skin.hover.clone(),
                pressed: skin.pressed.clone(),
            },
        ))
        .with_children(|b| {
            b.spawn((
                ImageBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    image: UiImage::new(skin.base.clone()),
                    ..default()
                },
                ButtonImage,
            ));
        });
}

fn login_ui_animate_background(
    time: Res<Time>,
    assets: Option<Res<LoginUiAssets>>,
    mut state: ResMut<LoginUiState>,
    mut q: Query<&mut UiImage, With<LoginBackground>>,
) {
    let Some(assets) = assets else {
        return;
    };

    state.bg_timer.tick(time.delta());
    if !state.bg_timer.finished() {
        return;
    }

    state.bg_frame = (state.bg_frame + 1) % assets.bg_frames.len();
    let h = assets.bg_frames[state.bg_frame].clone();
    for mut img in q.iter_mut() {
        img.texture = h.clone();
    }
}

fn login_ui_handle_focus_click(
    mut state: ResMut<LoginUiState>,
    q: Query<(&Interaction, &LoginInputArea), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, area) in q.iter() {
        if *interaction == Interaction::Pressed {
            state.focus = area.0;
        }
    }
}

fn login_ui_handle_text_input(
    mut ev_keys: EventReader<KeyboardInput>,
    mut exit: EventWriter<AppExit>,
    mut ev_login: EventWriter<LoginUiStartLogin>,
    mut state: ResMut<LoginUiState>,
) {
    for ev in ev_keys.read() {
        if !ev.state.is_pressed() {
            continue;
        }

        match ev.key_code {
            KeyCode::Escape => {
                exit.send(AppExit::Success);
            }
            KeyCode::Tab => {
                state.focus = match state.focus {
                    LoginField::Account => LoginField::Password,
                    LoginField::Password => LoginField::Account,
                };
            }
            KeyCode::Backspace => match state.focus {
                LoginField::Account => {
                    state.account.pop();
                }
                LoginField::Password => {
                    state.password.pop();
                }
            },
            KeyCode::Enter => {
                if is_valid_account(&state.account) && is_valid_password(&state.password) {
                    ev_login.send(LoginUiStartLogin);
                }
            }
            KeyCode::Space => match state.focus {
                LoginField::Account => state.account.push(' '),
                LoginField::Password => state.password.push(' '),
            },
            _ => {
                if let Key::Character(s) = &ev.logical_key {
                    if s.chars().all(|c| c.is_control()) {
                        continue;
                    }
                    match state.focus {
                        LoginField::Account => state.account.push_str(s.as_str()),
                        LoginField::Password => state.password.push_str(s.as_str()),
                    }
                }
            }
        }
    }
}

fn login_ui_update_text(state: Res<LoginUiState>, mut q: Query<(&LoginText, &mut Text)>) {
    for (field, mut text) in q.iter_mut() {
        match field.0 {
            LoginField::Account => text.sections[0].value = state.account.clone(),
            LoginField::Password => {
                text.sections[0].value = "*".repeat(state.password.chars().count())
            }
        }
    }
}

fn login_ui_update_button_skins(
    mut q: Query<
        (&Interaction, &ButtonSkin, &Children, Option<&ButtonEnabled>),
        (Changed<Interaction>, With<Button>),
    >,
    mut img_q: Query<&mut UiImage, With<ButtonImage>>,
) {
    for (interaction, skin, children, enabled) in q.iter_mut() {
        let enabled = enabled.map(|v| v.0).unwrap_or(true);
        let tint = if enabled {
            Color::srgb(1.0, 1.0, 1.0)
        } else {
            Color::srgb(0.55, 0.55, 0.55)
        };
        let handle = if !enabled {
            skin.base.clone()
        } else {
            match interaction {
                Interaction::Pressed => skin.pressed.clone(),
                Interaction::Hovered => skin.hover.clone(),
                Interaction::None => skin.base.clone(),
            }
        };
        for &c in children.iter() {
            if let Ok(mut img) = img_q.get_mut(c) {
                img.texture = handle.clone();
                img.color = tint;
            }
        }
    }
}

fn login_ui_update_ok_enabled(
    state: Res<LoginUiState>,
    mut q: Query<(&Interaction, &ButtonSkin, &Children, &mut ButtonEnabled), With<LoginOkButton>>,
    mut img_q: Query<&mut UiImage, With<ButtonImage>>,
) {
    let enabled_now = is_valid_account(&state.account) && is_valid_password(&state.password);

    for (interaction, skin, children, mut enabled) in q.iter_mut() {
        if enabled.0 == enabled_now {
            continue;
        }
        enabled.0 = enabled_now;

        let tint = if enabled.0 {
            Color::srgb(1.0, 1.0, 1.0)
        } else {
            Color::srgb(0.55, 0.55, 0.55)
        };

        let handle = if !enabled.0 {
            skin.base.clone()
        } else {
            match interaction {
                Interaction::Pressed => skin.pressed.clone(),
                Interaction::Hovered => skin.hover.clone(),
                Interaction::None => skin.base.clone(),
            }
        };
        for &c in children.iter() {
            if let Ok(mut img) = img_q.get_mut(c) {
                img.texture = handle.clone();
                img.color = tint;
            }
        }
    }
}

fn login_ui_update_caret(
    time: Res<Time>,
    state: Res<LoginUiState>,
    mut blink: ResMut<CaretBlink>,
    mut caret_q: Query<(&mut Style, &mut Visibility), With<LoginCaret>>,
) {
    // We avoid relying on Bevy UI computed layout types (they vary by Bevy version).
    // For login UI we can use a stable approximation based on character count.
    const FONT_SIZE: f32 = 16.0;
    const CHAR_W: f32 = FONT_SIZE * 0.55; // Arial-like approx

    blink.timer.tick(time.delta());
    if blink.timer.finished() {
        blink.visible = !blink.visible;
    }

    let (base_x, base_y, count) = match state.focus {
        LoginField::Account => (85.0f32, 85.0f32, state.account.chars().count()),
        LoginField::Password => (85.0f32, 108.0f32, state.password.chars().count()),
    };
    let w = count as f32 * CHAR_W;
    let x = (base_x + w).min(base_x + 136.0 - 1.0);

    for (mut style, mut vis) in caret_q.iter_mut() {
        style.left = Val::Px(x);
        style.top = Val::Px(base_y);
        *vis = if blink.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn login_ui_handle_buttons(
    mut exit: EventWriter<AppExit>,
    mut ev_login: EventWriter<LoginUiStartLogin>,
    state: Res<LoginUiState>,
    q: Query<(&Interaction, &LoginButtonKind, Option<&ButtonEnabled>), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, btn, enabled) in q.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if enabled.map(|v| v.0).unwrap_or(true) == false {
            continue;
        }

        match btn.0 {
            LoginButton::Close => {
                exit.send(AppExit::Success);
            }
            LoginButton::Ok => {
                if is_valid_account(&state.account) && is_valid_password(&state.password) {
                    ev_login.send(LoginUiStartLogin);
                }
            }
            LoginButton::Account => eprintln!("[login-ui] ACCOUNT"),
            LoginButton::Pass => eprintln!("[login-ui] PASS"),
            LoginButton::ViewKey => eprintln!("[login-ui] VIEW KEY"),
        }
    }
}

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct ChatLogText;

#[derive(Component)]
struct ChatInputText;

#[derive(Component)]
struct ChatStatusText;

#[derive(Clone, Debug)]
enum ObjKind {
    Player,
    Hero,
    Monster,
    Npc,
    Unknown,
}

#[derive(Clone, Debug)]
struct ObjState {
    kind: ObjKind,
    name: Option<String>,
    x: i32,
    y: i32,
    dir: u8,
}

#[derive(Resource)]
struct ChatState {
    input: String,
    lines: Vec<String>,
    connected: bool,
    net: Option<NetClient>,
    last_error: Option<String>,
    sent_version: bool,
    version_checked: bool,
    logged_in: bool,
    pending_login: Option<(String, String)>,
    pending_start_game: Option<i32>,
    keepalive_timer: Timer,
    map_index: Option<i32>,
    map_file_name: Option<String>,
    user_object_id: Option<u32>,
    user_name: Option<String>,
    user_location_x: Option<i32>,
    user_location_y: Option<i32>,
    user_direction: Option<u8>,
    user_hp: Option<i32>,
    user_mp: Option<i32>,
    unhandled_log_path: String,
    unhandled_log_notified: bool,
    unhandled_log_failed: bool,
    unhandled_counts: HashMap<i16, u32>,
    key_log_path: String,
    key_log_notified: bool,
    key_log_failed: bool,
    objects: HashMap<u32, ObjState>,
}

impl Default for ChatState {
    fn default() -> Self {
        ChatState {
            input: String::new(),
            lines: Vec::new(),
            connected: false,
            net: None,
            last_error: None,
            sent_version: false,
            version_checked: false,
            logged_in: false,
            pending_login: None,
            pending_start_game: None,
            keepalive_timer: Timer::from_seconds(5.0, TimerMode::Repeating),
            map_index: None,
            map_file_name: None,
            user_object_id: None,
            user_name: None,
            user_location_x: None,
            user_location_y: None,
            user_direction: None,
            user_hp: None,
            user_mp: None,
            unhandled_log_path: "logs/unhandled_packets.log".to_string(),
            unhandled_log_notified: false,
            unhandled_log_failed: false,
            unhandled_counts: HashMap::new(),
            key_log_path: "logs/key_events.log".to_string(),
            key_log_notified: false,
            key_log_failed: false,
            objects: HashMap::new(),
        }
    }
}

fn log_key_event(chat: &mut ChatState, line: String) {
    if !chat.key_log_notified {
        chat.lines
            .push(format!("[log] key events -> {}", chat.key_log_path));
        chat.key_log_notified = true;
    }

    if chat.key_log_failed {
        return;
    }

    if let Some(parent) = std::path::Path::new(&chat.key_log_path).parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            chat.lines.push(format!("[log] key log init error: {e}"));
            chat.key_log_failed = true;
            return;
        }
    }

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let mut f = match OpenOptions::new().create(true).append(true).open(&chat.key_log_path) {
        Ok(f) => f,
        Err(e) => {
            chat.lines.push(format!("[log] key log open error: {e}"));
            chat.key_log_failed = true;
            return;
        }
    };

    if let Err(e) = writeln!(f, "{}\t{}", now_ms, line) {
        chat.lines.push(format!("[log] key log write error: {e}"));
        chat.key_log_failed = true;
    }
}

fn log_unhandled_packet(chat: &mut ChatState, id: i16, payload_len: usize) {
    if !chat.unhandled_log_notified {
        chat.lines.push(format!("[net] unhandled packets -> {}", chat.unhandled_log_path));
        chat.unhandled_log_notified = true;
    }

    let count = chat.unhandled_counts.entry(id).or_insert(0);
    *count = count.saturating_add(1);
    if *count == 1 {
        chat.lines.push(format!("[net] unhandled server packet id={} (logged)", id));
    }

    if chat.unhandled_log_failed {
        return;
    }

    if let Some(parent) = std::path::Path::new(&chat.unhandled_log_path).parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            chat.lines.push(format!("[net] log init error: {e}"));
            chat.unhandled_log_failed = true;
            return;
        }
    }

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let mut f = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&chat.unhandled_log_path)
    {
        Ok(f) => f,
        Err(e) => {
            chat.lines.push(format!("[net] log open error: {e}"));
            chat.unhandled_log_failed = true;
            return;
        }
    };

    if let Err(e) = writeln!(f, "{}\tid={}\tlen={}", now_ms, id, payload_len) {
        chat.lines.push(format!("[net] log write error: {e}"));
        chat.unhandled_log_failed = true;
    }
}

fn setup(mut commands: Commands, runtime: Res<RuntimeConfig>, mut chat: ResMut<ChatState>) {
    commands.spawn(Camera2dBundle::default());

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

fn update_fps_text(diagnostics: Res<DiagnosticsStore>, mut query: Query<&mut Text, With<FpsText>>) {
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

fn pump_net_events(mut chat: ResMut<ChatState>) {
    let chat = &mut *chat;

    let mut events = Vec::new();
    if let Some(net) = chat.net.as_mut() {
        while let Some(ev) = net.try_recv() {
            events.push(ev);
        }
    } else {
        return;
    }

    let mut became_connected = false;

    for ev in events {
        match ev {
            NetEvent::Connected => {
                chat.connected = true;
                became_connected = true;
                chat.lines.push("[net] connected".to_string());
            }
            NetEvent::Disconnected => {
                chat.connected = false;
                chat.sent_version = false;
                chat.version_checked = false;
                chat.logged_in = false;
                chat.pending_start_game = None;
                chat.lines.push("[net] disconnected".to_string());
            }
            NetEvent::Error(e) => {
                chat.last_error = Some(e.clone());
                chat.lines.push(format!("[net] {e}"));
            }
            NetEvent::Packet(pkt) => {
                if pkt.id == ServerPacketId::ClientVersion as i16 {
                    if let Ok(msg) = SClientVersion::decode(&pkt.payload) {
                        chat.version_checked = msg.result == 1;
                        log_key_event(chat, format!("[login] version result={}", msg.result));
                    }
                } else if pkt.id == ServerPacketId::Login as i16 {
                    if let Ok(msg) = SLogin::decode(&pkt.payload) {
                        log_key_event(chat, format!("[login] login result={}", msg.result));
                    }
                } else if pkt.id == ServerPacketId::LoginBanned as i16 {
                    if let Ok(msg) = SLoginBanned::decode(&pkt.payload) {
                        log_key_event(chat, format!("[login] banned: {}", msg.reason));
                    }
                } else if pkt.id == ServerPacketId::LoginSuccess as i16 {
                    if let Ok(msg) = SLoginSuccess::decode(&pkt.payload) {
                        chat.logged_in = true;
                        log_key_event(
                            chat,
                            format!("[login] login success, chars={}", msg.characters.len()),
                        );
                        for ch in msg.characters {
                            log_key_event(
                                chat,
                                format!(
                                    "[char] index={} name={} level={} class={} gender={} ",
                                    ch.index, ch.name, ch.level, ch.class, ch.gender
                                ),
                            );
                        }
                    }
                } else if pkt.id == ServerPacketId::StartGame as i16 {
                    if let Ok(msg) = SStartGame::decode(&pkt.payload) {
                        log_key_event(
                            chat,
                            format!(
                                "[start_game] result={} resolution={} ",
                                msg.result, msg.resolution
                            ),
                        );
                    }
                } else if pkt.id == ServerPacketId::StartGameBanned as i16 {
                    if let Ok(msg) = SStartGameBanned::decode(&pkt.payload) {
                        log_key_event(chat, format!("[start_game] banned: {}", msg.reason));
                    }
                } else if pkt.id == ServerPacketId::StartGameDelay as i16 {
                    if let Ok(msg) = SStartGameDelay::decode(&pkt.payload) {
                        log_key_event(chat, format!("[start_game] delay ms={}", msg.milliseconds));
                    }
                } else if pkt.id == ServerPacketId::Connected as i16 {
                    let _ = SConnected::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::MapInformation as i16 {
                    if let Ok(msg) = SMapInformation::decode(&pkt.payload) {
                        chat.map_index = Some(msg.map_index);
                        chat.map_file_name = Some(msg.file_name.clone());
                        log_key_event(
                            chat,
                            format!(
                                "[map] info index={} file={} title={} ",
                                msg.map_index, msg.file_name, msg.title
                            ),
                        );
                    }
                } else if pkt.id == ServerPacketId::MapChanged as i16 {
                    if let Ok(msg) = SMapChanged::decode(&pkt.payload) {
                        chat.map_index = Some(msg.map_index);
                        chat.map_file_name = Some(msg.file_name.clone());
                        chat.user_location_x = Some(msg.location_x);
                        chat.user_location_y = Some(msg.location_y);
                        chat.user_direction = Some(msg.direction);
                        log_key_event(
                            chat,
                            format!(
                                "[map] changed index={} file={} x={} y={} dir={} ",
                                msg.map_index,
                                msg.file_name,
                                msg.location_x,
                                msg.location_y,
                                msg.direction
                            ),
                        );
                    }
                } else if pkt.id == ServerPacketId::NewItemInfo as i16 {
                    let _ = SNewItemInfo::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::NewQuestInfo as i16 {
                    let _ = SNewQuestInfo::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::NewRecipeInfo as i16 {
                    let _ = SNewRecipeInfo::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::ObjectNpc as i16 {
                    if let Ok(msg) = SObjectNpc::decode(&pkt.payload) {
                        chat.objects.insert(
                            msg.object_id,
                            ObjState {
                                kind: ObjKind::Npc,
                                name: Some(msg.name),
                                x: msg.location_x,
                                y: msg.location_y,
                                dir: msg.direction,
                            },
                        );
                    }
                } else if pkt.id == ServerPacketId::ObjectPlayer as i16 {
                    if let Ok(msg) = SObjectPlayer::decode(&pkt.payload) {
                        let is_new = !chat.objects.contains_key(&msg.object_id);
                        chat.objects.insert(
                            msg.object_id,
                            ObjState {
                                kind: ObjKind::Player,
                                name: Some(msg.name.clone()),
                                x: msg.location_x,
                                y: msg.location_y,
                                dir: msg.direction,
                            },
                        );
                        if is_new {
                            log_key_event(
                                chat,
                                format!(
                                    "[obj] player spawn id={} name={} x={} y={} dir={} ",
                                    msg.object_id,
                                    msg.name,
                                    msg.location_x,
                                    msg.location_y,
                                    msg.direction
                                ),
                            );
                        }
                    }
                } else if pkt.id == ServerPacketId::ObjectHero as i16 {
                    if let Ok(msg) = SObjectHero::decode(&pkt.payload) {
                        let b = msg.base;
                        let is_new = !chat.objects.contains_key(&b.object_id);
                        chat.objects.insert(
                            b.object_id,
                            ObjState {
                                kind: ObjKind::Hero,
                                name: Some(b.name.clone()),
                                x: b.location_x,
                                y: b.location_y,
                                dir: b.direction,
                            },
                        );
                        if is_new {
                            log_key_event(
                                chat,
                                format!(
                                    "[obj] hero spawn id={} name={} owner={} x={} y={} dir={} ",
                                    b.object_id,
                                    b.name,
                                    msg.owner_name,
                                    b.location_x,
                                    b.location_y,
                                    b.direction
                                ),
                            );
                        }
                    }
                } else if pkt.id == ServerPacketId::ObjectRemove as i16 {
                    if let Ok(msg) = SObjectRemove::decode(&pkt.payload) {
                        if let Some(old) = chat.objects.remove(&msg.object_id) {
                            match old.kind {
                                ObjKind::Player | ObjKind::Hero => {
                                    log_key_event(chat, format!("[obj] remove id={}", msg.object_id));
                                }
                                _ => {}
                            }
                        }
                    }
                } else if pkt.id == ServerPacketId::ObjectRun as i16 {
                    if let Ok(msg) = SObjectRun::decode(&pkt.payload) {
                        let b = msg.0;
                        chat.objects
                            .entry(b.object_id)
                            .and_modify(|s| {
                                s.x = b.location_x;
                                s.y = b.location_y;
                                s.dir = b.direction;
                            })
                            .or_insert(ObjState {
                                kind: ObjKind::Unknown,
                                name: None,
                                x: b.location_x,
                                y: b.location_y,
                                dir: b.direction,
                            });
                    }
                } else if pkt.id == ServerPacketId::CompleteQuest as i16 {
                    let _ = SCompleteQuest::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::ReceiveMail as i16 {
                    let _ = SReceiveMail::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::FriendUpdate as i16 {
                    let _ = SFriendUpdate::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::LoverUpdate as i16 {
                    let _ = SLoverUpdate::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::MentorUpdate as i16 {
                    let _ = SMentorUpdate::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::BaseStatsInfo as i16 {
                    let _ = SBaseStatsInfo::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::TimeOfDay as i16 {
                    let _ = STimeOfDay::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::ChangeAMode as i16 {
                    let _ = SChangeAMode::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::ChangePMode as i16 {
                    let _ = SChangePMode::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::SwitchGroup as i16 {
                    let _ = SSwitchGroup::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::DefaultNPC as i16 {
                    let _ = SDefaultNpc::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::GuildBuffList as i16 {
                    let _ = SGuildBuffList::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::ObjectMonster as i16 {
                    if let Ok(msg) = SObjectMonster::decode(&pkt.payload) {
                        chat.objects
                            .entry(msg.object_id)
                            .or_insert(ObjState {
                                kind: ObjKind::Monster,
                                name: Some(msg.name),
                                x: msg.location_x,
                                y: msg.location_y,
                                dir: msg.direction,
                            });
                    }
                } else if pkt.id == ServerPacketId::AddBuff as i16 {
                    let _ = SAddBuff::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::NPCUpdate as i16 {
                    let _ = SNpcUpdate::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::InTrapRock as i16 {
                    let _ = SInTrapRock::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::ObjectColourChanged as i16 {
                    let _ = SObjectColourChanged::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::UserInformation as i16 {
                    if let Ok(msg) = SUserInformation::decode(&pkt.payload) {
                        chat.user_object_id = Some(msg.object_id);
                        chat.user_name = Some(msg.name.clone());
                        chat.user_location_x = Some(msg.location_x);
                        chat.user_location_y = Some(msg.location_y);
                        chat.user_direction = Some(msg.direction);
                        chat.user_hp = Some(msg.hp);
                        chat.user_mp = Some(msg.mp);
                        log_key_event(
                            chat,
                            format!(
                                "[user] info id={} name={} class={} level={} hp={} mp={} ",
                                msg.object_id, msg.name, msg.class, msg.level, msg.hp, msg.mp
                            ),
                        );
                    } else {
                        chat.lines.push("[user] info decode failed".to_string());
                    }
                } else if pkt.id == ServerPacketId::UserLocation as i16 {
                    if let Ok(msg) = SUserLocation::decode(&pkt.payload) {
                        chat.user_location_x = Some(msg.location_x);
                        chat.user_location_y = Some(msg.location_y);
                        chat.user_direction = Some(msg.direction);
                        log_key_event(
                            chat,
                            format!(
                                "[user] loc x={} y={} dir={} ",
                                msg.location_x, msg.location_y, msg.direction
                            ),
                        );
                    }
                } else if pkt.id == ServerPacketId::HealthChanged as i16 {
                    if let Ok(msg) = SHealthChanged::decode(&pkt.payload) {
                        chat.user_hp = Some(msg.hp);
                        chat.user_mp = Some(msg.mp);
                        log_key_event(chat, format!("[user] hpmp hp={} mp={} ", msg.hp, msg.mp));
                    }
                } else if pkt.id == ServerPacketId::ObjectTeleportIn as i16 {
                    if let Ok(msg) = SObjectTeleportIn::decode(&pkt.payload) {
                        log_key_event(
                            chat,
                            format!(
                                "[obj] teleport_in id={} type={} ",
                                msg.object_id, msg.teleport_type
                            ),
                        );
                    }
                } else if pkt.id == ServerPacketId::GameShopInfo as i16 {
                    if let Ok(msg) = SGameShopInfo::decode(&pkt.payload) {
                        log_key_event(chat, format!("[shop] info bytes={}", msg.info_bytes.len()));
                    }
                } else if pkt.id == ServerPacketId::KeepAlive as i16 {
                    let _ = SKeepAlive::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::ObjectSpell as i16 {
                    if let Ok(msg) = crystal_shared_proto::magic::SObjectSpell::decode(&pkt.payload) {
                        log_key_event(
                            chat,
                            format!(
                                "[obj] spell id={} x={} y={} spell={} dir={} param={} ",
                                msg.object_id,
                                msg.location_x,
                                msg.location_y,
                                msg.spell,
                                msg.direction,
                                msg.param
                            ),
                        );
                    }
                } else if pkt.id == ServerPacketId::ObjectTurn as i16 {
                    if let Ok(msg) = SObjectTurn::decode(&pkt.payload) {
                        let b = msg.0;
                        log_key_event(
                            chat,
                            format!(
                                "[obj] turn id={} x={} y={} dir={} ",
                                b.object_id, b.location_x, b.location_y, b.direction
                            ),
                        );
                    }
                } else if pkt.id == ServerPacketId::ObjectWalk as i16 {
                    if let Ok(msg) = SObjectWalk::decode(&pkt.payload) {
                        let b = msg.0;
                        log_key_event(
                            chat,
                            format!(
                                "[obj] walk id={} x={} y={} dir={} ",
                                b.object_id, b.location_x, b.location_y, b.direction
                            ),
                        );
                    }
                } else if pkt.id == ServerPacketId::ObjectHealth as i16 {
                    if let Ok(msg) = SObjectHealth::decode(&pkt.payload) {
                        log_key_event(
                            chat,
                            format!(
                                "[obj] health id={} percent={} expire={} ",
                                msg.object_id, msg.percent, msg.expire
                            ),
                        );
                    }
                } else if pkt.id == ServerPacketId::NpcResponse as i16 {
                    let _ = SNpcResponse::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::FishingUpdate as i16 {
                    let _ = SFishingUpdate::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::Chat as i16 {
                    if let Ok(msg) = SChat::decode(&pkt.payload) {
                        chat.lines.push(msg.message);
                    }
                } else if pkt.id == ServerPacketId::ObjectChat as i16 {
                    if let Ok(msg) = SObjectChat::decode(&pkt.payload) {
                        chat.lines.push(format!("{}: {}", msg.object_id, msg.text));
                    }
                } else {
                    log_unhandled_packet(chat, pkt.id, pkt.payload.len());
                }
            }
        }
    }

    if became_connected && !chat.sent_version {
        if let Some(net) = chat.net.as_ref() {
            let pkt = CClientVersion {
                version_hash: Vec::new(),
            }
            .encode();
            match pkt {
                Ok(pkt) => {
                    let _ = net.send_raw(pkt);
                    chat.sent_version = true;
                }
                Err(e) => {
                    chat.lines.push(format!("[net] client_version encode error: {e}"));
                }
            }
        }
    }

    if chat.connected && chat.version_checked {
        if let Some((account_id, password)) = chat.pending_login.take() {
            if let Some(net) = chat.net.as_ref() {
                let pkt = CLogin { account_id, password }.encode();
                match pkt {
                    Ok(pkt) => {
                        let _ = net.send_raw(pkt);
                    }
                    Err(e) => {
                        chat.lines.push(format!("[login] encode error: {e}"));
                    }
                }
            }
        }
    }

    if chat.connected && chat.logged_in {
        if let Some(character_index) = chat.pending_start_game.take() {
            if let Some(net) = chat.net.as_ref() {
                let pkt = CStartGame { character_index }.encode();
                match pkt {
                    Ok(pkt) => {
                        let _ = net.send_raw(pkt);
                        chat.lines
                            .push(format!("[start_game] requested index={}", character_index));
                    }
                    Err(e) => {
                        chat.lines.push(format!("[start_game] encode error: {e}"));
                    }
                }
            }
        }
    }
}

fn send_keep_alive(mut chat: ResMut<ChatState>, time: Res<Time>) {
    let chat = &mut *chat;

    if !chat.connected {
        return;
    }
    let Some(net) = chat.net.as_ref() else {
        return;
    };

    chat.keepalive_timer.tick(time.delta());
    if !chat.keepalive_timer.finished() {
        return;
    }

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let pkt = CKeepAlive { time: now_ms }.encode();
    match pkt {
        Ok(pkt) => {
            let _ = net.send_raw(pkt);
        }
        Err(e) => {
            chat.lines.push(format!("[net] keepalive encode error: {e}"));
        }
    }
}

fn handle_chat_input(
    mut chat: ResMut<ChatState>,
    mut ev_keys: EventReader<KeyboardInput>,
) {
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
                        chat.lines.push("[login] usage: /login <account> <password>".to_string());
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
                                chat.lines.push("[start_game] usage: /start <character_index>".to_string());
                            }
                        }
                    } else {
                        chat.lines.push("[start_game] usage: /start <character_index>".to_string());
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

fn update_chat_ui(
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
