use bevy::prelude::*;
use crystal_lib::LibFile;

use crate::app_config::LibTestConfig;
use crate::shared::lib_image::lib_to_image_handle;

pub(crate) fn run_lib_test(cfg: LibTestConfig) {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(cfg)
        .add_systems(Startup, lib_test_setup)
        .run();
}

fn lib_test_setup(mut commands: Commands, cfg: Res<LibTestConfig>, mut images: ResMut<Assets<Image>>) {
    commands.spawn(Camera2dBundle::default());

    let lib = match LibFile::load(&cfg.lib_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[lib-test] load failed: {e}");
            return;
        }
    };

    let handle = match lib_to_image_handle(&lib, cfg.lib_index, &mut images) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[lib-test] get_image idx={} failed: {e}", cfg.lib_index);
            return;
        }
    };

    commands.spawn(SpriteBundle {
        texture: handle,
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });
}
