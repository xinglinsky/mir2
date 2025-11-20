use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

use super::data::{MapInfo, MineZone, MovementInfo, RespawnInfo, SafeZoneInfo};
use crate::stats::{Stat, Stats};
use crate::world::magic::MagicInfo;
use crate::world::monster::MonsterInfo;
use crate::world::npc::NpcInfo;
use crystal_shared_proto::item_types::ItemInfoData;

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

/// Load ItemInfo records directly from a C# Server.MirDB database file.
///
/// This mirrors the layout produced by Envir.SaveDB and ItemInfo.Save in the
/// original C# server. The function walks past the MapInfoList section and
/// then materialises the ItemInfoList section as Rust ItemInfoData values.
pub fn load_item_infos_from_mirdb<P: AsRef<Path>>(path: P) -> io::Result<Vec<ItemInfoData>> {
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

    // MapInfoList – walked to keep the stream in sync.
    let map_count = read_i32(&mut reader)?;
    if map_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative MapInfo count {} in Server.MirDB", map_count),
        ));
    }
    for _ in 0..map_count {
        let _ = read_map_info(&mut reader)?;
    }

    // ItemInfoList – materialised as ItemInfoData values.
    let item_count = read_i32(&mut reader)?;
    if item_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ItemInfo count {} in Server.MirDB", item_count),
        ));
    }

    let mut item_infos = Vec::with_capacity(item_count as usize);
    for _ in 0..item_count {
        let item = ItemInfoData::decode(&mut reader)?;
        item_infos.push(item);
    }

    Ok(item_infos)
}

/// Load MonsterInfo records from a C# Server.MirDB file.
///
/// This mirrors the layout produced by Envir.SaveDB and MonsterInfo.Save in the
/// original C# server. The function walks past the MapInfo and ItemInfo tables
/// and then materialises the MonsterInfoList section as Rust MonsterInfo values.
pub fn load_monster_infos_from_mirdb<P: AsRef<Path>>(path: P) -> io::Result<Vec<MonsterInfo>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Header written by Envir.SaveDB
    let version = read_i32(&mut reader)?;
    let custom_version = read_i32(&mut reader)?;

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

    // MapInfoList
    let map_count = read_i32(&mut reader)?;
    if map_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative MapInfo count {} in Server.MirDB", map_count),
        ));
    }
    for _ in 0..map_count {
        // Re-use the existing MapInfo parser to walk past this section.
        let _ = read_map_info(&mut reader)?;
    }

    // ItemInfoList – we currently don't materialise ItemInfo in Rust, but we
    // must keep the stream in sync by reading and discarding each record.
    let item_count = read_i32(&mut reader)?;
    if item_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ItemInfo count {} in Server.MirDB", item_count),
        ));
    }
    for _ in 0..item_count {
        skip_item_info(&mut reader, version, custom_version)?;
    }

    // MonsterInfoList – actually materialised as MonsterInfo values.
    let monster_count = read_i32(&mut reader)?;
    if monster_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative MonsterInfo count {} in Server.MirDB", monster_count),
        ));
    }

    let mut monster_infos = Vec::with_capacity(monster_count as usize);
    for _ in 0..monster_count {
        monster_infos.push(read_monster_info(&mut reader)?);
    }

    Ok(monster_infos)
}

