use std::io::{self, Read};

use crate::world::monster::MonsterInfo;

use super::header::*;

pub(super) fn read_monster_info<R: Read>(r: &mut R) -> io::Result<MonsterInfo> {
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

pub(super) fn skip_monster_info<R: Read>(r: &mut R, _version: i32, _custom_version: i32) -> io::Result<()> {
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
