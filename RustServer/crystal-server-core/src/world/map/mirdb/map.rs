use std::io::{self, Read};

use crate::world::map::data::{MapInfo, MineZone, MovementInfo, RespawnInfo, SafeZoneInfo};

use super::header::*;

pub(super) fn read_map_info<R: Read>(r: &mut R) -> io::Result<MapInfo> {
    let index = read_i32(r)?;
    let file_name = read_string(r)?;
    let title = read_string(r)?;
    let mini_map = read_u16(r)?;
    let light = read_u8(r)?;
    let big_map = read_u16(r)?;

    // Safe zones
    let safe_count = read_i32(r)?;
    if safe_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative SafeZone count {} in MapInfo", safe_count),
        ));
    }
    let mut safe_zones = Vec::with_capacity(safe_count as usize);
    for _ in 0..safe_count {
        safe_zones.push(read_safe_zone(r)?);
    }

    // Respawns
    let respawn_count = read_i32(r)?;
    if respawn_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative Respawn count {} in MapInfo", respawn_count),
        ));
    }
    let mut respawns = Vec::with_capacity(respawn_count as usize);
    for _ in 0..respawn_count {
        respawns.push(read_respawn(r)?);
    }

    // Movements
    let movement_count = read_i32(r)?;
    if movement_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative Movement count {} in MapInfo", movement_count),
        ));
    }
    let mut movements = Vec::with_capacity(movement_count as usize);
    for _ in 0..movement_count {
        movements.push(read_movement(r)?);
    }

    // Booleans and extra fields, mirroring C# MapInfo.Save layout.
    let no_teleport = read_bool(r)?;
    let no_reconnect = read_bool(r)?;
    let no_reconnect_map = read_string(r)?;

    let no_random = read_bool(r)?;
    let no_escape = read_bool(r)?;
    let no_recall = read_bool(r)?;
    let no_drug = read_bool(r)?;
    let no_position = read_bool(r)?;
    let no_throw_item = read_bool(r)?;
    let no_drop_player = read_bool(r)?;
    let no_drop_monster = read_bool(r)?;
    let no_names = read_bool(r)?;
    let fight = read_bool(r)?;

    let fire = read_bool(r)?;
    let fire_damage = read_i32(r)?;
    let lightning = read_bool(r)?;
    let lightning_damage = read_i32(r)?;

    let map_dark_light = read_u8(r)?;

    // Mine zones
    let mine_zone_count = read_i32(r)?;
    if mine_zone_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative MineZone count {} in MapInfo", mine_zone_count),
        ));
    }
    let mut mine_zones = Vec::with_capacity(mine_zone_count as usize);
    for _ in 0..mine_zone_count {
        let mx = read_i32(r)?;
        let my = read_i32(r)?;
        let msize = read_u16(r)?;
        let mine = read_u8(r)?;
        mine_zones.push(MineZone {
            mine,
            location_x: mx,
            location_y: my,
            size: msize,
        });
    }

    let mine_index = read_u8(r)?;
    let no_mount = read_bool(r)?;
    let need_bridle = read_bool(r)?;
    let no_fight = read_bool(r)?;

    let music = read_u16(r)?;
    let no_town_teleport = read_bool(r)?;
    let no_reincarnation = read_bool(r)?;

    let weather_particles = read_u16(r)?;
    let gt = read_bool(r)?;
    let gt_index = read_u8(r)?;

    Ok(MapInfo {
        index,
        file_name,
        title,
        mini_map,
        big_map,
        light,
        map_dark_light,
        music,
        weather_particles,
        no_teleport,
        no_reconnect,
        no_random,
        no_escape,
        no_recall,
        no_drug,
        no_position,
        no_throw_item,
        no_drop_player,
        no_drop_monster,
        no_names,
        no_mount,
        need_bridle,
        no_fight,
        fight,
        fire,
        fire_damage,
        lightning,
        lightning_damage,
        no_town_teleport,
        no_reincarnation,
        no_reconnect_map,
        mine_zones,
        mine_index,
        gt,
        gt_index,
        safe_zones,
        respawns,
        movements,
    })
}

pub(super) fn read_safe_zone<R: Read>(r: &mut R) -> io::Result<SafeZoneInfo> {
    let x = read_i32(r)?;
    let y = read_i32(r)?;
    let size = read_u16(r)?;
    let start_point = read_bool(r)?;

    Ok(SafeZoneInfo {
        location_x: x,
        location_y: y,
        size,
        start_point,
    })
}

pub(super) fn read_respawn<R: Read>(r: &mut R) -> io::Result<RespawnInfo> {
    let monster_index = read_i32(r)?;
    let x = read_i32(r)?;
    let y = read_i32(r)?;
    let count = read_u16(r)?;
    let spread = read_u16(r)?;
    let delay = read_u16(r)?;
    let direction = read_u8(r)?;
    let route_path = read_string(r)?;
    let random_delay = read_u16(r)?;
    let respawn_index = read_i32(r)?;
    let save_respawn_time = read_bool(r)?;
    let respawn_ticks = read_u16(r)?;

    Ok(RespawnInfo {
        monster_index,
        location_x: x,
        location_y: y,
        count,
        spread,
        delay,
        random_delay,
        direction,
        route_path,
        respawn_index,
        save_respawn_time,
        respawn_ticks,
    })
}

pub(super) fn read_movement<R: Read>(r: &mut R) -> io::Result<MovementInfo> {
    let map_index = read_i32(r)?;
    let sx = read_i32(r)?;
    let sy = read_i32(r)?;
    let dx = read_i32(r)?;
    let dy = read_i32(r)?;
    let need_hole = read_bool(r)?;
    let need_move = read_bool(r)?;
    let conquest_index = read_i32(r)?;
    let show_on_big_map = read_bool(r)?;
    let icon = read_i32(r)?;

    Ok(MovementInfo {
        map_index,
        source_x: sx,
        source_y: sy,
        dest_map_index: map_index,
        dest_x: dx,
        dest_y: dy,
        need_hole,
        need_move,
        conquest_index,
        show_on_big_map,
        icon,
    })
}
