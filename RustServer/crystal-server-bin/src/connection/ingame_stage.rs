use std::path::Path;

use crystal_server_core::account::{CharacterPosition, CharacterSummary};
use crystal_server_core::world::{self};
use crystal_shared_proto::item_types::UserItemData;
use crystal_shared_proto::login::{
    CAttack,
    CCallNPC,
    CKeepAlive,
    CRun,
    CTurn,
    CWalk,
    SKeepAlive,
};
use crystal_shared_proto::npc::{SNpcGoods, SNpcSell, SNpcRepair, SNpcsRepair};
use crystal_shared_proto::scene::{SNpcResponse, SObjectRemove};
use crystal_shared_proto::select::{SelectInfo, SLogOutFailed, SLogOutSuccess};
use crystal_shared_proto::user::SLoseGold;

use super::{LoginConnection, Stage};

impl LoginConnection {
    pub(crate) fn handle_log_out(&mut self, out: &mut Vec<Vec<u8>>) {
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

                let pos = CharacterPosition {
                    map_index: self.current_map_index,
                    x: self.current_x,
                    y: self.current_y,
                    direction: self.direction,
                };
                let _ = self
                    .store
                    .save_character_position(account_id, char_idx, &pos);
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
        if !trimmed.is_empty() {
            tracing::info!(
                target = "chat",
                session_id = self.session_id,
                map_index = self.current_map_index,
                "{}",
                trimmed,
            );
        }

        if trimmed.eq_ignore_ascii_case("/kill") {
            let killed_id = {
                let mut world = self.world.lock().unwrap();
                world.kill_nearest_monster(
                    self.current_map_index,
                    self.current_x,
                    self.current_y,
                )
            };

            if let Some(id) = killed_id {
                self.known_monsters.remove(&id);
                let pkt = SObjectRemove {
                    object_id: id as u32,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }

    pub(crate) fn handle_call_npc(&mut self, msg: CCallNPC, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        if msg.key.len() > 64 {
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

                if is_sell_only {
                    let sell = SNpcSell;
                    let raw = sell.encode();
                    out.push(Self::encode_raw(raw));
                    return;
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

                    let panel_type: u8 = 0;
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

                    if matches!(key_upper, "@BUYSELL" | "@BUYSELLNEW") {
                        let sell = SNpcSell;
                        let raw = sell.encode();
                        out.push(Self::encode_raw(raw));
                    }

                    return;
                }

                if key_upper == "@BUYBACK" {
                    let goods_items: Vec<UserItemData> = Vec::new();
                    let panel_type: u8 = 0;
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

                    return;
                }

                if key_upper == "@BUYUSED" {
                    let goods_items: Vec<UserItemData> = Vec::new();
                    let panel_type: u8 = 1;
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

                    return;
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

    pub(crate) fn handle_keep_alive(&mut self, msg: CKeepAlive, out: &mut Vec<Vec<u8>>) {
        let resp = SKeepAlive { time: msg.time };
        if let Ok(raw) = resp.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
}
