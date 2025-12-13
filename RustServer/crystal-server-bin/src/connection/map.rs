use crystal_server_core::world;
use crystal_server_core::world::WorldProvider;
use crystal_shared_proto::login::{CRequestMapInfo, CSearchMap, CTeleportToNPC};
use crystal_shared_proto::map::{
    SWorldMapSetupInfo,
    SNewMapInfo,
    SSearchMapResult,
};
use crystal_shared_proto::map_types::{
    ClientMapInfoData,
    ClientMovementInfoData,
    ClientNpcInfoData,
};
use crystal_shared_proto::user::SLoseGold;

use super::{LoginConnection, Stage};

const MIN_SEARCH_TEXT_LEN: usize = 3;

impl LoginConnection {
    fn send_world_map_setup_if_needed(&mut self, out: &mut Vec<Vec<u8>>) {
        if self.world_map_setup_sent {
            return;
        }

        if let Ok(pkt) = SWorldMapSetupInfo::from_world_map_setup(
            &self.world_config.world_map_setup,
            self.world_config.teleport_to_npc_cost,
        ) {
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
                self.world_map_setup_sent = true;
            }
        }
    }

    fn send_map_info_if_needed(&mut self, map_index: i32, out: &mut Vec<Vec<u8>>) {
        if self.sent_map_infos.contains(&map_index) {
            return;
        }

        let map_info = match self.world_db.get_map_info(map_index) {
            Some(m) => m,
            None => return,
        };

        let (width, height) = {
            let info = map_info.clone();
            let dir = &self.world_config.map_path;
            match crystal_server_core::world::map::load_map_from_file(info, dir.as_path()) {
                Ok(m) => (m.width as i32, m.height as i32),
                Err(e) => {
                    tracing::warn!("failed to load map {} for big-map info: {:?}", map_index, e);
                    (0, 0)
                }
            }
        };

        if width <= 0 || height <= 0 {
            return;
        }

        let mut movements: Vec<ClientMovementInfoData> = Vec::new();
        for m in map_info.movements.iter().filter(|m| m.show_on_big_map) {
            let dest_index = m.map_index;
            let title = self
                .world_db
                .get_map_info(dest_index)
                .map(|mi| mi.title.clone())
                .unwrap_or_default();

            movements.push(ClientMovementInfoData {
                destination: dest_index,
                title,
                location_x: m.source_x,
                location_y: m.source_y,
                icon: m.icon,
            });
        }

        let mut npcs: Vec<_> = self
            .world_db
            .npc_infos
            .iter()
            .filter(|n| n.map_index == map_index && n.show_on_big_map)
            .collect();
        npcs.sort_by_key(|n| n.big_map_icon);

        let npc_infos: Vec<ClientNpcInfoData> = npcs
            .into_iter()
            .map(|n| ClientNpcInfoData {
                object_id: n.index as u32,
                name: n.name.clone(),
                location_x: n.location_x,
                location_y: n.location_y,
                icon: n.big_map_icon,
                can_teleport_to: n.can_teleport_to,
            })
            .collect();

        let info = ClientMapInfoData {
            width,
            height,
            big_map: map_info.big_map as i32,
            title: map_info.title.clone(),
            movements,
            npcs: npc_infos,
        };

        if let Ok(pkt) = SNewMapInfo::from_map_info(map_index, &info) {
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
                self.sent_map_infos.insert(map_index);
            }
        }
    }

    pub(crate) fn handle_request_map_info(
        &mut self,
        msg: CRequestMapInfo,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        self.send_world_map_setup_if_needed(out);
        self.send_map_info_if_needed(msg.map_index, out);
    }

    pub(crate) fn handle_search_map(&mut self, msg: CSearchMap, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let text = msg.text.trim();
        if text.chars().count() < MIN_SEARCH_TEXT_LEN {
            return;
        }

        let query = text.to_lowercase();
        let mut map_index_opt: Option<i32> = None;
        let mut npc_index_opt: Option<u32> = None;

        {
            if let Some(map) = self
                .world_db
                .map_infos
                .iter()
                .find(|m| m.big_map > 0 && m.title.to_lowercase().starts_with(&query))
            {
                map_index_opt = Some(map.index);
            } else if let Some(npc) = self
                .world_db
                .npc_infos
                .iter()
                .find(|n| n.show_on_big_map && n.name.to_lowercase().starts_with(&query))
            {
                map_index_opt = Some(npc.map_index);
                npc_index_opt = Some(npc.index as u32);
            }
        }

        let (map_index, npc_index) = if let Some(map_index) = map_index_opt {
            // When we have a match, mirror C# behaviour by ensuring the
            // client has current big-map data for that map index.
            self.send_world_map_setup_if_needed(out);
            self.send_map_info_if_needed(map_index, out);

            (map_index, npc_index_opt.unwrap_or(0))
        } else {
            // No map or NPC matched; send a default result with both fields
            // zero, just like C# PlayerObject.SearchMap does.
            (0, 0)
        };

        let result = SSearchMapResult {
            map_index,
            npc_index,
        };

        if let Ok(raw) = result.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_teleport_to_npc(
        &mut self,
        msg: CTeleportToNPC,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => return,
        };

        let npc = match self
            .world_db
            .npc_infos
            .iter()
            .find(|n| {
                n.index as u32 == msg.object_id
                    && n.map_index == self.current_map_index
                    && n.can_teleport_to
            })
        {
            Some(n) => n,
            None => return,
        };

        let cost_i64 = self.world_config.teleport_to_npc_cost as i64;
        if stats.gold < cost_i64 {
            return;
        }

        let mut new_stats = stats.clone();
        new_stats.gold = new_stats.gold.saturating_sub(cost_i64);

        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let _ = self
                .store
                .save_character_stats(account_id, char_idx, &new_stats);
        }

        self.current_stats = Some(new_stats.clone());

        let lose = SLoseGold {
            gold: self.world_config.teleport_to_npc_cost as u32,
        };
        if let Ok(raw) = lose.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Choose a walkable destination near the NPC, approximating the C#
        // NPC.Front/ValidPoint behaviour by preferring the NPC's tile if
        // walkable and otherwise falling back to the closest walkable cell
        // within a small radius.
        let (dest_x, dest_y) = self.find_teleport_destination_near_npc(
            npc.map_index,
            npc.location_x,
            npc.location_y,
        );

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::Teleport {
                session_id: self.session_id,
                map_index: npc.map_index,
                x: dest_x,
                y: dest_y,
            })
        };

        let map_changed = self.handle_world_events(events, out);
        if map_changed {
            self.known_monsters.clear();
            self.known_npcs.clear();
            self.known_heroes.clear();
            self.update_visibility(out);
        }
    }

    fn find_teleport_destination_near_npc(
        &self,
        map_index: i32,
        npc_x: i32,
        npc_y: i32,
    ) -> (i32, i32) {
        let default_pos = (npc_x, npc_y);

        if let Some(info) = self.world_db.get_map_info(map_index).cloned() {
            let dir = &self.world_config.map_path;
            match crystal_server_core::world::map::load_map_from_file(info, dir.as_path()) {
                Ok(map) => {
                    let nx = npc_x.max(0) as u16;
                    let ny = npc_y.max(0) as u16;

                    if map.is_walkable(nx, ny) {
                        (nx as i32, ny as i32)
                    } else {
                        let mut best: Option<(i32, i32, i32)> = None;
                        let max_dist2: i32 = 25; // radius 5

                        for &(wx, wy) in &map.walkable_cells {
                            let dx = wx as i32 - npc_x;
                            let dy = wy as i32 - npc_y;
                            let dist2 = dx * dx + dy * dy;
                            if dist2 > max_dist2 {
                                continue;
                            }

                            match best {
                                Some((_, _, best_d2)) if dist2 >= best_d2 => {}
                                _ => {
                                    best = Some((wx as i32, wy as i32, dist2));
                                }
                            }
                        }

                        if let Some((bx, by, _)) = best {
                            (bx, by)
                        } else {
                            default_pos
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "TeleportToNPC: failed to load map {}: {:?}",
                        map_index,
                        e
                    );
                    default_pos
                }
            }
        } else {
            default_pos
        }
    }
}
