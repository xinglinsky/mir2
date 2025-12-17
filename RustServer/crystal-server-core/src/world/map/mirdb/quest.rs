use std::io::{self, Read};

use crate::quest::{QuestId, QuestInfo as CoreQuestInfo, QuestType, RequiredClass};
use crate::world::magic::MagicInfo;
use crate::conquest::{
    ConquestArcherInfo, ConquestFlagInfo, ConquestGateInfo, ConquestId, ConquestInfo,
    ConquestSiegeInfo, ConquestWallInfo,
};

use super::header::*;

#[derive(Clone, Debug)]
pub struct GameShopItemRecord {
    pub item_index: i32,
    pub g_index: i32,
    pub gold_price: u32,
    pub credit_price: u32,
    pub count: u16,
    pub class: String,
    pub category: String,
    pub stock: i32,
    pub i_stock: bool,
    pub deal: bool,
    pub top_item: bool,
    pub date_binary: i64,
    pub can_buy_credit: bool,
    pub can_buy_gold: bool,
}

/// Read a single QuestInfo record from the MirDB stream.
///
/// This mirrors QuestInfo.Save in Server/MirDatabase/QuestInfo.cs and only
/// materialises the static metadata stored in MirDB. Per-quest script
/// content (Descriptions, Tasks, Rewards) is not present in MirDB and is
/// therefore initialised as empty/default here.
pub(super) fn read_quest_info<R: Read>(
    r: &mut R,
    _version: i32,
    _custom_version: i32,
) -> io::Result<CoreQuestInfo> {
    // QuestInfo.Save layout from Server/MirDatabase/QuestInfo.cs
    let index = read_i32(r)?;
    let name = read_string(r)?;
    let group = read_string(r)?;
    let file_name = read_string(r)?;
    let required_min_level = read_i32(r)?;
    let mut required_max_level = read_i32(r)?;
    let required_quest = read_i32(r)?;
    let required_class = read_u8(r)?;
    let quest_type = read_u8(r)?;
    let goto_message = read_string(r)?;
    let kill_message = read_string(r)?;
    let item_message = read_string(r)?;
    let flag_message = read_string(r)?;
    let time_limit_in_seconds = read_i32(r)?;

    // Match the C# loader behaviour where a max level of 0 means "no upper
    // limit" and is normalised to ushort.MaxValue.
    if required_max_level == 0 {
        required_max_level = u16::MAX as i32;
    }

    Ok(CoreQuestInfo {
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
    })
}

