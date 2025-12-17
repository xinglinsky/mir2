use std::io::{self, Read};
use std::path::Path;

use crystal_shared_proto::item_types::ItemInfoData;

use crate::world::content;
use crate::world::monster::MonsterInfo;
use crate::world::npc::NpcInfo;
use crate::world::magic::MagicInfo;
use crate::stats::{Stat, Stats};
use crate::quest::{QuestId, QuestInfo as CoreQuestInfo, QuestType, RequiredClass};
use crate::world::map::mirdb::GameShopItemRecord;
use crate::world::map::data::{MapInfo, MineZone, MovementInfo, RespawnInfo, SafeZoneInfo};
use crate::conquest::{
    ConquestArcherInfo, ConquestFlagInfo, ConquestGateInfo, ConquestId, ConquestInfo,
    ConquestSiegeInfo, ConquestWallInfo,
};

fn read_i32_le<R: Read>(r: &mut R) -> io::Result<i32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(i32::from_le_bytes(b))
}

fn read_u32_le<R: Read>(r: &mut R) -> io::Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

fn read_i64_le<R: Read>(r: &mut R) -> io::Result<i64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(i64::from_le_bytes(b))
}

fn read_i16_le<R: Read>(r: &mut R) -> io::Result<i16> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b)?;
    Ok(i16::from_le_bytes(b))
}

fn read_u16_le<R: Read>(r: &mut R) -> io::Result<u16> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b)?;
    Ok(u16::from_le_bytes(b))
}

fn read_f32_le<R: Read>(r: &mut R) -> io::Result<f32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(f32::from_le_bytes(b))
}

fn read_u8<R: Read>(r: &mut R) -> io::Result<u8> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b)?;
    Ok(b[0])
}

fn read_bool<R: Read>(r: &mut R) -> io::Result<bool> {
    Ok(read_u8(r)? != 0)
}

fn read_7bit_i32<R: Read>(r: &mut R) -> io::Result<i32> {
    let mut count: i32 = 0;
    let mut shift = 0;

    loop {
        if shift >= 35 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid 7-bit encoded int (too many bytes)",
            ));
        }

        let byte = read_u8(r)?;
        count |= ((byte & 0x7F) as i32) << shift;
        if (byte & 0x80) == 0 {
            break;
        }
        shift += 7;
    }

    Ok(count)
}

fn read_string<R: Read>(r: &mut R) -> io::Result<String> {
    let byte_len = read_7bit_i32(r)?;
    if byte_len < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative string length {byte_len}"),
        ));
    }
    let mut buf = vec![0u8; byte_len as usize];
    r.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn read_stats<R: Read>(r: &mut R) -> io::Result<Stats> {
    let count = read_i32_le(r)?;
    if count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative Stats count {count}"),
        ));
    }

    let mut stats = Stats::default();
    for _ in 0..count {
        let stat_id = read_u8(r)?;
        let value = read_i32_le(r)?;
        let stat = Stat::from_u8(stat_id);
        stats.set(stat, value);
    }

    Ok(stats)
}

fn read_safe_zone_info<R: Read>(r: &mut R) -> io::Result<SafeZoneInfo> {
    let x = read_i32_le(r)?;
    let y = read_i32_le(r)?;
    let size = read_u16_le(r)?;
    let start_point = read_bool(r)?;

    Ok(SafeZoneInfo {
        location_x: x,
        location_y: y,
        size,
        start_point,
    })
}