/// Load NPCInfo records from a C# Server.MirDB file.
///
/// This mirrors the layout produced by Envir.SaveDB and NPCInfo.Save in the
/// original C# server. The function re-parses the header and then walks past
/// the MapInfo, ItemInfo and MonsterInfo tables before materialising the
/// NPCInfoList section as Rust NpcInfo values.
pub fn load_npc_infos_from_mirdb<P: AsRef<Path>>(path: P) -> io::Result<Vec<NpcInfo>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Header written by Envir.SaveDB
    let version = read_i32(&mut reader)?;
    let custom_version = read_i32(&mut reader)?;

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

    // MapInfoList
    let map_count = read_i32(&mut reader)?;
    if map_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative MapInfo count {} in Server.MirDB", map_count),
        ));
    }
    for _ in 0..map_count {
        // Re-use the existing MapInfo parser to walk past this section.
        let _ = read_map_info(&mut reader)?;
    }

    // ItemInfoList – we currently don't materialise ItemInfo in Rust, but we
    // must keep the stream in sync by reading and discarding each record.
    let item_count = read_i32(&mut reader)?;
    if item_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ItemInfo count {} in Server.MirDB", item_count),
        ));
    }
    for _ in 0..item_count {
        skip_item_info(&mut reader, version, custom_version)?;
    }

    // MonsterInfoList – similarly skipped for now.
    let monster_count = read_i32(&mut reader)?;
    if monster_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative MonsterInfo count {} in Server.MirDB", monster_count),
        ));
    }
    for _ in 0..monster_count {
        skip_monster_info(&mut reader, version, custom_version)?;
    }

    // NPCInfoList – actually materialised as NpcInfo values.
    let npc_count = read_i32(&mut reader)?;
    if npc_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative NPCInfo count {} in Server.MirDB", npc_count),
        ));
    }
    let mut npc_infos = Vec::with_capacity(npc_count as usize);
    for _ in 0..npc_count {
        npc_infos.push(read_npc_info(&mut reader)?);
    }

    // QuestInfoList – not yet exposed in Rust, but must be walked to keep the
    // stream aligned with the C# SaveDB layout.
    let quest_count = read_i32(&mut reader)?;
    if quest_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative QuestInfo count {} in Server.MirDB", quest_count),
        ));
    }
    for _ in 0..quest_count {
        skip_quest_info(&mut reader, version, custom_version)?;
    }

    // DragonInfo – single record.
    skip_dragon_info(&mut reader, version, custom_version)?;

    // MagicInfoList – skipped for now.
    let magic_count = read_i32(&mut reader)?;
    if magic_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative MagicInfo count {} in Server.MirDB", magic_count),
        ));
    }
    for _ in 0..magic_count {
        skip_magic_info(&mut reader, version, custom_version)?;
    }

    // GameShopList – only present for DB versions >= 63, mirroring C# LoadDB.
    if version >= 63 {
        let shop_count = read_i32(&mut reader)?;
        if shop_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative GameShopItem count {} in Server.MirDB", shop_count),
            ));
        }
        for _ in 0..shop_count {
            skip_game_shop_item(&mut reader, version, custom_version)?;
        }
    }

    // ConquestInfoList – present for DB versions >= 66.
    if version >= 66 {
        let conquest_count = read_i32(&mut reader)?;
        if conquest_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative ConquestInfo count {} in Server.MirDB", conquest_count),
            ));
        }
        for _ in 0..conquest_count {
            skip_conquest_info(&mut reader, version, custom_version)?;
        }
    }

    // RespawnTick – present for DB versions > 67.
    if version > 67 {
        skip_respawn_timer(&mut reader, version, custom_version)?;
    }

    // GTMapList – always written at the end of SaveDB.
    let gt_count = read_i32(&mut reader)?;
    if gt_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative GTMap count {} in Server.MirDB", gt_count),
        ));
    }
    for _ in 0..gt_count {
        skip_gt_map(&mut reader, version, custom_version)?;
    }

    Ok(npc_infos)
}

