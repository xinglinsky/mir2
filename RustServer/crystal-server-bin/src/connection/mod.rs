use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::sync::atomic::AtomicU32;

use crystal_server_core::account::{AccountStore, CharacterStats};
use crystal_server_core::world::{self, WorldConfig, WorldDatabase};
use crystal_shared_proto::item_types::UserItemData;
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
pub mod gameshop;
pub mod mail;
pub mod chat;
pub mod group;
pub mod guild;
pub mod item;
pub mod trade;
pub mod friend;
pub mod ranking;
pub mod quest;
pub mod inspect;

pub(crate) const NPC_OBJECT_ID_BASE: u32 = 0x2000_0000;
pub(crate) const HERO_OBJECT_ID_BASE: u32 = 0x4000_0000;

pub(crate) fn npc_object_id(npc_index: i32) -> u32 {
    NPC_OBJECT_ID_BASE | (npc_index as u32)
}

pub(crate) fn npc_index_from_object_id(object_id: u32) -> Option<i32> {
    if (object_id & NPC_OBJECT_ID_BASE) != NPC_OBJECT_ID_BASE {
        return None;
    }
    Some((object_id & !NPC_OBJECT_ID_BASE) as i32)
}

pub(crate) fn hero_object_id(owner_session_id: world::SessionId) -> u32 {
    HERO_OBJECT_ID_BASE | (owner_session_id as u32)
}

pub(crate) fn hero_owner_session_from_object_id(object_id: u32) -> Option<world::SessionId> {
    if (object_id & HERO_OBJECT_ID_BASE) != HERO_OBJECT_ID_BASE {
        return None;
    }
    Some((object_id & !HERO_OBJECT_ID_BASE) as world::SessionId)
}

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
    pub(crate) known_heroes: HashSet<world::SessionId>,
    /// NPC index (as sent in CCallNPC.object_id) for which the storage page
    /// is currently open, if any. Used to validate StoreItem/TakeBackItem
    /// requests similarly to C# PlayerObject.NPCPage/NPCObjectID.
    pub(crate) current_storage_npc_id: Option<u32>,
    pub(crate) player_summaries: Arc<Mutex<HashMap<world::SessionId, PlayerVisual>>>,
    pub(crate) chat_item_cache: Arc<Mutex<HashMap<u64, UserItemData>>>,
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
    /// Whether the client is currently allowed to request a full guild
    /// storage item list. This mirrors the C# PlayerObject.GuildCanRequestItems
    /// flag, which is set to true on guild join/create and set to false
    /// after the client issues a Type=3 GuildStorageItemChange request.
    pub(crate) guild_can_request_items: bool,
    /// Whether SWorldMapSetupInfo has been sent to this client.
    pub(crate) world_map_setup_sent: bool,
    /// Set of map indices for which SNewMapInfo has already been sent.
    pub(crate) sent_map_infos: HashSet<i32>,
    #[allow(dead_code)]
    pub(crate) trade_partner: Option<world::SessionId>,
    #[allow(dead_code)]
    pub(crate) trade_gold_offered: u32,
    #[allow(dead_code)]
    pub(crate) trade_locked: bool,
}