fn read_respawn_info<R: Read>(r: &mut R) -> io::Result<RespawnInfo> {
    let monster_index = read_i32_le(r)?;
    let location_x = read_i32_le(r)?;
    let location_y = read_i32_le(r)?;
    let count = read_u16_le(r)?;
    let spread = read_u16_le(r)?;
    let delay = read_u16_le(r)?;
    let direction = read_u8(r)?;
    let route_path = read_string(r)?;
    let random_delay = read_u16_le(r)?;
    let respawn_index = read_i32_le(r)?;
    let save_respawn_time = read_bool(r)?;
    let respawn_ticks = read_u16_le(r)?;

    Ok(RespawnInfo {
        monster_index,
        location_x,
        location_y,
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

fn read_movement_info<R: Read>(r: &mut R) -> io::Result<MovementInfo> {
    let map_index = read_i32_le(r)?;
    let source_x = read_i32_le(r)?;
    let source_y = read_i32_le(r)?;
    let dest_x = read_i32_le(r)?;
    let dest_y = read_i32_le(r)?;
    let need_hole = read_bool(r)?;
    let need_move = read_bool(r)?;
    let conquest_index = read_i32_le(r)?;
    let show_on_big_map = read_bool(r)?;
    let icon = read_i32_le(r)?;

    Ok(MovementInfo {
        map_index,
        source_x,
        source_y,
        dest_map_index: map_index,
        dest_x,
        dest_y,
        need_hole,
        need_move,
        conquest_index,
        show_on_big_map,
        icon,
    })
}

pub fn load_map_infos_from_exports<P: AsRef<Path>>(path: P) -> io::Result<Vec<MapInfo>> {
    let path = path.as_ref();
    let bytes = content::read_bytes(path)?;
    let mut cursor = std::io::Cursor::new(bytes);

    let count = read_i32_le(&mut cursor)?;
    if count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative map count {count} in exports"),
        ));
    }

    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        // MapInfo.Save layout from Server/MirDatabase/MapInfo.cs
        let index = read_i32_le(&mut cursor)?;
        let file_name = read_string(&mut cursor)?;
        let title = read_string(&mut cursor)?;
        let mini_map = read_u16_le(&mut cursor)?;
        let light = read_u8(&mut cursor)?;
        let big_map = read_u16_le(&mut cursor)?;

        let safe_count = read_i32_le(&mut cursor)?;
        if safe_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative SafeZone count {safe_count} in exports MapInfo"),
            ));
        }
        let mut safe_zones = Vec::with_capacity(safe_count as usize);
        for _ in 0..safe_count {
            safe_zones.push(read_safe_zone_info(&mut cursor)?);
        }

        let respawn_count = read_i32_le(&mut cursor)?;
        if respawn_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative Respawn count {respawn_count} in exports MapInfo"),
            ));
        }
        let mut respawns = Vec::with_capacity(respawn_count as usize);
        for _ in 0..respawn_count {
            respawns.push(read_respawn_info(&mut cursor)?);
        }

        let movement_count = read_i32_le(&mut cursor)?;
        if movement_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative Movement count {movement_count} in exports MapInfo"),
            ));
        }
        let mut movements = Vec::with_capacity(movement_count as usize);
        for _ in 0..movement_count {
            movements.push(read_movement_info(&mut cursor)?);
        }

        let no_teleport = read_bool(&mut cursor)?;
        let no_reconnect = read_bool(&mut cursor)?;
        let no_reconnect_map = read_string(&mut cursor)?;
        let no_random = read_bool(&mut cursor)?;
        let no_escape = read_bool(&mut cursor)?;
        let no_recall = read_bool(&mut cursor)?;
        let no_drug = read_bool(&mut cursor)?;
        let no_position = read_bool(&mut cursor)?;
        let no_throw_item = read_bool(&mut cursor)?;
        let no_drop_player = read_bool(&mut cursor)?;
        let no_drop_monster = read_bool(&mut cursor)?;
        let no_names = read_bool(&mut cursor)?;
        let fight = read_bool(&mut cursor)?;

        let fire = read_bool(&mut cursor)?;
        let fire_damage = read_i32_le(&mut cursor)?;
        let lightning = read_bool(&mut cursor)?;
        let lightning_damage = read_i32_le(&mut cursor)?;
        let map_dark_light = read_u8(&mut cursor)?;

        let mine_zone_count = read_i32_le(&mut cursor)?;
        if mine_zone_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative MineZone count {mine_zone_count} in exports MapInfo"),
            ));
        }
        let mut mine_zones = Vec::with_capacity(mine_zone_count as usize);
        for _ in 0..mine_zone_count {
            let mx = read_i32_le(&mut cursor)?;
            let my = read_i32_le(&mut cursor)?;
            let msize = read_u16_le(&mut cursor)?;
            let mine = read_u8(&mut cursor)?;
            mine_zones.push(MineZone {
                mine,
                location_x: mx,
                location_y: my,
                size: msize,
            });
        }
        let mine_index = read_u8(&mut cursor)?;

        let no_mount = read_bool(&mut cursor)?;
        let need_bridle = read_bool(&mut cursor)?;
        let no_fight = read_bool(&mut cursor)?;
        let music = read_u16_le(&mut cursor)?;
        let no_town_teleport = read_bool(&mut cursor)?;
        let no_reincarnation = read_bool(&mut cursor)?;
        let weather_particles = read_u16_le(&mut cursor)?;
        let gt = read_bool(&mut cursor)?;
        let gt_index = read_u8(&mut cursor)?;

        out.push(MapInfo {
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
        });
    }

    Ok(out)
}