/// Load MagicInfo records from a C# Server.MirDB file.
///
/// This mirrors the layout produced by Envir.SaveDB and MagicInfo.Save in the
/// original C# server. The function walks past the MapInfo, ItemInfo, MonsterInfo,
/// NPCInfo, QuestInfo and DragonInfo tables before materialising the MagicInfoList
/// section as Rust MagicInfo values.
pub fn load_magic_infos_from_mirdb<P: AsRef<Path>>(path: P) -> io::Result<Vec<MagicInfo>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Header written by Envir.SaveDB
    let version = read_i32(&mut reader)?;
    let custom_version = read_i32(&mut reader)?;

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

    // MapInfoList
    let map_count = read_i32(&mut reader)?;
    if map_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative MapInfo count {} in Server.MirDB", map_count),
        ));
    }
    for _ in 0..map_count {
        // Re-use the existing MapInfo parser to walk past this section.
        let _ = read_map_info(&mut reader)?;
    }

    // ItemInfoList
    let item_count = read_i32(&mut reader)?;
    if item_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ItemInfo count {} in Server.MirDB", item_count),
        ));
    }
    for _ in 0..item_count {
        skip_item_info(&mut reader, version, custom_version)?;
    }

    // MonsterInfoList
    let monster_count = read_i32(&mut reader)?;
    if monster_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative MonsterInfo count {} in Server.MirDB", monster_count),
        ));
    }
    for _ in 0..monster_count {
        skip_monster_info(&mut reader, version, custom_version)?;
    }

    // NPCInfoList
    let npc_count = read_i32(&mut reader)?;
    if npc_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative NPCInfo count {} in Server.MirDB", npc_count),
        ));
    }
    for _ in 0..npc_count {
        let _ = read_npc_info(&mut reader)?;
    }

    // QuestInfoList
    let quest_count = read_i32(&mut reader)?;
    if quest_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative QuestInfo count {} in Server.MirDB", quest_count),
        ));
    }
    for _ in 0..quest_count {
        skip_quest_info(&mut reader, version, custom_version)?;
    }

    // DragonInfo – single record.
    skip_dragon_info(&mut reader, version, custom_version)?;

    // MagicInfoList – actually materialised as MagicInfo values.
    let magic_count = read_i32(&mut reader)?;
    if magic_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative MagicInfo count {} in Server.MirDB", magic_count),
        ));
    }

    let mut magic_infos = Vec::with_capacity(magic_count as usize);
    for _ in 0..magic_count {
        magic_infos.push(read_magic_info(&mut reader, version, custom_version)?);
    }

    Ok(magic_infos)
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

fn read_monster_info<R: Read>(r: &mut R) -> io::Result<MonsterInfo> {
    // MonsterInfo.Save layout from Server/MirDatabase/MonsterInfo.cs
    let index = read_i32(r)?;
    let name = read_string(r)?;

    let image = read_u16(r)?;
    let ai = read_u8(r)?;
    let effect = read_u8(r)?;
    let level = read_u16(r)?;
    let view_range = read_u8(r)?;
    let cool_eye = read_u8(r)?;

    let stats = read_stats(r)?;

    let light = read_u8(r)?;

    let attack_speed = read_u16(r)?;
    let move_speed = read_u16(r)?;

    let experience = read_u32(r)?;

    let can_push = read_bool(r)?;
    let can_tame = read_bool(r)?;
    let auto_rev = read_bool(r)?;
    let undead = read_bool(r)?;

    let drop_path = read_string(r)?;

    Ok(MonsterInfo {
        index,
        name,
        image,
        ai,
        effect,
        view_range,
        cool_eye,
        level,
        light,
        attack_speed,
        move_speed,
        experience,
        drop_path,
        drops: Vec::new(),
        can_tame,
        can_push,
        auto_rev,
        undead,
        has_spawn_script: false,
        has_die_script: false,
        stats,
    })
}

