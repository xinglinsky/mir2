use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use super::state::{
    ButtonEnabled, ButtonImage, ButtonSkin, LoginBackground, LoginUiAssets, LoginUiLoginRoot,
    LoginUiNetState, LoginUiRoot, LoginUiSelectRoot, LoginUiStage, SelectCharacterDisplay,
    SelectCharacterBaseImage, SelectCharacterDisplayAnim, SelectCharacterOverlayImage,
    SelectCreditsButton, SelectDeleteCharacterButton, SelectExitButton, SelectLastAccessText,
    SelectNewCharacterButton, SelectSlotButton, SelectSlotClassText, SelectSlotImage,
    SelectSlotLevelText, SelectSlotNameText, SelectStartButton, SelectStatusText,
};

fn class_name(class: u8) -> &'static str {
    match class {
        0 => "Warrior",
        1 => "Wizard",
        2 => "Taoist",
        3 => "Assassin",
        4 => "Archer",
        _ => "Unknown",
    }
}

fn select_char_frames_index(class: u8, gender: u8) -> usize {
    let class = class.min(4) as usize;
    let gender = if gender == 0 { 0 } else { 1 };
    class * 2 + gender
}

pub(crate) fn login_ui_enter_select(
    mut commands: Commands,
    stage: Res<LoginUiStage>,
    mut net_state: ResMut<LoginUiNetState>,
    assets: Option<Res<LoginUiAssets>>,
    root_q: Query<Entity, With<LoginUiRoot>>,
    login_root_q: Query<Entity, With<LoginUiLoginRoot>>,
    select_root_q: Query<Entity, With<LoginUiSelectRoot>>,
    mut bg_q: Query<&mut UiImage, With<LoginBackground>>,
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

    net_state
        .characters
        .sort_by(|a, b| b.last_access_binary.cmp(&a.last_access_binary));

    if net_state.selected_character_index.is_none() {
        net_state.selected_character_index = net_state.characters.first().map(|c| c.index);
    }

    let Some(assets) = assets else {
        return;
    };

    for mut img in bg_q.iter_mut() {
        img.texture = assets.select_bg.clone();
    }

    let Ok(ui_root) = root_q.get_single() else {
        return;
    };

    commands.entity(ui_root).with_children(|root| {
        root.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                ..default()
            },
            LoginUiSelectRoot,
        ))
        .with_children(|panel| {
            panel.spawn(ImageBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(468.0),
                    top: Val::Px(20.0),
                    width: Val::Px(assets.select_title_size.0 as f32),
                    height: Val::Px(assets.select_title_size.1 as f32),
                    ..default()
                },
                image: UiImage::new(assets.select_title.clone()),
                ..default()
            });

            // Character slots
            let slot_pos = [(637.0, 194.0), (637.0, 298.0), (637.0, 402.0), (637.0, 506.0)];
            for (i, (x, y)) in slot_pos.into_iter().enumerate() {
                panel
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                position_type: PositionType::Absolute,
                                left: Val::Px(x),
                                top: Val::Px(y),
                                width: Val::Px(assets.select_slot_size.0 as f32),
                                height: Val::Px(assets.select_slot_size.1 as f32),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::NONE),
                            ..default()
                        },
                        SelectSlotButton(i),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            ImageBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                image: UiImage::new(assets.select_slot_empty.clone()),
                                ..default()
                            },
                            SelectSlotImage,
                            SelectSlotButton(i),
                        ));

                        b.spawn((
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
                                left: Val::Px(107.0),
                                top: Val::Px(9.0),
                                ..default()
                            }),
                            SelectSlotNameText,
                            SelectSlotButton(i),
                        ));

                        b.spawn((
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
                                left: Val::Px(107.0),
                                top: Val::Px(28.0),
                                ..default()
                            }),
                            SelectSlotLevelText,
                            SelectSlotButton(i),
                        ));

                        b.spawn((
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
                                left: Val::Px(178.0),
                                top: Val::Px(28.0),
                                ..default()
                            }),
                            SelectSlotClassText,
                            SelectSlotButton(i),
                        ));
                    });
            }

            // Character display (animated)
            panel
                .spawn((
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            left: Val::Px(260.0),
                            top: Val::Px(420.0),
                            width: Val::Px(1.0),
                            height: Val::Px(1.0),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::NONE),
                        visibility: Visibility::Hidden,
                        ..default()
                    },
                    SelectCharacterDisplay,
                    SelectCharacterDisplayAnim {
                        timer: Timer::from_seconds(0.25, TimerMode::Repeating),
                        frame: 0,
                        key: None,
                    },
                ))
                .with_children(|c| {
                    c.spawn((
                        ImageBundle {
                            style: Style {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                ..default()
                            },
                            image: UiImage::new(assets.select_slot_empty.clone()),
                            ..default()
                        },
                        SelectCharacterBaseImage,
                    ));

                    c.spawn((
                        ImageBundle {
                            style: Style {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                ..default()
                            },
                            image: UiImage {
                                texture: assets.select_slot_empty.clone(),
                                color: Color::srgba(1.0, 1.0, 1.0, 0.85),
                                ..default()
                            },
                            ..default()
                        },
                        SelectCharacterOverlayImage,
                    ));
                });

            // Last access label
            panel.spawn((
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
                    left: Val::Px(265.0),
                    top: Val::Px(609.0),
                    ..default()
                }),
                SelectLastAccessText,
            ));

            panel.spawn((
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
                    left: Val::Px(265.0),
                    top: Val::Px(630.0),
                    ..default()
                }),
                SelectStatusText,
                Visibility::Hidden,
            ));

            // Server label (C# Location=(432,60), Size=(155,17), centered)
            panel.spawn(
                TextBundle::from_section(
                    "Crystal M2",
                    TextStyle {
                        font_size: 16.0,
                        color: Color::WHITE,
                        ..default()
                    },
                )
                .with_style(Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(432.0),
                    top: Val::Px(60.0),
                    width: Val::Px(155.0),
                    height: Val::Px(17.0),
                    ..default()
                }),
            );

            // Bottom buttons (C# xPoint = ((ScreenWidth - 200) / 5);  ScreenWidth=1024
            let x_point = (1024.0 - 200.0) / 5.0;
            let y = 768.0 - 32.0;

            spawn_select_button(
                panel,
                &assets.select_btn_start,
                assets.select_btn_start_size,
                100.0 + x_point * 1.0 - x_point / 2.0 - 50.0,
                y,
                SelectStartButton,
                !net_state.characters.is_empty(),
            );
            spawn_select_button(panel, &assets.select_btn_new, assets.select_btn_new_size, 100.0 + x_point * 2.0 - x_point / 2.0 - 50.0, y, SelectNewCharacterButton, true);
            spawn_select_button(panel, &assets.select_btn_delete, assets.select_btn_delete_size, 100.0 + x_point * 3.0 - x_point / 2.0 - 50.0, y, SelectDeleteCharacterButton, true);
            spawn_select_button(panel, &assets.select_btn_credits, assets.select_btn_credits_size, 100.0 + x_point * 4.0 - x_point / 2.0 - 50.0, y, SelectCreditsButton, true);
            spawn_select_button(panel, &assets.select_btn_exit, assets.select_btn_exit_size, 100.0 + x_point * 5.0 - x_point / 2.0 - 50.0, y, SelectExitButton, true);
        });
    });
}

