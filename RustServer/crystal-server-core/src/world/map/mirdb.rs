use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

use super::data::{MapInfo, MineZone, MovementInfo, RespawnInfo, SafeZoneInfo};

/// Load MapInfo records directly from a C# Server.MirDB database file.
///
/// This mirrors the layout produced by Envir.SaveDB and MapInfo.Save in
/// the original C# server, but for now only materialises the subset of
/// fields we already model in Rust (index, names, minimap/bigmap, light,
/// dark light, music, weather, safe zones, respawns and movements).
/// All other flags and mine-zone data are read and discarded to keep
/// the stream position in sync.
pub fn load_map_infos_from_mirdb<P: AsRef<Path>>(path: P) -> io::Result<Vec<MapInfo>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Header written by Envir.SaveDB
    let version = read_i32(&mut reader)?;
    let _custom_version = read_i32(&mut reader)?;

    // Various index counters (max indices), currently ignored.
    let _map_index = read_i32(&mut reader)?;
    let _item_index = read_i32(&mut reader)?;
    let _monster_index = read_i32(&mut reader)?;
    let _npc_index = read_i32(&mut reader)?;
    let _quest_index = read_i32(&mut reader)?;
    let _gameshop_index = read_i32(&mut reader)?;
    let _conquest_index = read_i32(&mut reader)?;
    let _respawn_index = read_i32(&mut reader)?;

    if version < 60 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported Server.MirDB version {} (expected >= 60)", version),
        ));
    }

    let map_count = read_i32(&mut reader)?;
    if map_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative MapInfo count {} in Server.MirDB", map_count),
        ));
    }

    let mut map_infos = Vec::with_capacity(map_count as usize);
    for _ in 0..map_count {
        map_infos.push(read_map_info(&mut reader)?);
    }

    Ok(map_infos)
}

fn read_map_info<R: Read>(r: &mut R) -> io::Result<MapInfo> {
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

fn read_safe_zone<R: Read>(r: &mut R) -> io::Result<SafeZoneInfo> {
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

fn read_respawn<R: Read>(r: &mut R) -> io::Result<RespawnInfo> {
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

fn read_movement<R: Read>(r: &mut R) -> io::Result<MovementInfo> {
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

fn read_i32<R: Read>(r: &mut R) -> io::Result<i32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

fn read_u16<R: Read>(r: &mut R) -> io::Result<u16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

fn read_u8<R: Read>(r: &mut R) -> io::Result<u8> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_bool<R: Read>(r: &mut R) -> io::Result<bool> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0] != 0)
}

fn read_string<R: Read>(r: &mut R) -> io::Result<String> {
    let byte_len = read_7bit_int(r)?;
    if byte_len < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative string length {}", byte_len),
        ));
    }
    let mut buf = vec![0u8; byte_len as usize];
    r.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn read_7bit_int<R: Read>(r: &mut R) -> io::Result<i32> {
    let mut count: i32 = 0;
    let mut shift = 0;

    loop {
        if shift >= 35 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid 7-bit encoded int (too many bytes)",
            ));
        }

        let mut b = [0u8; 1];
        r.read_exact(&mut b)?;
        let byte = b[0];

        count |= ((byte & 0x7F) as i32) << shift;
        if (byte & 0x80) == 0 {
            break;
        }

        shift += 7;
    }

    Ok(count)
}
