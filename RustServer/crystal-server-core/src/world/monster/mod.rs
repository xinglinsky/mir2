use crate::stats::Stats;
use crate::world::drop::DropInfo;
use crate::world::types::{BuffType, PetKind};

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
pub struct MonsterBuff {
    pub buff_type: BuffType,
    pub expire_time_ms: i64,
    pub stats: Stats,
    pub infinite: bool,
    pub flag_for_removal: bool,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum MonsterAiState {
    Idle,
    Roam,
    Chase,
    Attack,
}

#[derive(Clone, Debug)]
pub struct MonsterInstance {
    pub id: u64,
    pub monster_index: i32,
    pub map_index: i32,
    pub x: i32,
    pub y: i32,
    pub home_x: i32,
    pub home_y: i32,
    pub direction: u8,
    pub hp: i32,
    pub is_pet: bool,
    pub owner_session_id: Option<u32>,
    pub pet_kind: Option<PetKind>,
    /// Respawn index from RespawnInfo, used to update runtime respawn counts
    /// when the monster dies or despawns.
    pub respawn_index: i32,
    /// Simple runtime AI state for this monster. More detailed behaviour
    /// (timers, pathing, etc.) will be layered on top of this enum.
    pub ai_state: MonsterAiState,
    /// Currently selected player target (session id) if any.
    pub target_session_id: Option<u32>,
    /// Next time (ms since epoch in world time) when this monster is
    /// allowed to perform a movement step towards its target.
    pub next_move_time_ms: i64,
    /// Next time (ms since epoch in world time) when this monster is
    /// allowed to perform a melee attack, mirroring C# MonsterObject.
    /// AttackTime / AttackSpeed.
    pub next_attack_time_ms: i64,
    /// Next time (ms since epoch in world time) when this monster should
    /// re-run its target search logic, mirroring C# MonsterObject.SearchTime
    /// and SearchDelay.
    pub search_time_ms: i64,
    /// Next time (ms since epoch in world time) when this monster is allowed
    /// to perform a random roam step if it has no target, mirroring C#
    /// MonsterObject.RoamTime and RoamDelay.
    pub roam_time_ms: i64,
    pub route_index: i32,
    pub route_wait_until_ms: i64,
    /// Whether this monster considers itself "alone" (no players nearby).
    /// Mirrors C# MonsterObject.Alone, which is used together with
    /// CheckAlone/AloneDelay to optionally skip AI processing when the map
    /// has no nearby players.
    pub alone: bool,
    /// Next time (ms since epoch in world time) when this monster should
    /// re-run its "alone" check, mirroring C# MonsterObject.AloneTime and
    /// AloneDelay.
    pub alone_time_ms: i64,
    pub buff_stats: Stats,
    pub buffs: Vec<MonsterBuff>,
}
