use std::path::Path;

use crystal_server_core::account::AccountStorage;
use crystal_server_core::world::{self, WorldProvider};
use crystal_shared_proto::item::{
    CDropItem,
    CStoreItem,
    CTakeBackItem,
    CRemoveSlotItem,
    CMergeItem,
    CSplitItem,
    CDropGold,
    CEquipSlotItem,
    CCombineItem,
    SDropItem,
    SEquipItem,
    SMoveItem,
    SRemoveItem,
    SRemoveSlotItem,
    SStoreItem,
    STakeBackItem,
    SUseItem,
    SMergeItem,
    SSplitItem,
    SSplitItem1,
    SCombineItem,
};
use crystal_shared_proto::login::{CEquipItem, CMoveItem, CRemoveItem, CUseItem, CAwakeningNeedMaterials, CAwakeningLockedItem, CAwakening, CDisassembleItem, CDowngradeAwakening, CResetAddedItem, CRequestIntelligentCreatureUpdates, CUpdateIntelligentCreature, CIntelligentCreaturePickup, CMarriageRequest, CMarriageReply, CChangeMarriage, CDivorceRequest, CDivorceReply, CAddMentor, CMentorReply, CAllowMentor, CCancelMentor, CGuildBuffUpdate, CNPCConfirmInput, CReportIssue, COpendoor, CGetRentedItems, CItemRentalRequest, CItemRentalFee, CItemRentalPeriod, CDepositRentalItem, CRetrieveRentalItem, CCancelItemRental, CItemRentalLockFee, CItemRentalLockItem, CConfirmItemRental, CGuildTerritoryPage, CPurchaseGuildTerritory};
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
                        self.known_players.clear();
                        self.known_heroes.clear();
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
                        self.known_players.clear();
                        self.known_heroes.clear();
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
                                        self.known_players.clear();
                                        self.known_heroes.clear();
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
            let pkt = SDropItem {
                unique_id: msg.unique_id,
                count: msg.count,
                hero_item: msg.hero_inventory,
                success: false,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        let (inv, eq) = {
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
                // Nothing to drop; send a failed SDropItem and refresh the
                // client view of slots so it can resync.
                let pkt = SDropItem {
                    unique_id: msg.unique_id,
                    count: msg.count,
                    hero_item: msg.hero_inventory,
                    success: false,
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
        };

        let item = match inv.slots[idx].clone() {
            Some(it) => it,
            None => {
                let pkt = SDropItem {
                    unique_id: msg.unique_id,
                    count: msg.count,
                    hero_item: msg.hero_inventory,
                    success: false,
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
        };

        if msg.count as u32 > item.count as u32 {
            let pkt = SDropItem {
                unique_id: msg.unique_id,
                count: msg.count,
                hero_item: msg.hero_inventory,
                success: false,
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

        // Determine whether the drop actually succeeded by comparing the
        // inventory state before and after applying the world command. We
        // treat either a reduced stack count or complete removal of the item
        // as success; unchanged count means the drop was rejected by server
        // rules (e.g. NoThrowItem, DontDrop).
        let mut remaining_count: Option<u16> = None;
        for slot in &inv_after.slots {
            if let Some(it) = slot {
                if it.unique_id == msg.unique_id {
                    remaining_count = Some(it.count);
                    break;
                }
            }
        }

        let success = match remaining_count {
            Some(after_count) => after_count < item.count,
            None => true,
        };

        let pkt = SDropItem {
            unique_id: msg.unique_id,
            count: msg.count,
            hero_item: msg.hero_inventory,
            success,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

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

    pub(crate) fn handle_store_item(&mut self, msg: CStoreItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let mut pkt = SStoreItem {
            from: msg.from,
            to: msg.to,
            success: false,
        };

        // Mirror C# behaviour: only allow storing items while interacting
        // with a storage NPC and remaining within DataRange of that NPC.
        let storage_npc_id = match self.current_storage_npc_id {
            Some(id) => id,
            None => {
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        if let Some(info) = self.world_db.get_npc_info(storage_npc_id as i32) {
            let dx = info.location_x - self.current_x;
            let dy = info.location_y - self.current_y;
            if info.map_index != self.current_map_index
                || dx.abs() > Self::DATA_RANGE
                || dy.abs() > Self::DATA_RANGE
            {
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        } else {
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        let account_id = match &self.account_id {
            Some(id) => id.clone(),
            None => {
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        if msg.from < 0 || msg.to < 0 {
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        let from = msg.from as usize;
        let to = msg.to as usize;

        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        if from >= inv.slots.len() {
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        let mut storage = match self.store.load_account_storage(&account_id) {
            Ok(Some(s)) => s,
            Ok(None) => AccountStorage {
                slots: vec![None; 80],
                has_expanded_storage: false,
                expanded_storage_expiry_binary: 0,
            },
            Err(_) => {
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        if to >= storage.slots.len() {
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Mirror AccountInfo.IsValidStorageIndex behaviour.
        const STORAGE_GRID_SIZE: usize = 80;
        if to >= STORAGE_GRID_SIZE {
            let level = to / STORAGE_GRID_SIZE;
            let max_level = if storage.has_expanded_storage { 1 } else { 0 };
            if level > max_level {
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        }

        let item = match inv.slots.get(from).and_then(|s| s.clone()) {
            Some(it) => it,
            None => {
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        if storage.slots[to].is_some() {
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Enforce BindMode.DontStore (0x0008) like guild storage.
        if let Some(info) = self.world_db.get_item_info(item.item_index) {
            const BIND_DONT_STORE: i16 = 0x0008;
            if (info.bind & BIND_DONT_STORE) != 0 {
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        } else {
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        if let Some(ref rental) = item.rental_information {
            const BIND_DONT_STORE: i16 = 0x0008;
            if (rental.binding_flags & BIND_DONT_STORE) != 0 {
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        }

        // Perform move Inventory -> Storage.
        storage.slots[to] = Some(item);
        inv.slots[from] = None;

        let _ = self.store.save_account_storage(&account_id, &storage);

        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
        }

        pkt.success = true;
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
    }

    pub(crate) fn handle_take_back_item(&mut self, msg: CTakeBackItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let mut pkt = STakeBackItem {
            from: msg.from,
            to: msg.to,
            success: false,
        };

        // Mirror C# behaviour: only allow taking items back while
        // interacting with a storage NPC and remaining within DataRange of
        // that NPC.
        let storage_npc_id = match self.current_storage_npc_id {
            Some(id) => id,
            None => {
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        if let Some(info) = self.world_db.get_npc_info(storage_npc_id as i32) {
            let dx = info.location_x - self.current_x;
            let dy = info.location_y - self.current_y;
            if info.map_index != self.current_map_index
                || dx.abs() > Self::DATA_RANGE
                || dy.abs() > Self::DATA_RANGE
            {
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        } else {
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        let account_id = match &self.account_id {
            Some(id) => id.clone(),
            None => {
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        if msg.from < 0 || msg.to < 0 {
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        let from = msg.from as usize;
        let to = msg.to as usize;

        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        if to >= inv.slots.len() {
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        let mut storage = match self.store.load_account_storage(&account_id) {
            Ok(Some(s)) => s,
            Ok(None) => AccountStorage {
                slots: vec![None; 80],
                has_expanded_storage: false,
                expanded_storage_expiry_binary: 0,
            },
            Err(_) => {
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        if from >= storage.slots.len() {
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Mirror AccountInfo.IsValidStorageIndex behaviour.
        const STORAGE_GRID_SIZE: usize = 80;
        if from >= STORAGE_GRID_SIZE {
            let level = from / STORAGE_GRID_SIZE;
            let max_level = if storage.has_expanded_storage { 1 } else { 0 };
            if level > max_level {
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        }

        if inv.slots[to].is_some() {
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        let item = match storage.slots.get(from).and_then(|s| s.clone()) {
            Some(it) => it,
            None => {
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        // Perform move Storage -> Inventory.
        inv.slots[to] = Some(item);
        storage.slots[from] = None;

        let _ = self.store.save_account_storage(&account_id, &storage);

        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
        }

        pkt.success = true;
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

    pub(crate) fn handle_merge_item(&mut self, msg: CMergeItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Support Inventory (1) and Storage (2) grids
        // Grid 1 = Inventory, Grid 2 = Storage
        if (msg.grid_from != 1 && msg.grid_from != 2) || (msg.grid_to != 1 && msg.grid_to != 2) {
            // Only Inventory and Storage are supported for now
            let pkt = SMergeItem {
                grid_from: msg.grid_from,
                grid_to: msg.grid_to,
                id_from: msg.id_from,
                id_to: msg.id_to,
                success: false,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Handle Storage grids in connection layer (like StoreItem/TakeBackItem)
        let success = if msg.grid_from == 2 || msg.grid_to == 2 {
            // Storage involved - handle in connection layer
            let account_id = match self.account_id.as_ref() {
                Some(id) => id.clone(),
                None => return,
            };

            // Load storage
            let mut storage = match self.store.load_account_storage(&account_id) {
                Ok(Some(s)) => s,
                Ok(None) => crystal_server_core::account::AccountStorage {
                    slots: vec![None; 80],
                    has_expanded_storage: false,
                    expanded_storage_expiry_binary: 0,
                },
                Err(_) => return,
            };

            // Get inventory
            let (mut inv, eq) = {
                let world = self.world.lock().unwrap();
                world
                    .player_items(self.session_id)
                    .unwrap_or((
                        crystal_server_core::item::Inventory::new_default(),
                        crystal_server_core::item::Equipment::new_default(),
                    ))
            };

            // Find items based on grid
            let (from_item, from_slot, from_is_storage) = if msg.grid_from == 1 {
                // From inventory
                match inv.slots.iter().position(|s| s.as_ref().map(|i| i.unique_id) == Some(msg.id_from)) {
                    Some(idx) => {
                        match inv.slots[idx].as_ref() {
                            Some(item) => (Some(item.clone()), Some(idx), false),
                            None => return,
                        }
                    }
                    None => return,
                }
            } else {
                // From storage
                match storage.slots.iter().position(|s| s.as_ref().map(|i| i.unique_id) == Some(msg.id_from)) {
                    Some(idx) => {
                        match storage.slots[idx].as_ref() {
                            Some(item) => (Some(item.clone()), Some(idx), true),
                            None => return,
                        }
                    }
                    None => return,
                }
            };

            let (to_item, to_slot, to_is_storage) = if msg.grid_to == 1 {
                // To inventory
                match inv.slots.iter().position(|s| s.as_ref().map(|i| i.unique_id) == Some(msg.id_to)) {
                    Some(idx) => {
                        match inv.slots[idx].as_ref() {
                            Some(item) => (Some(item.clone()), Some(idx), false),
                            None => return,
                        }
                    }
                    None => return,
                }
            } else {
                // To storage
                match storage.slots.iter().position(|s| s.as_ref().map(|i| i.unique_id) == Some(msg.id_to)) {
                    Some(idx) => {
                        match storage.slots[idx].as_ref() {
                            Some(item) => (Some(item.clone()), Some(idx), true),
                            None => return,
                        }
                    }
                    None => return,
                }
            };

            let (from_item, to_item) = match (from_item, to_item) {
                (Some(f), Some(t)) => (f, t),
                _ => return,
            };

            // Check if items can be merged
            if from_item.item_index != to_item.item_index {
                return;
            }

            let info = match self.world_db.get_item_info(from_item.item_index) {
                Some(i) => i,
                None => return,
            };

            if info.stack_size <= 1 {
                return;
            }

            if to_item.count >= info.stack_size as u16 {
                return;
            }

            let available_space = (info.stack_size as u16) - to_item.count;
            let merge_amount = from_item.count.min(available_space);

            // Update target item
            if to_is_storage {
                if let Some(ref mut target) = storage.slots[to_slot.unwrap()] {
                    target.count += merge_amount;
                }
            } else {
                if let Some(ref mut target) = inv.slots[to_slot.unwrap()] {
                    target.count += merge_amount;
                }
            }

            // Update or remove source item
            if from_item.count <= merge_amount {
                if from_is_storage {
                    storage.slots[from_slot.unwrap()] = None;
                } else {
                    inv.slots[from_slot.unwrap()] = None;
                }
            } else {
                if from_is_storage {
                    if let Some(ref mut source) = storage.slots[from_slot.unwrap()] {
                        source.count -= merge_amount;
                    }
                } else {
                    if let Some(ref mut source) = inv.slots[from_slot.unwrap()] {
                        source.count -= merge_amount;
                    }
                }
            }

            // Save storage if modified
            if from_is_storage || to_is_storage {
                let _ = self.store.save_account_storage(&account_id, &storage);
            }

            // Update world inventory
            {
                let mut world = self.world.lock().unwrap();
                world.set_player_items(self.session_id, inv, eq);
            }

            true
        } else {
            // Both grids are Inventory - use world method
            let mut world = self.world.lock().unwrap();
            world.merge_item_for_player(
                self.session_id,
                msg.grid_from,
                msg.grid_to,
                msg.id_from,
                msg.id_to,
            )
        };

        let pkt = SMergeItem {
            grid_from: msg.grid_from,
            grid_to: msg.grid_to,
            id_from: msg.id_from,
            id_to: msg.id_to,
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

    pub(crate) fn handle_split_item(&mut self, msg: CSplitItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Support Inventory (1) and Storage (2) grids
        if msg.grid != 1 && msg.grid != 2 {
            // Only Inventory and Storage are supported for now
            let pkt = SSplitItem {
                grid: msg.grid,
                unique_id: msg.unique_id,
                count: msg.count,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Handle Storage grid in connection layer (like StoreItem/TakeBackItem)
        let success = if msg.grid == 2 {
            // Storage - handle in connection layer
            let account_id = match self.account_id.as_ref() {
                Some(id) => id.clone(),
                None => return,
            };

            // Load storage
            let mut storage = match self.store.load_account_storage(&account_id) {
                Ok(Some(s)) => s,
                Ok(None) => crystal_server_core::account::AccountStorage {
                    slots: vec![None; 80],
                    has_expanded_storage: false,
                    expanded_storage_expiry_binary: 0,
                },
                Err(_) => return,
            };

            // Find the item to split
            let from_index = match storage.slots.iter().position(|s| s.as_ref().map(|i| i.unique_id) == Some(msg.unique_id)) {
                Some(idx) => idx,
                None => return,
            };

            let from_item = match storage.slots[from_index].as_ref() {
                Some(it) => it.clone(),
                None => return,
            };

            if msg.count >= from_item.count {
                return;
            }

            // Find an empty slot
            let to_index = match storage.slots.iter().position(|s| s.is_none()) {
                Some(idx) => idx,
                None => return,
            };

            // Create split item
            let mut split_item = from_item.clone();
            split_item.count = msg.count;
            let time_based = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            split_item.unique_id = ((self.session_id as u64) << 32) | (time_based & 0xFFFF_FFFF);

            // Update source item
            if let Some(ref mut source) = storage.slots[from_index] {
                source.count -= msg.count;
            }

            // Place split item
            storage.slots[to_index] = Some(split_item);

            // Save storage
            let _ = self.store.save_account_storage(&account_id, &storage);

            true
        } else {
            // Inventory - use world method
            let mut world = self.world.lock().unwrap();
            world.split_item_for_player(self.session_id, msg.grid, msg.unique_id, msg.count)
        };

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

            // Find the split item to send SSplitItem
            let mut split_item: Option<crystal_shared_proto::item_types::UserItemData> = None;
            for slot in &inv.slots {
                if let Some(item) = slot {
                    if item.unique_id == msg.unique_id {
                        split_item = Some(item.clone());
                        break;
                    }
                }
            }

            if let Some(_item) = split_item {
                let pkt = SSplitItem1 {
                    grid: msg.grid,
                    unique_id: msg.unique_id,
                    count: msg.count,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }

            let refresh = SUserSlotsRefresh {
                inventory: inv.slots,
                equipment: eq.slots,
            };
            if let Ok(raw) = refresh.encode() {
                out.push(Self::encode_raw(raw));
            }
        }
    }

    pub(crate) fn handle_drop_gold(&mut self, msg: CDropGold, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        if msg.amount == 0 {
            return;
        }

        // Check if player has enough gold
        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => {
                tracing::debug!(
                    "DropGold: no current_stats available for session_id={}",
                    self.session_id
                );
                return;
            }
        };

        let amount_i64 = msg.amount as i64;
        if stats.gold < amount_i64 {
            tracing::debug!(
                "DropGold: insufficient gold: amount={} gold={}",
                msg.amount,
                stats.gold
            );
            return;
        }

        // Deduct gold and save to database
        let mut new_stats = stats.clone();
        new_stats.gold = new_stats.gold.saturating_sub(amount_i64);

        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let _ = self
                .store
                .save_character_stats(account_id, char_idx, &new_stats);
        }

        // Update cached stats
        self.current_stats = Some(new_stats);

        // Create gold drop on ground via WorldCommand
        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::DropGold {
                session_id: self.session_id,
                amount: msg.amount,
            })
        };

        let _ = self.handle_world_events(events, out);

        // Send SLoseGold to notify client of gold deduction
        use crystal_shared_proto::user::status::SLoseGold;
        let pkt = SLoseGold {
            gold: msg.amount,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_remove_slot_item(&mut self, msg: CRemoveSlotItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Support Inventory (1) and Storage (2) grid_to
        let success = if msg.grid_to == 2 {
            // Storage - handle in connection layer
            let (account_id, _char_idx) = match (&self.account_id, self.current_char_index) {
                (Some(a), Some(i)) => (a.clone(), i),
                _ => {
                    let pkt = SRemoveSlotItem {
                        grid: msg.grid,
                        grid_to: msg.grid_to,
                        unique_id: msg.unique_id,
                        to: msg.to,
                        success: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    return;
                }
            };

            // Load storage
            let mut storage = match self.store.load_account_storage(&account_id) {
                Ok(Some(s)) => s,
                Ok(None) => AccountStorage {
                    slots: vec![None; 80],
                    has_expanded_storage: false,
                    expanded_storage_expiry_binary: 0,
                },
                Err(_) => {
                    let pkt = SRemoveSlotItem {
                        grid: msg.grid,
                        grid_to: msg.grid_to,
                        unique_id: msg.unique_id,
                        to: msg.to,
                        success: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    return;
                }
            };

            // Validate target slot
            if msg.to < 0 || msg.to as usize >= storage.slots.len() {
                let pkt = SRemoveSlotItem {
                    grid: msg.grid,
                    grid_to: msg.grid_to,
                    unique_id: msg.unique_id,
                    to: msg.to,
                    success: false,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }

            let to_index = msg.to as usize;
            if storage.slots[to_index].is_some() {
                // Target slot must be empty
                let pkt = SRemoveSlotItem {
                    grid: msg.grid,
                    grid_to: msg.grid_to,
                    unique_id: msg.unique_id,
                    to: msg.to,
                    success: false,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }

            // Get player to find parent item
            let (parent_item_opt, slot_item_opt) = {
                let world = self.world.lock().unwrap();
                let (inv, eq) = match world.player_items(self.session_id) {
                    Some(items) => items,
                    None => {
                        let pkt = SRemoveSlotItem {
                            grid: msg.grid,
                            grid_to: msg.grid_to,
                            unique_id: msg.unique_id,
                            to: msg.to,
                            success: false,
                        };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                        return;
                    }
                };

                // Find parent item based on grid
                let parent_item: Option<&crystal_shared_proto::item_types::UserItemData> = match msg.grid {
                    13 => {
                        // Mount: get from equipment slot 13
                        eq.slots.get(13).and_then(|s| s.as_ref())
                    }
                    7 => {
                        // Fishing: get from equipment slot 7 (Weapon)
                        eq.slots.get(7).and_then(|s| s.as_ref())
                    }
                    8 => {
                        // Socket: find by from_unique_id in equipment or inventory
                        eq.slots
                            .iter()
                            .find(|s| s.as_ref().map(|i| i.unique_id) == Some(msg.from_unique_id))
                            .and_then(|s| s.as_ref())
                            .or_else(|| {
                                inv.slots
                                    .iter()
                                    .find(|s| s.as_ref().map(|i| i.unique_id) == Some(msg.from_unique_id))
                                    .and_then(|s| s.as_ref())
                            })
                    }
                    _ => None,
                };

                // Find slot item
                let slot_item = parent_item.and_then(|parent| {
                    parent
                        .slots
                        .iter()
                        .find(|s| s.as_ref().and_then(|b| Some(b.unique_id)) == Some(msg.unique_id))
                        .and_then(|s| s.as_ref().map(|b| (**b).clone()))
                });

                (parent_item.cloned(), slot_item)
            };

            let slot_item = match slot_item_opt {
                Some(item) => item,
                None => {
                    let pkt = SRemoveSlotItem {
                        grid: msg.grid,
                        grid_to: msg.grid_to,
                        unique_id: msg.unique_id,
                        to: msg.to,
                        success: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    return;
                }
            };

            // Check if slot item is cursed or has wedding ring
            if slot_item.cursed || slot_item.wedding_ring != -1 {
                let pkt = SRemoveSlotItem {
                    grid: msg.grid,
                    grid_to: msg.grid_to,
                    unique_id: msg.unique_id,
                    to: msg.to,
                    success: false,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }

            // Remove slot item from parent using world method
            // First, try to remove from a temporary inventory slot, then update world
            // This is a workaround since we need mutable access to player
            // For Storage, we'll use a helper method that removes the slot item
            let removed = {
                let mut world = self.world.lock().unwrap();
                // Get current items
                let (mut inv, mut eq) = match world.player_items(self.session_id) {
                    Some(items) => items,
                    None => {
                        let pkt = SRemoveSlotItem {
                            grid: msg.grid,
                            grid_to: msg.grid_to,
                            unique_id: msg.unique_id,
                            to: msg.to,
                            success: false,
                        };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                        return;
                    }
                };

                // Find and remove slot item from parent
                let mut removed = false;
                match msg.grid {
                    13 => {
                        // Mount
                        if let Some(parent) = eq.slots.get_mut(13) {
                            if let Some(parent_item) = parent.as_mut() {
                                if let Some(_) = parent_item
                                    .slots
                                    .iter_mut()
                                    .find(|s| s.as_ref().and_then(|b| Some(b.unique_id)) == Some(msg.unique_id))
                                    .and_then(|s| s.take())
                                {
                                    removed = true;
                                }
                            }
                        }
                    }
                    7 => {
                        // Fishing
                        if let Some(parent) = eq.slots.get_mut(7) {
                            if let Some(parent_item) = parent.as_mut() {
                                if let Some(_) = parent_item
                                    .slots
                                    .iter_mut()
                                    .find(|s| s.as_ref().and_then(|b| Some(b.unique_id)) == Some(msg.unique_id))
                                    .and_then(|s| s.take())
                                {
                                    removed = true;
                                }
                            }
                        }
                    }
                    8 => {
                        // Socket - find in equipment or inventory
                        for slot in &mut eq.slots {
                            if let Some(parent_item) = slot.as_mut() {
                                if parent_item.unique_id == msg.from_unique_id {
                                    if let Some(_) = parent_item
                                        .slots
                                        .iter_mut()
                                        .find(|s| s.as_ref().and_then(|b| Some(b.unique_id)) == Some(msg.unique_id))
                                        .and_then(|s| s.take())
                                    {
                                        removed = true;
                                        break;
                                    }
                                }
                            }
                        }
                        if !removed {
                            for slot in &mut inv.slots {
                                if let Some(parent_item) = slot.as_mut() {
                                    if parent_item.unique_id == msg.from_unique_id {
                                        if let Some(_) = parent_item
                                            .slots
                                            .iter_mut()
                                            .find(|s| s.as_ref().and_then(|b| Some(b.unique_id)) == Some(msg.unique_id))
                                            .and_then(|s| s.take())
                                        {
                                            removed = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }

                if removed {
                    // Update world with modified items
                    world.set_player_items(self.session_id, inv, eq);
                    // Recalculate stats if equipment changed
                    if msg.grid == 13 || msg.grid == 7 {
                        world.recalc_player_equipment_stats(self.session_id);
                    }
                }

                removed
            };

            if !removed {
                let pkt = SRemoveSlotItem {
                    grid: msg.grid,
                    grid_to: msg.grid_to,
                    unique_id: msg.unique_id,
                    to: msg.to,
                    success: false,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }

            // Place slot item in storage
            storage.slots[to_index] = Some(slot_item);

            // Save storage
            if self.store.save_account_storage(&account_id, &storage).is_err() {
                let pkt = SRemoveSlotItem {
                    grid: msg.grid,
                    grid_to: msg.grid_to,
                    unique_id: msg.unique_id,
                    to: msg.to,
                    success: false,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }

            true
        } else {
            // Inventory - use world method
            let mut world = self.world.lock().unwrap();
            world.remove_slot_item_for_player(
                self.session_id,
                msg.grid,
                msg.grid_to,
                msg.unique_id,
                msg.to,
                msg.from_unique_id,
            )
        };

        let pkt = SRemoveSlotItem {
            grid: msg.grid,
            grid_to: msg.grid_to,
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

    pub(crate) fn handle_equip_slot_item(&mut self, msg: CEquipSlotItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Check if player is dead
        let is_dead = {
            let world = self.world.lock().unwrap();
            world
                .player_current_hp_mp(self.session_id)
                .map(|(hp, _)| hp <= 0)
                .unwrap_or(true)
        };

        if is_dead {
            return;
        }

        // Support Inventory (1) and Storage (2) grid
        let success = if msg.grid == 2 {
            // Storage - handle in connection layer
            // Check Storage NPC access
            let storage_npc_id = match self.current_storage_npc_id {
                Some(id) => id,
                None => {
                    let pkt = SEquipSlotItem {
                        grid: msg.grid,
                        unique_id: msg.unique_id,
                        to: msg.to,
                        grid_to: msg.grid_to,
                        success: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    return;
                }
            };

            if let Some(info) = self.world_db.get_npc_info(storage_npc_id as i32) {
                let dx = info.location_x - self.current_x;
                let dy = info.location_y - self.current_y;
                if info.map_index != self.current_map_index
                    || dx.abs() > Self::DATA_RANGE
                    || dy.abs() > Self::DATA_RANGE
                {
                    let pkt = SEquipSlotItem {
                        grid: msg.grid,
                        unique_id: msg.unique_id,
                        to: msg.to,
                        grid_to: msg.grid_to,
                        success: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    return;
                }
            } else {
                let pkt = SEquipSlotItem {
                    grid: msg.grid,
                    unique_id: msg.unique_id,
                    to: msg.to,
                    grid_to: msg.grid_to,
                    success: false,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }

            let account_id = match &self.account_id {
                Some(id) => id.clone(),
                None => {
                    let pkt = SEquipSlotItem {
                        grid: msg.grid,
                        unique_id: msg.unique_id,
                        to: msg.to,
                        grid_to: msg.grid_to,
                        success: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    return;
                }
            };

            // Load storage
            let mut storage = match self.store.load_account_storage(&account_id) {
                Ok(Some(s)) => s,
                Ok(None) => AccountStorage {
                    slots: vec![None; 80],
                    has_expanded_storage: false,
                    expanded_storage_expiry_binary: 0,
                },
                Err(_) => {
                    let pkt = SEquipSlotItem {
                        grid: msg.grid,
                        unique_id: msg.unique_id,
                        to: msg.to,
                        grid_to: msg.grid_to,
                        success: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    return;
                }
            };

            // Find source item in storage
            let source_item = match storage.slots.iter().find(|slot| {
                slot.as_ref()
                    .map(|item| item.unique_id == msg.unique_id)
                    .unwrap_or(false)
            }) {
                Some(Some(item)) => item.clone(),
                _ => {
                    let pkt = SEquipSlotItem {
                        grid: msg.grid,
                        unique_id: msg.unique_id,
                        to: msg.to,
                        grid_to: msg.grid_to,
                        success: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    return;
                }
            };

            // Get player inventory and equipment from world
            let (mut inv, mut eq) = {
                let world = self.world.lock().unwrap();
                world
                    .player_items(self.session_id)
                    .unwrap_or((
                        crystal_server_core::item::Inventory::new_default(),
                        crystal_server_core::item::Equipment::new_default(),
                    ))
            };

            // Call world method to validate and equip (it will check target item, etc.)
            // We need to temporarily add the item to inventory, then call the world method
            // But actually, we should validate first, then move from storage to slot
            // Let's use a different approach: validate using world method with a temporary inventory item
            // Or better: extract validation logic and do it here, then update both storage and world

            // For now, let's use a simpler approach:
            // 1. Validate the operation using world method (but it expects item in inventory)
            // 2. Actually, we need to check if we can equip this item to the target slot
            // 3. The world method does all the validation, so we need to either:
            //    a) Temporarily add to inventory, call world method, then remove from storage
            //    b) Duplicate the validation logic here
            // Let's go with approach (a) for consistency

            // Find empty slot in inventory
            let temp_inv_slot = inv.slots.iter().position(|slot| slot.is_none());
            if temp_inv_slot.is_none() {
                let pkt = SEquipSlotItem {
                    grid: msg.grid,
                    unique_id: msg.unique_id,
                    to: msg.to,
                    grid_to: msg.grid_to,
                    success: false,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
            let temp_inv_slot = temp_inv_slot.unwrap();

            // Temporarily add to inventory
            inv.slots[temp_inv_slot] = Some(source_item.clone());
            {
                let mut world = self.world.lock().unwrap();
                world.set_player_items(self.session_id, inv.clone(), eq.clone());
            }

            // Now call world method (it will validate and move item from inventory to slot)
            let success = {
                let mut world = self.world.lock().unwrap();
                world.equip_slot_item_for_player(
                    self.session_id,
                    1, // grid = Inventory
                    msg.grid_to,
                    msg.unique_id,
                    msg.to,
                    msg.to_unique_id,
                )
            };

            if success {
                // World method successfully moved item from inventory to slot
                // Now remove from storage (item was already taken from inventory by world method)
                for slot in storage.slots.iter_mut() {
                    if slot.as_ref().map(|item| item.unique_id == msg.unique_id).unwrap_or(false) {
                        *slot = None;
                        break;
                    }
                }

                // Save storage
                if self.store.save_account_storage(&account_id, &storage).is_err() {
                    // Rollback: need to restore item to storage and remove from slot
                    // This is complex, so for now we'll just log the error
                    // In production, we should implement proper rollback
                    let pkt = SEquipSlotItem {
                        grid: msg.grid,
                        unique_id: msg.unique_id,
                        to: msg.to,
                        grid_to: msg.grid_to,
                        success: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    return;
                }

                // Get updated items from world (already updated by world method)
                let (inv, eq) = {
                    let world = self.world.lock().unwrap();
                    world
                        .player_items(self.session_id)
                        .unwrap_or((
                            crystal_server_core::item::Inventory::new_default(),
                            crystal_server_core::item::Equipment::new_default(),
                        ))
                };

                // Send refresh
                let refresh = SUserSlotsRefresh {
                    inventory: inv.slots,
                    equipment: eq.slots,
                };
                if let Ok(raw) = refresh.encode() {
                    out.push(Self::encode_raw(raw));
                }
            } else {
                // World method failed - restore item to storage
                // The item is still in inventory (world method didn't move it)
                // So we need to remove it from inventory and put it back in storage
                {
                    let mut world = self.world.lock().unwrap();
                    let (mut inv, eq) = world
                        .player_items(self.session_id)
                        .unwrap_or((
                            crystal_server_core::item::Inventory::new_default(),
                            crystal_server_core::item::Equipment::new_default(),
                        ));
                    
                    // Remove from inventory (it was temporarily added)
                    if let Some(item) = inv.slots[temp_inv_slot].take() {
                        // Put it back in storage at the original position
                        for slot in storage.slots.iter_mut() {
                            if slot.is_none() {
                                *slot = Some(item);
                                break;
                            }
                        }
                    }
                    
                    world.set_player_items(self.session_id, inv, eq);
                }
                
                // Save storage with restored item
                let _ = self.store.save_account_storage(&account_id, &storage);
            }

            success
        } else {
            // Inventory - use world method
            let mut world = self.world.lock().unwrap();
            world.equip_slot_item_for_player(
                self.session_id,
                msg.grid,
                msg.grid_to,
                msg.unique_id,
                msg.to,
                msg.to_unique_id,
            )
        };

        // Send response
        use crystal_shared_proto::user::status::SEquipSlotItem;
        let pkt = SEquipSlotItem {
            grid: msg.grid,
            unique_id: msg.unique_id,
            to: msg.to,
            grid_to: msg.grid_to,
            success,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        if success {
            // Send inventory/equipment refresh
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

    pub(crate) fn handle_combine_item(&mut self, msg: CCombineItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Check if player is dead
        let is_dead = {
            let world = self.world.lock().unwrap();
            world
                .player_current_hp_mp(self.session_id)
                .map(|(hp, _)| hp <= 0)
                .unwrap_or(true)
        };

        if is_dead {
            return;
        }

        // Call world method to combine items
        let (success, destroy) = {
            let mut world = self.world.lock().unwrap();
            world.combine_item_for_player(
                self.session_id,
                msg.grid,
                msg.id_from,
                msg.id_to,
            )
        };

        // Send response
        let pkt = SCombineItem {
            grid: msg.grid,
            id_from: msg.id_from,
            id_to: msg.id_to,
            success,
            destroy,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        if success {
            // Send inventory/equipment refresh
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

    pub(crate) fn handle_awakening_need_materials(&mut self, _msg: CAwakeningNeedMaterials, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement awakening need materials logic
        // This should:
        // 1. Check if NPC page is @AWAKENING
        // 2. Find item in inventory by unique_id
        // 3. Calculate required materials for awakening
        // 4. Send SNPCAwakening with material requirements
    }

    pub(crate) fn handle_awakening_locked_item(&mut self, msg: CAwakeningLockedItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement awakening locked item logic
        // This should:
        // 1. Find item in inventory by unique_id
        // 2. Set/clear locked flag for awakening
        // 3. Send SAwakeningLockedItem response
        
        use crystal_shared_proto::npc::SAwakeningLockedItem;
        let pkt = SAwakeningLockedItem {
            unique_id: msg.unique_id,
            locked: msg.locked,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_awakening(&mut self, _msg: CAwakening, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement awakening logic
        // This should:
        // 1. Check if NPC page is @AWAKENING
        // 2. Find item in inventory by unique_id
        // 3. Validate awakening type and position
        // 4. Check if player has required materials
        // 5. Consume materials
        // 6. Apply awakening to item
        // 7. Send appropriate response packets
    }

    pub(crate) fn handle_disassemble_item(&mut self, _msg: CDisassembleItem, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement disassemble item logic
        // This should:
        // 1. Check if NPC page is @DISASSEMBLE
        // 2. Find item in inventory by unique_id
        // 3. Validate item can be disassembled
        // 4. Calculate disassemble rewards
        // 5. Remove item from inventory
        // 6. Add rewards to inventory
        // 7. Send appropriate response packets
    }

    pub(crate) fn handle_downgrade_awakening(&mut self, _msg: CDowngradeAwakening, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement downgrade awakening logic
        // This should:
        // 1. Check if NPC page is @AWAKENING
        // 2. Find item in inventory by unique_id
        // 3. Validate item has awakening
        // 4. Downgrade awakening level
        // 5. Return some materials
        // 6. Send appropriate response packets
    }

    pub(crate) fn handle_reset_added_item(&mut self, _msg: CResetAddedItem, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement reset added item logic
        // This should:
        // 1. Check if NPC page is @RESETADDED
        // 2. Find item in inventory by unique_id
        // 3. Validate item has added stats
        // 4. Calculate reset cost
        // 5. Deduct gold
        // 6. Reset added stats
        // 7. Send appropriate response packets
    }

    pub(crate) fn handle_request_intelligent_creature_updates(&mut self, msg: CRequestIntelligentCreatureUpdates, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement request intelligent creature updates logic
        // This should:
        // 1. Set player's SendIntelligentCreatureUpdates flag
        // 2. If update is true, start sending creature updates
        // 3. If update is false, stop sending creature updates
        
        tracing::debug!("RequestIntelligentCreatureUpdates: update={}", msg.update);
    }

    pub(crate) fn handle_update_intelligent_creature(&mut self, msg: CUpdateIntelligentCreature, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement update intelligent creature logic
        // This should:
        // 1. Parse ClientIntelligentCreature from creature_bytes
        // 2. If release_me is true, release the creature
        // 3. If summon_me is true, summon the creature
        // 4. If unsummon_me is true, unsummon the creature
        // 5. Otherwise, update creature settings (pet name, pet mode, etc.)
        // 6. Send appropriate response packets
        
        tracing::debug!(
            "UpdateIntelligentCreature: creature_bytes_len={}, summon_me={}, unsummon_me={}, release_me={}",
            msg.creature_bytes.len(),
            msg.summon_me,
            msg.unsummon_me,
            msg.release_me
        );
    }

    pub(crate) fn handle_intelligent_creature_pickup(&mut self, msg: CIntelligentCreaturePickup, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement intelligent creature pickup logic
        // This should:
        // 1. Check if player has a summoned intelligent creature
        // 2. If mouse_mode is true, move creature to specified location
        // 3. If mouse_mode is false, make creature pick up items at location
        // 4. Send appropriate response packets
        
        tracing::debug!(
            "IntelligentCreaturePickup: mouse_mode={}, location=({}, {})",
            msg.mouse_mode,
            msg.location_x,
            msg.location_y
        );
    }

    pub(crate) fn handle_marriage_request(&mut self, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement marriage request logic
        // This should:
        // 1. Check if player is near another player
        // 2. Send marriage request to target player
        // 3. Handle marriage request timeout
    }

    pub(crate) fn handle_marriage_reply(&mut self, _msg: CMarriageReply, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement marriage reply logic
        // This should:
        // 1. Check if player has pending marriage request
        // 2. If accept_invite is true, complete marriage
        // 3. If accept_invite is false, reject marriage request
    }

    pub(crate) fn handle_change_marriage(&mut self, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement change marriage logic
        // This should:
        // 1. If player is not married, toggle AllowMarriage flag
        // 2. If player is married, toggle AllowLoverRecall flag
        // 3. Send appropriate chat message
    }

    pub(crate) fn handle_divorce_request(&mut self, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement divorce request logic
        // This should:
        // 1. Check if player is married
        // 2. Send divorce request to spouse
        // 3. Handle divorce request timeout
    }

    pub(crate) fn handle_divorce_reply(&mut self, _msg: CDivorceReply, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement divorce reply logic
        // This should:
        // 1. Check if player has pending divorce request
        // 2. If accept_invite is true, complete divorce
        // 3. If accept_invite is false, reject divorce request
    }

    pub(crate) fn handle_add_mentor(&mut self, _msg: CAddMentor, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement add mentor logic
        // This should:
        // 1. Find target player by name
        // 2. Check if target player can be a mentor
        // 3. Send mentor request to target player
        // 4. Handle mentor request timeout
    }

    pub(crate) fn handle_mentor_reply(&mut self, _msg: CMentorReply, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement mentor reply logic
        // This should:
        // 1. Check if player has pending mentor request
        // 2. If accept_invite is true, establish mentor relationship
        // 3. If accept_invite is false, reject mentor request
    }

    pub(crate) fn handle_allow_mentor(&mut self, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement allow mentor logic
        // This should:
        // 1. Toggle AllowMentor flag
        // 2. Send appropriate chat message
    }

    pub(crate) fn handle_cancel_mentor(&mut self, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement cancel mentor logic
        // This should:
        // 1. Check if player has a mentor or student
        // 2. Cancel mentor relationship
        // 3. Notify both parties
    }

    pub(crate) fn handle_guild_buff_update(&mut self, _msg: CGuildBuffUpdate, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement guild buff update logic
        // This should:
        // 1. Check if player is in a guild
        // 2. If action is 0, send guild buff list
        // 3. If action is 1, enable a buff
        // 4. If action is 2, activate a buff
    }

    pub(crate) fn handle_npc_confirm_input(&mut self, _msg: CNPCConfirmInput, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement NPC confirm input logic
        // This should:
        // 1. Find NPC by ID
        // 2. Validate NPC proximity
        // 3. Process NPC page input confirmation
    }

    pub(crate) fn handle_report_issue(&mut self, _msg: CReportIssue, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement report issue logic
        // This should:
        // 1. Save issue report with image and message
        // 2. Log to admin/system
        // 3. Send confirmation to player
    }

    pub(crate) fn handle_opendoor(&mut self, _msg: COpendoor, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement opendoor logic
        // This should:
        // 1. Check if player is near a door
        // 2. Validate door index
        // 3. Open/close door
        // 4. Broadcast door state change
    }

    pub(crate) fn handle_get_rented_items(&mut self, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement get rented items logic
        // This should:
        // 1. Retrieve player's rented items list
        // 2. Send rented items to client
    }

    pub(crate) fn handle_item_rental_request(&mut self, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement item rental request logic
        // This should:
        // 1. Check if NPC page is @RENTAL
        // 2. Open rental interface
    }

    pub(crate) fn handle_item_rental_fee(&mut self, _msg: CItemRentalFee, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement item rental fee logic
        // This should:
        // 1. Set rental fee amount
        // 2. Calculate total cost
    }

    pub(crate) fn handle_item_rental_period(&mut self, _msg: CItemRentalPeriod, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement item rental period logic
        // This should:
        // 1. Set rental period in days
        // 2. Calculate expiry date
    }

    pub(crate) fn handle_deposit_rental_item(&mut self, _msg: CDepositRentalItem, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement deposit rental item logic
        // This should:
        // 1. Check if NPC page is @RENTAL
        // 2. Move item from inventory to rental slot
        // 3. Validate item can be rented
    }

    pub(crate) fn handle_retrieve_rental_item(&mut self, _msg: CRetrieveRentalItem, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement retrieve rental item logic
        // This should:
        // 1. Check if NPC page is @RENTAL
        // 2. Move item from rental slot to inventory
        // 3. Validate inventory space
    }

    pub(crate) fn handle_cancel_item_rental(&mut self, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement cancel item rental logic
        // This should:
        // 1. Cancel current rental transaction
        // 2. Return items to inventory
    }

    pub(crate) fn handle_item_rental_lock_fee(&mut self, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement item rental lock fee logic
        // This should:
        // 1. Lock rental fee amount
        // 2. Prevent further fee changes
    }

    pub(crate) fn handle_item_rental_lock_item(&mut self, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement item rental lock item logic
        // This should:
        // 1. Lock rental items
        // 2. Prevent further item changes
    }

    pub(crate) fn handle_confirm_item_rental(&mut self, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement confirm item rental logic
        // This should:
        // 1. Validate rental transaction
        // 2. Deduct gold
        // 3. Complete rental transaction
        // 4. Set rental expiry date
    }

    pub(crate) fn handle_guild_territory_page(&mut self, _msg: CGuildTerritoryPage, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement guild territory page logic
        // This should:
        // 1. Check if player is in a guild
        // 2. Send guild territory page data
    }

    pub(crate) fn handle_purchase_guild_territory(&mut self, _msg: CPurchaseGuildTerritory, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement purchase guild territory logic
        // This should:
        // 1. Check if player is guild leader
        // 2. Check if territory is available
        // 3. Deduct guild gold
        // 4. Purchase territory
    }
}
