use std::io::{self, Read};

use crate::world::npc::NpcInfo;

use super::header::*;

pub(super) fn read_npc_info<R: Read>(r: &mut R) -> io::Result<NpcInfo> {
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