fn spawn_select_button<T: Component>(
    parent: &mut ChildBuilder,
    skin: &super::state::ButtonTriplet,
    size: (u32, u32),
    x: f32,
    y: f32,
    marker: T,
    enabled: bool,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(x),
                    top: Val::Px(y),
                    width: Val::Px(size.0 as f32),
                    height: Val::Px(size.1 as f32),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                ..default()
            },
            marker,
            ButtonEnabled(enabled),
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

pub(crate) fn login_ui_select_handle_buttons(
    mut stage: ResMut<LoginUiStage>,
    mut net_state: ResMut<LoginUiNetState>,
    q_slots: Query<(&Interaction, &SelectSlotButton), (Changed<Interaction>, With<Button>)>,
    q_start: Query<(&Interaction, &ButtonEnabled), (Changed<Interaction>, With<Button>, With<SelectStartButton>)>,
    q_new: Query<&Interaction, (Changed<Interaction>, With<Button>, With<SelectNewCharacterButton>)>,
    q_delete: Query<&Interaction, (Changed<Interaction>, With<Button>, With<SelectDeleteCharacterButton>)>,
    q_exit: Query<&Interaction, (Changed<Interaction>, With<Button>, With<SelectExitButton>)>,
) {
    for (interaction, btn) in q_slots.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(ch) = net_state.characters.get(btn.0) {
                net_state.selected_character_index = Some(ch.index);
            }
        }
    }

    for (interaction, enabled) in q_start.iter() {
        if *interaction == Interaction::Pressed && enabled.0 {
            if let Some(idx) = net_state.selected_character_index {
                net_state.pending_start_game = Some(idx);
            }
        }
    }

    for interaction in q_new.iter() {
        if *interaction == Interaction::Pressed {
            net_state.last_error = Some("New Character: not implemented yet".to_string());
        }
    }

    for interaction in q_delete.iter() {
        if *interaction == Interaction::Pressed {
            if net_state.selected_character_index.is_some() {
                net_state.last_error = Some("Delete Character: not implemented yet".to_string());
            } else {
                net_state.last_error = Some("Select a character to delete".to_string());
            }
        }
    }

    for interaction in q_exit.iter() {
        if *interaction == Interaction::Pressed {
            *stage = LoginUiStage::Login;
            // Go back to a clean login flow (reconnect on next OK)
            net_state.net = None;
            net_state.connected = false;
            net_state.just_connected = false;
            net_state.sent_version = false;
            net_state.version_checked = false;
            net_state.pending_login = None;
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
    }
}