pub(super) fn read_conquest_info<R: Read>(
    r: &mut R,
    _version: i32,
    _custom_version: i32,
) -> io::Result<ConquestInfo> {
    // ConquestInfo.Save layout from Server/MirDatabase/ConquestInfo.cs
    let id = ConquestId(read_i32(r)?);
    let full_map = read_bool(r)?;
    let location_x = read_i32(r)?;
    let location_y = read_i32(r)?;
    let size = read_u16(r)?;
    let name = read_string(r)?;
    let map_index = read_i32(r)?;
    let palace_index = read_i32(r)?;
    let guard_index = read_i32(r)?;
    let gate_index = read_i32(r)?;
    let wall_index = read_i32(r)?;
    let siege_index = read_i32(r)?;
    let flag_index = read_i32(r)?;

    let guard_count = read_i32(r)?;
    if guard_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ConquestGuard count {} in ConquestInfo", guard_count),
        ));
    }
    let mut guards = Vec::with_capacity(guard_count as usize);
    for _ in 0..guard_count {
        guards.push(read_conquest_archer_info(r)?);
    }

    let extra_map_count = read_i32(r)?;
    if extra_map_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ExtraMaps count {} in ConquestInfo", extra_map_count),
        ));
    }
    let mut extra_maps = Vec::with_capacity(extra_map_count as usize);
    for _ in 0..extra_map_count {
        extra_maps.push(read_i32(r)?);
    }

    let gate_count = read_i32(r)?;
    if gate_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ConquestGate count {} in ConquestInfo", gate_count),
        ));
    }
    let mut gates = Vec::with_capacity(gate_count as usize);
    for _ in 0..gate_count {
        gates.push(read_conquest_gate_info(r)?);
    }

    let wall_count = read_i32(r)?;
    if wall_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ConquestWall count {} in ConquestInfo", wall_count),
        ));
    }
    let mut walls = Vec::with_capacity(wall_count as usize);
    for _ in 0..wall_count {
        walls.push(read_conquest_wall_info(r)?);
    }

    let siege_count = read_i32(r)?;
    if siege_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ConquestSiege count {} in ConquestInfo", siege_count),
        ));
    }
    let mut sieges = Vec::with_capacity(siege_count as usize);
    for _ in 0..siege_count {
        sieges.push(read_conquest_siege_info(r)?);
    }

    let flag_count = read_i32(r)?;
    if flag_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative ConquestFlag count {} in ConquestInfo", flag_count),
        ));
    }
    let mut flags = Vec::with_capacity(flag_count as usize);
    for _ in 0..flag_count {
        flags.push(read_conquest_flag_info(r)?);
    }

    let start_hour = read_u8(r)?;
    let war_length = read_i32(r)?;
    let conquest_type = read_u8(r)?;
    let game = read_u8(r)?;

    let monday = read_bool(r)?;
    let tuesday = read_bool(r)?;
    let wednesday = read_bool(r)?;
    let thursday = read_bool(r)?;
    let friday = read_bool(r)?;
    let saturday = read_bool(r)?;
    let sunday = read_bool(r)?;

    let king_location_x = read_i32(r)?;
    let king_location_y = read_i32(r)?;
    let king_size = read_u16(r)?;

    let control_point_index = read_i32(r)?;
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
    let mut control_points = Vec::with_capacity(control_point_count as usize);
    for _ in 0..control_point_count {
        control_points.push(read_conquest_flag_info(r)?);
    }

    Ok(ConquestInfo {
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
    })
}

fn read_conquest_archer_info<R: Read>(r: &mut R) -> io::Result<ConquestArcherInfo> {
    let index = read_i32(r)?;
    let location_x = read_i32(r)?;
    let location_y = read_i32(r)?;
    let mob_index = read_i32(r)?;
    let name = read_string(r)?;
    let repair_cost = read_u32(r)?;
    Ok(ConquestArcherInfo {
        index,
        location_x,
        location_y,
        mob_index,
        name,
        repair_cost,
    })
}

fn read_conquest_gate_info<R: Read>(r: &mut R) -> io::Result<ConquestGateInfo> {
    let index = read_i32(r)?;
    let location_x = read_i32(r)?;
    let location_y = read_i32(r)?;
    let mob_index = read_i32(r)?;
    let name = read_string(r)?;
    let repair_cost = read_i32(r)?;
    Ok(ConquestGateInfo {
        index,
        location_x,
        location_y,
        mob_index,
        name,
        repair_cost,
    })
}

fn read_conquest_wall_info<R: Read>(r: &mut R) -> io::Result<ConquestWallInfo> {
    let index = read_i32(r)?;
    let location_x = read_i32(r)?;
    let location_y = read_i32(r)?;
    let mob_index = read_i32(r)?;
    let name = read_string(r)?;
    let repair_cost = read_i32(r)?;
    Ok(ConquestWallInfo {
        index,
        location_x,
        location_y,
        mob_index,
        name,
        repair_cost,
    })
}

fn read_conquest_siege_info<R: Read>(r: &mut R) -> io::Result<ConquestSiegeInfo> {
    let index = read_i32(r)?;
    let location_x = read_i32(r)?;
    let location_y = read_i32(r)?;
    let mob_index = read_i32(r)?;
    let name = read_string(r)?;
    let repair_cost = read_i32(r)?;
    Ok(ConquestSiegeInfo {
        index,
        location_x,
        location_y,
        mob_index,
        name,
        repair_cost,
    })
}

fn read_conquest_flag_info<R: Read>(r: &mut R) -> io::Result<ConquestFlagInfo> {
    let index = read_i32(r)?;
    let location_x = read_i32(r)?;
    let location_y = read_i32(r)?;
    let name = read_string(r)?;
    let file_name = read_string(r)?;
    Ok(ConquestFlagInfo {
        index,
        location_x,
        location_y,
        name,
        file_name,
    })
}

