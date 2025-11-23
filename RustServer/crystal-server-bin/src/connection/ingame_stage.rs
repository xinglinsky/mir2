use std::path::Path;

use crystal_server_core::account::{CharacterPosition, CharacterSummary};
use crystal_server_core::world::{self, WorldProvider};
use crystal_shared_proto::guild::SGuildStatus;
use crystal_shared_proto::item::{CBuyItem, CSellItem, SEquipItem, SMoveItem, SRemoveItem, SSellItem, SUseItem};
use crystal_shared_proto::item_types::UserItemData;
use crystal_shared_proto::login::{
    CAttack,
    CCallNPC,
    CGuildInvite,
    CGuildNameReturn,
    CEquipItem,
    CKeepAlive,
    CMoveItem,
    CRemoveItem,
    CRun,
    CUseItem,
    CTownRevive,
    CPickUp,
    CTurn,
    CWalk,
    CRequestMapInfo,
    CTeleportToNPC,
    CSearchMap,
    SDisconnect,
    SKeepAlive,
};
use crystal_shared_proto::map::{
    SMapEffect,
    SMapInformation,
    SWorldMapSetupInfo,
    SNewMapInfo,
    SSearchMapResult,
};
use crystal_shared_proto::map_types::{
    ClientMapInfoData,
    ClientMovementInfoData,
    ClientNpcInfoData,
};
use crystal_shared_proto::npc::{SNpcGoods, SNpcSell, SNpcRepair, SNpcsRepair, SNpcUpdate};
use crystal_shared_proto::scene::{
    SChat,
    SNpcResponse,
    SObjectGuildNameChanged,
    SObjectRemove,
    SRevived,
    SObjectRevived,
    SLevelChanged,
    SObjectLeveled,
    SObjectTeleportOut,
    SObjectTeleportIn,
    STeleportIn,
};
use crystal_shared_proto::select::{SelectInfo, SLogOutFailed, SLogOutSuccess};
use crystal_shared_proto::user::{SGainedGold, SLoseGold, SUserSlotsRefresh, SHealthChanged};

use super::{LoginConnection, Stage};

const MIN_SEARCH_TEXT_LEN: usize = 3;

impl LoginConnection {
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

    pub(crate) fn handle_turn(&mut self, msg: CTurn, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        println!(
            "[ingame] handle_turn: session={} dir={} map={} pos=({}, {})",
            self.session_id,
            msg.direction,
            self.current_map_index,
            self.current_x,
            self.current_y,
        );

        let _ = self.apply_step(msg.direction, 0, out);
    }

    pub(crate) fn handle_walk(&mut self, msg: CWalk, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        println!(
            "[ingame] handle_walk: session={} dir={} map={} pos=({}, {})",
            self.session_id,
            msg.direction,
            self.current_map_index,
            self.current_x,
            self.current_y,
        );

        let map_changed = self.apply_step(msg.direction, 1, out);

        if map_changed {
            self.known_monsters.clear();
            self.known_npcs.clear();
        }

        self.update_visibility(out);
    }

    pub(crate) fn handle_run(&mut self, msg: CRun, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        println!(
            "[ingame] handle_run: session={} dir={} map={} pos=({}, {})",
            self.session_id,
            msg.direction,
            self.current_map_index,
            self.current_x,
            self.current_y,
        );

        let map_changed = self.apply_step(msg.direction, 2, out);

        if map_changed {
            self.known_monsters.clear();
            self.known_npcs.clear();
        }

        self.update_visibility(out);
    }

    pub(crate) fn handle_chat(&mut self, message: String, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let trimmed = message.trim();

        // Mirror C# MirConnection.Chat: if the message exceeds Globals.MaxChatLength,
        // immediately disconnect the client with reason=2 (Packet Error).
        // The exact MaxChatLength is defined in the C# Globals; here we
        // conservatively treat anything over 255 characters as invalid.
        if trimmed.chars().count() > 255 {
            let pkt = SDisconnect { reason: 2 };
            let raw = pkt.encode();
            out.push(Self::encode_raw(raw));
            self.closing = true;
            return;
        }
        if !trimmed.is_empty() {
            tracing::info!(
                target = "chat",
                session_id = self.session_id,
                map_index = self.current_map_index,
                "{}",
                trimmed,
            );
        }

        // Delegate all GM/admin commands to the gm_commands module.
        if self.handle_gm_chat(trimmed, out) {
            return;
        }
    }

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
            let mut world = self.world.lock().unwrap();
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
        msg: crystal_shared_proto::item::CDropItem,
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
            let mut world = self.world.lock().unwrap();
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

