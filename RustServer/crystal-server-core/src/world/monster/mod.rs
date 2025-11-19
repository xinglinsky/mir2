use crate::stats::Stats;
use crate::world::drop::DropInfo;

#[derive(Clone, Debug)]
pub struct MonsterInfo {
    pub index: i32,
    pub name: String,
    pub image: u16,
    pub ai: u8,
    pub effect: u8,
    pub view_range: u8,
    pub cool_eye: u8,
    pub level: u16,
    pub light: u8,
    pub attack_speed: u16,
    pub move_speed: u16,
    pub experience: u32,
    pub drop_path: String,
    pub drops: Vec<DropInfo>,
    pub can_tame: bool,
    pub can_push: bool,
    pub auto_rev: bool,
    pub undead: bool,
    pub has_spawn_script: bool,
    pub has_die_script: bool,
    pub stats: Stats,
}

#[derive(Clone, Debug)]
pub struct MonsterInstance {
    pub id: u64,
    pub monster_index: i32,
    pub map_index: i32,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
}
