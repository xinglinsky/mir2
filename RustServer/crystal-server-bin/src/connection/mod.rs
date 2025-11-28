use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::sync::atomic::AtomicU32;

use crystal_server_core::account::{AccountStore, CharacterStats};
use crystal_server_core::world::{self, WorldConfig, WorldDatabase};
use crystal_shared_proto::select::SelectInfo;

pub mod session;
pub mod visibility;
pub mod npc;
pub mod movement;
pub mod handler;
pub mod login_stage;
pub mod select_stage;
pub mod gm_commands;
pub mod map;
pub mod market;
pub mod chat;
pub mod group;
pub mod guild;
pub mod item;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    Connected,
    VersionChecked,
    Select,
    InGame,
}

#[derive(Clone, Debug)]
pub(crate) struct PlayerVisual {
    pub(crate) name: String,
    pub(crate) guild_name: String,
    pub(crate) guild_rank_name: String,
    pub(crate) name_colour_argb: i32,
    pub(crate) class: u8,
    pub(crate) gender: u8,
    pub(crate) level: u16,
    pub(crate) hair: u8,
}

pub(crate) struct LoginConnection {
    pub(crate) stage: Stage,
    pub(crate) session_id: world::SessionId,
    pub(crate) account_id: Option<String>,
    pub(crate) online_accounts: Arc<Mutex<HashMap<String, world::SessionId>>>,
    pub(crate) characters: Vec<SelectInfo>,
    pub(crate) store: Arc<dyn AccountStore>,
    pub(crate) world_db: Arc<WorldDatabase>,
    pub(crate) world_config: WorldConfig,
    pub(crate) world: Arc<Mutex<world::World<WorldDatabase>>>,
    pub(crate) exp_table: Arc<Vec<i64>>,
    pub(crate) current_map_index: i32,
    pub(crate) current_x: i32,
    pub(crate) current_y: i32,
    pub(crate) direction: u8,
    pub(crate) current_char_index: Option<i32>,
    pub(crate) current_stats: Option<CharacterStats>,
    pub(crate) known_monsters: HashSet<u64>,
    pub(crate) known_npcs: HashSet<i32>,
    pub(crate) known_players: HashSet<world::SessionId>,
    pub(crate) player_summaries: Arc<Mutex<HashMap<world::SessionId, PlayerVisual>>>,
    pub(crate) outboxes: Arc<Mutex<HashMap<world::SessionId, Vec<Vec<u8>>>>>,
    pub(crate) active_connections: Arc<AtomicU32>,
    pub(crate) last_active: Instant,
    pub(crate) timeout_ms: u64,
    pub(crate) closing: bool,
    #[allow(dead_code)]
    pub(crate) last_move_kind: Option<u8>,
    #[allow(dead_code)]
    pub(crate) can_create_guild: bool,
    pub(crate) is_gm: bool,
    #[allow(dead_code)]
    pub(crate) gm_login: bool,
    #[allow(dead_code)]
    pub(crate) pending_guild_invite: Option<String>,
    /// Whether SWorldMapSetupInfo has been sent to this client.
    pub(crate) world_map_setup_sent: bool,
    /// Set of map indices for which SNewMapInfo has already been sent.
    pub(crate) sent_map_infos: HashSet<i32>,
}
