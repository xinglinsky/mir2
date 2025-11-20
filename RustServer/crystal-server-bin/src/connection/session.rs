use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crystal_server_core::account::AccountStore;
use crystal_server_core::world::{self, WorldConfig, WorldDatabase};
use crystal_server_core::world::WorldProvider;
use crystal_server_core::world::magic::{UserMagic as WorldUserMagic, encode_client_magic_bytes};
use crystal_shared_proto::io::write_bool;
use crystal_shared_proto::packet::RawPacket;
use crystal_shared_proto::scene::{SMagicLeveled, SNewMagic};

use super::{LoginConnection, PlayerVisual, Stage};

impl LoginConnection {
    pub(crate) const DATA_RANGE: i32 = 16;

    pub(crate) fn new(
        session_id: world::SessionId,
        store: Arc<dyn AccountStore>,
        world_db: Arc<WorldDatabase>,
        world_config: WorldConfig,
        world: Arc<Mutex<world::World<WorldDatabase>>>,
        exp_table: Arc<Vec<i64>>,
        player_summaries: Arc<Mutex<HashMap<world::SessionId, PlayerVisual>>>,
    ) -> Self {
        LoginConnection {
            stage: Stage::Connected,
            session_id,
            account_id: None,
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
        }
    }

    pub(crate) fn encode_raw(raw: RawPacket) -> Vec<u8> {
        raw.encode()
    }

    /// Send a NewMagic packet for the given learned magic, mirroring the
    /// C# SendMagicInfo(UserMagic) flow. This builds ClientMagic.Save(writer)
    /// bytes from MagicInfo + UserMagic and appends the Hero bool (false).
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
}