pub fn load_item_infos_from_exports<P: AsRef<Path>>(path: P) -> io::Result<Vec<ItemInfoData>> {
    let path = path.as_ref();
    let bytes = content::read_bytes(path)?;
    let mut cursor = std::io::Cursor::new(bytes);

    let count = read_i32_le(&mut cursor)?;
    if count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative item count {count} in exports"),
        ));
    }

    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        out.push(ItemInfoData::decode(&mut cursor)?);
    }

    Ok(out)
}

pub fn load_conquest_infos_from_exports<P: AsRef<Path>>(path: P) -> io::Result<Vec<ConquestInfo>> {
    let path = path.as_ref();
    let bytes = content::read_bytes(path)?;
    let mut cursor = std::io::Cursor::new(bytes);

    let count = read_i32_le(&mut cursor)?;
    if count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative conquest count {count} in exports"),
        ));
    }

    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        // ConquestInfo.Save layout from Server/MirDatabase/ConquestInfo.cs
        let id = ConquestId(read_i32_le(&mut cursor)?);
        let full_map = read_bool(&mut cursor)?;
        let location_x = read_i32_le(&mut cursor)?;
        let location_y = read_i32_le(&mut cursor)?;
        let size = read_u16_le(&mut cursor)?;
        let name = read_string(&mut cursor)?;
        let map_index = read_i32_le(&mut cursor)?;
        let palace_index = read_i32_le(&mut cursor)?;
        let guard_index = read_i32_le(&mut cursor)?;
        let gate_index = read_i32_le(&mut cursor)?;
        let wall_index = read_i32_le(&mut cursor)?;
        let siege_index = read_i32_le(&mut cursor)?;
        let flag_index = read_i32_le(&mut cursor)?;

        let guard_count = read_i32_le(&mut cursor)?;
        if guard_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative ConquestGuard count {guard_count} in exports"),
            ));
        }
        let mut guards = Vec::with_capacity(guard_count as usize);
        for _ in 0..guard_count {
            let index = read_i32_le(&mut cursor)?;
            let gx = read_i32_le(&mut cursor)?;
            let gy = read_i32_le(&mut cursor)?;
            let mob_index = read_i32_le(&mut cursor)?;
            let gname = read_string(&mut cursor)?;
            let repair_cost = read_u32_le(&mut cursor)?;
            guards.push(ConquestArcherInfo {
                index,
                location_x: gx,
                location_y: gy,
                mob_index,
                name: gname,
                repair_cost,
            });
        }

        let extra_map_count = read_i32_le(&mut cursor)?;
        if extra_map_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative ExtraMaps count {extra_map_count} in exports"),
            ));
        }
        let mut extra_maps = Vec::with_capacity(extra_map_count as usize);
        for _ in 0..extra_map_count {
            extra_maps.push(read_i32_le(&mut cursor)?);
        }

        let gate_count = read_i32_le(&mut cursor)?;
        if gate_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative ConquestGate count {gate_count} in exports"),
            ));
        }
        let mut gates = Vec::with_capacity(gate_count as usize);
        for _ in 0..gate_count {
            let index = read_i32_le(&mut cursor)?;
            let gx = read_i32_le(&mut cursor)?;
            let gy = read_i32_le(&mut cursor)?;
            let mob_index = read_i32_le(&mut cursor)?;
            let gname = read_string(&mut cursor)?;
            let repair_cost = read_i32_le(&mut cursor)?;
            gates.push(ConquestGateInfo {
                index,
                location_x: gx,
                location_y: gy,
                mob_index,
                name: gname,
                repair_cost,
            });
        }

        let wall_count = read_i32_le(&mut cursor)?;
        if wall_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative ConquestWall count {wall_count} in exports"),
            ));
        }
        let mut walls = Vec::with_capacity(wall_count as usize);
        for _ in 0..wall_count {
            let index = read_i32_le(&mut cursor)?;
            let wx = read_i32_le(&mut cursor)?;
            let wy = read_i32_le(&mut cursor)?;
            let mob_index = read_i32_le(&mut cursor)?;
            let wname = read_string(&mut cursor)?;
            let repair_cost = read_i32_le(&mut cursor)?;
            walls.push(ConquestWallInfo {
                index,
                location_x: wx,
                location_y: wy,
                mob_index,
                name: wname,
                repair_cost,
            });
        }

        let siege_count = read_i32_le(&mut cursor)?;
        if siege_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative ConquestSiege count {siege_count} in exports"),
            ));
        }
        let mut sieges = Vec::with_capacity(siege_count as usize);
        for _ in 0..siege_count {
            let index = read_i32_le(&mut cursor)?;
            let sx = read_i32_le(&mut cursor)?;
            let sy = read_i32_le(&mut cursor)?;
            let mob_index = read_i32_le(&mut cursor)?;
            let sname = read_string(&mut cursor)?;
            let repair_cost = read_i32_le(&mut cursor)?;
            sieges.push(ConquestSiegeInfo {
                index,
                location_x: sx,
                location_y: sy,
                mob_index,
                name: sname,
                repair_cost,
            });
        }

        let flag_count = read_i32_le(&mut cursor)?;
        if flag_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative ConquestFlag count {flag_count} in exports"),
            ));
        }
        let mut flags = Vec::with_capacity(flag_count as usize);
        for _ in 0..flag_count {
            let index = read_i32_le(&mut cursor)?;
            let fx = read_i32_le(&mut cursor)?;
            let fy = read_i32_le(&mut cursor)?;
            let fname = read_string(&mut cursor)?;
            let file_name = read_string(&mut cursor)?;
            flags.push(ConquestFlagInfo {
                index,
                location_x: fx,
                location_y: fy,
                name: fname,
                file_name,
            });
        }

        let start_hour = read_u8(&mut cursor)?;
        let war_length = read_i32_le(&mut cursor)?;
        let conquest_type = read_u8(&mut cursor)?;
        let game = read_u8(&mut cursor)?;

        let monday = read_bool(&mut cursor)?;
        let tuesday = read_bool(&mut cursor)?;
        let wednesday = read_bool(&mut cursor)?;
        let thursday = read_bool(&mut cursor)?;
        let friday = read_bool(&mut cursor)?;
        let saturday = read_bool(&mut cursor)?;
        let sunday = read_bool(&mut cursor)?;

        let king_location_x = read_i32_le(&mut cursor)?;
        let king_location_y = read_i32_le(&mut cursor)?;
        let king_size = read_u16_le(&mut cursor)?;

        let control_point_index = read_i32_le(&mut cursor)?;
        let control_point_count = read_i32_le(&mut cursor)?;
        if control_point_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative control point count {control_point_count} in exports"),
            ));
        }
        let mut control_points = Vec::with_capacity(control_point_count as usize);
        for _ in 0..control_point_count {
            let index = read_i32_le(&mut cursor)?;
            let cx = read_i32_le(&mut cursor)?;
            let cy = read_i32_le(&mut cursor)?;
            let cname = read_string(&mut cursor)?;
            let file_name = read_string(&mut cursor)?;
            control_points.push(ConquestFlagInfo {
                index,
                location_x: cx,
                location_y: cy,
                name: cname,
                file_name,
            });
        }

        out.push(ConquestInfo {
            id,
            full_map,
            location_x,
            location_y,
            size,
            name,
            map_index,
            palace_index,
            extra_maps,
            guards,
            gates,
            walls,
            sieges,
            flags,
            guard_index,
            gate_index,
            wall_index,
            siege_index,
            flag_index,
            start_hour,
            war_length,
            conquest_type,
            game,
            monday,
            tuesday,
            wednesday,
            thursday,
            friday,
            saturday,
            sunday,
            king_location_x,
            king_location_y,
            king_size,
            control_points,
            control_point_index,
        });
    }

    Ok(out)
}

