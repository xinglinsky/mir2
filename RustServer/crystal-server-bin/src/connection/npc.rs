use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crystal_server_core::account::AccountStorage;
use crystal_server_core::world;
use crystal_shared_proto::io::{write_bool, write_f32_le, write_i32_le};
use crystal_shared_proto::item::{SUserStorage};
use crystal_shared_proto::item_types::{AwakeData, ItemInfoData, StatsMap, UserItemData};
use crystal_shared_proto::login::{CCallNPC, SDisconnect};
use crystal_shared_proto::npc::{SNpcGoods, SNpcSell, SNpcStorage, SNpcRepair, SNpcsRepair};
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

    /// Expand simple NPC dialog placeholders such as <$USERNAME>, <$LEVEL>,
    /// <$MAP>, <$HP>, <$GAMEGOLD> and <$PARCELAMOUNT> in the given page
    /// lines. This mirrors the C# NPCSegment.ReplaceValue behaviour for a
    /// subset of placeholders that are currently supported by the Rust
    /// server.
    fn expand_npc_placeholders(&self, lines: Vec<String>, npc_name: Option<&str>) -> Vec<String> {
        if lines.is_empty() {
            return lines;
        }

        // First check which placeholders are actually present to avoid
        // unnecessary DB/world lookups.
        let need_username = lines.iter().any(|l| l.contains("<$USERNAME>"));
        let need_level = lines.iter().any(|l| l.contains("<$LEVEL>"));
        let need_class = lines.iter().any(|l| l.contains("<$CLASS>"));
        let need_map = lines.iter().any(|l| l.contains("<$MAP>"));
        let need_x = lines.iter().any(|l| l.contains("<$X_COORD>"));
        let need_y = lines.iter().any(|l| l.contains("<$Y_COORD>"));
        let need_hp = lines.iter().any(|l| l.contains("<$HP>"));
        let need_maxhp = lines.iter().any(|l| l.contains("<$MAXHP>"));
        let need_mp = lines.iter().any(|l| l.contains("<$MP>"));
        let need_maxmp = lines.iter().any(|l| l.contains("<$MAXMP>"));
        let need_gold = lines.iter().any(|l| l.contains("<$GAMEGOLD>"));
        let need_credit = lines.iter().any(|l| l.contains("<$CREDIT>"));
        let need_pk = lines.iter().any(|l| l.contains("<$PKPOINT>"));
        let need_date = lines.iter().any(|l| l.contains("<$DATE>"));
        let need_usercount = lines.iter().any(|l| l.contains("<$USERCOUNT>"));
        let need_parcel = lines.iter().any(|l| l.contains("<$PARCELAMOUNT>"));
        let need_npcname = lines.iter().any(|l| l.contains("<$NPCNAME>"));
        let need_roll_result = lines.iter().any(|l| l.contains("<$ROLLRESULT>"));

        // Equipment slot placeholders.
        let need_armour = lines.iter().any(|l| l.contains("<$ARMOUR>"));
        let need_weapon = lines.iter().any(|l| l.contains("<$WEAPON>"));
        let need_ring_l = lines.iter().any(|l| l.contains("<$RING_L>"));
        let need_ring_r = lines.iter().any(|l| l.contains("<$RING_R>"));
        let need_bracelet_l = lines.iter().any(|l| l.contains("<$BRACELET_L>"));
        let need_bracelet_r = lines.iter().any(|l| l.contains("<$BRACELET_R>"));
        let need_necklace = lines.iter().any(|l| l.contains("<$NECKLACE>"));
        let need_belt = lines.iter().any(|l| l.contains("<$BELT>"));
        let need_boots = lines.iter().any(|l| l.contains("<$BOOTS>"));
        let need_helmet = lines.iter().any(|l| l.contains("<$HELMET>"));
        let need_amulet = lines.iter().any(|l| l.contains("<$AMULET>"));
        let need_stone = lines.iter().any(|l| l.contains("<$STONE>"));
        let need_torch = lines.iter().any(|l| l.contains("<$TORCH>"));
        let need_mount = lines.iter().any(|l| l.contains("<$MOUNT>"));
        let need_mount_loyalty = lines.iter().any(|l| l.contains("<$MOUNTLOYALTY>"));

        // Guild / guild war placeholders.
        let need_guild_war_time = lines.iter().any(|l| l.contains("<$GUILDWARTIME>"));
        let need_guild_war_fee = lines.iter().any(|l| l.contains("<$GUILDWARFEE>"));
        let need_guild_name = lines.iter().any(|l| l.contains("<$GUILDNAME>"));
        let need_agit_guild_name = lines.iter().any(|l| l.contains("<$AGITGUILDNAME>"));
        let need_guild_rental_days_left =
            lines.iter().any(|l| l.contains("<$GUILDGTRENTALDAYSLEFT>"));

        // Character summary based values: USERNAME, LEVEL, CLASS.
        let mut username: Option<String> = None;
        let mut level: Option<String> = None;
        let mut class_name: Option<String> = None;
        if need_username || need_level || need_class {
            if let Some(char_idx) = self.current_char_index {
                if let Some(ch) = self.characters.iter().find(|c| c.index == char_idx) {
                    if need_username {
                        username = Some(ch.name.clone());
                    }
                    if need_level {
                        level = Some(ch.level.to_string());
                    }
                    if need_class {
                        let cname = match ch.class {
                            0 => "Warrior",
                            1 => "Wizard",
                            2 => "Taoist",
                            3 => "Assassin",
                            _ => "Unknown",
                        };
                        class_name = Some(cname.to_string());
                    }
                }
            }
        }

        // Map/position placeholders.
        let mut map_name: Option<String> = None;
        let mut x_str: Option<String> = None;
        let mut y_str: Option<String> = None;
        if need_map || need_x || need_y {
            if need_map {
                if let Some(info) = self
                    .world_db
                    .map_infos
                    .iter()
                    .find(|m| m.index == self.current_map_index)
                {
                    map_name = Some(info.file_name.clone());
                }
            }
            if need_x {
                x_str = Some(self.current_x.to_string());
            }
            if need_y {
                y_str = Some(self.current_y.to_string());
            }
        }

        // HP/MP from the world PlayerState.
        let mut hp_str: Option<String> = None;
        let mut maxhp_str: Option<String> = None;
        let mut mp_str: Option<String> = None;
        let mut maxmp_str: Option<String> = None;
        if need_hp || need_maxhp || need_mp || need_maxmp {
            let world = self.world.lock().unwrap();
            if need_hp || need_mp {
                if let Some((hp, mp)) = world.player_current_hp_mp(self.session_id) {
                    if need_hp {
                        hp_str = Some(hp.to_string());
                    }
                    if need_mp {
                        mp_str = Some(mp.to_string());
                    }
                }
            }
            if need_maxhp || need_maxmp {
                if let Some((max_hp, max_mp)) = world.player_max_hp_mp(self.session_id) {
                    if need_maxhp {
                        maxhp_str = Some(max_hp.to_string());
                    }
                    if need_maxmp {
                        maxmp_str = Some(max_mp.to_string());
                    }
                }
            }
        }

        // Account-level stats (gold/credit) from CharacterStats.
        let mut gold_str: Option<String> = None;
        let mut credit_str: Option<String> = None;
        if (need_gold || need_credit) && self.current_stats.is_some() {
            if let Some(stats) = &self.current_stats {
                if need_gold {
                    gold_str = Some(stats.gold.to_string());
                }
                if need_credit {
                    credit_str = Some(stats.credit.to_string());
                }
            }
        }

        // PK points from world PlayerState; if not available defaults to 0.
        let mut pk_str: Option<String> = None;
        if need_pk {
            let world = self.world.lock().unwrap();
            let pk = world.player_pk_points(self.session_id).unwrap_or(0);
            pk_str = Some(pk.to_string());
        }

        // Date and usercount are global, independent of character.
        let mut date_str: Option<String> = None;
        if need_date {
            let now = chrono::Local::now();
            date_str = Some(now.format("%Y-%m-%d").to_string());
        }

        let mut usercount_str: Option<String> = None;
        if need_usercount {
            let summaries = self.player_summaries.lock().unwrap();
            usercount_str = Some(summaries.len().to_string());
        }

        // Parcel amount from stored mail, mirroring GetMailAwaitingCollectionAmount.
        let mut parcel_str: Option<String> = None;
        if need_parcel {
            if let (Some(ref account_id), Some(char_idx)) =
                (self.account_id.as_ref(), self.current_char_index)
            {
                if let Ok(mails) = self.store.load_character_mail(account_id, char_idx) {
                    let count = mails.iter().filter(|m| !m.collected).count();
                    parcel_str = Some(count.to_string());
                }
            }
        }

        // NPC name from the provided hint, with underscores replaced by spaces
        // to match C# NPCNAME behaviour.
        let npc_name_str: Option<String> = if need_npcname {
            npc_name.map(|n| n.replace('_', " "))
        } else {
            None
        };

        let mut roll_result_str: Option<String> = None;
        if need_roll_result {
            let world = self.world.lock().unwrap();
            let val = world.get_player_npc_data(self.session_id, "NPCRollResult");
            roll_result_str = Some(val.unwrap_or_else(|| "Not Rolled".to_string()));
        }

        // Equipment-slot placeholders: look up the current equipment from the
        // world and then resolve the ItemInfoData name as a FriendlyName.
        let mut armour_str: Option<String> = None;
        let mut weapon_str: Option<String> = None;
        let mut ring_l_str: Option<String> = None;
        let mut ring_r_str: Option<String> = None;
        let mut bracelet_l_str: Option<String> = None;
        let mut bracelet_r_str: Option<String> = None;
        let mut necklace_str: Option<String> = None;
        let mut belt_str: Option<String> = None;
        let mut boots_str: Option<String> = None;
        let mut helmet_str: Option<String> = None;
        let mut amulet_str: Option<String> = None;
        let mut stone_str: Option<String> = None;
        let mut torch_str: Option<String> = None;
        let mut mount_str: Option<String> = None;
        let mut mount_loyalty_str: Option<String> = None;

        if need_armour
            || need_weapon
            || need_ring_l
            || need_ring_r
            || need_bracelet_l
            || need_bracelet_r
            || need_necklace
            || need_belt
            || need_boots
            || need_helmet
            || need_amulet
            || need_stone
            || need_torch
            || need_mount
            || need_mount_loyalty
        {
            let world = self.world.lock().unwrap();
            if let Some((_, equipment)) = world.player_items(self.session_id) {
                let slot_name = |slot: usize, empty: &str| -> String {
                    if let Some(item) = equipment.slots.get(slot).and_then(|s| s.as_ref()) {
                        if let Some(info) = self.world_db.item_infos.iter().find(|i| i.index == item.item_index) {
                            return info.name.clone();
                        }
                    }
                    empty.to_string()
                };

                if need_armour {
                    armour_str = Some(slot_name(1, "No Armour"));
                }
                if need_weapon {
                    weapon_str = Some(slot_name(0, "No Weapon"));
                }
                if need_ring_l {
                    ring_l_str = Some(slot_name(7, "No Ring"));
                }
                if need_ring_r {
                    ring_r_str = Some(slot_name(8, "No Ring"));
                }
                if need_bracelet_l {
                    bracelet_l_str = Some(slot_name(5, "No Bracelet"));
                }
                if need_bracelet_r {
                    bracelet_r_str = Some(slot_name(6, "No Bracelet"));
                }
                if need_necklace {
                    necklace_str = Some(slot_name(4, "No Necklace"));
                }
                if need_belt {
                    belt_str = Some(slot_name(10, "No Belt"));
                }
                if need_boots {
                    boots_str = Some(slot_name(11, "No Boots"));
                }
                if need_helmet {
                    helmet_str = Some(slot_name(2, "No Helmet"));
                }
                if need_amulet {
                    amulet_str = Some(slot_name(9, "No Amulet"));
                }
                if need_stone {
                    stone_str = Some(slot_name(12, "No Stone"));
                }
                if need_torch {
                    torch_str = Some(slot_name(3, "No Torch"));
                }
                if need_mount {
                    mount_str = Some(slot_name(13, "No Mount"));
                }
                if need_mount_loyalty {
                    if let Some(item) = equipment.slots.get(13).and_then(|s| s.as_ref()) {
                        mount_loyalty_str =
                            Some(format!("{} ({})", item.current_dura, item.max_dura));
                    } else {
                        mount_loyalty_str = Some("No Mount".to_string());
                    }
                }
            }
        }

        // Guild-related placeholders.
        let mut guild_war_time_str: Option<String> = None;
        let mut guild_war_fee_str: Option<String> = None;
        let mut guild_name_str: Option<String> = None;
        let mut agit_guild_name_str: Option<String> = None;
        let mut guild_rental_days_left_str: Option<String> = None;

        if need_guild_war_time || need_guild_war_fee {
            let cfg = world::configs::guild_settings();
            if need_guild_war_time {
                guild_war_time_str = Some(cfg.war_time.to_string());
            }
            if need_guild_war_fee {
                guild_war_fee_str = Some(cfg.war_cost.to_string());
            }
        }

        if need_guild_name || need_agit_guild_name || need_guild_rental_days_left {
            let world = self.world.lock().unwrap();
            let player_guild = world
                .player_guild_name(self.session_id)
                .unwrap_or_default();

            if need_guild_name {
                if player_guild.is_empty() {
                    guild_name_str = Some("No Guild".to_string());
                } else {
                    guild_name_str = Some(format!("{} Guild", player_guild));
                }
            }

            if need_agit_guild_name {
                if player_guild.is_empty() {
                    agit_guild_name_str = Some("NO GUILD".to_string());
                } else {
                    agit_guild_name_str = Some(player_guild.clone());
                }
            }

            if need_guild_rental_days_left {
                if player_guild.is_empty() {
                    guild_rental_days_left_str = Some("0".to_string());
                } else if let Some(guild) = world.get_guild_info_by_name(&player_guild) {
                    let now_ticks = Self::unix_ms_to_dotnet_binary(Self::now_millis());
                    const TICKS_PER_DAY: i64 = 24 * 60 * 60 * 10_000_000;
                    let delta = guild.gt_rent_ticks - now_ticks;
                    let days = if TICKS_PER_DAY > 0 {
                        delta / TICKS_PER_DAY
                    } else {
                        0
                    };
                    guild_rental_days_left_str = Some(days.to_string());
                } else {
                    guild_rental_days_left_str = Some("0".to_string());
                }
            }
        }

        // Apply replacements line by line.
        let mut out_lines = Vec::with_capacity(lines.len());
        for mut line in lines {
            if let Some(ref v) = username {
                if line.contains("<$USERNAME>") {
                    line = line.replace("<$USERNAME>", v);
                }
            }
            if let Some(ref v) = level {
                if line.contains("<$LEVEL>") {
                    line = line.replace("<$LEVEL>", v);
                }
            }
            if let Some(ref v) = class_name {
                if line.contains("<$CLASS>") {
                    line = line.replace("<$CLASS>", v);
                }
            }
            if let Some(ref v) = map_name {
                if line.contains("<$MAP>") {
                    line = line.replace("<$MAP>", v);
                }
            }
            if let Some(ref v) = x_str {
                if line.contains("<$X_COORD>") {
                    line = line.replace("<$X_COORD>", v);
                }
            }
            if let Some(ref v) = y_str {
                if line.contains("<$Y_COORD>") {
                    line = line.replace("<$Y_COORD>", v);
                }
            }
            if let Some(ref v) = hp_str {
                if line.contains("<$HP>") {
                    line = line.replace("<$HP>", v);
                }
            }
            if let Some(ref v) = maxhp_str {
                if line.contains("<$MAXHP>") {
                    line = line.replace("<$MAXHP>", v);
                }
            }
            if let Some(ref v) = mp_str {
                if line.contains("<$MP>") {
                    line = line.replace("<$MP>", v);
                }
            }
            if let Some(ref v) = maxmp_str {
                if line.contains("<$MAXMP>") {
                    line = line.replace("<$MAXMP>", v);
                }
            }
            if let Some(ref v) = gold_str {
                if line.contains("<$GAMEGOLD>") {
                    line = line.replace("<$GAMEGOLD>", v);
                }
            }
            if let Some(ref v) = credit_str {
                if line.contains("<$CREDIT>") {
                    line = line.replace("<$CREDIT>", v);
                }
            }
            if let Some(ref v) = pk_str {
                if line.contains("<$PKPOINT>") {
                    line = line.replace("<$PKPOINT>", v);
                }
            }
            if let Some(ref v) = date_str {
                if line.contains("<$DATE>") {
                    line = line.replace("<$DATE>", v);
                }
            }
            if let Some(ref v) = usercount_str {
                if line.contains("<$USERCOUNT>") {
                    line = line.replace("<$USERCOUNT>", v);
                }
            }
            if let Some(ref v) = parcel_str {
                if line.contains("<$PARCELAMOUNT>") {
                    line = line.replace("<$PARCELAMOUNT>", v);
                }
            }
            if let Some(ref v) = npc_name_str {
                if line.contains("<$NPCNAME>") {
                    line = line.replace("<$NPCNAME>", v);
                }
            }

            if let Some(ref v) = roll_result_str {
                if line.contains("<$ROLLRESULT>") {
                    line = line.replace("<$ROLLRESULT>", v);
                }
            }

            if let Some(ref v) = guild_war_time_str {
                if line.contains("<$GUILDWARTIME>") {
                    line = line.replace("<$GUILDWARTIME>", v);
                }
            }
            if let Some(ref v) = guild_war_fee_str {
                if line.contains("<$GUILDWARFEE>") {
                    line = line.replace("<$GUILDWARFEE>", v);
                }
            }
            if let Some(ref v) = guild_name_str {
                if line.contains("<$GUILDNAME>") {
                    line = line.replace("<$GUILDNAME>", v);
                }
            }
            if let Some(ref v) = agit_guild_name_str {
                if line.contains("<$AGITGUILDNAME>") {
                    line = line.replace("<$AGITGUILDNAME>", v);
                }
            }
            if let Some(ref v) = guild_rental_days_left_str {
                if line.contains("<$GUILDGTRENTALDAYSLEFT>") {
                    line = line.replace("<$GUILDGTRENTALDAYSLEFT>", v);
                }
            }

            if let Some(ref v) = armour_str {
                if line.contains("<$ARMOUR>") {
                    line = line.replace("<$ARMOUR>", v);
                }
            }
            if let Some(ref v) = weapon_str {
                if line.contains("<$WEAPON>") {
                    line = line.replace("<$WEAPON>", v);
                }
            }
            if let Some(ref v) = ring_l_str {
                if line.contains("<$RING_L>") {
                    line = line.replace("<$RING_L>", v);
                }
            }
            if let Some(ref v) = ring_r_str {
                if line.contains("<$RING_R>") {
                    line = line.replace("<$RING_R>", v);
                }
            }
            if let Some(ref v) = bracelet_l_str {
                if line.contains("<$BRACELET_L>") {
                    line = line.replace("<$BRACELET_L>", v);
                }
            }
            if let Some(ref v) = bracelet_r_str {
                if line.contains("<$BRACELET_R>") {
                    line = line.replace("<$BRACELET_R>", v);
                }
            }
            if let Some(ref v) = necklace_str {
                if line.contains("<$NECKLACE>") {
                    line = line.replace("<$NECKLACE>", v);
                }
            }
            if let Some(ref v) = belt_str {
                if line.contains("<$BELT>") {
                    line = line.replace("<$BELT>", v);
                }
            }
            if let Some(ref v) = boots_str {
                if line.contains("<$BOOTS>") {
                    line = line.replace("<$BOOTS>", v);
                }
            }
            if let Some(ref v) = helmet_str {
                if line.contains("<$HELMET>") {
                    line = line.replace("<$HELMET>", v);
                }
            }
            if let Some(ref v) = amulet_str {
                if line.contains("<$AMULET>") {
                    line = line.replace("<$AMULET>", v);
                }
            }
            if let Some(ref v) = stone_str {
                if line.contains("<$STONE>") {
                    line = line.replace("<$STONE>", v);
                }
            }
            if let Some(ref v) = torch_str {
                if line.contains("<$TORCH>") {
                    line = line.replace("<$TORCH>", v);
                }
            }
            if let Some(ref v) = mount_str {
                if line.contains("<$MOUNT>") {
                    line = line.replace("<$MOUNT>", v);
                }
            }

            if let Some(ref v) = mount_loyalty_str {
                if line.contains("<$MOUNTLOYALTY>") {
                    line = line.replace("<$MOUNTLOYALTY>", v);
                }
            }

            out_lines.push(line);
        }

        out_lines
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

    pub(crate) fn page_has_roll_action(path: &Path, key: &str) -> bool {
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => return false,
        };
        let lines: Vec<&str> = text.lines().collect();

        let target_key = key.to_ascii_uppercase();
        let mut current_label: Option<String> = None;
        let mut in_act = false;

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

            if trimmed.starts_with('#') {
                if in_act {
                    in_act = false;
                }
                i += 1;
                continue;
            }

            if in_act {
                if !trimmed.is_empty() {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if !parts.is_empty()
                        && (parts[0].eq_ignore_ascii_case("ROLLDIE")
                            || parts[0].eq_ignore_ascii_case("ROLLYUT"))
                    {
                        return true;
                    }
                }
            }

            i += 1;
        }

        false
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
                // Reset storage NPC context by default; it will be set again
                // below if this call opens the @STORAGE page.
                self.current_storage_npc_id = None;

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

                            if Self::page_has_roll_action(&script_path, &key) {
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default();
                                let nanos = now.subsec_nanos();
                                let result = (nanos % 6) + 1;

                                {
                                    let mut world = self.world.lock().unwrap();
                                    world.set_player_npc_data(
                                        self.session_id,
                                        "NPCRollResult",
                                        result.to_string(),
                                    );
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

                let page_raw = maybe_page.unwrap_or_else(|| vec![npc.name.clone()]);
                let page = self.expand_npc_placeholders(page_raw, Some(&npc.name));
                let resp = SNpcResponse { page };
                if let Ok(raw) = resp.encode() {
                    out.push(Self::encode_raw(raw));
                }

                // Special-case the storage page: when the player clicks an
                // [@STORAGE] entry, mirror the C# behaviour of sending the
                // current account Storage contents followed by an NPCStorage
                // packet to open the warehouse dialog.
                if key_upper == "@STORAGE" {
                    // Remember which NPC index opened the storage page so
                    // that subsequent StoreItem/TakeBackItem requests can be
                    // validated against NPC proximity, similar to C#
                    // PlayerObject.NPCPage/NPCObjectID.
                    self.current_storage_npc_id = Some(msg.object_id);

                    if let Some(ref account_id) = self.account_id {
                        let account_storage = match self.store.load_account_storage(account_id) {
                            Ok(Some(s)) => s,
                            Ok(None) => AccountStorage {
                                slots: vec![None; 80],
                                has_expanded_storage: false,
                                expanded_storage_expiry_binary: 0,
                            },
                            Err(_) => AccountStorage {
                                slots: vec![None; 80],
                                has_expanded_storage: false,
                                expanded_storage_expiry_binary: 0,
                            },
                        };

                        let mut storage_bytes = Vec::new();
                        let has_storage_array = !account_storage.slots.is_empty();
                        let _ = write_bool(&mut storage_bytes, has_storage_array);
                        if has_storage_array {
                            let _ = write_i32_le(
                                &mut storage_bytes,
                                account_storage.slots.len() as i32,
                            );
                            for slot in &account_storage.slots {
                                let _ = write_bool(&mut storage_bytes, slot.is_some());
                                if let Some(item) = slot {
                                    if let Ok(bytes) = item.encode_to_bytes() {
                                        storage_bytes.extend_from_slice(&bytes);
                                    }
                                }
                            }
                        }

                        let storage_pkt = SUserStorage { storage_bytes };
                        if let Ok(raw) = storage_pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let npc_storage = SNpcStorage;
                        let raw = npc_storage.encode();
                        out.push(Self::encode_raw(raw));
                    }
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

            if let Some(page_raw) = maybe_page {
                let page = self.expand_npc_placeholders(page_raw, None);
                let resp = SNpcResponse { page };
                if let Ok(raw) = resp.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        }
    }
}
