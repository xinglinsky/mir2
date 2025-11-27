use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicU32;
use std::time::Instant;

use crystal_server_core::account::{AccountStore, CharacterPosition, CharacterSummary};
use crystal_server_core::world::{self, WorldConfig, WorldDatabase};
use crystal_server_core::world::WorldProvider;
use crystal_server_core::world::magic::{UserMagic as WorldUserMagic, encode_client_magic_bytes};
use crystal_shared_proto::io::write_bool;
use crystal_shared_proto::login::{CKeepAlive, SKeepAlive};
use crystal_shared_proto::packet::RawPacket;
use crystal_shared_proto::scene::{SMagicLeveled, SNewMagic};
use crystal_shared_proto::select::{SelectInfo, SLogOutFailed, SLogOutSuccess};

use super::{LoginConnection, PlayerVisual, Stage};

impl LoginConnection {
    pub(crate) const DATA_RANGE: i32 = 16;
    /// Synthetic object id used for the Default NPC script (00Default.txt).
    /// The client learns this id from an SDefaultNpc packet and will use it
    /// in subsequent CCallNPC requests for DefaultNPCType.UseItem,
    /// TownScroll/DungeonScroll, etc.
    pub(crate) const DEFAULT_NPC_ID: u32 = 1_500_000;

    pub(crate) fn new(
        session_id: world::SessionId,
        store: Arc<dyn AccountStore>,
        world_db: Arc<WorldDatabase>,
        world_config: WorldConfig,
        world: Arc<Mutex<world::World<WorldDatabase>>>,
        exp_table: Arc<Vec<i64>>,
        player_summaries: Arc<Mutex<HashMap<world::SessionId, PlayerVisual>>>,
        outboxes: Arc<Mutex<HashMap<world::SessionId, Vec<Vec<u8>>>>>,
        online_accounts: Arc<Mutex<HashMap<String, world::SessionId>>>,
        active_connections: Arc<AtomicU32>,
        timeout_ms: u64,
    ) -> Self {
        LoginConnection {
            stage: Stage::Connected,
            session_id,
            account_id: None,
            online_accounts,
            characters: Vec::new(),
            store,
            world_db,
            world_config,
            world,
            exp_table,
            current_map_index: 0,
            current_x: 0,
            current_y: 0,
            direction: 0,
            current_char_index: None,
            current_stats: None,
            known_monsters: HashSet::new(),
            known_npcs: HashSet::new(),
            known_players: HashSet::new(),
            player_summaries,
            outboxes,
            active_connections,
            last_active: Instant::now(),
            timeout_ms,
            closing: false,
            last_move_kind: None,
            can_create_guild: false,
            is_gm: false,
            gm_login: false,
            pending_guild_invite: None,
            world_map_setup_sent: false,
            sent_map_infos: HashSet::new(),
        }
    }

    pub(crate) fn encode_raw(raw: RawPacket) -> Vec<u8> {
        raw.encode()
    }

    /// Send a NewMagic packet for the given learned magic, mirroring the
    /// C# SendMagicInfo(UserMagic) flow. This builds ClientMagic.Save(writer)
    /// bytes from MagicInfo + UserMagic and appends the Hero bool (false).
    #[allow(dead_code)]
    pub(crate) fn send_new_magic(&self, magic: &WorldUserMagic, out: &mut Vec<Vec<u8>>) {
        if let Some(info) = self.world_db.get_magic_info(magic.spell) {
            if let Ok(mut bytes) = encode_client_magic_bytes(info, magic, 0) {
                if write_bool(&mut bytes, false).is_err() {
                    return;
                }
                let pkt = SNewMagic { magic_bytes: bytes };
                let raw = pkt.encode();
                out.push(Self::encode_raw(raw));
            }
        }
    }

    /// Send a MagicLeveled packet to notify the client that a magic's level
    /// and experience have changed, mirroring C# HumanObject.MagicLeveled.
    #[allow(dead_code)]
    pub(crate) fn send_magic_leveled(
        &self,
        spell: u8,
        level: u8,
        experience: u16,
        out: &mut Vec<Vec<u8>>,
    ) {
        let pkt = SMagicLeveled {
            object_id: self.session_id,
            spell,
            level,
            experience,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_log_out(&mut self, out: &mut Vec<Vec<u8>>) {
        self.on_disconnect(out);
    }

    fn on_disconnect(&mut self, out: &mut Vec<Vec<u8>>) {
        if self.stage == Stage::InGame {
            if let (Some(ref account_id), Some(char_idx)) =
                (self.account_id.as_ref(), self.current_char_index)
            {
                let magics = {
                    let world = self.world.lock().unwrap();
                    world.player_magics(self.session_id)
                };
                let _ = self
                    .store
                    .save_character_magics(account_id, char_idx, &magics);

                let (inventory, equipment) = {
                    let world = self.world.lock().unwrap();
                    world
                        .player_items(self.session_id)
                        .unwrap_or((
                            crystal_server_core::item::Inventory::new_default(),
                            crystal_server_core::item::Equipment::new_default(),
                        ))
                };
                let _ = self
                    .store
                    .save_character_items(account_id, char_idx, &inventory, &equipment);

                let pos = CharacterPosition {
                    map_index: self.current_map_index,
                    x: self.current_x,
                    y: self.current_y,
                    direction: self.direction,
                };
                let _ = self
                    .store
                    .save_character_position(account_id, char_idx, &pos);

                if let Some(ch) = self
                    .characters
                    .iter()
                    .find(|c| c.index == char_idx)
                {
                    let _ = self
                        .store
                        .update_character_level(account_id, char_idx, ch.level);
                }
            }

            // Remove the player from the world state and occupancy tracking so
            // that disconnected characters no longer block movement.
            {
                let mut world = self.world.lock().unwrap();
                world.remove_player_from_world(self.session_id);
            }
        }

        if let Some(ref acc_id) = self.account_id {
            let chars: Vec<SelectInfo> = self
                .store
                .list_characters(acc_id)
                .unwrap_or_default()
                .into_iter()
                .map(|c: CharacterSummary| SelectInfo {
                    index: c.index,
                    name: c.name,
                    level: c.level,
                    class: c.class,
                    gender: c.gender,
                    last_access_binary: c.last_access_binary,
                })
                .collect();
            self.characters = chars.clone();

            let resp = SLogOutSuccess { characters: chars };
            if let Ok(raw) = resp.encode() {
                out.push(Self::encode_raw(raw));
            }

            self.stage = Stage::Select;
            self.current_char_index = None;

            {
                let mut map = self.player_summaries.lock().unwrap();
                map.remove(&self.session_id);
            }
        } else {
            let resp = SLogOutFailed;
            out.push(Self::encode_raw(resp.encode()));
        }
    }

    pub(crate) fn handle_keep_alive(&mut self, msg: CKeepAlive, out: &mut Vec<Vec<u8>>) {
        let resp = SKeepAlive { time: msg.time };
        if let Ok(raw) = resp.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
}
