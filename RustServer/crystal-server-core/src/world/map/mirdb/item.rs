use std::io::{self, Read};

use super::header::*;

pub(super) fn skip_item_info<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
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