fn read_npc_info<R: Read>(r: &mut R) -> io::Result<NpcInfo> {
    let index = read_i32(r)?;
    let map_index = read_i32(r)?;

    let collect_count = read_i32(r)?;
    if collect_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative CollectQuestIndexes count {} in NPCInfo", collect_count),
        ));
    }
    let mut collect_quest_indexes = Vec::with_capacity(collect_count as usize);
    for _ in 0..collect_count {
        collect_quest_indexes.push(read_i32(r)?);
    }

    let finish_count = read_i32(r)?;
    if finish_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative FinishQuestIndexes count {} in NPCInfo", finish_count),
        ));
    }
    let mut finish_quest_indexes = Vec::with_capacity(finish_count as usize);
    for _ in 0..finish_count {
        finish_quest_indexes.push(read_i32(r)?);
    }

    let file_name = read_string(r)?;
    let name = read_string(r)?;

    let location_x = read_i32(r)?;
    let location_y = read_i32(r)?;
    let image = read_u16(r)?;
    let rate = read_u16(r)?;

    let time_visible = read_bool(r)?;
    let hour_start = read_u8(r)?;
    let minute_start = read_u8(r)?;
    let hour_end = read_u8(r)?;
    let minute_end = read_u8(r)?;
    let min_lev = read_i16(r)?;
    let max_lev = read_i16(r)?;
    let day_of_week = read_string(r)?;
    let class_required = read_string(r)?;
    let conquest = read_i32(r)?;
    let flag_needed = read_i32(r)?;

    let show_on_big_map = read_bool(r)?;
    let big_map_icon = read_i32(r)?;
    let can_teleport_to = read_bool(r)?;
    let conquest_visible = read_bool(r)?;

    Ok(NpcInfo {
        index,
        file_name,
        name,
        map_index,
        location_x,
        location_y,
        rate,
        image,
        time_visible,
        hour_start,
        minute_start,
        hour_end,
        minute_end,
        min_lev,
        max_lev,
        day_of_week,
        class_required,
        // The current C# Save format always writes Conquest (int) and no longer
        // persists the older Sabuk boolean; we default this to false.
        sabuk: false,
        flag_needed,
        conquest,
        show_on_big_map,
        big_map_icon,
        can_teleport_to,
        conquest_visible,
        collect_quest_indexes,
        finish_quest_indexes,
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

fn read_magic_info<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<MagicInfo> {
    // MagicInfo.Save layout from Server/MirDatabase/MagicInfo.cs
    let name = read_string(r)?;
    let spell = read_u8(r)?;
    let base_cost = read_u8(r)?;
    let level_cost = read_u8(r)?;
    let icon = read_u8(r)?;
    let level1 = read_u8(r)?;
    let level2 = read_u8(r)?;
    let level3 = read_u8(r)?;
    let need1 = read_u16(r)?;
    let need2 = read_u16(r)?;
    let need3 = read_u16(r)?;
    let delay_base = read_u32(r)?;
    let delay_reduction = read_u32(r)?;
    let power_base = read_u16(r)?;
    let power_bonus = read_u16(r)?;
    let mpower_base = read_u16(r)?;
    let mpower_bonus = read_u16(r)?;
    let range = read_u8(r)?;
    let multiplier_base = read_f32(r)?;
    let multiplier_bonus = read_f32(r)?;

    Ok(MagicInfo {
        name,
        spell,
        base_cost,
        level_cost,
        icon,
        level1,
        level2,
        level3,
        need1,
        need2,
        need3,
        delay_base,
        delay_reduction,
        power_base,
        power_bonus,
        mpower_base,
        mpower_bonus,
        range,
        multiplier_base,
        multiplier_bonus,
    })
}

fn skip_item_info<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
    // ItemInfo.Save layout from Shared/Data/ItemData.cs
    let _index = read_i32(r)?;
    let _name = read_string(r)?;
    let _item_type = read_u8(r)?;
    let _grade = read_u8(r)?;
    let _required_type = read_u8(r)?;
    let _required_class = read_u8(r)?;
    let _required_gender = read_u8(r)?;
    let _set = read_u8(r)?;

    let _shape = read_i16(r)?;
    let _weight = read_u8(r)?;
    let _light = read_u8(r)?;
    let _required_amount = read_u8(r)?;

    let _image = read_u16(r)?;
    let _durability = read_u16(r)?;

    let _stack_size = read_u16(r)?;
    let _price = read_u32(r)?;

    let _start_item = read_bool(r)?;

    let _effect = read_u8(r)?;

    let _flags = read_u8(r)?;

    let _bind = read_i16(r)?;
    let _unique = read_i16(r)?;

    let _random_stats_id = read_u8(r)?;

    let _can_fast_run = read_bool(r)?;
    let _can_awakening = read_bool(r)?;
    let _slots = read_u8(r)?;

    skip_stats(r)?;

    let has_tooltip = read_bool(r)?;
    if has_tooltip {
        let _tooltip = read_string(r)?;
    }

    Ok(())
}

fn skip_monster_info<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
    // MonsterInfo.Save layout from Server/MirDatabase/MonsterInfo.cs
    let _index = read_i32(r)?;
    let _name = read_string(r)?;

    let _image = read_u16(r)?;
    let _ai = read_u8(r)?;
    let _effect = read_u8(r)?;
    let _level = read_u16(r)?;
    let _view_range = read_u8(r)?;
    let _cool_eye = read_u8(r)?;

    skip_stats(r)?;

    let _light = read_u8(r)?;

    let _attack_speed = read_u16(r)?;
    let _move_speed = read_u16(r)?;

    let _experience = read_u32(r)?;

    let _can_push = read_bool(r)?;
    let _can_tame = read_bool(r)?;
    let _auto_rev = read_bool(r)?;
    let _undead = read_bool(r)?;

    let _drop_path = read_string(r)?;

    Ok(())
}