pub(crate) fn login_ui_select_handle_keypress(
    stage: Res<LoginUiStage>,
    mut net_state: ResMut<LoginUiNetState>,
    mut ev_keys: EventReader<KeyboardInput>,
) {
    if *stage != LoginUiStage::Select {
        return;
    }

    for ev in ev_keys.read() {
        if !ev.state.is_pressed() {
            continue;
        }
        if ev.key_code == KeyCode::Enter {
            if let Some(idx) = net_state.selected_character_index {
                net_state.pending_start_game = Some(idx);
            }
        }
    }
}

pub(crate) fn login_ui_select_update_interface(
    stage: Res<LoginUiStage>,
    assets: Option<Res<LoginUiAssets>>,
    net_state: Res<LoginUiNetState>,
    mut q_start: Query<&mut ButtonEnabled, With<SelectStartButton>>,
    mut image_queries: ParamSet<(
        Query<(&SelectSlotButton, &mut UiImage), With<SelectSlotImage>>,
        Query<(&mut UiImage, &mut Style), With<SelectCharacterBaseImage>>,
        Query<(&mut UiImage, &mut Style), With<SelectCharacterOverlayImage>>,
    )>,
    mut text_vis_queries: ParamSet<(
        Query<(&SelectSlotButton, &mut Text), With<SelectSlotNameText>>,
        Query<(&SelectSlotButton, &mut Text), With<SelectSlotLevelText>>,
        Query<(&SelectSlotButton, &mut Text), With<SelectSlotClassText>>,
        Query<(&mut Text, &mut Visibility), With<SelectLastAccessText>>,
        Query<(&mut Text, &mut Visibility), With<SelectStatusText>>,
        Query<(&mut Visibility, &mut SelectCharacterDisplayAnim), With<SelectCharacterDisplay>>,
    )>,
) {
    if *stage != LoginUiStage::Select {
        return;
    }
    let Some(assets) = assets else {
        return;
    };

    let selected_idx = net_state.selected_character_index;

    // 使用 ParamSet 来避免查询冲突
    for (btn, mut img) in image_queries.p0().iter_mut() {
        let slot = btn.0;
        let Some(ch) = net_state.characters.get(slot) else {
            img.texture = assets.select_slot_empty.clone();
            continue;
        };

        let cls = (ch.class.min(4)) as usize;
        let is_selected = Some(ch.index) == selected_idx;
        img.texture = if is_selected {
            assets
                .select_slot_selected
                .get(cls)
                .cloned()
                .unwrap_or_else(|| assets.select_slot_empty.clone())
        } else {
            assets
                .select_slot_unselected
                .get(cls)
                .cloned()
                .unwrap_or_else(|| assets.select_slot_empty.clone())
        };
    }

    // 使用 ParamSet 来避免查询冲突
    for (btn, mut text) in text_vis_queries.p0().iter_mut() {
        let slot = btn.0;
        text.sections[0].value = net_state
            .characters
            .get(slot)
            .map(|c| c.name.clone())
            .unwrap_or_default();
    }
    
    for (btn, mut text) in text_vis_queries.p1().iter_mut() {
        let slot = btn.0;
        text.sections[0].value = net_state
            .characters
            .get(slot)
            .map(|c| c.level.to_string())
            .unwrap_or_default();
    }
    
    for (btn, mut text) in text_vis_queries.p2().iter_mut() {
        let slot = btn.0;
        text.sections[0].value = net_state
            .characters
            .get(slot)
            .map(|c| class_name(c.class).to_string())
            .unwrap_or_default();
    }

    let selected = selected_idx
        .and_then(|idx| net_state.characters.iter().find(|c| c.index == idx));

    if let Ok(mut enabled) = q_start.get_single_mut() {
        enabled.0 = selected.is_some();
    }

    if let Ok((mut last, mut vis)) = text_vis_queries.p3().get_single_mut() {
        if let Some(ch) = selected {
            *vis = Visibility::Visible;
            let v = if ch.last_access_binary == 0 {
                "Never".to_string()
            } else {
                format!("{}", ch.last_access_binary)
            };
            last.sections[0].value = format!("Last Online: {v}");
        } else {
            *vis = Visibility::Hidden;
            last.sections[0].value = "".to_string();
        }
    }

    if let Ok((mut status, mut vis)) = text_vis_queries.p4().get_single_mut() {
        if let Some(s) = &net_state.last_error {
            *vis = Visibility::Visible;
            status.sections[0].value = s.clone();
        } else {
            *vis = Visibility::Hidden;
            status.sections[0].value = "".to_string();
        }
    }

    if let Ok((mut vis, mut anim)) = text_vis_queries.p5().get_single_mut() {
        let key = selected.map(|c| (c.class, c.gender));
        if anim.key != key {
            anim.key = key;
            anim.frame = 0;
            anim.timer.reset();
        }

        *vis = if anim.key.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        if let Some((class, gender)) = anim.key {
            let idx = select_char_frames_index(class, gender);

            if let (Some(frames), Some(offsets)) = (
                assets.select_char_frames.get(idx),
                assets.select_char_offsets.get(idx),
            ) {
                if let Some(h) = frames.first() {
                    if let Ok((mut img, mut style)) = image_queries.p1().get_single_mut() {
                        img.texture = h.clone();
                        if let Some((x, y)) = offsets.first().copied() {
                            style.left = Val::Px(x as f32);
                            style.top = Val::Px(y as f32);
                        }
                    }
                }
            }

            if let (Some(frames), Some(offsets)) = (
                assets.select_char_overlay_frames.get(idx),
                assets.select_char_overlay_offsets.get(idx),
            ) {
                if let Some(h) = frames.first() {
                    if let Ok((mut img, mut style)) = image_queries.p2().get_single_mut() {
                        img.texture = h.clone();
                        if let Some((x, y)) = offsets.first().copied() {
                            style.left = Val::Px(x as f32);
                            style.top = Val::Px(y as f32);
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn login_ui_select_animate_character_display(
    stage: Res<LoginUiStage>,
    time: Res<Time>,
    assets: Option<Res<LoginUiAssets>>,
    mut q_disp: Query<(&mut Visibility, &mut SelectCharacterDisplayAnim), With<SelectCharacterDisplay>>,
    mut image_queries: ParamSet<(
        Query<(&mut UiImage, &mut Style), With<SelectCharacterBaseImage>>,
        Query<(&mut UiImage, &mut Style), With<SelectCharacterOverlayImage>>,
    )>,
) {
    if *stage != LoginUiStage::Select {
        return;
    }
    let Some(assets) = assets else {
        return;
    };

    let Ok((_vis, mut anim)) = q_disp.get_single_mut() else {
        return;
    };

    let Some((class, gender)) = anim.key else {
        return;
    };

    let idx = select_char_frames_index(class, gender);

    anim.timer.tick(time.delta());
    if !anim.timer.finished() {
        return;
    }

    if let (Some(frames), Some(offsets)) = (
        assets.select_char_frames.get(idx),
        assets.select_char_offsets.get(idx),
    ) {
        if !frames.is_empty() {
            anim.frame = (anim.frame + 1) % frames.len();
            if let Ok((mut img, mut style)) = image_queries.p0().get_single_mut() {
                img.texture = frames[anim.frame].clone();
                if let Some((x, y)) = offsets.get(anim.frame).copied() {
                    style.left = Val::Px(x as f32);
                    style.top = Val::Px(y as f32);
                }
            }
        }
    }

    if let (Some(frames), Some(offsets)) = (
        assets.select_char_overlay_frames.get(idx),
        assets.select_char_overlay_offsets.get(idx),
    ) {
        if !frames.is_empty() {
            if let Ok((mut img, mut style)) = image_queries.p1().get_single_mut() {
                img.texture = frames[anim.frame % frames.len()].clone();
                if let Some((x, y)) = offsets.get(anim.frame % offsets.len()).copied() {
                    style.left = Val::Px(x as f32);
                    style.top = Val::Px(y as f32);
                }
            }
        }
    }
}
