use bevy::app::AppExit;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use crystal_lib::LibFile;

use crystal_client_app::LoginUiConfig;
use crate::shared::lib_image::{lib_to_image_handle, lib_to_sprite, lib_to_sprite_with_offset};

use super::state::*;

pub(crate) fn login_ui_setup(
    mut commands: Commands,
    cfg: Res<LoginUiConfig>,
    mut images: ResMut<Assets<Image>>,
    existing_cams: Query<Entity, With<Camera>>,
) {
    let mut cams = existing_cams.iter();
    if let Some(first) = cams.next() {
        for extra in cams {
            commands.entity(extra).despawn_recursive();
        }
        commands.entity(first).insert(Camera {
            order: 0,
            ..default()
        });
    } else {
        commands.spawn(Camera2dBundle::default());
    }

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

    fn img_size(lib: &LibFile, idx: usize) -> (u32, u32) {
        let img = lib.get_image(idx).unwrap();
        (img.width as u32, img.height as u32)
    }

    let select_bg = lib_to_image_handle(&prguse, 65, &mut images).unwrap();
    let select_title = lib_to_image_handle(&title, 40, &mut images).unwrap();
    let select_title_size = img_size(&title, 40);

    let select_btn_start = ButtonTriplet {
        base: lib_to_image_handle(&title, 340, &mut images).unwrap(),
        hover: lib_to_image_handle(&title, 341, &mut images).unwrap(),
        pressed: lib_to_image_handle(&title, 342, &mut images).unwrap(),
    };
    let select_btn_start_size = img_size(&title, 340);
    let select_btn_new = ButtonTriplet {
        base: lib_to_image_handle(&title, 343, &mut images).unwrap(),
        hover: lib_to_image_handle(&title, 344, &mut images).unwrap(),
        pressed: lib_to_image_handle(&title, 345, &mut images).unwrap(),
    };
    let select_btn_new_size = img_size(&title, 343);
    let select_btn_delete = ButtonTriplet {
        base: lib_to_image_handle(&title, 346, &mut images).unwrap(),
        hover: lib_to_image_handle(&title, 347, &mut images).unwrap(),
        pressed: lib_to_image_handle(&title, 348, &mut images).unwrap(),
    };
    let select_btn_delete_size = img_size(&title, 346);
    let select_btn_credits = ButtonTriplet {
        base: lib_to_image_handle(&title, 349, &mut images).unwrap(),
        hover: lib_to_image_handle(&title, 350, &mut images).unwrap(),
        pressed: lib_to_image_handle(&title, 351, &mut images).unwrap(),
    };
    let select_btn_credits_size = img_size(&title, 349);
    let select_btn_exit = ButtonTriplet {
        base: lib_to_image_handle(&title, 352, &mut images).unwrap(),
        hover: lib_to_image_handle(&title, 353, &mut images).unwrap(),
        pressed: lib_to_image_handle(&title, 354, &mut images).unwrap(),
    };
    let select_btn_exit_size = img_size(&title, 352);

    let select_slot_empty = lib_to_image_handle(&prguse, 44, &mut images).unwrap();
    let mut select_slot_size = img_size(&prguse, 44);
    let mut select_slot_unselected = Vec::new();
    let mut select_slot_selected = Vec::new();
    for i in 0..5 {
        select_slot_unselected.push(lib_to_image_handle(&title, 660 + i, &mut images).unwrap());
        select_slot_selected.push(lib_to_image_handle(&title, 665 + i, &mut images).unwrap());
        let (w, h) = img_size(&title, 660 + i);
        select_slot_size.0 = select_slot_size.0.max(w);
        select_slot_size.1 = select_slot_size.1.max(h);
        let (w, h) = img_size(&title, 665 + i);
        select_slot_size.0 = select_slot_size.0.max(w);
        select_slot_size.1 = select_slot_size.1.max(h);
    }

    fn load_chrsel_anim_with_offset(
        chrsel: &LibFile,
        base: usize,
        images: &mut Assets<Image>,
    ) -> (Vec<Handle<Image>>, Vec<(i16, i16)>) {
        let mut frames = Vec::new();
        let mut offsets = Vec::new();
        for i in 0..16 {
            let (h, _w, _h, x, y) = lib_to_sprite_with_offset(chrsel, base + i, images).unwrap();
            frames.push(h);
            offsets.push((x, y));
        }
        (frames, offsets)
    }

    fn load_chrsel_overlay_with_offset(
        chrsel: &LibFile,
        base: usize,
        images: &mut Assets<Image>,
    ) -> (Vec<Handle<Image>>, Vec<(i16, i16)>) {
        // C# AfterDraw: draw blend Index+560 at same display location.
        load_chrsel_anim_with_offset(chrsel, base + 560, images)
    }

    let bases = [20usize, 300, 40, 320, 60, 340, 80, 360, 100, 140];
    let mut select_char_frames = Vec::new();
    let mut select_char_offsets = Vec::new();
    let mut select_char_overlay_frames = Vec::new();
    let mut select_char_overlay_offsets = Vec::new();
    for base in bases {
        let (frames, offsets) = load_chrsel_anim_with_offset(&chrsel, base, &mut images);
        select_char_frames.push(frames);
        select_char_offsets.push(offsets);

        let (oframes, ooffs) = load_chrsel_overlay_with_offset(&chrsel, base, &mut images);
        select_char_overlay_frames.push(oframes);
        select_char_overlay_offsets.push(ooffs);
    }

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
        dialog_w,
        dialog_h,
        title_login,
        title_login_w: title_login_w as u32,
        label_id,
        label_pass,
        btn_ok,
        btn_new,
        btn_change_pass,
        btn_safe,
        btn_cancel,

        select_bg,
        select_title,
        select_title_size,
        select_btn_start,
        select_btn_start_size,
        select_btn_new,
        select_btn_new_size,
        select_btn_delete,
        select_btn_delete_size,
        select_btn_credits,
        select_btn_credits_size,
        select_btn_exit,
        select_btn_exit_size,
        select_slot_empty,
        select_slot_size,
        select_slot_unselected,
        select_slot_selected,
        select_char_frames,
        select_char_offsets,
        select_char_overlay_frames,
        select_char_overlay_offsets,
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
    });

    // Spawn initial login dialog
    spawn_login_dialog(
        &mut commands,
        ui_root,
        dialog_bg_ui,
        dialog_w as u32,
        dialog_h as u32,
        title_login_ui,
        title_login_w as u32,
        label_id_ui,
        label_pass_ui,
        btn_ok_ui,
        btn_new_ui,
        btn_change_pass_ui,
        btn_safe_ui,
        btn_cancel_ui,
    );
}