        let mut item = match inv.slots[idx].clone() {
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

    fn guild_member_exists_for_session(&self) -> bool {
        let map = self.player_summaries.lock().unwrap();
        if let Some(v) = map.get(&self.session_id) {
            !v.guild_name.is_empty()
        } else {
            false
        }
    }

    pub(crate) fn send_system_chat(&self, text: &str, out: &mut Vec<Vec<u8>>) {
        let pkt = SChat {
            message: text.to_string(),
            chat_type: 2,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_create_guild_command(&mut self, name_raw: &str, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let name = name_raw.trim();
        let len = name.chars().count();
        if len < 3 || len > 20 {
            self.send_system_chat("Guild name must be between 3 and 20 characters.", out);
            return;
        }

        if name.contains('\\') {
            self.send_system_chat("Guild name contains invalid characters.", out);
            return;
        }

        if self.guild_member_exists_for_session() {
            self.send_system_chat("You are already part of a guild.", out);
            return;
        }

        let guild_info = {
            let mut world = self.world.lock().unwrap();
            world.create_guild(name)
        };

        let Some(guild) = guild_info else {
            let msg = format!("Guild {} already exists.", name);
            self.send_system_chat(&msg, out);
            return;
        };

        let _ = self.store.save_guild(&guild);

        {
            let mut map = self.player_summaries.lock().unwrap();
            if let Some(v) = map.get_mut(&self.session_id) {
                v.guild_name = guild.name.clone();
                v.guild_rank_name = "Leader".to_string();
            }
        }

        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let _ = self
                .store
                .save_character_guild(account_id, char_idx, &guild.name, 0);
        }

        // Notify self and nearby players that this object's guild name has
        // changed, so clients like B immediately see A's new guild tag
        // without requiring a visibility refresh.
        let guild_name_changed = SObjectGuildNameChanged {
            object_id: self.session_id,
            guild_name: guild.name.clone(),
        };
        if let Ok(pkt) = guild_name_changed.encode() {
            let raw = Self::encode_raw(pkt);
            // Send to self
            out.push(raw.clone());
            // Broadcast to other players in view
            self.enqueue_for_viewers(
                self.current_map_index,
                self.current_x,
                self.current_y,
                raw,
            );
        }

        let status = SGuildStatus {
            guild_name: guild.name.clone(),
            guild_rank_name: "Leader".to_string(),
            level: guild.level,
            experience: guild.experience,
            max_experience: guild.max_experience,
            gold: guild.gold,
            spare_points: guild.spare_points,
            member_count: 1,
            max_members: guild.member_cap,
            voting: false,
            item_count: guild.stored_items.len() as u8,
            buff_count: guild.buff_list.len() as u8,
            my_options: 0xff,
            my_rank_id: 0,
        };
        if let Ok(raw) = status.encode() {
            out.push(Self::encode_raw(raw));
        }

        let ok = format!("Successfully created guild {}", name);
        self.send_system_chat(&ok, out);
    }

    pub(crate) fn handle_guild_name_return(
        &mut self,
        msg: CGuildNameReturn,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let name = msg.name.trim();
        if name.is_empty() {
            return;
        }

        self.handle_create_guild_command(name, out);
    }

    pub(crate) fn handle_guild_invite(
        &mut self,
        msg: CGuildInvite,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        if self.pending_guild_invite.is_none() {
            self.send_system_chat("You have not been invited to a guild.", out);
            return;
        }

        if !msg.accept_invite {
            self.pending_guild_invite = None;
            return;
        }

        self.send_system_chat(
            "Guild invite accept/decline handling is not implemented yet.",
            out,
        );
        self.pending_guild_invite = None;
    }

    pub(crate) fn handle_call_npc(&mut self, msg: CCallNPC, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Mirror C# MirConnection.CallNPC: if the key is unreasonably long,
        // treat it as a malformed packet and disconnect with reason=2.
        if msg.key.chars().count() > 30 {
            let pkt = SDisconnect { reason: 2 };
            let raw = pkt.encode();
            out.push(Self::encode_raw(raw));
            self.closing = true;
            return;
        }

        if msg.object_id == super::LoginConnection::DEFAULT_NPC_ID {
            self.handle_default_npc_call(msg.key, out);
            return;
        }

        if let Some(npc) = self
            .world_db
            .npc_infos
            .iter()
            .find(|n| n.index as u32 == msg.object_id && n.map_index == self.current_map_index)
        {
            tracing::debug!(
                "CallNPC: map={} npc_index={} file_name='{}' key='{}'",
                self.current_map_index,
                npc.index,
                npc.file_name,
                msg.key,
            );
            let dx = npc.location_x - self.current_x;
            let dy = npc.location_y - self.current_y;

            if dx.abs() <= Self::DATA_RANGE && dy.abs() <= Self::DATA_RANGE {
                let key = Self::normalize_npc_key(&msg.key);
                let key_upper = key.as_str();

                let is_buy_panel = matches!(
                    key_upper,
                    "@BUY" | "@BUYNEW" | "@BUYSELL" | "@BUYSELLNEW"
                );
                let is_sell_only = key_upper == "@SELL";

                let mut shop_goods: Option<(Vec<UserItemData>, u8)> = None;
                let mut send_npc_sell_only = false;
                let mut send_npc_sell_after_goods = false;

                if is_sell_only {
                    send_npc_sell_only = true;
                }

                if is_buy_panel {
                    let mut goods_items = Vec::new();
                    let root_deploy = Path::new("./deploy/Envir/NPCs");
                    let root_plain = Path::new("./Envir/NPCs");
                    let root = if root_deploy.exists() { root_deploy } else { root_plain };
                    if root.exists() {
                        if let Some(script_path) =
                            Self::find_npc_script_path(root, &npc.file_name)
                        {
                            tracing::debug!(
                                "CallNPC shop: using script path {:?} for npc_index={}",
                                script_path,
                                npc.index,
                            );
                            if let Ok(specs) =
                                Self::load_npc_trade_goods_from_file(&script_path)
                            {
                                let base_uid = (npc.index as u64) << 32;
                                for (idx, (name, count)) in specs.iter().enumerate() {
                                    if let Some(info) = self
                                        .world_db
                                        .item_infos
                                        .iter()
                                        .find(|i| i.name.eq_ignore_ascii_case(name))
                                    {
                                        let unique_id = base_uid + idx as u64 + 1;
                                        let item = Self::make_shop_user_item(
                                            info,
                                            unique_id,
                                            *count,
                                        );
                                        goods_items.push(item);
                                    }
                                }
                            }
                        }
                    }

                    // PanelType.Buy
                    shop_goods = Some((goods_items, 0));

                    if matches!(key_upper, "@BUYSELL" | "@BUYSELLNEW") {
                        send_npc_sell_after_goods = true;
                    }
                }

                if key_upper == "@BUYBACK" {
                    // TODO: populate from NPC buy-back history once that state exists.
                    let goods_items: Vec<UserItemData> = Vec::new();
                    // PanelType.Buy
                    shop_goods = Some((goods_items, 0));
                }

                if key_upper == "@BUYUSED" {
                    // TODO: populate from NPC UsedGoods once that state exists.
                    let goods_items: Vec<UserItemData> = Vec::new();
                    // PanelType.BuySub
                    shop_goods = Some((goods_items, 1));
                }

                let root_deploy = Path::new("./deploy/Envir/NPCs");
                let root_plain = Path::new("./Envir/NPCs");
                let root = if root_deploy.exists() { root_deploy } else { root_plain };
                let mut maybe_page: Option<Vec<String>> = None;

                if root.exists() {
                    if let Some(script_path) = Self::find_npc_script_path(root, &npc.file_name) {
                        tracing::debug!(
                            "CallNPC dialog: using script path {:?} for npc_index={}",
                            script_path,
                            npc.index,
                        );
                        if let Ok((pages, moves)) = Self::load_npc_script_from_file(&script_path) {
                            let paid = Self::extract_paid_teleport_info(&script_path, &key);

                            if let Some((map_name, tx, ty)) = moves.get(&key) {
                                let mut dest_index: Option<i32> = None;

                                if let Ok(idx) = map_name.parse::<i32>() {
                                    if self
                                        .world_db
                                        .map_infos
                                        .iter()
                                        .any(|m| m.index == idx)
                                    {
                                        dest_index = Some(idx);
                                    }
                                }

                                if dest_index.is_none() {
                                    if let Some(info) = self
                                        .world_db
                                        .map_infos
                                        .iter()
                                        .find(|m| m.file_name.eq_ignore_ascii_case(map_name))
                                    {
                                        dest_index = Some(info.index);
                                    }
                                }

                                if let Some(map_index) = dest_index {
                                    if let Some((price, fail_label)) = paid {
                                        if let Some(mut stats) = self.current_stats.clone() {
                                            if stats.gold < price {
                                                if let Some(fail_key) = fail_label {
                                                    maybe_page = pages.get(&fail_key).cloned();
                                                }
                                            } else {
                                                stats.gold = stats.gold.saturating_sub(price);
                                                if let (Some(ref account_id), Some(char_idx)) =
                                                    (self.account_id.as_ref(), self.current_char_index)
                                                {
                                                    let _ = self
                                                        .store
                                                        .save_character_stats(account_id, char_idx, &stats);
                                                }
                                                self.current_stats = Some(stats.clone());

                                                let lose = SLoseGold { gold: price as u32 };
                                                if let Ok(raw) = lose.encode() {
                                                    out.push(Self::encode_raw(raw));
                                                }

                                                let x = *tx;
                                                let y = *ty;

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
                                                    self.update_visibility(out);
                                                }

                                                return;
                                            }
                                        }
                                    } else {
                                        let x = *tx;
                                        let y = *ty;

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
                                            self.update_visibility(out);
                                        }

                                        return;
                                    }
                                }
                            }

                            if key.eq_ignore_ascii_case("@MAIN") {
                                if let Some(page_alt) = pages.get("@MAIN-1") {
                                    maybe_page = Some(page_alt.clone());
                                } else if let Some(page_main) = pages.get("@MAIN") {
                                    maybe_page = Some(page_main.clone());
                                }
                            } else {
                                maybe_page = pages.get(&key).cloned();
                            }
                        }
                    }
                }

                let page = maybe_page.unwrap_or_else(|| vec![npc.name.clone()]);
                let resp = SNpcResponse { page };
                if let Ok(raw) = resp.encode() {
                    out.push(Self::encode_raw(raw));
                }

                if let Some((goods_items, panel_type)) = shop_goods {
                    let rate: f32 = (npc.rate as f32) / 100.0;
                    if let Ok(bytes) = Self::build_npc_goods_bytes(
                        &goods_items,
                        rate,
                        panel_type,
                        false,
                    ) {
                        let pkt = SNpcGoods { goods_bytes: bytes };
                        let raw = pkt.encode();
                        out.push(Self::encode_raw(raw));
                    }
                }

                if send_npc_sell_only || send_npc_sell_after_goods {
                    let sell = SNpcSell;
                    let raw = sell.encode();
                    out.push(Self::encode_raw(raw));
                }

                if key_upper == "@REPAIR" {
                    let rate: f32 = (npc.rate as f32) / 100.0;
                    let pkt = SNpcRepair { rate };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                } else if key_upper == "@SREPAIR" {
                    let rate: f32 = (npc.rate as f32) / 100.0;
                    let pkt = SNpcsRepair { rate };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        }
    }

    fn handle_default_npc_call(&mut self, raw_key: String, out: &mut Vec<Vec<u8>>) {
        let key = Self::normalize_npc_key(&raw_key);
        let root_deploy = Path::new("./deploy/Envir/SystemScripts/00Default");
        let root_plain = Path::new("./Envir/SystemScripts/00Default");
        let root = if root_deploy.exists() { root_deploy } else { root_plain };
        if !root.exists() {
            return;
        }

        let scripts = ["TownScroll.txt", "DungeonScroll.txt"];

        for name in &scripts {
            let script_path = root.join(name);
            if !script_path.is_file() {
                continue;
            }

            let (pages, moves) = match Self::load_npc_script_from_file(&script_path) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if let Some((map_name, tx, ty)) = moves.get(&key) {
                let mut dest_index: Option<i32> = None;

                if let Ok(idx) = map_name.parse::<i32>() {
                    if self
                        .world_db
                        .map_infos
                        .iter()
                        .any(|m| m.index == idx)
                    {
                        dest_index = Some(idx);
                    }
                }

                if dest_index.is_none() {
                    if let Some(info) = self
                        .world_db
                        .map_infos
                        .iter()
                        .find(|m| m.file_name.eq_ignore_ascii_case(map_name))
                    {
                        dest_index = Some(info.index);
                    }
                }

                if let Some(map_index) = dest_index {
                    let x = *tx;
                    let y = *ty;

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
                        self.update_visibility(out);
                    }

                    return;
                }
            }

            let mut maybe_page: Option<Vec<String>> = None;
            if key.eq_ignore_ascii_case("@MAIN") {
                if let Some(page_alt) = pages.get("@MAIN-1") {
                    maybe_page = Some(page_alt.clone());
                } else if let Some(page_main) = pages.get("@MAIN") {
                    maybe_page = Some(page_main.clone());
                }
            } else {
                if let Some(page) = pages.get(&key) {
                    maybe_page = Some(page.clone());
                }
            }

            if let Some(page) = maybe_page {
                let resp = SNpcResponse { page };
                if let Ok(raw) = resp.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        }
    }

    pub(crate) fn handle_attack(&mut self, msg: CAttack, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::Attack {
                session_id: self.session_id,
                direction: msg.direction,
                spell: msg.spell,
            })
        };

        let _ = self.handle_world_events(events, out);
        self.update_visibility(out);
    }

