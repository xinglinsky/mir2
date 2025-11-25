use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crystal_server_core::world;
use crystal_shared_proto::io::{write_bool, write_f32_le, write_i32_le};
use crystal_shared_proto::item_types::{AwakeData, ItemInfoData, StatsMap, UserItemData};
use crystal_shared_proto::login::{CCallNPC, SDisconnect};
use crystal_shared_proto::npc::{SNpcGoods, SNpcSell, SNpcRepair, SNpcsRepair};
use crystal_shared_proto::scene::SNpcResponse;
use crystal_shared_proto::user::SLoseGold;

use super::{LoginConnection, Stage};

impl LoginConnection {
    pub(crate) fn normalize_npc_key(key: &str) -> String {
        let mut s = key.trim().to_ascii_uppercase();
        if s.starts_with('[') && s.ends_with(']') && s.len() >= 3 {
            s = s[1..s.len() - 1].to_string();
        }
        if !s.starts_with('@') {
            s = format!("@{}", s);
        }
        s
    }

    pub(crate) fn find_npc_script_path(root: &Path, file_name: &str) -> Option<PathBuf> {
        // In the original C# server, NpcInfo.FileName stores a path relative to the
        // Envir/NPCs folder, for example "BichonProvince\\BichonWall\\BookStore".
        // First try interpreting the value as such a relative path (with or without
        // ".txt" extension). If that fails, fall back to a recursive search by
        // basename as before.

        // Normalise separators to forward slashes for portability.
        let rel = file_name.replace('\\', "/");
        let rel_path = Path::new(&rel);

        // Candidate 1: root / rel_path (as-is).
        let candidate1 = root.join(rel_path);
        if candidate1.is_file() {
            return Some(candidate1);
        }

        // Candidate 2: ensure .txt extension.
        let mut candidate2 = candidate1.clone();
        if candidate2.extension().is_none() {
            candidate2.set_extension("txt");
        }
        if candidate2.is_file() {
            return Some(candidate2);
        }

        // Fallback: recursive search by basename + .txt (legacy behaviour).
        let target = format!("{}.txt", file_name).to_lowercase();
        let mut stack = vec![root.to_path_buf()];

        while let Some(dir) = stack.pop() {
            let read_dir = match fs::read_dir(&dir) {
                Ok(r) => r,
                Err(_) => continue,
            };

            for entry_res in read_dir {
                let entry = match entry_res {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if name.to_lowercase() == target {
                        return Some(path);
                    }
                }
            }
        }

        None
    }

    pub(crate) fn load_npc_script_from_file(
        path: &Path,
    ) -> io::Result<(
        HashMap<String, Vec<String>>,
        HashMap<String, (String, i32, i32)>,
    )> {
        let text = fs::read_to_string(path)?;
        let lines: Vec<&str> = text.lines().collect();
        let mut pages: HashMap<String, Vec<String>> = HashMap::new();
        let mut moves: HashMap<String, (String, i32, i32)> = HashMap::new();

        let mut current_label: Option<String> = None;
        let mut i: usize = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            if trimmed.starts_with("[@") {
                if let Some(end) = trimmed.find(']') {
                    let label_inner = &trimmed[1..end];
                    let key = label_inner.to_ascii_uppercase();
                    current_label = Some(key);
                } else {
                    current_label = None;
                }
                i += 1;
                continue;
            }

            if let Some(label) = &current_label {
                if trimmed.eq_ignore_ascii_case("#SAY") {
                    let mut page_lines: Vec<String> = Vec::new();
                    i += 1;
                    while i < lines.len() {
                        let l = lines[i];
                        let t = l.trim_start();
                        if t.starts_with("[@") || (t.starts_with('#') && !t.eq_ignore_ascii_case("#SAY")) {
                            break;
                        }
                        page_lines.push(l.to_string());
                        i += 1;
                    }
                    pages.insert(label.clone(), page_lines);
                    continue;
                } else if trimmed.eq_ignore_ascii_case("#ACT") {
                    i += 1;
                    while i < lines.len() {
                        let l = lines[i];
                        let t = l.trim();
                        if t.is_empty() {
                            i += 1;
                            continue;
                        }
                        if t.starts_with("[@") || t.starts_with('#') {
                            i -= 1;
                            break;
                        }

                        let parts: Vec<&str> = t.split_whitespace().collect();
                        if !parts.is_empty() && parts[0].eq_ignore_ascii_case("MOVE") {
                            if parts.len() >= 2 {
                                let map_name = parts[1].to_string();
                                let x = parts
                                    .get(2)
                                    .and_then(|s| s.parse::<i32>().ok())
                                    .unwrap_or(0);
                                let y = parts
                                    .get(3)
                                    .and_then(|s| s.parse::<i32>().ok())
                                    .unwrap_or(0);
                                moves.insert(label.clone(), (map_name, x, y));
                            }
                            break;
                        }

                        i += 1;
                    }
                    continue;
                }
            }

            i += 1;
        }