pub(crate) fn login_ui_enter_login(
    mut commands: Commands,
    stage: Res<LoginUiStage>,
    assets: Option<Res<LoginUiAssets>>,
    state: Res<LoginUiState>,
    root_q: Query<Entity, With<LoginUiRoot>>,
    login_root_q: Query<Entity, With<LoginUiLoginRoot>>,
    select_root_q: Query<Entity, With<LoginUiSelectRoot>>,
    status_q: Query<Entity, With<InGameStatusText>>,
    mut bg_q: Query<&mut UiImage, With<LoginBackground>>,
) {
    if !stage.is_changed() {
        return;
    }
    if *stage != LoginUiStage::Login {
        return;
    }

    let Some(assets) = assets else {
        return;
    };

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

    for mut img in bg_q.iter_mut() {
        if let Some(h) = assets.bg_frames.get(state.bg_frame) {
            img.texture = h.clone();
        }
    }

    spawn_login_dialog(
        &mut commands,
        ui_root,
        assets.dialog_bg.clone(),
        assets.dialog_w,
        assets.dialog_h,
        assets.title_login.clone(),
        assets.title_login_w,
        assets.label_id.clone(),
        assets.label_pass.clone(),
        assets.btn_ok.clone(),
        assets.btn_new.clone(),
        assets.btn_change_pass.clone(),
        assets.btn_safe.clone(),
        assets.btn_cancel.clone(),
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_login_dialog(
    commands: &mut Commands,
    ui_root: Entity,
    dialog_bg_ui: Handle<Image>,
    dialog_w: u32,
    dialog_h: u32,
    title_login_ui: Handle<Image>,
    title_login_w: u32,
    label_id_ui: Handle<Image>,
    label_pass_ui: Handle<Image>,
    btn_ok: ButtonTriplet,
    btn_new: ButtonTriplet,
    btn_change_pass: ButtonTriplet,
    btn_safe: ButtonTriplet,
    btn_cancel: ButtonTriplet,
) {
    let title_x = ((dialog_w as i32 - title_login_w as i32) / 2).max(0) as f32;

    commands.entity(ui_root).with_children(|root| {
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
            spawn_login_ok_button(dlg, &btn_ok, 227.0, 81.0, 42.0, 42.0);
            spawn_login_button(dlg, LoginButton::Account, &btn_new, 60.0, 163.0, 90.0, 22.0);
            spawn_login_button(
                dlg,
                LoginButton::Pass,
                &btn_change_pass,
                166.0,
                163.0,
                90.0,
                22.0,
            );

            spawn_login_button(dlg, LoginButton::ViewKey, &btn_safe, 60.0, 189.0, 90.0, 22.0);
            spawn_login_button(dlg, LoginButton::Close, &btn_cancel, 288.0, 4.0, 32.0, 32.0);
        });
    });
}