    pub(crate) fn handle_pick_up(&mut self, _msg: CPickUp, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::PickUp {
                session_id: self.session_id,
            })
        };

        let _ = self.handle_world_events(events, out);
    }

    pub(crate) fn handle_town_revive(&mut self, _msg: CTownRevive, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Determine the bind location for this character (equivalent to C#
        // BindMapIndex/BindLocation). If none is stored, fall back to the
        // current map/position.
        let (dest_map, dest_x, dest_y, dest_dir) = if let (
            Some(ref account_id),
            Some(char_idx),
        ) = (self.account_id.as_ref(), self.current_char_index)
        {
            if let Ok(Some(pos)) = self.store.load_character_bind(account_id, char_idx) {
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

        // Revive the player at the chosen bind location on the world side and
        // then issue a Teleport command so that a MapChanged event is emitted,
        // mirroring the C# TownRevive behaviour of reviving at town.
        let (map_index, x, y, direction, hp, mp, events) = {
            let mut world = self.world.lock().unwrap();
            let Some((map_index, x, y, direction, hp, mp)) = world.revive_player_to_position(
                self.session_id,
                dest_map,
                dest_x,
                dest_y,
                dest_dir,
            ) else {
                return;
            };
            let events = world.handle_command(world::WorldCommand::Teleport {
                session_id: self.session_id,
                map_index,
                x,
                y,
            });
            (map_index, x, y, direction, hp, mp, events)
        };

        // Let the shared world-event handler emit SMapChanged and update
        // visibility state for the new town map.
        let map_changed = self.handle_world_events(events, out);
        if map_changed {
            self.known_monsters.clear();
            self.known_npcs.clear();
            self.update_visibility(out);
        }

        self.current_map_index = map_index;
        self.current_x = x;
        self.current_y = y;
        self.direction = direction;

        if let Some(stats) = self.current_stats.as_mut() {
            stats.hp = hp;
            stats.mp = mp;
        }

        let hc = SHealthChanged { hp, mp };
        if let Ok(raw) = hc.encode() {
            out.push(Self::encode_raw(raw));
        }

        let revived = SRevived;
        let raw = revived.encode();
        out.push(Self::encode_raw(raw));

        let obj_revived = SObjectRevived {
            object_id: self.session_id,
            effect: true,
        };
        if let Ok(pkt) = obj_revived.encode() {
            let raw = Self::encode_raw(pkt);
            out.push(raw.clone());
            self.enqueue_for_viewers(map_index, x, y, raw);
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

    pub(crate) fn handle_sell_item(&mut self, msg: CSellItem, out: &mut Vec<Vec<u8>>) {
        tracing::debug!(
            "SellItem: start stage={:?} unique_id={} count={}",
            self.stage,
            msg.unique_id,
            msg.count,
        );

        if self.stage != Stage::InGame {
            tracing::debug!("SellItem: early-return, not in-game stage");
            return;
        }

        if msg.count == 0 {
            tracing::debug!("SellItem: early-return, count is 0");
            return;
        }

        let Some(stats) = self.current_stats.clone() else {
            tracing::debug!("SellItem: early-return, no current_stats available");
            return;
        };

        // Locate nearest NPC within DATA_RANGE on current map, mirroring the
        // distance gating in CallNPC. For now we just pick the first matching
        // NPC; later this can be refined to track the active NPC from CallNPC.
        let npc_opt = self
            .world_db
            .npc_infos
            .iter()
            .find(|n| {
                n.map_index == self.current_map_index
                    && (n.location_x - self.current_x).abs() <= Self::DATA_RANGE
                    && (n.location_y - self.current_y).abs() <= Self::DATA_RANGE
            });

        let npc = match npc_opt {
            Some(n) => n,
            None => {
                tracing::debug!("SellItem: early-return, no nearby NPC on map={}", self.current_map_index);
                return;
            }
        };

        // Load the player's current items from the world.
        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        // Find the item in the inventory by unique_id.
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
                tracing::debug!(
                    "SellItem: early-return, item with unique_id={} not found in inventory",
                    msg.unique_id
                );
                return;
            }
        };

        let mut item = match inv.slots[idx].clone() {
            Some(it) => it,
            None => {
                tracing::debug!("SellItem: early-return, inventory slot {} is empty", idx);
                return;
            }
        };

        if msg.count as u32 > item.count as u32 {
            tracing::debug!(
                "SellItem: early-return, requested count={} > item.count={} for unique_id={}",
                msg.count,
                item.count,
                msg.unique_id
            );
            return;
        }

        // Look up ItemInfo to compute sale price.
        let info = match self
            .world_db
            .item_infos
            .iter()
            .find(|i| i.index == item.item_index)
        {
            Some(i) => i,
            None => {
                tracing::debug!(
                    "SellItem: early-return, ItemInfo not found for item_index={} (unique_id={})",
                    item.item_index,
                    msg.unique_id
                );
                return;
            }
        };

        // Base sell price is half of the normal price * count, scaled by NPC rate.
        let base_total = match info.price.checked_mul(msg.count as u32) {
            Some(v) => v,
            None => {
                tracing::debug!(
                    "SellItem: early-return, base_total overflow price={} count={}",
                    info.price,
                    msg.count
                );
                return;
            }
        };

        let half = base_total / 2;
        let rate = (npc.rate as f32) / 100.0;
        let mut gold_gain = ((half as f32) * rate).floor() as u32;
        if gold_gain == 0 && half > 0 {
            gold_gain = 1;
        }

        let new_gold_u64 = (stats.gold as i64 as i128) + (gold_gain as i128);
        if new_gold_u64 > i64::MAX as i128 {
            tracing::debug!(
                "SellItem: early-return, gold overflow if adding gain={}, current_gold={}",
                gold_gain,
                stats.gold
            );
            return;
        }

        // Update inventory: either remove the stack or decrement the count.
        if msg.count as u16 == item.count {
            inv.slots[idx] = None;
        } else {
            let mut updated = item.clone();
            updated.count = updated.count.saturating_sub(msg.count);
            inv.slots[idx] = Some(updated);
        }

        // Persist updated items to the world.
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
        }

        // Update gold in current_stats and store.
        let mut new_stats = stats.clone();
        new_stats.gold = new_stats.gold.saturating_add(gold_gain as i64);

        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let _ = self
                .store
                .save_character_stats(account_id, char_idx, &new_stats);
        }

        self.current_stats = Some(new_stats.clone());

        // Notify client about the sale result (SSellItem mirrors C# ServerPackets.SellItem).
        let sell_pkt = SSellItem {
            unique_id: msg.unique_id,
            count: msg.count,
            success: true,
        };
        if let Ok(raw) = sell_pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Also notify client about gained gold and refreshed slots.
        let gained = SGainedGold { gold: gold_gain };
        if let Ok(raw) = gained.encode() {
            out.push(Self::encode_raw(raw));
        }

        let refresh = SUserSlotsRefresh {
            inventory: inv.slots,
            equipment: eq.slots,
        };
        if let Ok(raw) = refresh.encode() {
            out.push(Self::encode_raw(raw));
        }

        tracing::debug!(
            "SellItem: success unique_id={} count={} gold_gain={} gold_after={}",
            msg.unique_id,
            msg.count,
            gold_gain,
            new_stats.gold
        );
    }

    pub(crate) fn handle_buy_item(&mut self, msg: CBuyItem, out: &mut Vec<Vec<u8>>) {
        tracing::debug!(
            "BuyItem: start stage={:?} count={} panel_type={} item_index={}",
            self.stage,
            msg.count,
            msg.panel_type,
            msg.item_index,
        );

        if self.stage != Stage::InGame {
            tracing::debug!("BuyItem: early-return, not in-game stage");
            return;
        }

        if msg.count == 0 {
            tracing::debug!("BuyItem: early-return, count is 0");
            return;
        }

        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => {
                tracing::debug!("BuyItem: early-return, no current_stats available");
                return;
            }
        };

        // Only handle standard Buy panel for now (PanelType.Buy = 0 in C#).
        if msg.panel_type != 0 {
            tracing::debug!(
                "BuyItem: early-return, unsupported panel_type={} (only 0/Buy supported)",
                msg.panel_type
            );
            return;
        }

        // Derive npc_index from the high 32 bits of the unique item_index.
        let npc_index = (msg.item_index >> 32) as i32;
        tracing::debug!(
            "BuyItem: derived npc_index={} from item_index={}",
            npc_index,
            msg.item_index
        );

        let npc_opt = self
            .world_db
            .npc_infos
            .iter()
            .find(|n| n.index == npc_index && n.map_index == self.current_map_index);

        let npc = match npc_opt {
            Some(n) => n,
            None => {
                tracing::debug!(
                    "BuyItem: early-return, npc not found npc_index={} map_index={}",
                    npc_index,
                    self.current_map_index
                );
                return;
            }
        };

        // Range check against the NPC, mirroring CallNPC distance checks.
        let dx = npc.location_x - self.current_x;
        let dy = npc.location_y - self.current_y;
        if dx.abs() > Self::DATA_RANGE || dy.abs() > Self::DATA_RANGE {
            tracing::debug!(
                "BuyItem: early-return, out of range dx={} dy={} data_range={}",
                dx,
                dy,
                Self::DATA_RANGE
            );
            return;
        }

        // Reload the NPC's [TRADE] list to resolve the selected goods line.
        let root_deploy = Path::new("./deploy/Envir/NPCs");
        let root_plain = Path::new("./Envir/NPCs");
        let root = if root_deploy.exists() { root_deploy } else { root_plain };
        if !root.exists() {
            tracing::debug!("BuyItem: early-return, NPC script root not found");
            return;
        }

        let script_path = match Self::find_npc_script_path(root, &npc.file_name) {
            Some(p) => p,
            None => {
                tracing::debug!(
                    "BuyItem: early-return, npc script file not found for file_name='{}'",
                    npc.file_name
                );
                return;
            }
        };
        tracing::debug!("BuyItem: using npc script path {:?}", script_path);

        let specs = match Self::load_npc_trade_goods_from_file(&script_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::debug!(
                    "BuyItem: early-return, failed to load trade goods: {:?}",
                    e
                );
                return;
            }
        };
        tracing::debug!("BuyItem: loaded {} trade goods entries", specs.len());

        // Compute the goods index from the low 32 bits of the unique id.
        let base_uid = (npc.index as u64) << 32;
        if msg.item_index <= base_uid {
            tracing::debug!(
                "BuyItem: early-return, item_index {} <= base_uid {}",
                msg.item_index,
                base_uid
            );
            return;
        }
        let rel = msg.item_index - base_uid;
        if rel == 0 {
            tracing::debug!("BuyItem: early-return, rel computed as 0");
            return;
        }
        let idx = (rel - 1) as usize;
        if idx >= specs.len() {
            tracing::debug!(
                "BuyItem: early-return, computed idx={} out of bounds (len={})",
                idx,
                specs.len()
            );
            return;
        }

        let (ref name, _script_count) = specs[idx];
        tracing::debug!("BuyItem: resolved goods idx={} name='{}'", idx, name);

        // Locate the ItemInfoData by name.
        let info = match self
            .world_db
            .item_infos
            .iter()
            .find(|i| i.name.eq_ignore_ascii_case(name))
        {
            Some(i) => i,
            None => {
                tracing::debug!(
                    "BuyItem: early-return, ItemInfo not found for name='{}'",
                    name
                );
                return;
            }
        };

        // Enforce stack size limit consistent with C# goods.Info.StackSize.
        if msg.count as u32 > info.stack_size as u32 {
            tracing::debug!(
                "BuyItem: early-return, requested count={} exceeds stack_size={} for item='{}'",
                msg.count,
                info.stack_size,
                info.name
            );
            return;
        }

        // Compute base price and apply NPC rate.
        let base_price = match info.price.checked_mul(msg.count as u32) {
            Some(v) => v,
            None => {
                tracing::debug!(
                    "BuyItem: early-return, price overflow price={} count={}",
                    info.price,
                    msg.count
                );
                return;
            }
        };

        let rate = (npc.rate as f32) / 100.0;
        let mut cost = ((base_price as f32) * rate).floor() as u32;
        if cost == 0 && base_price > 0 {
            cost = 1;
        }
        tracing::debug!(
            "BuyItem: pricing item='{}' base_price={} rate={} cost={} player_gold={}",
            info.name,
            base_price,
            rate,
            cost,
            stats.gold
        );

        let cost_i64 = cost as i64;
        if stats.gold < cost_i64 {
            tracing::debug!(
                "BuyItem: early-return, insufficient gold: have={} cost={}",
                stats.gold,
                cost_i64
            );
            return;
        }

        // Deduct gold from the cached CharacterStats and persist to the store.
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

        let lose = SLoseGold { gold: cost };
        if let Ok(raw) = lose.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Insert the purchased item into the first empty inventory slot.
        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((crystal_server_core::item::Inventory::new_default(), crystal_server_core::item::Equipment::new_default()))
        };

        // Mirror C# HumanObject.AddItem behaviour: belt slots occupy the
        // first few inventory indices (BeltSize = 6 by default). Normal
        // items should prefer bag slots starting at index 6, and only fall
        // back to the belt region if the bag is completely full.
        let belt_size: usize = 6;
        let inv_len = inv.slots.len();

        let mut free_slot: Option<usize> = None;

        // 1) Prefer non-belt bag area [belt_size .. len)
        if inv_len > belt_size {
            for i in belt_size..inv_len {
                if inv.slots[i].is_none() {
                    free_slot = Some(i);
                    break;
                }
            }
        }

        // 2) If bag is full, fall back to belt area [0 .. belt_size)
        if free_slot.is_none() {
            let upper = belt_size.min(inv_len);
            for i in 0..upper {
                if inv.slots[i].is_none() {
                    free_slot = Some(i);
                    break;
                }
            }
        }

        let slot_index = match free_slot {
            Some(i) => i,
            None => {
                tracing::debug!(
                    "BuyItem: early-return, no free inventory slot for item='{}'",
                    info.name
                );
                return;
            }
        };

        // Generate a per-session unique_id using session_id and time; this is
        // sufficient for inventory operations and mirrors the spirit of the C#
        // UniqueID usage.
        let unique_id = {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let ts = now.as_nanos() as u64;
            ((self.session_id as u64) << 32) ^ ts
        };

        let user_item = crystal_server_core::item::create_fresh_user_item(
            info,
            unique_id,
            msg.count,
        );

        inv.slots[slot_index] = Some(user_item);

        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
        }

        let refresh = SUserSlotsRefresh {
            inventory: inv.slots,
            equipment: eq.slots,
        };
        if let Ok(raw) = refresh.encode() {
            out.push(Self::encode_raw(raw));
        }

        tracing::debug!(
            "BuyItem: success item='{}' count={} cost={} slot={} gold_after={}",
            info.name,
            msg.count,
            cost,
            slot_index,
            new_stats.gold
        );
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
                    .unwrap_or((crystal_server_core::item::Inventory::new_default(), crystal_server_core::item::Equipment::new_default()))
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
                    .unwrap_or((crystal_server_core::item::Inventory::new_default(), crystal_server_core::item::Equipment::new_default()))
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
        let (dest_x, dest_y) = {
            let default_pos = (npc.location_x, npc.location_y);

            if let Some(info) = self.world_db.get_map_info(npc.map_index).cloned() {
                let dir = &self.world_config.map_path;
                match crystal_server_core::world::map::load_map_from_file(info, dir.as_path()) {
                    Ok(map) => {
                        let nx = npc.location_x.max(0) as u16;
                        let ny = npc.location_y.max(0) as u16;

                        if map.is_walkable(nx, ny) {
                            (nx as i32, ny as i32)
                        } else {
                            let mut best: Option<(i32, i32, i32)> = None;
                            let max_dist2: i32 = 25; // radius 5

                            for &(wx, wy) in &map.walkable_cells {
                                let dx = wx as i32 - npc.location_x;
                                let dy = wy as i32 - npc.location_y;
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
                            npc.map_index,
                            e
                        );
                        default_pos
                    }
                }
            } else {
                default_pos
            }
        };

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
            self.update_visibility(out);
        }
    }

    pub(crate) fn handle_keep_alive(&mut self, msg: CKeepAlive, out: &mut Vec<Vec<u8>>) {
        let resp = SKeepAlive { time: msg.time };
        if let Ok(raw) = resp.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
}