pub fn load_game_shop_items_from_exports<P: AsRef<Path>>(
    path: P,
) -> io::Result<Vec<GameShopItemRecord>> {
    let path = path.as_ref();
    let bytes = content::read_bytes(path)?;
    let mut cursor = std::io::Cursor::new(bytes);

    let count = read_i32_le(&mut cursor)?;
    if count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative game shop item count {count} in exports"),
        ));
    }

    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        // GameShopItem.Save(writer, packet:false) layout from Shared/Data/ItemData.cs
        let item_index = read_i32_le(&mut cursor)?;
        let g_index = read_i32_le(&mut cursor)?;
        let gold_price = read_u32_le(&mut cursor)?;
        let credit_price = read_u32_le(&mut cursor)?;
        let count = read_u16_le(&mut cursor)?;
        let class = read_string(&mut cursor)?;
        let category = read_string(&mut cursor)?;
        let stock = read_i32_le(&mut cursor)?;
        let i_stock = read_bool(&mut cursor)?;
        let deal = read_bool(&mut cursor)?;
        let top_item = read_bool(&mut cursor)?;
        let date_binary = read_i64_le(&mut cursor)?;
        let can_buy_credit = read_bool(&mut cursor)?;
        let can_buy_gold = read_bool(&mut cursor)?;

        out.push(GameShopItemRecord {
            item_index,
            g_index,
            gold_price,
            credit_price,
            count,
            class,
            category,
            stock,
            i_stock,
            deal,
            top_item,
            date_binary,
            can_buy_credit,
            can_buy_gold,
        });
    }

    Ok(out)
}

