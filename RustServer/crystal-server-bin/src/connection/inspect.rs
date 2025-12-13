use crystal_shared_proto::login::{CInspect, CObserve, CRequestChatItem, CRequestUserName};
use crystal_shared_proto::item::SChatItemStats;
use crystal_shared_proto::stats::SUserName;
use crystal_shared_proto::user::SPlayerInspect;
use crystal_server_core::world;
use crystal_server_core::world::configs::setup_config;

use super::{hero_owner_session_from_object_id, LoginConnection, Stage};

impl LoginConnection {
    pub(crate) fn handle_request_user_name(&mut self, msg: CRequestUserName, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let name_opt = if let Some(target_sid) = {
            let world = self.world.lock().unwrap();
            world.find_session_by_character_index(msg.user_id as i32)
        } {
            let world = self.world.lock().unwrap();
            world.player_name(target_sid)
        } else if let (Some(account_id), _) = (self.account_id.clone(), self.current_char_index) {
            self.store
                .list_characters(&account_id)
                .ok()
                .and_then(|chars| {
                    chars
                        .into_iter()
                        .find(|c| c.index == msg.user_id as i32)
                        .map(|c| c.name)
                })
        } else {
            None
        };

        let Some(name) = name_opt else {
            return;
        };

        let pkt = SUserName {
            id: msg.user_id,
            name,
        };

        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_inspect(&mut self, msg: CInspect, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        if msg.hero {
            let target_sid = {
                hero_owner_session_from_object_id(msg.object_id)
                    .unwrap_or(self.session_id)
            };

            let snapshot_opt = {
                let world = self.world.lock().unwrap();
                world.player_inspect_snapshot(target_sid)
            };

            let Some((name, _guild_name, class, gender, hair, level, _equipment, _player_allow_observe)) =
                snapshot_opt
            else {
                return;
            };

            let pkt = SPlayerInspect {
                name: format!("{}'s Hero", name),
                guild_name: String::new(),
                guild_rank: String::new(),
                equipment: vec![None; 14],
                class,
                gender,
                hair,
                level,
                lover_name: String::new(),
                allow_observe: false,
                is_hero: true,
            };

            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        let target_sid_opt = if msg.ranking {
            let world = self.world.lock().unwrap();
            world.find_session_by_character_index(msg.object_id as i32)
        } else {
            let world = self.world.lock().unwrap();
            world.session_id_from_object_id(msg.object_id)
        };

        let Some(target_sid) = target_sid_opt else {
            return;
        };

        let snapshot_opt = {
            let world = self.world.lock().unwrap();
            world.player_inspect_snapshot(target_sid)
        };

        let Some((name, guild_name, class, gender, hair, level, equipment, player_allow_observe)) = snapshot_opt
        else {
            return;
        };

        let guild_rank = {
            let map = self.player_summaries.lock().unwrap();
            if let Some(v) = map.get(&target_sid) {
                v.guild_rank_name.clone()
            } else {
                String::new()
            }
        };

        let allow_observe = player_allow_observe && setup_config().observe.allow_observe;

        let pkt = SPlayerInspect {
            name,
            guild_name,
            guild_rank,
            equipment,
            class,
            gender,
            hair,
            level,
            lover_name: String::new(),
            allow_observe,
            is_hero: false,
        };

        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_observe(&mut self, msg: CObserve, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        if !setup_config().observe.allow_observe {
            self.send_system_chat("观察功能已在服务器设置中关闭。", out);
            return;
        }

        let target_sid_opt = {
            let world = self.world.lock().unwrap();
            world.find_session_by_name(&msg.name)
        };

        let Some(target_sid) = target_sid_opt else {
            self.send_system_chat("未找到目标玩家。", out);
            return;
        };

        let allow_target = {
            let world = self.world.lock().unwrap();
            world.player_allow_observe(target_sid).unwrap_or(false)
        };

        if !allow_target {
            self.send_system_chat("该玩家已关闭观察功能。", out);
            return;
        }

        let loc_opt = {
            let world = self.world.lock().unwrap();
            world.player_location(target_sid)
        };

        let Some((map_index, x, y, _direction)) = loc_opt else {
            self.send_system_chat("无法获取目标位置。", out);
            return;
        };

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::Teleport {
                session_id: self.session_id,
                map_index,
                x,
                y,
            })
        };

        let map_changed = self.handle_world_events(events, out);
        if map_changed {
            self.known_monsters.clear();
            self.known_npcs.clear();
            self.known_heroes.clear();
            self.update_visibility(out);
        }

        self.send_system_chat("已进入观察位置（最小实现）。", out);
    }

    pub(crate) fn handle_request_chat_item(&mut self, msg: CRequestChatItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let item_opt = {
            let cache = self.chat_item_cache.lock().unwrap();
            cache.get(&msg.chat_item_id).cloned()
        };

        let Some(item) = item_opt else {
            return;
        };

        if let Ok(pkt) = SChatItemStats::from_user_item(msg.chat_item_id, &item) {
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
        }
    }
}
