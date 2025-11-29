use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;

use super::data::MapInfo;
use crate::world::magic::MagicInfo;
use crate::world::monster::MonsterInfo;
use crate::world::npc::NpcInfo;
use crystal_shared_proto::item_types::ItemInfoData;

mod header;
mod map;
mod item;
mod monster;
mod npc;
mod quest;

use self::header::*;
use self::map::*;
use self::item::*;
use self::monster::*;
use self::npc::*;
pub use self::quest::*;

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

/// Load GameShopItem records from the GameShopList section of a C# Server.MirDB
/// file, mirroring the layout produced by Envir.SaveDB and
/// Shared/Data/GameShopItem.Save (packet: false).
pub fn load_game_shop_items_from_mirdb<P: AsRef<Path>>(
    path: P,
) -> io::Result<Vec<GameShopItemRecord>> {
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

    // NPCInfoList – walked using read_npc_info to keep the stream in sync.
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

    // MagicInfoList
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

    // GameShopList – only present for DB versions >= 63.
    let mut result = Vec::new();
    if version >= 63 {
        let shop_count = read_i32(&mut reader)?;
        if shop_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative GameShopItem count {} in Server.MirDB", shop_count),
            ));
        }

        result = Vec::with_capacity(shop_count as usize);
        for _ in 0..shop_count {
            let rec = read_game_shop_item(&mut reader, version, custom_version)?;
            result.push(rec);
        }
    }

    Ok(result)
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