pub fn load_magic_infos_from_exports<P: AsRef<Path>>(path: P) -> io::Result<Vec<MagicInfo>> {
    let path = path.as_ref();
    let bytes = content::read_bytes(path)?;
    let mut cursor = std::io::Cursor::new(bytes);

    let count = read_i32_le(&mut cursor)?;
    if count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative magic count {count} in exports"),
        ));
    }

    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        // MagicInfo.Save layout from Server/MirDatabase/MagicInfo.cs
        let name = read_string(&mut cursor)?;
        let spell = read_u8(&mut cursor)?;
        let base_cost = read_u8(&mut cursor)?;
        let level_cost = read_u8(&mut cursor)?;
        let icon = read_u8(&mut cursor)?;
        let level1 = read_u8(&mut cursor)?;
        let level2 = read_u8(&mut cursor)?;
        let level3 = read_u8(&mut cursor)?;
        let need1 = read_u16_le(&mut cursor)?;
        let need2 = read_u16_le(&mut cursor)?;
        let need3 = read_u16_le(&mut cursor)?;
        let delay_base = read_u32_le(&mut cursor)?;
        let delay_reduction = read_u32_le(&mut cursor)?;
        let power_base = read_u16_le(&mut cursor)?;
        let power_bonus = read_u16_le(&mut cursor)?;
        let mpower_base = read_u16_le(&mut cursor)?;
        let mpower_bonus = read_u16_le(&mut cursor)?;
        let range = read_u8(&mut cursor)?;
        let multiplier_base = read_f32_le(&mut cursor)?;
        let multiplier_bonus = read_f32_le(&mut cursor)?;

        out.push(MagicInfo {
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
        });
    }

    Ok(out)
}

pub fn load_quest_infos_from_exports<P: AsRef<Path>>(path: P) -> io::Result<Vec<CoreQuestInfo>> {
    let path = path.as_ref();
    let bytes = content::read_bytes(path)?;
    let mut cursor = std::io::Cursor::new(bytes);

    let count = read_i32_le(&mut cursor)?;
    if count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative quest count {count} in exports"),
        ));
    }

    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let index = read_i32_le(&mut cursor)?;
        let name = read_string(&mut cursor)?;
        let group = read_string(&mut cursor)?;
        let file_name = read_string(&mut cursor)?;
        let required_min_level = read_i32_le(&mut cursor)?;
        let mut required_max_level = read_i32_le(&mut cursor)?;
        let required_quest = read_i32_le(&mut cursor)?;
        let required_class = read_u8(&mut cursor)?;
        let quest_type = read_u8(&mut cursor)?;
        let goto_message = read_string(&mut cursor)?;
        let kill_message = read_string(&mut cursor)?;
        let item_message = read_string(&mut cursor)?;
        let flag_message = read_string(&mut cursor)?;
        let time_limit_in_seconds = read_i32_le(&mut cursor)?;

        if required_max_level == 0 {
            required_max_level = u16::MAX as i32;
        }

        out.push(CoreQuestInfo {
            id: QuestId(index),
            npc_index: 0,
            finish_npc_index: 0,
            name,
            group,
            file_name,
            goto_message,
            kill_message,
            item_message,
            flag_message,
            description: Vec::new(),
            task_description: Vec::new(),
            return_description: Vec::new(),
            completion_description: Vec::new(),
            required_min_level,
            required_max_level,
            required_quest,
            required_class: RequiredClass(required_class),
            quest_type: QuestType(quest_type),
            time_limit_seconds: time_limit_in_seconds,
            carry_items: Vec::new(),
            kill_tasks: Vec::new(),
            item_tasks: Vec::new(),
            flag_tasks: Vec::new(),
            fixed_rewards: Vec::new(),
            select_rewards: Vec::new(),
            gold_reward: 0,
            exp_reward: 0,
            credit_reward: 0,
        });
    }

    Ok(out)
}

