use std::path::Path;

use crystal_server_core::world::{self, WorldProvider};
use crystal_shared_proto::item::{CDropItem, SEquipItem, SMoveItem, SRemoveItem, SUseItem};
use crystal_shared_proto::login::{CEquipItem, CMoveItem, CRemoveItem, CUseItem};
use crystal_shared_proto::npc::SNpcUpdate;
use crystal_shared_proto::scene::{
    SObjectTeleportIn,
    SObjectTeleportOut,
    SNpcResponse,
    STeleportIn,
};
use crystal_shared_proto::user::{SHealthChanged, SUserSlotsRefresh};

use super::{LoginConnection, Stage};

impl LoginConnection {
    pub(crate) fn handle_use_item(&mut self, msg: CUseItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        if msg.grid != 1 {
            let pkt = SUseItem {
                unique_id: msg.unique_id,
                success: false,
                grid: msg.grid,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        let mut found_index: Option<usize> = None;
        for (idx, slot) in inv.slots.iter().enumerate() {
            if let Some(item) = slot {
                if item.unique_id == msg.unique_id {
                    found_index = Some(idx);
                    break;
                }
            }
        }

        let idx = match found_index {
            Some(i) => i,
            None => {
                let pkt = SUseItem {
                    unique_id: msg.unique_id,
                    success: false,
                    grid: msg.grid,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        let item = match inv.slots[idx].clone() {
            Some(it) => it,
            None => {
                let pkt = SUseItem {
                    unique_id: msg.unique_id,
                    success: false,
                    grid: msg.grid,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        let can_use = {
            let world = self.world.lock().unwrap();
            world.can_use_item_for_player(self.session_id, &item)
        };

        if !can_use {
            let pkt = SUseItem {
                unique_id: msg.unique_id,
                success: false,
                grid: msg.grid,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        let info = match self
            .world_db
            .item_infos
            .iter()
            .find(|i| i.index == item.item_index)
        {
            Some(i) => i,
            None => {
                let pkt = SUseItem {
                    unique_id: msg.unique_id,
                    success: false,
                    grid: msg.grid,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        if info.item_type == 21 {
            let shape = info.shape;
            if shape == 3 || shape == 4 {
                let opened = self.open_default_useitem_page(shape, out);
                if !opened {
                    let pkt = SUseItem {
                        unique_id: msg.unique_id,
                        success: false,
                        grid: msg.grid,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    return;
                }

                if item.count <= 1 {
                    inv.slots[idx] = None;
                } else {
                    let mut updated = item.clone();
                    updated.count = updated.count.saturating_sub(1);
                    inv.slots[idx] = Some(updated);
                }

                {
                    let mut world = self.world.lock().unwrap();
                    world.set_player_items(self.session_id, inv.clone(), eq.clone());
                }

                let pkt = SUseItem {
                    unique_id: msg.unique_id,
                    success: true,
                    grid: msg.grid,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }

                let refresh = SUserSlotsRefresh {
                    inventory: inv.slots,
                    equipment: eq.slots,
                };
                if let Ok(raw) = refresh.encode() {
                    out.push(Self::encode_raw(raw));
                }

                return;
            } else {
                let pkt = SUseItem {
                    unique_id: msg.unique_id,
                    success: false,
                    grid: msg.grid,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        }

        // Handle scroll items (ItemType::Scroll == 17) with a minimal
        // teleport behaviour mirroring the C# PlayerObject.UseItem scroll
        // cases for shapes 0 (DungeonEscape), 1 (TownTeleport), and 2
        // (RandomTeleport). All other shapes fall through as unsupported
        // for now.
        if info.item_type == 17 {
            let mut teleported = false;

            match info.shape {
                // Shape 0: Dungeon Escape -> teleport near bind location on
                // the bind map using a simple random offset.
                0 => {
                    let (dest_map, dest_x, dest_y, _dest_dir) = if let (
                        Some(ref account_id),
                        Some(char_idx),
                    ) = (self.account_id.as_ref(), self.current_char_index)
                    {
                        if let Ok(Some(pos)) = self.store.load_character_bind(account_id, char_idx)
                        {
                            (pos.map_index, pos.x, pos.y, pos.direction)
                        } else {
                            (
                                self.current_map_index,
                                self.current_x,
                                self.current_y,
                                self.direction,
                            )
                        }
                    } else {
                        (
                            self.current_map_index,
                            self.current_x,
                            self.current_y,
                            self.direction,
                        )
                    };

                    let radius: i32 = 100;
                    let span = radius * 2 + 1;
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .subsec_nanos() as i32;
                    let dx = if span > 0 { now % span - radius } else { 0 };
                    let dy = if span > 0 { (now / span) % span - radius } else { 0 };
                    let tx = (dest_x + dx).max(0);
                    let ty = (dest_y + dy).max(0);

                    let events = {
                        let mut world = self.world.lock().unwrap();
                        world.handle_command(world::WorldCommand::Teleport {
                            session_id: self.session_id,
                            map_index: dest_map,
                            x: tx,
                            y: ty,
                        })
                    };

                    let map_changed = self.handle_world_events(events, out);
                    if map_changed {
                        self.known_monsters.clear();
                        self.known_npcs.clear();
                        self.update_visibility(out);
                    }

                    teleported = true;
                }
                // Shape 1: Town Teleport -> teleport directly to bind
                // location.
                1 => {
                    let (dest_map, dest_x, dest_y, _dest_dir) = if let (
                        Some(ref account_id),
                        Some(char_idx),
                    ) = (self.account_id.as_ref(), self.current_char_index)
                    {
                        if let Ok(Some(pos)) = self.store.load_character_bind(account_id, char_idx)
                        {
                            (pos.map_index, pos.x, pos.y, pos.direction)
                        } else {
                            (
                                self.current_map_index,
                                self.current_x,
                                self.current_y,
                                self.direction,
                            )
                        }
                    } else {
                        (
                            self.current_map_index,
                            self.current_x,
                            self.current_y,
                            self.direction,
                        )
                    };

                    let events = {
                        let mut world = self.world.lock().unwrap();
                        world.handle_command(world::WorldCommand::Teleport {
                            session_id: self.session_id,
                            map_index: dest_map,
                            x: dest_x,
                            y: dest_y,
                        })
                    };

                    let map_changed = self.handle_world_events(events, out);
                    if map_changed {
                        self.known_monsters.clear();
                        self.known_npcs.clear();
                        self.update_visibility(out);
                    }

                    teleported = true;
                }
                // Shape 2: Random Teleport -> teleport to a random walkable
                // location on the current map. This mirrors the C#
                // MapObject.TeleportRandom implementation, which samples
                // from Map.WalkableCells to ensure the destination is
                // always a valid, walkable tile.
                2 => {
                    if let Some(map_info) = self.world_db.get_map_info(self.current_map_index) {
                        let info = map_info.clone();
                        let dir = &self.world_config.map_path;

                        match crystal_server_core::world::map::load_map_from_file(info, dir.as_path()) {
                            Ok(map) => {
                                if map.walkable_cells.is_empty() {
                                    // No known walkable cells; treat as a
                                    // failure and let the outer logic send
                                    // a failed SUseItem without consuming
                                    // the scroll.
                                } else {
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .subsec_nanos() as usize;
                                    let idx = now % map.walkable_cells.len();
                                    let (tx, ty) = map.walkable_cells[idx];

                                    let events = {
                                        let mut world = self.world.lock().unwrap();
                                        world.handle_command(world::WorldCommand::Teleport {
                                            session_id: self.session_id,
                                            map_index: self.current_map_index,
                                            x: tx as i32,
                                            y: ty as i32,
                                        })
                                    };

                                    let map_changed = self.handle_world_events(events, out);
                                    if map_changed {
                                        self.known_monsters.clear();
                                        self.known_npcs.clear();
                                        self.update_visibility(out);
                                    }

                                    teleported = true;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "RandomTeleport: failed to load map {}: {:?}",
                                    self.current_map_index,
                                    e
                                );
                            }
                        }
                    }
                }
                _ => {}
            }

            if !teleported {
                let pkt = SUseItem {
                    unique_id: msg.unique_id,
                    success: false,
                    grid: msg.grid,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }

            let tele_out = SObjectTeleportOut {
                object_id: self.session_id,
                teleport_type: 0,
            };
            if let Ok(raw) = tele_out.encode() {
                out.push(Self::encode_raw(raw));
            }

            let tele_in = STeleportIn;
            out.push(Self::encode_raw(tele_in.encode()));

            let obj_tele_in = SObjectTeleportIn {
                object_id: self.session_id,
                teleport_type: 0,
            };
            if let Ok(raw) = obj_tele_in.encode() {
                out.push(Self::encode_raw(raw));
            }

            // Scroll teleport succeeded: consume the item, update inventory,
            // and notify the client. HP/MP are unchanged so we skip the
            // potion-specific health delta logic below.
            if item.count <= 1 {
                inv.slots[idx] = None;
            } else {
                let mut updated = item.clone();
                updated.count = updated.count.saturating_sub(1);
                inv.slots[idx] = Some(updated);
            }

            {
                let mut world = self.world.lock().unwrap();
                world.set_player_items(self.session_id, inv.clone(), eq.clone());
            }

            let pkt = SUseItem {
                unique_id: msg.unique_id,
                success: true,
                grid: msg.grid,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }

            let refresh = SUserSlotsRefresh {
                inventory: inv.slots,
                equipment: eq.slots,
            };
            if let Ok(raw) = refresh.encode() {
                out.push(Self::encode_raw(raw));
            }

            return;
        }

        let (hp_delta, mp_delta) = if info.item_type == 13 {
            let base_stats = crystal_server_core::stats_util::stats_from_map(&info.stats);
            let hp = base_stats.get(crystal_server_core::stats::Stat::HP);
            let mp = base_stats.get(crystal_server_core::stats::Stat::MP);

            // Only support simple HP/MP potions for now. If both deltas are
            // zero, treat the item as an unsupported potion (e.g. buff-only)
            // and do not consume it.
            if hp == 0 && mp == 0 {
                let pkt = SUseItem {
                    unique_id: msg.unique_id,
                    success: false,
                    grid: msg.grid,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }

            (hp, mp)
        } else {
            // Unsupported item type (scripts, etc.) for now: indicate
            // failure and do not consume the item.
            let pkt = SUseItem {
                unique_id: msg.unique_id,
                success: false,
                grid: msg.grid,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        };

        let (cur_hp, cur_mp) = {
            let world = self.world.lock().unwrap();
            world
                .player_current_hp_mp(self.session_id)
                .unwrap_or((0, 0))
        };

        let target_hp = cur_hp.saturating_add(hp_delta);
        let target_mp = cur_mp.saturating_add(mp_delta);

        {
            let mut world = self.world.lock().unwrap();
            let _ = world.set_player_hp_mp(self.session_id, target_hp, target_mp);
        }

        if item.count <= 1 {
            inv.slots[idx] = None;
        } else {
            let mut updated = item.clone();
            updated.count = updated.count.saturating_sub(1);
            inv.slots[idx] = Some(updated);
        }

        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
        }

        let pkt = SUseItem {
            unique_id: msg.unique_id,
            success: true,
            grid: msg.grid,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        let refresh = SUserSlotsRefresh {
            inventory: inv.slots,
            equipment: eq.slots,
        };
        if let Ok(raw) = refresh.encode() {
            out.push(Self::encode_raw(raw));
        }

        let (hp_after, mp_after) = {
            let world = self.world.lock().unwrap();
            world
                .player_current_hp_mp(self.session_id)
                .unwrap_or((target_hp, target_mp))
        };

        let hc = SHealthChanged {
            hp: hp_after,
            mp: mp_after,
        };
        if let Ok(raw) = hc.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    fn open_default_useitem_page(&mut self, shape: i16, out: &mut Vec<Vec<u8>>) -> bool {
        let root_deploy = Path::new("./deploy/Envir/SystemScripts/00Default");
        let root_plain = Path::new("./Envir/SystemScripts/00Default");
        let root = if root_deploy.exists() { root_deploy } else { root_plain };
        if !root.exists() {
            return false;
        }

        let script_name = match shape {
            3 => "TownScroll.txt",
            4 => "DungeonScroll.txt",
            _ => return false,
        };

        let script_path = root.join(script_name);
        if !script_path.is_file() {
            return false;
        }

        let (pages, _) = match Self::load_npc_script_from_file(&script_path) {
            Ok(v) => v,
            Err(_) => return false,
        };

        let key = format!("@_USEITEM({})", shape);
        let page = match pages.get(&key) {
            Some(p) => p.clone(),
            None => return false,
        };

        let upd = SNpcUpdate {
            npc_id: super::LoginConnection::DEFAULT_NPC_ID,
        };
        if let Ok(raw) = upd.encode() {
            out.push(Self::encode_raw(raw));
        }

        let resp = SNpcResponse { page };
        if let Ok(raw) = resp.encode() {
            out.push(Self::encode_raw(raw));
        }

        true
    }

    pub(crate) fn handle_drop_item(
        &mut self,
        msg: CDropItem,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        // For now we only support dropping from the main inventory, and we
        // ignore hero_inventory semantics.
        if msg.count == 0 {
            return;
        }

        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        let mut found_index: Option<usize> = None;
        for (idx, slot) in inv.slots.iter().enumerate() {
            if let Some(item) = slot {
                if item.unique_id == msg.unique_id {
                    found_index = Some(idx);
                    break;
                }
            }
        }

        let idx = match found_index {
            Some(i) => i,
            None => {
                // Nothing to drop; just refresh client view of slots.
                let refresh = SUserSlotsRefresh {
                    inventory: inv.slots,
                    equipment: eq.slots,
                };
                if let Ok(raw) = refresh.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        let item = match inv.slots[idx].clone() {
            Some(it) => it,
            None => {
                let refresh = SUserSlotsRefresh {
                    inventory: inv.slots,
                    equipment: eq.slots,
                };
                if let Ok(raw) = refresh.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        if msg.count as u32 > item.count as u32 {
            let refresh = SUserSlotsRefresh {
                inventory: inv.slots,
                equipment: eq.slots,
            };
            if let Ok(raw) = refresh.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Apply the world-side drop command so that map_items and
        // WorldEvent::ItemDropped are created. The world implementation will
        // adjust its own copy of the inventory; we keep the connection-side
        // view in sync by cloning back from world after the command.
        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::DropItem {
                session_id: self.session_id,
                unique_id: msg.unique_id,
                count: msg.count,
            })
        };

        let _ = self.handle_world_events(events, out);

        let (inv_after, eq_after) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        let refresh = SUserSlotsRefresh {
            inventory: inv_after.slots,
            equipment: eq_after.slots,
        };
        if let Ok(raw) = refresh.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_move_item(&mut self, msg: CMoveItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let success = {
            let mut world = self.world.lock().unwrap();
            world.move_item_in_grid(self.session_id, msg.grid, msg.from, msg.to)
        };

        let pkt = SMoveItem {
            grid: msg.grid,
            from: msg.from,
            to: msg.to,
            success,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        if success {
            let (inv, eq) = {
                let world = self.world.lock().unwrap();
                world
                    .player_items(self.session_id)
                    .unwrap_or((
                        crystal_server_core::item::Inventory::new_default(),
                        crystal_server_core::item::Equipment::new_default(),
                    ))
            };

            let refresh = SUserSlotsRefresh {
                inventory: inv.slots,
                equipment: eq.slots,
            };
            if let Ok(raw) = refresh.encode() {
                out.push(Self::encode_raw(raw));
            }
        }
    }

    pub(crate) fn handle_equip_item(&mut self, msg: CEquipItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let success = {
            let mut world = self.world.lock().unwrap();
            world.equip_item_for_player(self.session_id, msg.grid, msg.unique_id, msg.to)
        };

        let pkt = SEquipItem {
            grid: msg.grid,
            unique_id: msg.unique_id,
            to: msg.to,
            success,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        if success {
            let (inv, eq) = {
                let world = self.world.lock().unwrap();
                world
                    .player_items(self.session_id)
                    .unwrap_or((
                        crystal_server_core::item::Inventory::new_default(),
                        crystal_server_core::item::Equipment::new_default(),
                    ))
            };

            let refresh = SUserSlotsRefresh {
                inventory: inv.slots,
                equipment: eq.slots,
            };
            if let Ok(raw) = refresh.encode() {
                out.push(Self::encode_raw(raw));
            }
        }
    }

    pub(crate) fn handle_remove_item(&mut self, msg: CRemoveItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let success = {
            let mut world = self.world.lock().unwrap();
            world.remove_item_for_player(self.session_id, msg.grid, msg.unique_id, msg.to)
        };

        let pkt = SRemoveItem {
            grid: msg.grid,
            unique_id: msg.unique_id,
            to: msg.to,
            success,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        if success {
            let (inv, eq) = {
                let world = self.world.lock().unwrap();
                world
                    .player_items(self.session_id)
                    .unwrap_or((
                        crystal_server_core::item::Inventory::new_default(),
                        crystal_server_core::item::Equipment::new_default(),
                    ))
            };

            let refresh = SUserSlotsRefresh {
                inventory: inv.slots,
                equipment: eq.slots,
            };
            if let Ok(raw) = refresh.encode() {
                out.push(Self::encode_raw(raw));
            }
        }
    }
}