fn read_stats<R: Read>(r: &mut R) -> io::Result<Stats> {
    // Matches Shared/Data/Stat.cs Stats.Save
    let count = read_i32(r)?;
    if count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative Stats count {}", count),
        ));
    }

    let mut stats = Stats::default();
    for _ in 0..count {
        let stat_id = read_u8(r)?;
        let value = read_i32(r)?;
        let stat = Stat::from_u8(stat_id);
        stats.set(stat, value);
    }

    Ok(stats)
}

fn skip_stats<R: Read>(r: &mut R) -> io::Result<()> {
    let _ = read_stats(r)?;
    Ok(())
}

fn skip_quest_info<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
    // QuestInfo.Save layout from Server/MirDatabase/QuestInfo.cs
    let _index = read_i32(r)?;
    let _name = read_string(r)?;
    let _group = read_string(r)?;
    let _file_name = read_string(r)?;
    let _required_min_level = read_i32(r)?;
    let _required_max_level = read_i32(r)?;
    let _required_quest = read_i32(r)?;
    let _required_class = read_u8(r)?;
    let _quest_type = read_u8(r)?;
    let _goto_message = read_string(r)?;
    let _kill_message = read_string(r)?;
    let _item_message = read_string(r)?;
    let _flag_message = read_string(r)?;
    let _time_limit_in_seconds = read_i32(r)?;

    Ok(())
}

fn skip_dragon_info<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
    // DragonInfo.Save layout from Server/MirDatabase/DragonInfo.cs
    let _enabled = read_bool(r)?;
    let _map_file_name = read_string(r)?;
    let _monster_name = read_string(r)?;
    let _body_name = read_string(r)?;

    let _location_x = read_i32(r)?;
    let _location_y = read_i32(r)?;
    let _drop_area_top_x = read_i32(r)?;
    let _drop_area_top_y = read_i32(r)?;
    let _drop_area_bottom_x = read_i32(r)?;
    let _drop_area_bottom_y = read_i32(r)?;

    // Exps array has length Globals.MaxDragonLevel - 1, which is 12 for this
    // server version (see Shared/Globals.cs MaxDragonLevel = 13).
    for _ in 0..12 {
        let _exp = read_i64(r)?;
    }

    Ok(())
}