pub fn load_monster_infos_from_exports<P: AsRef<Path>>(path: P) -> io::Result<Vec<MonsterInfo>> {
    let path = path.as_ref();
    let bytes = content::read_bytes(path)?;
    let mut cursor = std::io::Cursor::new(bytes);

    let count = read_i32_le(&mut cursor)?;
    if count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative monster count {count} in exports"),
        ));
    }

    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let index = read_i32_le(&mut cursor)?;
        let name = read_string(&mut cursor)?;
        let image = read_u16_le(&mut cursor)?;
        let ai = read_u8(&mut cursor)?;
        let effect = read_u8(&mut cursor)?;
        let level = read_u16_le(&mut cursor)?;
        let view_range = read_u8(&mut cursor)?;
        let cool_eye = read_u8(&mut cursor)?;
        let stats = read_stats(&mut cursor)?;
        let light = read_u8(&mut cursor)?;
        let attack_speed = read_u16_le(&mut cursor)?;
        let move_speed = read_u16_le(&mut cursor)?;
        let experience = read_u32_le(&mut cursor)?;
        let can_push = read_bool(&mut cursor)?;
        let can_tame = read_bool(&mut cursor)?;
        let auto_rev = read_bool(&mut cursor)?;
        let undead = read_bool(&mut cursor)?;
        let drop_path = read_string(&mut cursor)?;

        out.push(MonsterInfo {
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
        });
    }

    Ok(out)
}

pub fn load_npc_infos_from_exports<P: AsRef<Path>>(path: P) -> io::Result<Vec<NpcInfo>> {
    let path = path.as_ref();
    let bytes = content::read_bytes(path)?;
    let mut cursor = std::io::Cursor::new(bytes);

    let count = read_i32_le(&mut cursor)?;
    if count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative npc count {count} in exports"),
        ));
    }

    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let index = read_i32_le(&mut cursor)?;
        let map_index = read_i32_le(&mut cursor)?;

        let collect_count = read_i32_le(&mut cursor)?;
        if collect_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative CollectQuestIndexes count {collect_count} in exports NPCInfo"),
            ));
        }
        let mut collect_quest_indexes = Vec::with_capacity(collect_count as usize);
        for _ in 0..collect_count {
            collect_quest_indexes.push(read_i32_le(&mut cursor)?);
        }

        let finish_count = read_i32_le(&mut cursor)?;
        if finish_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative FinishQuestIndexes count {finish_count} in exports NPCInfo"),
            ));
        }
        let mut finish_quest_indexes = Vec::with_capacity(finish_count as usize);
        for _ in 0..finish_count {
            finish_quest_indexes.push(read_i32_le(&mut cursor)?);
        }

        let file_name = read_string(&mut cursor)?;
        let name = read_string(&mut cursor)?;

        let location_x = read_i32_le(&mut cursor)?;
        let location_y = read_i32_le(&mut cursor)?;
        let image = read_u16_le(&mut cursor)?;
        let rate = read_u16_le(&mut cursor)?;

        let time_visible = read_bool(&mut cursor)?;
        let hour_start = read_u8(&mut cursor)?;
        let minute_start = read_u8(&mut cursor)?;
        let hour_end = read_u8(&mut cursor)?;
        let minute_end = read_u8(&mut cursor)?;
        let min_lev = read_i16_le(&mut cursor)?;
        let max_lev = read_i16_le(&mut cursor)?;
        let day_of_week = read_string(&mut cursor)?;
        let class_required = read_string(&mut cursor)?;
        let conquest = read_i32_le(&mut cursor)?;
        let flag_needed = read_i32_le(&mut cursor)?;

        let show_on_big_map = read_bool(&mut cursor)?;
        let big_map_icon = read_i32_le(&mut cursor)?;
        let can_teleport_to = read_bool(&mut cursor)?;
        let conquest_visible = read_bool(&mut cursor)?;

        out.push(NpcInfo {
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
            sabuk: false,
            flag_needed,
            conquest,
            show_on_big_map,
            big_map_icon,
            can_teleport_to,
            conquest_visible,
            collect_quest_indexes,
            finish_quest_indexes,
        });
    }

    Ok(out)
}