        Ok((pages, moves))
    }

    pub(crate) fn extract_paid_teleport_info(path: &Path, key: &str) -> Option<(i64, Option<String>)> {
        let text = fs::read_to_string(path).ok()?;
        let lines: Vec<&str> = text.lines().collect();

        let target_key = key.to_ascii_uppercase();
        let mut current_label: Option<String> = None;
        let mut in_act = false;
        let mut price: Option<i64> = None;
        let mut fail_goto: Option<String> = None;

        let mut i: usize = 0;
        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            if trimmed.starts_with("[@") {
                if let Some(end) = trimmed.find(']') {
                    let label_inner = &trimmed[1..end];
                    current_label = Some(label_inner.to_ascii_uppercase());
                    in_act = false;
                } else {
                    current_label = None;
                    in_act = false;
                }
                i += 1;
                continue;
            }

            if current_label.as_deref() != Some(&target_key) {
                i += 1;
                continue;
            }

            if trimmed.eq_ignore_ascii_case("#ACT") {
                in_act = true;
                i += 1;
                continue;
            }

            if trimmed.eq_ignore_ascii_case("#ELSEACT") {
                in_act = false;

                let mut j = i + 1;
                while j < lines.len() {
                    let t = lines[j].trim();
                    if t.is_empty() {
                        j += 1;
                        continue;
                    }
                    if t.starts_with("[@") || t.starts_with('#') {
                        break;
                    }
                    let parts: Vec<&str> = t.split_whitespace().collect();
                    if !parts.is_empty() && parts[0].eq_ignore_ascii_case("GOTO") && parts.len() >= 2 {
                        let target_raw = parts[1];
                        let target_norm = Self::normalize_npc_key(target_raw);
                        fail_goto = Some(target_norm);
                    }
                    break;
                }

                i += 1;
                continue;
            }

            if in_act {
                if !trimmed.is_empty() {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if !parts.is_empty() && parts[0].eq_ignore_ascii_case("TAKEGOLD") && parts.len() >= 2 {
                        if let Ok(val) = parts[1].parse::<i64>() {
                            price = Some(val);
                        }
                    }
                }
            }

            i += 1;
        }

        if price.is_some() || fail_goto.is_some() {
            Some((price.unwrap_or(0), fail_goto))
        } else {
            None
        }
    }

    /// Build the raw goods_bytes payload for an SNpcGoods packet, mirroring the
    /// C# ServerPackets.NPCGoods.WritePacket layout:
    ///   count (i32)
    ///   repeated UserItem.Save(writer) blobs
    ///   Rate (f32)
    ///   Type (PanelType as byte)
    ///   HideAddedStats (bool)
    pub(crate) fn build_npc_goods_bytes(
        goods: &[UserItemData],
        rate: f32,
        panel_type: u8,
        hide_added_stats: bool,
    ) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();

        write_i32_le(&mut buf, goods.len() as i32)?;
        for item in goods {
            item.encode(&mut buf)?;
        }

        write_f32_le(&mut buf, rate)?;
        buf.push(panel_type);
        write_bool(&mut buf, hide_added_stats)?;

        Ok(buf)
    }

    /// Parse [Trade] sections from an NPC script file, returning (ItemName, Count)
    /// pairs similar to C# NPCScript.ParseGoods.
    pub(crate) fn load_npc_trade_goods_from_file(
        path: &Path,
    ) -> io::Result<Vec<(String, u16)>> {
        let text = fs::read_to_string(path)?;
        let lines: Vec<&str> = text.lines().collect();
        let mut goods: Vec<(String, u16)> = Vec::new();

        let mut i: usize = 0;
        while i < lines.len() {
            let trimmed = lines[i].trim();
            if trimmed.to_ascii_uppercase().starts_with("[TRADE]") {
                i += 1;
                while i < lines.len() {
                    let line = lines[i];
                    let t = line.trim();
                    if t.starts_with('[') {
                        break;
                    }
                    if t.is_empty() {
                        i += 1;
                        continue;
                    }

                    let parts: Vec<&str> = t.split_whitespace().collect();
                    if parts.is_empty() {
                        i += 1;
                        continue;
                    }

                    let name = parts[0].to_string();
                    let mut count: u16 = 1;
                    if parts.len() >= 2 {
                        if let Ok(v) = parts[1].parse::<u16>() {
                            count = v;
                        }
                    }

                    goods.push((name, count));
                    i += 1;
                }
                continue;
            }
            i += 1;
        }

        Ok(goods)
    }

    /// Build a minimal UserItemData for a shop item from ItemInfoData, mimicking
    /// the core behaviour of C# Envir.CreateShopItem.
    pub(crate) fn make_shop_user_item(
        info: &ItemInfoData,
        unique_id: u64,
        count: u16,
    ) -> UserItemData {
        UserItemData {
            unique_id,
            item_index: info.index,
            current_dura: info.durability,
            max_dura: info.durability,
            count,
            soul_bound_id: 0,
            identified: !info.need_identify,
            cursed: false,
            slots: Vec::new(),
            gem_count: 0,
            added_stats: StatsMap { entries: Vec::new() },
            awake: AwakeData {
                awake_type: 0,
                values: Vec::new(),
            },
            refined_value: 0,
            refine_added: 0,
            refine_success_chance: 0,
            wedding_ring: 0,
            expire_info: None,
            rental_information: None,
            is_shop_item: true,
            sealed_info: None,
            gm_made: false,
        }
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
}
