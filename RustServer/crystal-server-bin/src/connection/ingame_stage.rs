use std::path::Path;

use crystal_server_core::account::{CharacterPosition, CharacterSummary};
use crystal_server_core::world::{self};
use crystal_shared_proto::guild::SGuildStatus;
use crystal_shared_proto::item::{SEquipItem, SMoveItem, SRemoveItem};
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
    CTownRevive,
    CTurn,
    CWalk,
    SDisconnect,
    SKeepAlive,
};
use crystal_shared_proto::npc::{SNpcGoods, SNpcSell, SNpcRepair, SNpcsRepair};
use crystal_shared_proto::scene::{
    SChat,
    SNpcResponse,
    SObjectGuildNameChanged,
    SObjectRemove,
    SRevived,
    SObjectRevived,
};
use crystal_shared_proto::select::{SelectInfo, SLogOutFailed, SLogOutSuccess};
use crystal_shared_proto::user::{SGainedGold, SLoseGold, SUserSlotsRefresh, SHealthChanged};

use super::{LoginConnection, Stage};

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

        let _ = self.apply_step(msg.direction, 0, out);
    }

    pub(crate) fn handle_walk(&mut self, msg: CWalk, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

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

        if let Some(rest) = trimmed.strip_prefix("/createguild ") {
            self.handle_create_guild_command(rest, out);
            return;
        }

        if let Some(rest) = trimmed.strip_prefix("/showmemoney ") {
            if let Ok(delta) = rest.trim().parse::<i64>() {
                if delta > 0 {
                    if let Some(mut stats) = self.current_stats.clone() {
                        let new_gold = stats.gold.saturating_add(delta);
                        stats.gold = new_gold;
                        if let (Some(ref account_id), Some(char_idx)) =
                            (self.account_id.as_ref(), self.current_char_index)
                        {
                            let _ = self
                                .store
                                .save_character_stats(account_id, char_idx, &stats);
                        }
                        self.current_stats = Some(stats.clone());

                        let gained = SGainedGold {
                            gold: delta as u32,
                        };
                        if let Ok(raw) = gained.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
            }
            return;
        }

        if trimmed.eq_ignore_ascii_case("/kill") {
            let killed = {
                let mut world = self.world.lock().unwrap();
                world.kill_nearest_monster(
                    self.current_map_index,
                    self.current_x,
                    self.current_y,
                )
            };

            if let Some((id, monster_exp)) = killed {
                self.known_monsters.remove(&id);
                let pkt = SObjectRemove {
                    object_id: id as u32,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }

                if monster_exp > 0 {
                    let events = vec![world::WorldEvent::GainExperience {
                        session_id: self.session_id,
                        amount: monster_exp,
                    }];
                    let _ = self.handle_world_events(events, out);
                }
            }
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

    fn send_system_chat(&self, text: &str, out: &mut Vec<Vec<u8>>) {
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

    pub(crate) fn handle_town_revive(&mut self, _msg: CTownRevive, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Determine the bind location for this character (equivalent to C#
        // BindMapIndex/BindLocation). If none is stored, fall back to the
        // current map/position.
        let (dest_map, dest_x, dest_y, dest_dir) = if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
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

    pub(crate) fn handle_keep_alive(&mut self, msg: CKeepAlive, out: &mut Vec<Vec<u8>>) {
        let resp = SKeepAlive { time: msg.time };
        if let Ok(raw) = resp.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
}