pub(super) fn skip_quest_info<R: Read>(r: &mut R, version: i32, custom_version: i32) -> io::Result<()> {
    let _ = read_quest_info(r, version, custom_version)?;
    Ok(())
}

pub(super) fn skip_dragon_info<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
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

pub(super) fn skip_magic_info<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
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

pub(super) fn read_game_shop_item<R: Read>(
    r: &mut R,
    _version: i32,
    _custom_version: i32,
) -> io::Result<GameShopItemRecord> {
    let item_index = read_i32(r)?;
    let g_index = read_i32(r)?;
    let gold_price = read_u32(r)?;
    let credit_price = read_u32(r)?;
    let count = read_u16(r)?;
    let class = read_string(r)?;
    let category = read_string(r)?;
    let stock = read_i32(r)?;
    let i_stock = read_bool(r)?;
    let deal = read_bool(r)?;
    let top_item = read_bool(r)?;
    let date_binary = read_i64(r)?;
    let can_buy_credit = read_bool(r)?;
    let can_buy_gold = read_bool(r)?;

    Ok(GameShopItemRecord {
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
    })
}

pub(super) fn skip_game_shop_item<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
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

pub(super) fn skip_conquest_info<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
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

pub(super) fn skip_conquest_archer_info<R: Read>(r: &mut R) -> io::Result<()> {
    let _index = read_i32(r)?;
    let _location_x = read_i32(r)?;
    let _location_y = read_i32(r)?;
    let _mob_index = read_i32(r)?;
    let _name = read_string(r)?;
    let _repair_cost = read_u32(r)?;

    Ok(())
}

pub(super) fn skip_conquest_gate_info<R: Read>(r: &mut R) -> io::Result<()> {
    let _index = read_i32(r)?;
    let _location_x = read_i32(r)?;
    let _location_y = read_i32(r)?;
    let _mob_index = read_i32(r)?;
    let _name = read_string(r)?;
    let _repair_cost = read_i32(r)?;

    Ok(())
}

pub(super) fn skip_conquest_wall_info<R: Read>(r: &mut R) -> io::Result<()> {
    let _index = read_i32(r)?;
    let _location_x = read_i32(r)?;
    let _location_y = read_i32(r)?;
    let _mob_index = read_i32(r)?;
    let _name = read_string(r)?;
    let _repair_cost = read_i32(r)?;

    Ok(())
}

pub(super) fn skip_conquest_siege_info<R: Read>(r: &mut R) -> io::Result<()> {
    let _index = read_i32(r)?;
    let _location_x = read_i32(r)?;
    let _location_y = read_i32(r)?;
    let _mob_index = read_i32(r)?;
    let _name = read_string(r)?;
    let _repair_cost = read_i32(r)?;

    Ok(())
}

pub(super) fn skip_conquest_flag_info<R: Read>(r: &mut R) -> io::Result<()> {
    let _index = read_i32(r)?;
    let _location_x = read_i32(r)?;
    let _location_y = read_i32(r)?;
    let _name = read_string(r)?;
    let _file_name = read_string(r)?;

    Ok(())
}

pub(super) fn skip_respawn_timer<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
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

pub(super) fn skip_gt_map<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
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

pub(super) fn read_magic_info<R: Read>(r: &mut R, version: i32, _custom_version: i32) -> io::Result<MagicInfo> {
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

    // C# loader behaviour:
    //   if (version > 66) Range = reader.ReadByte(); else Range stays at default 9.
    //   if (version > 70) read MultiplierBase/MultiplierBonus, else use defaults 1.0/0.0.
    let mut range: u8 = 9;
    if version > 66 {
        range = read_u8(r)?;
    }

    let mut multiplier_base: f32 = 1.0;
    let mut multiplier_bonus: f32 = 0.0;
    if version > 70 {
        multiplier_base = read_f32(r)?;
        multiplier_bonus = read_f32(r)?;
    }

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