fn skip_magic_info<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
    // MagicInfo.Save layout from Server/MirDatabase/MagicInfo.cs
    let _name = read_string(r)?;
    let _spell = read_u8(r)?;
    let _base_cost = read_u8(r)?;
    let _level_cost = read_u8(r)?;
    let _icon = read_u8(r)?;
    let _level1 = read_u8(r)?;
    let _level2 = read_u8(r)?;
    let _level3 = read_u8(r)?;
    let _need1 = read_u16(r)?;
    let _need2 = read_u16(r)?;
    let _need3 = read_u16(r)?;
    let _delay_base = read_u32(r)?;
    let _delay_reduction = read_u32(r)?;
    let _power_base = read_u16(r)?;
    let _power_bonus = read_u16(r)?;
    let _mpower_base = read_u16(r)?;
    let _mpower_bonus = read_u16(r)?;
    let _range = read_u8(r)?;
    let _multiplier_base = read_f32(r)?;
    let _multiplier_bonus = read_f32(r)?;

    Ok(())
}

fn skip_game_shop_item<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
    // GameShopItem.Save(writer, packet: false) layout from Shared/Data/ItemData.cs
    let _item_index = read_i32(r)?;
    let _g_index = read_i32(r)?;
    let _gold_price = read_u32(r)?;
    let _credit_price = read_u32(r)?;
    let _count = read_u16(r)?;
    let _class = read_string(r)?;
    let _category = read_string(r)?;
    let _stock = read_i32(r)?;
    let _i_stock = read_bool(r)?;
    let _deal = read_bool(r)?;
    let _top_item = read_bool(r)?;
    let _date_binary = read_i64(r)?;
    let _can_buy_credit = read_bool(r)?;
    let _can_buy_gold = read_bool(r)?;

    Ok(())
}

fn skip_conquest_info<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
    // ConquestInfo.Save layout from Server/MirDatabase/ConquestInfo.cs
    let _index = read_i32(r)?;
    let _full_map = read_bool(r)?;
    let _location_x = read_i32(r)?;
    let _location_y = read_i32(r)?;
    let _size = read_u16(r)?;
    let _name = read_string(r)?;
    let _map_index = read_i32(r)?;
    let _palace_index = read_i32(r)?;
    let _guard_index = read_i32(r)?;
    let _gate_index = read_i32(r)?;
    let _wall_index = read_i32(r)?;
    let _siege_index = read_i32(r)?;
    let _flag_index = read_i32(r)?;

    let guard_count = read_i32(r)?;
    if guard_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ConquestGuard count {} in ConquestInfo", guard_count),
        ));
    }
    for _ in 0..guard_count {
        skip_conquest_archer_info(r)?;
    }

    let extra_map_count = read_i32(r)?;
    if extra_map_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ExtraMaps count {} in ConquestInfo", extra_map_count),
        ));
    }
    for _ in 0..extra_map_count {
        let _extra_map_index = read_i32(r)?;
    }

    let gate_count = read_i32(r)?;
    if gate_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ConquestGate count {} in ConquestInfo", gate_count),
        ));
    }
    for _ in 0..gate_count {
        skip_conquest_gate_info(r)?;
    }

    let wall_count = read_i32(r)?;
    if wall_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ConquestWall count {} in ConquestInfo", wall_count),
        ));
    }
    for _ in 0..wall_count {
        skip_conquest_wall_info(r)?;
    }

    let siege_count = read_i32(r)?;
    if siege_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ConquestSiege count {} in ConquestInfo", siege_count),
        ));
    }
    for _ in 0..siege_count {
        skip_conquest_siege_info(r)?;
    }

    let flag_count = read_i32(r)?;
    if flag_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ConquestFlag count {} in ConquestInfo", flag_count),
        ));
    }
    for _ in 0..flag_count {
        skip_conquest_flag_info(r)?;
    }

    let _start_hour = read_u8(r)?;
    let _war_length = read_i32(r)?;
    let _conquest_type = read_u8(r)?;
    let _game_type = read_u8(r)?;

    let _monday = read_bool(r)?;
    let _tuesday = read_bool(r)?;
    let _wednesday = read_bool(r)?;
    let _thursday = read_bool(r)?;
    let _friday = read_bool(r)?;
    let _saturday = read_bool(r)?;
    let _sunday = read_bool(r)?;

    let _king_location_x = read_i32(r)?;
    let _king_location_y = read_i32(r)?;
    let _king_size = read_u16(r)?;

    let _control_point_index = read_i32(r)?;
    let control_point_count = read_i32(r)?;
    if control_point_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "negative Conquest control point count {} in ConquestInfo",
                control_point_count
            ),
        ));
    }
    for _ in 0..control_point_count {
        skip_conquest_flag_info(r)?;
    }

    Ok(())
}