pub(crate) fn login_ui_enter_in_game(
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

pub(crate) fn login_ui_update_in_game_status(
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
    s.push_str(&format!(
        "connected={} version_ok={} logged_in={} start_ok={}\n",
        net_state.connected,
        net_state.version_checked,
        net_state.logged_in,
        net_state.start_game_ok
    ));
    s.push_str(&format!("map={:?} {:?}\n", net_state.map_index, net_state.map_file_name));
    s.push_str(&format!(
        "user={:?} loc=({:?},{:?})\n",
        net_state.user_name, net_state.user_location_x, net_state.user_location_y
    ));
    if let Some(e) = &net_state.last_error {
        s.push_str(&format!("error={e}\n"));
    }

    text.sections[0].value = s;
}

pub(crate) fn login_ui_update_input_frames(
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
            border.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
        } else if valid {
            border.0 = Color::srgb(0.0, 1.0, 0.0);
        } else {
            border.0 = Color::srgb(1.0, 0.0, 0.0);
        }
    }
}

pub(crate) fn login_ui_animate_background(
    time: Res<Time>,
    stage: Res<LoginUiStage>,
    assets: Option<Res<LoginUiAssets>>,
    mut state: ResMut<LoginUiState>,
    mut q: Query<&mut UiImage, With<LoginBackground>>,
) {
    if *stage != LoginUiStage::Login {
        return;
    }
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

pub(crate) fn login_ui_handle_focus_click(
    mut state: ResMut<LoginUiState>,
    q: Query<(&Interaction, &LoginInputArea), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, area) in q.iter() {
        if *interaction == Interaction::Pressed {
            state.focus = area.0;
        }
    }
}

pub(crate) fn login_ui_handle_text_input(
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

pub(crate) fn login_ui_update_text(state: Res<LoginUiState>, mut q: Query<(&LoginText, &mut Text)>) {
    for (field, mut text) in q.iter_mut() {
        match field.0 {
            LoginField::Account => text.sections[0].value = state.account.clone(),
            LoginField::Password => {
                text.sections[0].value = "*".repeat(state.password.chars().count())
            }
        }
    }
}

pub(crate) fn login_ui_update_button_skins(
    mut q: Query<
        (&Interaction, &ButtonSkin, &Children, Option<&ButtonEnabled>),
        (Or<(Changed<Interaction>, Changed<ButtonEnabled>)>, With<Button>),
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

pub(crate) fn login_ui_update_ok_enabled(
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

pub(crate) fn login_ui_update_caret(
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

pub(crate) fn login_ui_handle_buttons(
    mut exit: EventWriter<AppExit>,
    mut ev_login: EventWriter<LoginUiStartLogin>,
    state: Res<LoginUiState>,
    q: Query<
        (&Interaction, &LoginButtonKind, Option<&ButtonEnabled>),
        (Changed<Interaction>, With<Button>),
    >,
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
            border_color: BorderColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
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