fn skip_conquest_archer_info<R: Read>(r: &mut R) -> io::Result<()> {
    let _index = read_i32(r)?;
    let _location_x = read_i32(r)?;
    let _location_y = read_i32(r)?;
    let _mob_index = read_i32(r)?;
    let _name = read_string(r)?;
    let _repair_cost = read_u32(r)?;

    Ok(())
}

fn skip_conquest_gate_info<R: Read>(r: &mut R) -> io::Result<()> {
    let _index = read_i32(r)?;
    let _location_x = read_i32(r)?;
    let _location_y = read_i32(r)?;
    let _mob_index = read_i32(r)?;
    let _name = read_string(r)?;
    let _repair_cost = read_i32(r)?;

    Ok(())
}

fn skip_conquest_wall_info<R: Read>(r: &mut R) -> io::Result<()> {
    let _index = read_i32(r)?;
    let _location_x = read_i32(r)?;
    let _location_y = read_i32(r)?;
    let _mob_index = read_i32(r)?;
    let _name = read_string(r)?;
    let _repair_cost = read_i32(r)?;

    Ok(())
}

fn skip_conquest_siege_info<R: Read>(r: &mut R) -> io::Result<()> {
    let _index = read_i32(r)?;
    let _location_x = read_i32(r)?;
    let _location_y = read_i32(r)?;
    let _mob_index = read_i32(r)?;
    let _name = read_string(r)?;
    let _repair_cost = read_i32(r)?;

    Ok(())
}

fn skip_conquest_flag_info<R: Read>(r: &mut R) -> io::Result<()> {
    let _index = read_i32(r)?;
    let _location_x = read_i32(r)?;
    let _location_y = read_i32(r)?;
    let _name = read_string(r)?;
    let _file_name = read_string(r)?;

    Ok(())
}

fn skip_respawn_timer<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
    // RespawnTimer.Save layout from Server/MirEnvir/RespawnTimer.cs
    let _base_spawn_rate = read_u8(r)?;
    let _current_tick_counter = read_u64(r)?;

    let option_count = read_i32(r)?;
    if option_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative RespawnTickOption count {} in RespawnTimer", option_count),
        ));
    }
    for _ in 0..option_count {
        // RespawnTickOption.Save
        let _user_count = read_i32(r)?;
        let _delay_loss = read_f64(r)?;
    }

    Ok(())
}

fn skip_gt_map<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
    // GTMap.Save layout from Server/Library/MirDatabase/GTMap.cs
    let _index = read_i32(r)?;
    let _key = read_i32(r)?;
    let _name = read_string(r)?;
    let _owner = read_string(r)?;
    let _leader = read_string(r)?;
    let _leader2 = read_string(r)?;
    let _price = read_i32(r)?;
    let _days = read_i32(r)?;
    let _begin = read_i32(r)?;

    Ok(())
}

fn read_i32<R: Read>(r: &mut R) -> io::Result<i32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

fn read_u32<R: Read>(r: &mut R) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_i64<R: Read>(r: &mut R) -> io::Result<i64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(i64::from_le_bytes(buf))
}

fn read_u64<R: Read>(r: &mut R) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_i16<R: Read>(r: &mut R) -> io::Result<i16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(i16::from_le_bytes(buf))
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

fn read_f32<R: Read>(r: &mut R) -> io::Result<f32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
}

fn read_f64<R: Read>(r: &mut R) -> io::Result<f64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(f64::from_le_bytes(buf))
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
