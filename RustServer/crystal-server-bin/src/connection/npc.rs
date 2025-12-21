use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crystal_server_core::account::AccountStorage;
use crystal_server_core::world;
use crystal_server_core::world::WorldProvider;
use crystal_shared_proto::io::{write_bool, write_f32_le, write_i32_le};
use crystal_shared_proto::item::{SNewItemInfo, SUserStorage, SCraftItem};

use crystal_shared_proto::item_types::{AwakeData, ItemInfoData, StatsMap, UserItemData};
use crystal_shared_proto::login::{CCallNPC, SDisconnect};
use crystal_shared_proto::npc::{SNpcGoods, SNpcSell, SNpcStorage, SNpcRepair, SNpcsRepair, SRoll, CCraftItem, CRepairItem, CSRepairItem, CDepositRefineItem, CRetrieveRefineItem, CRefineCancel, CRefineItem, CCheckRefine, CReplaceWedRing};
use crystal_shared_proto::item::CBuyItemBack;

use crystal_shared_proto::scene::SNpcResponse;
use crystal_shared_proto::notice::{SOpenBrowser, SPlaySound, SSetTimer, SExpireTimer};
use crystal_shared_proto::user::SLoseGold;
use rand::{thread_rng, Rng};

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

        // Candidate 1: root / rel_path (as-is).
        let candidate1 = root.join(file_name);
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

    /// Handle a CraftItem request from the client. For now this only performs
    /// very basic validation and replies with SCraftItem { success: false } as
    /// a stub; full crafting logic will be wired through the World layer.
    pub(crate) fn handle_craft_item(&mut self, _msg: CCraftItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let resp = SCraftItem { success: false };
        if let Ok(raw) = resp.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    /// Scan the #ACT block for the given key for an OPENBROWSER command and
    /// return its URL parameter if present.
    pub(crate) fn extract_open_browser_url(path: &Path, key: &str) -> Option<String> {
        let text = fs::read_to_string(path).ok()?;
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
                    let key = label_inner.to_ascii_uppercase();
                    current_label = Some(key);
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

            if in_act && !trimmed.is_empty() {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 && parts[0].eq_ignore_ascii_case("OPENBROWSER") {
                    // In C#, parts[1] is stored as the Url parameter.
                    return Some(parts[1].to_string());
                }
            }

            i += 1;
        }

        None
    }

    /// Scan the #ACT block for the given key for a SETTIMER command and
    /// return its first occurrence as (key, seconds, type_id, global).
    pub(crate) fn extract_set_timer(
        path: &Path,
        key: &str,
    ) -> Option<(String, i32, u8, bool)> {
        let text = fs::read_to_string(path).ok()?;
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
                    let key = label_inner.to_ascii_uppercase();
                    current_label = Some(key);
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

            if in_act && !trimmed.is_empty() {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 4 && parts[0].eq_ignore_ascii_case("SETTIMER") {
                    let timer_key = parts[1].to_string();
                    if let (Ok(sec), Ok(ty)) = (parts[2].parse::<i32>(), parts[3].parse::<u8>()) {
                        let seconds = sec.max(0);
                        let global = if parts.len() >= 5 {
                            parts[4].eq_ignore_ascii_case("true")
                        } else {
                            false
                        };
                        return Some((timer_key, seconds, ty, global));
                    }
                }
            }

            i += 1;
        }

        None
    }

    /// Scan the #ACT block for the given key for an EXPIRETIMER command and
    /// return its first timer key parameter if present.
    pub(crate) fn extract_expire_timer(path: &Path, key: &str) -> Option<String> {
        let text = fs::read_to_string(path).ok()?;
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
                    let key = label_inner.to_ascii_uppercase();
                    current_label = Some(key);
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

            if in_act && !trimmed.is_empty() {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 && parts[0].eq_ignore_ascii_case("EXPIRETIMER") {
                    return Some(parts[1].to_string());
                }
            }

            i += 1;
        }

        None
    }

    /// Scan the #ACT block for the given key for a PLAYSOUND command and
    /// return its sound id parameter if present.
    pub(crate) fn extract_play_sound(path: &Path, key: &str) -> Option<i32> {
        let text = fs::read_to_string(path).ok()?;
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
                    let key = label_inner.to_ascii_uppercase();
                    current_label = Some(key);
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

            if in_act && !trimmed.is_empty() {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 && parts[0].eq_ignore_ascii_case("PLAYSOUND") {
                    if let Ok(id) = parts[1].parse::<i32>() {
                        return Some(id);
                    }
                }
            }

            i += 1;
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
                // New page label.
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

    /// Render a single NPC page for the given key, interpreting simple
    /// `#IF` blocks that contain only `CHECKTIMER` conditions and
    /// selecting between `#SAY` and `#ELSESAY` text accordingly. Pages
    /// that do not contain any CHECKTIMER-based conditions, or whose
    /// conditions use other CheckType variants, will return None so that
    /// the legacy `pages` map can be used instead.
    fn render_npc_page_with_if(
        &self,
        script_path: &Path,
        key: &str,
    ) -> Option<Vec<String>> {
        let text = fs::read_to_string(script_path).ok()?;
        let lines: Vec<&str> = text.lines().collect();

        let target_key = key.to_ascii_uppercase();
        let mut in_page = false;
        let mut i: usize = 0;
        let mut out_lines: Vec<String> = Vec::new();
        let mut found_if = false;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            if trimmed.starts_with("[@") {
                // New page label.
                if let Some(end) = trimmed.find(']') {
                    let label_inner = &trimmed[1..end];
                    let label = label_inner.to_ascii_uppercase();
                    if label == target_key {
                        in_page = true;
                        i += 1;
                        continue;
                    }

                    if in_page {
                        // We were inside the target page and hit the next
                        // label; stop processing.
                        break;
                    }
                }
                i += 1;
                continue;
            }

            if !in_page {
                i += 1;
                continue;
            }

            // Handle conditional segments that start with #IF.
            if trimmed.eq_ignore_ascii_case("#IF") {
                match self.process_if_segment(&lines, i) {
                    Some((segment_lines, next_i, handled_any)) => {
                        if handled_any {
                            found_if = true;
                        }
                        out_lines.extend(segment_lines);
                        i = next_i;
                        continue;
                    }
                    None => {
                        // Encountered an #IF block that we cannot safely
                        // interpret (e.g. it uses unsupported checks).
                        // Fall back to legacy behaviour for this page.
                        return None;
                    }
                }
            }

            // Unconditional #SAY blocks are rendered as-is.
            if trimmed.eq_ignore_ascii_case("#SAY") {
                i += 1;
                while i < lines.len() {
                    let l = lines[i];
                    let t = l.trim_start();
                    if t.starts_with("[@") || (t.starts_with('#') && !t.eq_ignore_ascii_case("#SAY")) {
                        break;
                    }
                    out_lines.push(l.to_string());
                    i += 1;
                }
                continue;
            }

            // Other directives (#ACT, #ELSESAY, etc.) and empty/comment
            // lines are ignored for dialog text purposes here.
            i += 1;
        }

        if found_if && !out_lines.is_empty() {
            Some(out_lines)
        } else {
            None
        }
    }

    /// Process a single `#IF` segment that may contain supported conditions
    /// (CHECKTIMER, LEVEL, CHECKITEM, etc). Returns the rendered text lines
    /// together with the index of the next line after this segment, and a
    /// flag indicating whether at least one condition was handled. If the
    /// segment contains unsupported checks, None is returned so that the
    /// caller can fall back to the legacy script handling.
    fn process_if_segment(
        &self,
        lines: &[&str],
        if_index: usize,
    ) -> Option<(Vec<String>, usize, bool)> {
        let mut i = if_index + 1;
        let mut timer_conds: Vec<(String, String, i64)> = Vec::new();
        let mut level_conds: Vec<(String, i64)> = Vec::new();
        let mut gold_conds: Vec<(String, i64)> = Vec::new();
        let mut credit_conds: Vec<(String, i64)> = Vec::new();
        let mut pk_conds: Vec<(String, i64)> = Vec::new();
        let mut gender_conds: Vec<u8> = Vec::new();
        let mut class_conds: Vec<u8> = Vec::new();
        let mut map_conds: Vec<String> = Vec::new();
        let mut range_conds: Vec<(i32, i32, i32)> = Vec::new();
        let mut day_conds: Vec<String> = Vec::new();
        let mut hour_conds: Vec<u32> = Vec::new();
        let mut minute_conds: Vec<u32> = Vec::new();
        let mut random_conds: Vec<i32> = Vec::new();
        let mut checkitem_conds: Vec<(String, i32)> = Vec::new();
        let mut checkhum_conds: Vec<(String, i32)> = Vec::new();
        let mut checkmon_conds: Vec<(String, i32)> = Vec::new();
        let mut checkexactmon_conds: Vec<(String, String, i32)> = Vec::new();

        // Collect condition lines until we hit another directive or label.
        while i < lines.len() {
            let trimmed = lines[i].trim();

            if trimmed.is_empty() || trimmed.starts_with(';') {
                i += 1;
                continue;
            }

            if trimmed.starts_with("[@") || trimmed.starts_with('#') {
                break;
            }

            let upper = trimmed.to_ascii_uppercase();
            if upper.starts_with("CHECKTIMER") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 4 {
                    return None;
                }

                // Expected layout: CHECKTIMER <op> <time_secs> <key>
                let op = parts[1].to_string();
                let time_secs = match parts[2].parse::<i64>() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                let base_key = parts[3].to_string();
                timer_conds.push((op, base_key, time_secs));
            } else if upper.starts_with("LEVEL") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 3 {
                    return None;
                }

                let op = parts[1].to_string();
                let value = match parts[2].parse::<i64>() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                level_conds.push((op, value));
            } else if upper.starts_with("CHECKGOLD") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 3 {
                    return None;
                }

                let op = parts[1].to_string();
                let value = match parts[2].parse::<i64>() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                gold_conds.push((op, value));
            } else if upper.starts_with("CHECKCREDIT") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 3 {
                    return None;
                }

                let op = parts[1].to_string();
                let value = match parts[2].parse::<i64>() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                credit_conds.push((op, value));
            } else if upper.starts_with("CHECKPKPOINT") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 3 {
                    return None;
                }

                let op = parts[1].to_string();
                let value = match parts[2].parse::<i64>() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                pk_conds.push((op, value));
            } else if upper.starts_with("CHECKGENDER") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 2 {
                    return None;
                }

                let val = parts[1];
                let expected = if val.eq_ignore_ascii_case("MALE") {
                    0u8
                } else if val.eq_ignore_ascii_case("FEMALE") {
                    1u8
                } else {
                    return None;
                };
                gender_conds.push(expected);
            } else if upper.starts_with("CHECKCLASS") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 2 {
                    return None;
                }

                let val = parts[1];
                let expected = if val.eq_ignore_ascii_case("WARRIOR") {
                    0u8
                } else if val.eq_ignore_ascii_case("WIZARD") {
                    1u8
                } else if val.eq_ignore_ascii_case("TAOIST") {
                    2u8
                } else if val.eq_ignore_ascii_case("ASSASSIN") {
                    3u8
                } else if val.eq_ignore_ascii_case("ARCHER") {
                    4u8
                } else {
                    return None;
                };
                class_conds.push(expected);
            } else if upper.starts_with("CHECKMAP") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 2 {
                    return None;
                }

                map_conds.push(parts[1].to_string());
            } else if upper.starts_with("CHECKRANGE") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 4 {
                    return None;
                }

                let x = match parts[1].parse::<i32>() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                let y = match parts[2].parse::<i32>() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                let range = match parts[3].parse::<i32>() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                range_conds.push((x, y, range));
            } else if upper.starts_with("DAYOFWEEK") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 2 {
                    return None;
                }

                day_conds.push(parts[1].to_ascii_uppercase());
            } else if upper.starts_with("HOUR") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 2 {
                    return None;
                }

                let hour = match parts[1].parse::<u32>() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                hour_conds.push(hour);
            } else if upper.starts_with("MIN") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 2 {
                    return None;
                }

                let minute = match parts[1].parse::<u32>() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                minute_conds.push(minute);
            } else if upper.starts_with("RANDOM") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 2 {
                    return None;
                }

                let val = match parts[1].parse::<i32>() {
                    Ok(v) if v > 0 => v,
                    _ => return None,
                };
                random_conds.push(val);
            } else if upper.starts_with("CHECKITEM") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 2 {
                    return None;
                }
                let item_name = parts[1].to_string();
                let count = if parts.len() >= 3 {
                    parts[2].parse::<i32>().unwrap_or(1)
                } else {
                    1
                };
                checkitem_conds.push((item_name, count));
            } else if upper.starts_with("CHECKHUM") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 3 {
                    // We only support CHECKHUM <Map> <Count> for now.
                    // C# also supports <Map> <Range> <Count>, which we treat as unsupported.
                    return None;
                }
                let map_name = parts[1].to_string();
                let count = match parts[2].parse::<i32>() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                checkhum_conds.push((map_name, count));
            } else if upper.starts_with("CHECKMON") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 3 {
                    return None;
                }
                let map_name = parts[1].to_string();
                let count = match parts[2].parse::<i32>() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                checkmon_conds.push((map_name, count));
            } else if upper.starts_with("CHECKEXACTMON") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 4 {
                    return None;
                }
                let map_name = parts[1].to_string();
                let mon_name = parts[2].to_string();
                let count = match parts[3].parse::<i32>() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                checkexactmon_conds.push((map_name, mon_name, count));
            } else if upper.starts_with("CHECK") || upper.starts_with("LEVEL") {
                // Other CHECK* or LEVEL-based conditions are not yet
                // supported in this minimal renderer.
                return None;
            } else {
                // Unknown condition syntax – bail out to avoid breaking
                // complex scripts.
                return None;
            }

            i += 1;
        }

        // Collect #SAY / #ELSESAY text for this segment.
        let mut say_lines: Vec<String> = Vec::new();
        let mut else_lines: Vec<String> = Vec::new();
        let mut block: u8 = 0; // 0 = none, 1 = say, 2 = else-say

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            if trimmed.starts_with("[@") || trimmed.eq_ignore_ascii_case("#IF") {
                break;
            }

            if trimmed.eq_ignore_ascii_case("#SAY") {
                block = 1;
                i += 1;
                continue;
            }

            if trimmed.eq_ignore_ascii_case("#ELSESAY") {
                block = 2;
                i += 1;
                continue;
            }

            if trimmed.starts_with('#') {
                // #ACT / #ELSEACT etc. mark the end of this segment from a
                // dialog-text perspective.
                break;
            }

            match block {
                1 => say_lines.push(line.to_string()),
                2 => else_lines.push(line.to_string()),
                _ => {}
            }

            i += 1;
        }

        // TODO: properly evaluate all collected conditions. For now we only
        // implement a subset (e.g. PK points) that are required for basic
        // NPC behaviour.
        let mut pass = true;

        // CHECKPKPOINT <op> <value> (e.g. CHECKPKPOINT > 200)
        if !pk_conds.is_empty() {
            let world = self.world.lock().unwrap();
            let pk_points = world.player_pk_points(self.session_id).unwrap_or(0) as i64;
            drop(world);

            for (op, value) in &pk_conds {
                let ok = match op.as_str() {
                    ">" => pk_points > *value,
                    ">=" => pk_points >= *value,
                    "<" => pk_points < *value,
                    "<=" => pk_points <= *value,
                    "==" | "=" => pk_points == *value,
                    "!=" | "<>" => pk_points != *value,
                    _ => {
                        // Unsupported operator: fall back to legacy behaviour
                        return None;
                    }
                };
                if !ok {
                    pass = false;
                    break;
                }
            }
        }

        let mut out: Vec<String> = Vec::new();
        if pass {
            out.extend(say_lines);
        } else if !else_lines.is_empty() {
            out.extend(else_lines);
        }

        let handled_any = !timer_conds.is_empty()
            || !level_conds.is_empty()
            || !gold_conds.is_empty()
            || !credit_conds.is_empty()
            || !pk_conds.is_empty()
            || !gender_conds.is_empty()
            || !class_conds.is_empty()
            || !map_conds.is_empty()
            || !range_conds.is_empty()
            || !day_conds.is_empty()
            || !hour_conds.is_empty()
            || !minute_conds.is_empty()
            || !random_conds.is_empty()
            || !checkitem_conds.is_empty()
            || !checkhum_conds.is_empty()
            || !checkmon_conds.is_empty()
            || !checkexactmon_conds.is_empty();

        Some((out, i, handled_any))
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
        let need_guild_extend_fee =
            lines.iter().any(|l| l.contains("<$GUILDEXTENDFEE>"));
        let need_guild_rent_fee =
            lines.iter().any(|l| l.contains("<$GUILDRENTFEE>"));

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
        let mut guild_extend_fee_str: Option<String> = None;
        let mut guild_rent_fee_str: Option<String> = None;

        if need_guild_war_time || need_guild_war_fee {
            let cfg = world::configs::guild_settings();
            if need_guild_war_time {
                guild_war_time_str = Some(cfg.war_time.to_string());
            }
            if need_guild_war_fee {
                guild_war_fee_str = Some(cfg.war_cost.to_string());
            }
        }

        // Global GT rent fee does not depend on the player's guild state.
        if need_guild_rent_fee {
            let scfg = crate::world::configs::setup_config::setup_config();
            guild_rent_fee_str = Some(scfg.game.buy_gt_gold.to_string());
        }

        if need_guild_name
            || need_agit_guild_name
            || need_guild_rental_days_left
            || need_guild_extend_fee
        {
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

            if need_guild_extend_fee {
                if player_guild.is_empty() {
                    guild_extend_fee_str = Some("None".to_string());
                } else if let Some(guild) = world.get_guild_info_by_name(&player_guild) {
                    let now_ticks = Self::unix_ms_to_dotnet_binary(Self::now_millis());
                    if guild.has_gt(now_ticks) {
                        // Convert stored .NET ticks into a human-readable local
                        // date/time string, approximating C# GTRent.ToString().
                        let rent_ms = Self::dotnet_binary_to_unix_ms(guild.gt_rent_ticks);
                        let expire_str = if rent_ms <= 0 {
                            "Never".to_string()
                        } else if let Some(dt_utc) =
                            chrono::DateTime::<chrono::Utc>::from_timestamp_millis(rent_ms)
                        {
                            let dt_local: chrono::DateTime<chrono::Local> =
                                dt_utc.with_timezone(&chrono::Local);
                            dt_local.format("%Y-%m-%d %H:%M:%S").to_string()
                        } else {
                            rent_ms.to_string()
                        };

                        let scfg = crate::world::configs::setup_config::setup_config();
                        guild_extend_fee_str = Some(format!(
                            "Expire On: {} ,Extend fee: {}",
                            expire_str, scfg.game.extend_gt_gold,
                        ));
                    } else {
                        guild_extend_fee_str = Some("None".to_string());
                    }
                } else {
                    guild_extend_fee_str = Some("None".to_string());
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

            if let Some(ref v) = guild_extend_fee_str {
                if line.contains("<$GUILDEXTENDFEE>") {
                    line = line.replace("<$GUILDEXTENDFEE>", v);
                }
            }

            if let Some(ref v) = guild_rent_fee_str {
                if line.contains("<$GUILDRENTFEE>") {
                    line = line.replace("<$GUILDRENTFEE>", v);
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

    pub(crate) fn extract_roll_action(path: &Path, key: &str) -> Option<(i32, String, bool)> {
        let text = fs::read_to_string(path).ok()?;
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

            if in_act && !trimmed.is_empty() {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3
                    && (parts[0].eq_ignore_ascii_case("ROLLDIE")
                        || parts[0].eq_ignore_ascii_case("ROLLYUT"))
                {
                    let roll_type = if parts[0].eq_ignore_ascii_case("ROLLDIE") {
                        0
                    } else {
                        1
                    };
                    let page = parts[1].to_string();
                    let auto_roll = parts[2].eq_ignore_ascii_case("true");
                    return Some((roll_type, page, auto_roll));
                }
            }

            i += 1;
        }

        None
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

        let npc_index_opt = super::npc_index_from_object_id(msg.object_id);

        if let Some(npc) = self
            .world_db
            .npc_infos
            .iter()
            .find(|n| {
                let matches_id = if let Some(npc_index) = npc_index_opt {
                    n.index == npc_index
                } else {
                    n.index as u32 == msg.object_id
                };
                matches_id && n.map_index == self.current_map_index
            })
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

                if key_upper == "@MANAGEHERO" {
                    self.send_manage_heroes(out);
                    return;
                }

                if key_upper == "@CREATEHERO" {
                    self.send_hero_create_request(out);
                    return;
                }

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
                                tracing::debug!(
                                    "CallNPC shop: loaded {} trade specs from {:?}",
                                    specs.len(),
                                    script_path,
                                );
                                let base_uid = (npc.index as u64) << 32;
                                for (idx, (name, count)) in specs.iter().enumerate() {
                                    if let Some(info) = self
                                        .world_db
                                        .item_infos
                                        .iter()
                                        .find(|i| i.name.eq_ignore_ascii_case(name))
                                    {
                                        tracing::debug!(
                                            "CallNPC shop: spec {} name='{}' -> item_index={} count={}",
                                            idx,
                                            name,
                                            info.index,
                                            count,
                                        );

                                        let unique_id = base_uid + idx as u64 + 1;
                                        let item = Self::make_shop_user_item(
                                            info,
                                            unique_id,
                                            *count,
                                        );
                                        goods_items.push(item);
                                    } else {
                                        tracing::warn!(
                                            "CallNPC shop: trade spec '{}' (idx={}) has no matching ItemInfo; npc_index={}",
                                            name,
                                            idx,
                                            npc.index,
                                        );
                                    }
                                }
                                tracing::debug!(
                                    "CallNPC shop: built {} goods items (from {} specs) for npc_index={}",
                                    goods_items.len(),
                                    specs.len(),
                                    npc.index,
                                );
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
                    // Populate from NPC buy-back history
                    let goods_items = {
                        let world = self.world.lock().unwrap();
                        world.buyback_items_for(
                            self.session_id,
                            self.current_map_index,
                            npc.index,
                        )
                    };
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
                                                    self.known_players.clear();
                                                    self.known_heroes.clear();
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
                                                    self.known_players.clear();
                                                    self.known_heroes.clear();
                                                    self.update_visibility(out);
                                                }

                                        return;
                                    }
                                }
                            }

                            if let Some((roll_type, roll_page, auto_roll)) =
                                Self::extract_roll_action(&script_path, &key)
                            {
                                // Mirror C# Envir.Random.Next(1, 7): sample an
                                // integer in the inclusive range [1, 6].
                                let mut rng = thread_rng();
                                let result: i32 = rng.gen_range(1..=6);

                                {
                                    let mut world = self.world.lock().unwrap();
                                    world.set_player_npc_data(
                                        self.session_id,
                                        "NPCRollResult",
                                        result.to_string(),
                                    );
                                }

                                let pkt = SRoll {
                                    roll_type,
                                    page: roll_page,
                                    result,
                                    auto_roll,
                                };
                                if let Ok(raw) = pkt.encode() {
                                    out.push(Self::encode_raw(raw));
                                }
                            }

                            if let Some(url) =
                                Self::extract_open_browser_url(&script_path, &key)
                            {
                                let pkt = SOpenBrowser { url };
                                if let Ok(raw) = pkt.encode() {
                                    out.push(Self::encode_raw(raw));
                                }
                            }

                            if let Some(sound_id) =
                                Self::extract_play_sound(&script_path, &key)
                            {
                                let pkt = SPlaySound { sound: sound_id };
                                if let Ok(raw) = pkt.encode() {
                                    out.push(Self::encode_raw(raw));
                                }
                            }

                            // Global and per-player timers, mirroring the C#
                            // ActionType.SetTimer/ExpireTimer behaviour.
                            if let Some((timer_key, seconds, type_id, global)) =
                                Self::extract_set_timer(&script_path, &key)
                            {
                                let mut world = self.world.lock().unwrap();
                                if global {
                                    // Global timer: key is "_-" + script key,
                                    // only stored server-side and used by
                                    // NPC CheckTimer; it is not sent to the
                                    // client TimerDialog.
                                    let full_key = format!("_-{}", timer_key);
                                    world.set_timer(full_key, seconds, type_id);
                                } else if let Some(name) =
                                    world.player_name(self.session_id)
                                {
                                    // Per-player timer: key is
                                    // "<PlayerName>-<ScriptKey>", stored in
                                    // the shared timer map and also sent to
                                    // the client so TimerDialog can track it.
                                    let full_key = format!("{}-{}", name, timer_key);
                                    world.set_timer(full_key.clone(), seconds, type_id);
                                    drop(world);

                                    let pkt = SSetTimer {
                                        key: full_key,
                                        type_id,
                                        seconds,
                                    };
                                    if let Ok(raw) = pkt.encode() {
                                        out.push(Self::encode_raw(raw));
                                    }
                                }
                            }

                            if let Some(expire_base_key) =
                                Self::extract_expire_timer(&script_path, &key)
                            {
                                let mut world = self.world.lock().unwrap();

                                // Global timer key uses the "_-" prefix.
                                let global_key = format!("_-{}", expire_base_key);
                                world.remove_timer(&global_key);

                                if let Some(name) = world.player_name(self.session_id) {
                                    // Per-player timer key uses the
                                    // "<PlayerName>-<ScriptKey>" format.
                                    let full_key =
                                        format!("{}-{}", name, expire_base_key);
                                    world.remove_timer(&full_key);
                                    drop(world);

                                    let pkt = SExpireTimer { key: full_key };
                                    if let Ok(raw) = pkt.encode() {
                                        out.push(Self::encode_raw(raw));
                                    }
                                }
                            }

                            // Special-case the storage page: when the player
                            // clicks an [@STORAGE] entry, mirror the C# behaviour
                            // of sending the current account Storage contents
                            // followed by an NPCStorage packet to open the
                            // warehouse dialog.
                            if key_upper == "@STORAGE" {
                                // Remember which NPC index opened the storage page so
                                // that subsequent StoreItem/TakeBackItem requests can be
                                // validated against NPC proximity, similar to C#
                                // PlayerObject.NPCPage/NPCObjectID.
                                self.current_storage_npc_id = Some(npc.index as u32);

                                if let Some(ref account_id) = self.account_id {
                                    let account_storage =
                                        match self.store.load_account_storage(account_id) {
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
                                                    storage_bytes
                                                        .extend_from_slice(&bytes);
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

                            // After executing any actions (teleport, timers, storage, shop, etc.),
                            // choose which dialog page to send. If a fail label for paid teleport
                            // has already populated maybe_page, honour that first; otherwise, try
                            // the IF-aware renderer and fall back to the simple pages map.
                            if maybe_page.is_none() {
                                if let Some(page_if) = self.render_npc_page_with_if(&script_path, &key) {
                                    maybe_page = Some(page_if);
                                } else if key.eq_ignore_ascii_case("@MAIN") {
                                    if let Some(page_alt) = pages.get("@MAIN-1") {
                                        maybe_page = Some(page_alt.clone());
                                    } else if let Some(page_main) = pages.get("@MAIN") {
                                        maybe_page = Some(page_main.clone());
                                    }
                                } else if let Some(page) = pages.get(&key) {
                                    maybe_page = Some(page.clone());
                                }
                            }

                            if let Some(page_raw) = maybe_page {
                                let page = self.expand_npc_placeholders(page_raw, None);
                                let resp = SNpcResponse { page };
                                if let Ok(raw) = resp.encode() {
                                    out.push(Self::encode_raw(raw));
                                }
                            }

                            // Now send shop-related packets after the dialog response so that the
                            // client has the NPC dialog open (NPCDialog.Visible == true) when
                            // handling NPCGoods, matching the C# server behaviour.
                            if let Some((goods_items, panel_type)) = shop_goods {
                                // Ensure the client has ItemInfo definitions for all goods before
                                // sending the NPCGoods list, similar to the C# server's CheckItem
                                // behaviour.
                                for item in &goods_items {
                                    if let Some(info) = self
                                        .world_db
                                        .item_infos
                                        .iter()
                                        .find(|i| i.index == item.item_index)
                                    {
                                        if let Ok(pkt) = SNewItemInfo::from_item_info(info) {
                                            if let Ok(raw) = pkt.encode() {
                                                out.push(Self::encode_raw(raw));
                                            }
                                        }
                                    } else {
                                        tracing::warn!(
                                            "CallNPC shop: goods item_index={} has no ItemInfoData when sending NewItemInfo; npc_index={}",
                                            item.item_index,
                                            npc.index,
                                        );
                                    }
                                }

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
            }
        }
    }

    fn handle_default_npc_call(&mut self, raw_key: String, out: &mut Vec<Vec<u8>>) {
        let key = Self::normalize_npc_key(&raw_key);

        if key.eq_ignore_ascii_case("@MANAGEHERO") {
            self.send_manage_heroes(out);
            return;
        }

        if key.eq_ignore_ascii_case("@CREATEHERO") {
            self.send_hero_create_request(out);
            return;
        }

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
                        self.known_players.clear();
                        self.known_heroes.clear();
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
            } else if let Some(page) = pages.get(&key) {
                maybe_page = Some(page.clone());
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

    pub(crate) fn handle_repair_item(&mut self, msg: CRepairItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Send SRepairItem response immediately (mirroring C# behavior)
        use crystal_shared_proto::item::SRepairItem;
        let pkt = SRepairItem {
            unique_id: msg.unique_id,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Check if player is dead - use player_current_hp_mp to check if player exists
        // If player doesn't exist or HP is 0, consider them dead
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

        // Find the item in inventory
        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        let item_index = match inv.slots.iter().position(|s| {
            s.as_ref().map(|i| i.unique_id) == Some(msg.unique_id)
        }) {
            Some(idx) => idx,
            None => return,
        };

        let (_item_index_val, max_dura, current_dura, cost) = {
            let item = match inv.slots[item_index].as_ref() {
                Some(it) => it,
                None => return,
            };

            // Get item info to check bind flags
            let info = match WorldProvider::get_item_info(&*self.world_db, item.item_index) {
                Some(i) => i,
                None => return,
            };

            // Check bind flags: DontRepair (0x0010)
            const BIND_DONT_REPAIR: i16 = 0x0010;
            if (info.bind & BIND_DONT_REPAIR) != 0 {
                self.send_system_chat("无法修理此物品。", out);
                return;
            }

            // Calculate repair cost
            // RepairPrice formula: Based on item price and durability loss
            // The cost is proportional to the percentage of durability lost
            let durability_lost = item.max_dura.saturating_sub(item.current_dura);
            if durability_lost == 0 {
                // Item is already at full durability
                return;
            }

            // Calculate repair cost
            // Formula: (durability_lost / max_dura) * (item_price / repair_factor)
            // This matches common MMO repair cost calculations
            let max_dura_f32 = item.max_dura as f32;
            let durability_lost_f32 = durability_lost as f32;
            let item_price_f32 = info.price as f32;
            
            // Repair factor: typically items cost 1-5% of their price per full repair
            // We use a factor of 50, meaning full repair costs 2% of item price
            let repair_factor: f32 = 50.0;
            
            // Calculate cost based on percentage of durability lost
            let durability_percent = durability_lost_f32 / max_dura_f32.max(1.0);
            let cost = (durability_percent * item_price_f32 / repair_factor) as u32;
            
            // Ensure minimum cost of 1 gold
            let cost = cost.max(1);

            // Check if player has enough gold
            let stats = match self.current_stats.clone() {
                Some(s) => s,
                None => return,
            };

            if stats.gold < cost as i64 {
                self.send_system_chat("金币不足，无法修理。", out);
                return;
            }

            // Deduct gold
            let mut new_stats = stats.clone();
            new_stats.gold = new_stats.gold.saturating_sub(cost as i64);

            if let (Some(ref account_id), Some(char_idx)) =
                (self.account_id.as_ref(), self.current_char_index)
            {
                let _ = self
                    .store
                    .save_character_stats(account_id, char_idx, &new_stats);
            }

            self.current_stats = Some(new_stats);

            // Repair item: restore current durability to max
            // For normal repair, reduce max durability slightly (C# behavior)
            let new_max_dura = item.max_dura.saturating_sub((durability_lost / 30).max(1));
            let new_current_dura = new_max_dura;

            (item.item_index, new_max_dura, new_current_dura, cost)
        };

        // Update item in inventory
        if let Some(item) = inv.slots[item_index].as_mut() {
            item.max_dura = max_dura;
            item.current_dura = current_dura;
        }

        // Update inventory in world
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
        }

        // Send ItemRepaired packet
        use crystal_shared_proto::item::SItemRepaired;
        let repaired = SItemRepaired {
            unique_id: msg.unique_id,
            max_dura,
            current_dura,
        };
        if let Ok(raw) = repaired.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Send SLoseGold
        use crystal_shared_proto::user::status::SLoseGold;
        let lose_gold = SLoseGold { gold: cost };
        if let Ok(raw) = lose_gold.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Refresh inventory
        let refresh = crystal_shared_proto::user::SUserSlotsRefresh {
            inventory: inv.slots,
            equipment: eq.slots,
        };
        if let Ok(raw) = refresh.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_srepair_item(&mut self, msg: CSRepairItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Send SRepairItem response immediately (mirroring C# behavior)
        use crystal_shared_proto::item::SRepairItem;
        let pkt = SRepairItem {
            unique_id: msg.unique_id,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Check if player is dead - use player_current_hp_mp to check if player exists
        // If player doesn't exist or HP is 0, consider them dead
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

        // Find the item in inventory
        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        let item_index = match inv.slots.iter().position(|s| {
            s.as_ref().map(|i| i.unique_id) == Some(msg.unique_id)
        }) {
            Some(idx) => idx,
            None => return,
        };

        let (max_dura, current_dura, cost) = {
            let item = match inv.slots[item_index].as_ref() {
                Some(it) => it,
                None => return,
            };

            // Get item info to check bind flags
            let info = match WorldProvider::get_item_info(&*self.world_db, item.item_index) {
                Some(i) => i,
                None => return,
            };

            // Check bind flags: NoSRepair (0x0040) for special repair
            const BIND_NO_SREPAIR: i16 = 0x0040;
            if (info.bind & BIND_NO_SREPAIR) != 0 {
                self.send_system_chat("无法进行特殊修理此物品。", out);
                return;
            }

            // Calculate special repair cost (3x normal repair cost)
            let durability_lost = item.max_dura.saturating_sub(item.current_dura);
            if durability_lost == 0 {
                // Item is already at full durability
                return;
            }

            // Calculate repair cost using same formula as RepairItem, but 3x for special repair
            // Formula: (durability_lost / max_dura) * (item_price / repair_factor) * 3
            let max_dura_f32 = item.max_dura as f32;
            let durability_lost_f32 = durability_lost as f32;
            let item_price_f32 = info.price as f32;
            
            // Repair factor: typically items cost 1-5% of their price per full repair
            // We use a factor of 50, meaning full repair costs 2% of item price
            let repair_factor: f32 = 50.0;
            
            // Calculate cost based on percentage of durability lost
            let durability_percent = durability_lost_f32 / max_dura_f32.max(1.0);
            let base_cost = (durability_percent * item_price_f32 / repair_factor) as u32;
            
            // Special repair is 3x normal repair cost
            let cost = base_cost * 3;
            
            // Ensure minimum cost of 1 gold
            let cost = cost.max(1);

            // Check if player has enough gold
            let stats = match self.current_stats.clone() {
                Some(s) => s,
                None => return,
            };

            if stats.gold < cost as i64 {
                self.send_system_chat("金币不足，无法进行特殊修理。", out);
                return;
            }

            // Deduct gold
            let mut new_stats = stats.clone();
            new_stats.gold = new_stats.gold.saturating_sub(cost as i64);

            if let (Some(ref account_id), Some(char_idx)) =
                (self.account_id.as_ref(), self.current_char_index)
            {
                let _ = self
                    .store
                    .save_character_stats(account_id, char_idx, &new_stats);
            }

            self.current_stats = Some(new_stats);

            // Special repair: restore current durability to max WITHOUT reducing max durability
            (item.max_dura, item.max_dura, cost)
        };

        // Update item in inventory
        if let Some(item) = inv.slots[item_index].as_mut() {
            item.current_dura = current_dura;
        }

        // Update inventory in world
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
        }

        // Send ItemRepaired packet
        use crystal_shared_proto::item::SItemRepaired;
        let repaired = SItemRepaired {
            unique_id: msg.unique_id,
            max_dura,
            current_dura,
        };
        if let Ok(raw) = repaired.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Send SLoseGold
        use crystal_shared_proto::user::status::SLoseGold;
        let lose_gold = SLoseGold { gold: cost };
        if let Ok(raw) = lose_gold.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Refresh inventory
        let refresh = crystal_shared_proto::user::SUserSlotsRefresh {
            inventory: inv.slots,
            equipment: eq.slots,
        };
        if let Ok(raw) = refresh.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_buy_item_back(&mut self, msg: CBuyItemBack, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        if msg.count == 0 {
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

        // Find nearby NPC
        let npc = match self
            .world_db
            .npc_infos
            .iter()
            .find(|n| {
                n.map_index == self.current_map_index
                    && (n.location_x - self.current_x).abs() <= Self::DATA_RANGE
                    && (n.location_y - self.current_y).abs() <= Self::DATA_RANGE
            }) {
            Some(n) => n,
            None => return,
        };

        // Get buyback items for this player and NPC
        let buyback_item = {
            let world = self.world.lock().unwrap();
            world
                .buyback_items_for(self.session_id, self.current_map_index, npc.index)
                .into_iter()
                .find(|item| item.unique_id == msg.unique_id)
        };

        let buyback_item = match buyback_item {
            Some(item) => item,
            None => {
                self.send_system_chat("回购列表中未找到该物品。", out);
                return;
            }
        };

        // Validate count
        if msg.count as u32 > buyback_item.count as u32 {
            self.send_system_chat("回购数量不能超过物品数量。", out);
            return;
        }

        // Get item info for price calculation
        let info = match self
            .world_db
            .item_infos
            .iter()
            .find(|i| i.index == buyback_item.item_index)
        {
            Some(i) => i,
            None => return,
        };

        // Calculate price: use the item's price, apply NPC rate
        let base_price = match info.price.checked_mul(msg.count as u32) {
            Some(v) => v,
            None => return,
        };

        let rate = (npc.rate as f32) / 100.0;
        let mut cost = ((base_price as f32) * rate).floor() as u32;
        if cost == 0 && base_price > 0 {
            cost = 1;
        }

        // Check if player has enough gold
        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => return,
        };

        if stats.gold < cost as i64 {
            self.send_system_chat("金币不足，无法回购。", out);
            return;
        }

        // Create item to add to inventory
        let mut item_to_add = buyback_item.clone();
        item_to_add.count = msg.count;

        // Check if player can gain item
        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        // Try to add item to inventory (simplified - should use proper CanGainItem logic)
        let mut added = false;
        for slot in inv.slots.iter_mut() {
            if slot.is_none() {
                *slot = Some(item_to_add.clone());
                added = true;
                break;
            }
        }

        if !added {
            self.send_system_chat("背包已满，无法回购。", out);
            return;
        }

        // Deduct gold
        let mut new_stats = stats.clone();
        new_stats.gold = new_stats.gold.saturating_sub(cost as i64);

        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let _ = self
                .store
                .save_character_stats(account_id, char_idx, &new_stats);
        }

        self.current_stats = Some(new_stats);

        // Remove item from buyback list (entire stack, not just the purchased count)
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
            world.remove_buyback_item(
                self.session_id,
                self.current_map_index,
                npc.index,
                msg.unique_id,
            );
        }

        // Send SLoseGold
        use crystal_shared_proto::user::status::SLoseGold;
        let lose_gold = SLoseGold { gold: cost };
        if let Ok(raw) = lose_gold.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Refresh inventory
        let refresh = crystal_shared_proto::user::SUserSlotsRefresh {
            inventory: inv.slots,
            equipment: eq.slots,
        };
        if let Ok(raw) = refresh.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Update BuyBack panel with remaining items
        let remaining_items = {
            let world = self.world.lock().unwrap();
            world.buyback_items_for(self.session_id, self.current_map_index, npc.index)
        };

        // Build goods_bytes for SNpcGoods packet
        if let Ok(bytes) = Self::build_npc_goods_bytes(&remaining_items, (npc.rate as f32) / 100.0, 0, false) {
            // PanelType.Buy = 0
            use crystal_shared_proto::npc::SNpcGoods;
            let goods_pkt = SNpcGoods { goods_bytes: bytes };
            let raw = goods_pkt.encode();
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_deposit_refine_item(&mut self, msg: CDepositRefineItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Check if NPC page is @REFINE (need to track current NPC page)
        // For now, we'll skip this check and allow the operation if other conditions are met

        // Find nearby NPC
        let npc = match self
            .world_db
            .npc_infos
            .iter()
            .find(|n| {
                n.map_index == self.current_map_index
                    && (n.location_x - self.current_x).abs() <= Self::DATA_RANGE
                    && (n.location_y - self.current_y).abs() <= Self::DATA_RANGE
            }) {
            Some(n) => n,
            None => {
                use crystal_shared_proto::item::SDepositRefineItem;
                let pkt = SDepositRefineItem {
                    from: msg.from,
                    to: msg.to,
                    success: false,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        // Validate indices
        if msg.from < 0 || msg.to < 0 {
            use crystal_shared_proto::item::SDepositRefineItem;
            let pkt = SDepositRefineItem {
                from: msg.from,
                to: msg.to,
                success: false,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        let from_index = msg.from as usize;
        let to_index = msg.to as usize;

        // Get player inventory, equipment, and refine slots
        let (mut inv, eq, mut refine_slots) = {
            let world = self.world.lock().unwrap();
            let (inv, eq) = world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ));
            let refine = world
                .player_refine_slots(self.session_id)
                .unwrap_or_else(|| vec![None; 16]);
            (inv, eq, refine)
        };

        // Validate indices
        if from_index >= inv.slots.len() || to_index >= refine_slots.len() {
            use crystal_shared_proto::item::SDepositRefineItem;
            let pkt = SDepositRefineItem {
                from: msg.from,
                to: msg.to,
                success: false,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Get item from inventory
        let item = match inv.slots[from_index].take() {
            Some(item) => item,
            None => {
                use crystal_shared_proto::item::SDepositRefineItem;
                let pkt = SDepositRefineItem {
                    from: msg.from,
                    to: msg.to,
                    success: false,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        // Check if target refine slot is empty
        if refine_slots[to_index].is_some() {
            // Put item back
            inv.slots[from_index] = Some(item);
            use crystal_shared_proto::item::SDepositRefineItem;
            let pkt = SDepositRefineItem {
                from: msg.from,
                to: msg.to,
                success: false,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Move item to refine slot
        refine_slots[to_index] = Some(item);

        // Update world
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
            world.set_player_refine_slots(self.session_id, refine_slots);
        }

        // Send success response
        use crystal_shared_proto::item::SDepositRefineItem;
        let pkt = SDepositRefineItem {
            from: msg.from,
            to: msg.to,
            success: true,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Send inventory refresh
        let refresh = crystal_shared_proto::user::SUserSlotsRefresh {
            inventory: inv.slots,
            equipment: vec![], // Equipment unchanged
        };
        if let Ok(raw) = refresh.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_retrieve_refine_item(&mut self, msg: CRetrieveRefineItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Validate indices
        if msg.from < 0 || msg.to < 0 {
            use crystal_shared_proto::item::SRetrieveRefineItem;
            let pkt = SRetrieveRefineItem {
                from: msg.from,
                to: msg.to,
                success: false,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        let from_index = msg.from as usize;
        let to_index = msg.to as usize;

        // Get player inventory, equipment, and refine slots
        let (mut inv, eq, mut refine_slots) = {
            let world = self.world.lock().unwrap();
            let (inv, eq) = world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ));
            let refine = world
                .player_refine_slots(self.session_id)
                .unwrap_or_else(|| vec![None; 16]);
            (inv, eq, refine)
        };

        // Validate indices
        if from_index >= refine_slots.len() || to_index >= inv.slots.len() {
            use crystal_shared_proto::item::SRetrieveRefineItem;
            let pkt = SRetrieveRefineItem {
                from: msg.from,
                to: msg.to,
                success: false,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Get item from refine slot
        let item = match refine_slots[from_index].take() {
            Some(item) => item,
            None => {
                use crystal_shared_proto::item::SRetrieveRefineItem;
                let pkt = SRetrieveRefineItem {
                    from: msg.from,
                    to: msg.to,
                    success: false,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        // Check if target inventory slot is empty
        if inv.slots[to_index].is_some() {
            // Put item back
            refine_slots[from_index] = Some(item);
            use crystal_shared_proto::item::SRetrieveRefineItem;
            let pkt = SRetrieveRefineItem {
                from: msg.from,
                to: msg.to,
                success: false,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Move item to inventory
        inv.slots[to_index] = Some(item);

        // Update world
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
            world.set_player_refine_slots(self.session_id, refine_slots);
        }

        // Send success response
        use crystal_shared_proto::item::SRetrieveRefineItem;
        let pkt = SRetrieveRefineItem {
            from: msg.from,
            to: msg.to,
            success: true,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Send inventory refresh
        let refresh = crystal_shared_proto::user::SUserSlotsRefresh {
            inventory: inv.slots,
            equipment: vec![], // Equipment unchanged
        };
        if let Ok(raw) = refresh.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_refine_cancel(&mut self, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Get player refine slots and inventory
        let (mut inv, eq, mut refine_slots) = {
            let world = self.world.lock().unwrap();
            let (inv, eq) = world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ));
            let refine = world
                .player_refine_slots(self.session_id)
                .unwrap_or_else(|| vec![None; 16]);
            (inv, eq, refine)
        };

        // Move all items from refine slots back to inventory
        let mut items_moved = false;
        for refine_index in 0..refine_slots.len() {
            if let Some(item) = refine_slots[refine_index].take() {
                // Find empty slot in inventory
                if let Some(inv_index) = inv.slots.iter().position(|s| s.is_none()) {
                    inv.slots[inv_index] = Some(item);
                    items_moved = true;
                } else {
                    // No space in inventory - put item back
                    refine_slots[refine_index] = Some(item);
                }
            }
        }

        // Update world if items were moved
        if items_moved {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
            world.set_player_refine_slots(self.session_id, refine_slots);

            // Send inventory refresh
            let refresh = crystal_shared_proto::user::SUserSlotsRefresh {
                inventory: inv.slots,
                equipment: vec![], // Equipment unchanged
            };
            if let Ok(raw) = refresh.encode() {
                out.push(Self::encode_raw(raw));
            }
        }
    }

    pub(crate) fn handle_refine_item(&mut self, msg: CRefineItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Send SRefineItem response immediately (mirroring C# behavior)
        use crystal_shared_proto::item::SRefineItem;
        let pkt = SRefineItem {
            unique_id: msg.unique_id,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
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

        // TODO: Check if NPC page is @REFINE (need to track current NPC page)
        // For now, we'll skip this check

        // Find nearby NPC
        let npc = match self
            .world_db
            .npc_infos
            .iter()
            .find(|n| {
                n.map_index == self.current_map_index
                    && (n.location_x - self.current_x).abs() <= Self::DATA_RANGE
                    && (n.location_y - self.current_y).abs() <= Self::DATA_RANGE
            }) {
            Some(n) => n,
            None => {
                self.send_system_chat("附近没有NPC。", out);
                return;
            }
        };

        // Get player inventory and refine slots
        let (mut inv, eq, mut refine_slots) = {
            let world = self.world.lock().unwrap();
            let (inv, eq) = world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ));
            let refine = world
                .player_refine_slots(self.session_id)
                .unwrap_or_else(|| vec![None; 16]);
            (inv, eq, refine)
        };

        // Find item in inventory
        let item_index = match inv.slots.iter().position(|s| {
            s.as_ref().map(|i| i.unique_id == msg.unique_id).unwrap_or(false)
        }) {
            Some(idx) => idx,
            None => {
                self.send_system_chat("背包中未找到该物品。", out);
                return;
            }
        };

        let mut item = match inv.slots[item_index].take() {
            Some(item) => item,
            None => return,
        };

        // Check if item is already being refined (RefineAdded != 0)
        if item.refine_added != 0 {
            // Put item back
            inv.slots[item_index] = Some(item);
            self.send_system_chat("该物品需要先检查才能再次精炼。", out);
            return;
        }

        // Get item info
        let info = match WorldProvider::get_item_info(&*self.world_db, item.item_index) {
            Some(i) => i,
            None => {
                inv.slots[item_index] = Some(item);
                self.send_system_chat("无法获取物品信息。", out);
                return;
            }
        };

        // Check bind flags: DontUpgrade (0x0020)
        const BIND_DONT_UPGRADE: i16 = 0x0020;
        if (info.bind & BIND_DONT_UPGRADE) != 0 {
            inv.slots[item_index] = Some(item);
            self.send_system_chat("该物品无法精炼。", out);
            return;
        }

        // TODO: Check rental information binding flags
        // TODO: Check OnlyRefineWeapon setting

        // Calculate cost: (RequiredAmount * 10) * RefineCost (default 125)
        const REFINE_COST: u32 = 125;
        let required_amount = info.required_amount as u32;
        let cost = required_amount * 10 * REFINE_COST;

        // Check if player has enough gold
        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => {
                inv.slots[item_index] = Some(item);
                self.send_system_chat("无法获取玩家状态。", out);
                return;
            }
        };

        if stats.gold < cost as i64 {
            inv.slots[item_index] = Some(item);
            self.send_system_chat("金币不足，无法精炼。", out);
            return;
        }

        // Deduct gold with error handling
        let mut new_stats = stats.clone();
        new_stats.gold = new_stats.gold.saturating_sub(cost as i64);

        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            if let Err(e) = self
                .store
                .save_character_stats(account_id, char_idx, &new_stats)
            {
                // Error saving stats, restore gold and abort
                inv.slots[item_index] = Some(item);
                self.send_system_chat(&format!("保存数据失败: {:?}", e), out);
                return;
            }
        } else {
            inv.slots[item_index] = Some(item);
            self.send_system_chat("无法获取账户信息。", out);
            return;
        }

        self.current_stats = Some(new_stats);

        // Send SLoseGold
        use crystal_shared_proto::user::status::SLoseGold;
        let lose_gold = SLoseGold { gold: cost };
        if let Ok(raw) = lose_gold.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Process refine slots materials with error handling
        // Calculate total stats from refine slots
        let mut total_dc: i16 = 0;
        let mut total_mc: i16 = 0;
        let mut total_sc: i16 = 0;
        let mut required_level: i16 = 0;
        let mut durability_count: u8 = 0;
        let mut current_dura_count: u8 = 0;
        let mut item_amount: u8 = 0;
        let mut ore_purity: i16 = 0;
        let mut ore_amount: u8 = 0;

        for refine_slot in refine_slots.iter_mut() {
            if let Some(ref ingredient) = refine_slot {
                let ingredient_info = match WorldProvider::get_item_info(&*self.world_db, ingredient.item_index) {
                    Some(i) => i,
                    None => {
                        // Skip invalid item and clear slot
                        *refine_slot = None;
                        continue;
                    }
                };

                // Skip weapons in refine slots
                if ingredient_info.item_type == 0 {
                    *refine_slot = None;
                    continue;
                }

                // Check if ingredient has DC/MC/SC stats
                let has_stats = ingredient_info.stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxDC as u8).map(|(_, v)| v) > Some(&0)
                    || ingredient_info.stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxMC as u8).map(|(_, v)| v) > Some(&0)
                    || ingredient_info.stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxSC as u8).map(|(_, v)| v) > Some(&0);

                if has_stats {
                    total_dc += (ingredient_info.stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::MinDC as u8).map(|(_, v)| v).unwrap_or(&0)
                        + ingredient_info.stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxDC as u8).map(|(_, v)| v).unwrap_or(&0)) as i16;
                    total_dc += *ingredient.added_stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxDC as u8).map(|(_, v)| v).unwrap_or(&0) as i16;

                    total_mc += (ingredient_info.stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::MinMC as u8).map(|(_, v)| v).unwrap_or(&0)
                        + ingredient_info.stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxMC as u8).map(|(_, v)| v).unwrap_or(&0)) as i16;
                    total_mc += *ingredient.added_stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxMC as u8).map(|(_, v)| v).unwrap_or(&0) as i16;

                    total_sc += (ingredient_info.stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::MinSC as u8).map(|(_, v)| v).unwrap_or(&0)
                        + ingredient_info.stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxSC as u8).map(|(_, v)| v).unwrap_or(&0)) as i16;
                    total_sc += *ingredient.added_stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxSC as u8).map(|(_, v)| v).unwrap_or(&0) as i16;

                    required_level += ingredient_info.required_amount as i16;

                    // Check durability (floor(max_dura / 1000) == floor(info.durability / 1000))
                    let max_dura_floor = (ingredient.max_dura / 1000) as u16;
                    let info_dura_floor = (ingredient_info.durability / 1000) as u16;
                    if max_dura_floor == info_dura_floor {
                        durability_count += 1;
                    }

                    // Check current durability (floor(current_dura / 1000) == floor(max_dura / 1000))
                    let current_dura_floor = (ingredient.current_dura / 1000) as u16;
                    if current_dura_floor == max_dura_floor {
                        current_dura_count += 1;
                    }

                    item_amount += 1;
                }

                // Check if ingredient is RefineOre
                // For now, we'll use a simple check - items with "Ore" in the name
                // TODO: Get RefineOreName from settings
                if ingredient_info.name.contains("Ore") {
                    ore_purity += (ingredient.current_dura / 1000) as i16;
                    ore_amount += 1;
                }

                // Clear refine slot
                *refine_slot = None;
            }
        }

        // Set refine_added to default increase (Settings.RefineIncrease, default 1)
        const REFINE_INCREASE: u8 = 1;
        item.refine_added = REFINE_INCREASE;

        // Calculate success chance if we have materials with stats
        if total_dc == 0 && total_mc == 0 && total_sc == 0 {
            // No stats from materials - simple refine
            item.refine_success_chance = 0;
        } else if ore_amount == 0 {
            // No ore - simple refine
            item.refine_success_chance = 0;
        } else {
            // Calculate refine stat (DC, MC, or SC based on which is highest)
            let refine_stat = if total_dc > total_mc && total_dc > total_sc {
                // RefinedValue.DC
                item.refined_value = 0; // DC = 0
                total_dc
            } else if total_mc > total_dc && total_mc > total_sc {
                // RefinedValue.MC
                item.refined_value = 1; // MC = 1
                total_mc
            } else if total_sc > total_dc && total_sc > total_mc {
                // RefinedValue.SC
                item.refined_value = 2; // SC = 2
                total_sc
            } else {
                // Default to DC
                item.refined_value = 0;
                total_dc
            };

            // Calculate success chance (simplified version)
            // Item success: (refineStat * 5) - RequiredAmount + 5, capped at 10
            let mut item_success = (refine_stat * 5) - (info.required_amount as i16) + 5;
            if item_success > 10 {
                item_success = 10;
            }
            if item_success < 0 {
                item_success = 0;
            }

            // Additional bonuses
            if item_amount > 0 && (required_level / item_amount as i16) > (info.required_amount as i16 - 5) {
                item_success += 10;
            }
            if durability_count == item_amount && item_amount > 0 {
                item_success += 10;
            }
            if current_dura_count == item_amount && item_amount > 0 {
                item_success += 5;
            }

            // Ore success (simplified)
            let mut ore_success = 0;
            if ore_amount >= item_amount {
                ore_success += 15;
            }
            if item_amount > 0 && (ore_purity / ore_amount as i16) >= (refine_stat / item_amount as i16) {
                ore_success += 15;
            }
            if ore_purity == refine_stat {
                ore_success += 5;
            }

            // Luck success: (AddedStats[Luck] + 5), capped at 10
            let mut luck_success = *item.added_stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::Luck as u8).map(|(_, v)| v).unwrap_or(&0) + 5;
            if luck_success > 10 {
                luck_success = 10;
            }
            if luck_success < 0 {
                luck_success = 0;
            }

            // Base success chance (Settings.RefineBaseChance, default 20)
            const REFINE_BASE_CHANCE: i32 = 20;
            let mut success_chance = item_success as i32 + ore_success as i32 + luck_success as i32 + REFINE_BASE_CHANCE;

            // Reduce success chance based on existing added stats
            let added_stats_total = *item.added_stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxDC as u8).map(|(_, v)| v).unwrap_or(&0)
                + *item.added_stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxMC as u8).map(|(_, v)| v).unwrap_or(&0)
                + *item.added_stats.entries.iter().find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxSC as u8).map(|(_, v)| v).unwrap_or(&0);

            // TODO: Apply RefineWepStatReduce or RefineItemStatReduce based on item type
            let stat_reduce_factor: f32 = if info.item_type == 0 {
                0.5 // Weapon - TODO: use RefineWepStatReduce
            } else {
                1.0 // Other - TODO: use RefineItemStatReduce
            };

            let adjusted_added_stats = (added_stats_total as f32 * stat_reduce_factor) as i32;
            let capped_added_stats = adjusted_added_stats.min(50);

            success_chance -= capped_added_stats;

            item.refine_success_chance = success_chance;
        }

        // Set refine time (Settings.RefineTime * Settings.Minute, default 20 minutes)
        const REFINE_TIME_MINUTES: i64 = 20;
        const MINUTE_MS: i64 = 60_000;
        let refine_time_ms = REFINE_TIME_MINUTES * MINUTE_MS;

        // Update world: set current_refine and clear refine slots
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
            world.set_player_refine_slots(self.session_id, refine_slots);
            world.set_player_current_refine(self.session_id, Some(item.clone()));
            world.set_player_refine_time_remaining(self.session_id, refine_time_ms);
        }

        // Send system message
        self.send_system_chat(
            &format!("您的物品正在精炼中，请在 {} 分钟后检查。", REFINE_TIME_MINUTES),
            out,
        );
    }

    pub(crate) fn handle_check_refine(&mut self, msg: CCheckRefine, out: &mut Vec<Vec<u8>>) {
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

        // TODO: Check if NPC page is @REFINECHECK (need to track current NPC page)
        // For now, we'll skip this check

        // Get player inventory
        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        // Find item in inventory
        let item_index = match inv.slots.iter().position(|s| {
            s.as_ref().map(|i| i.unique_id == msg.unique_id).unwrap_or(false)
        }) {
            Some(idx) => idx,
            None => return,
        };

        let mut item = match inv.slots[item_index].take() {
            Some(item) => item,
            None => return,
        };

        // Check if item has been refined (RefineAdded > 0)
        if item.refine_added == 0 {
            // Get item name from info
            let item_name = match WorldProvider::get_item_info(&*self.world_db, item.item_index) {
                Some(info) => info.name.clone(),
                None => "Unknown Item".to_string(),
            };
            
            self.send_system_chat(
                &format!("{} doesn't need to be checked as it hasn't been refined yet.", item_name),
                out,
            );
            inv.slots[item_index] = Some(item);
            return;
        }

        // Use random chance to determine success/failure
        let mut rng = rand::thread_rng();
        let success_roll = rng.gen_range(1..100);
        
        // Check if refinement fails (roll > success chance)
        if success_roll > item.refine_success_chance as i32 {
            // Failed - reset refined value to None (255 in our implementation)
            item.refined_value = 255; // RefinedValue::None
        }

        // Check for critical success (Settings.RefineCritChance, default 5%)
        const REFINE_CRIT_CHANCE: i32 = 5;
        let crit_roll = rng.gen_range(1..100);
        
        if crit_roll < REFINE_CRIT_CHANCE {
            // Critical success - increase refine added
            // Settings.RefineCritIncrease, default 2
            const REFINE_CRIT_INCREASE: u8 = 2;
            item.refine_added = item.refine_added.saturating_mul(REFINE_CRIT_INCREASE);
        }

        // Apply results based on refined value
        let mut item_destroyed = false;
        
        if item.refined_value == 255 && item.refine_added > 0 {
            // Failed refinement - destroy the item
            let item_name = match WorldProvider::get_item_info(&*self.world_db, item.item_index) {
                Some(info) => info.name.clone(),
                None => "Unknown Item".to_string(),
            };
            
            self.send_system_chat(
                &format!("Your {} smashed into a thousand pieces upon testing.", item_name),
                out,
            );
            
            // Send RefineItem packet to notify client of destruction
            use crystal_shared_proto::item::SRefineItem;
            let pkt = SRefineItem {
                unique_id: item.unique_id,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            
            item.refine_success_chance = 0;
            item_destroyed = true;
        } else if item.refine_added > 0 {
            // Success - add stats to item
            let stat_added = item.refine_added as i32;
            
            match item.refined_value {
                0 => {
                    // DC - RefinedValue::DC = 0
                    let item_name = match WorldProvider::get_item_info(&*self.world_db, item.item_index) {
                        Some(info) => info.name.clone(),
                        None => "Unknown Item".to_string(),
                    };
                    
                    self.send_system_chat(
                        &format!("Congratulations, your {} now has +{} extra DC.", item_name, item.refine_added),
                        out,
                    );
                    // Add to added_stats
                    let current_dc = *item.added_stats.entries.iter()
                        .find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxDC as u8)
                        .map(|(_, v)| v)
                        .unwrap_or(&0);
                    let new_dc = current_dc.saturating_add(stat_added);
                    
                    // Update or add the stat entry
                    if let Some(entry) = item.added_stats.entries.iter_mut()
                        .find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxDC as u8) {
                        entry.1 = new_dc;
                    } else {
                        item.added_stats.entries.push((crystal_server_core::stats::Stat::MaxDC as u8, new_dc));
                    }
                }
                1 => {
                    // MC - RefinedValue::MC = 1
                    let item_name = match WorldProvider::get_item_info(&*self.world_db, item.item_index) {
                        Some(info) => info.name.clone(),
                        None => "Unknown Item".to_string(),
                    };
                    
                    self.send_system_chat(
                        &format!("Congratulations, your {} now has +{} extra MC.", item_name, item.refine_added),
                        out,
                    );
                    // Add to added_stats
                    let current_mc = *item.added_stats.entries.iter()
                        .find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxMC as u8)
                        .map(|(_, v)| v)
                        .unwrap_or(&0);
                    let new_mc = current_mc.saturating_add(stat_added);
                    
                    // Update or add the stat entry
                    if let Some(entry) = item.added_stats.entries.iter_mut()
                        .find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxMC as u8) {
                        entry.1 = new_mc;
                    } else {
                        item.added_stats.entries.push((crystal_server_core::stats::Stat::MaxMC as u8, new_mc));
                    }
                }
                2 => {
                    // SC - RefinedValue::SC = 2
                    let item_name = match WorldProvider::get_item_info(&*self.world_db, item.item_index) {
                        Some(info) => info.name.clone(),
                        None => "Unknown Item".to_string(),
                    };
                    
                    self.send_system_chat(
                        &format!("Congratulations, your {} now has +{} extra SC.", item_name, item.refine_added),
                        out,
                    );
                    // Add to added_stats
                    let current_sc = *item.added_stats.entries.iter()
                        .find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxSC as u8)
                        .map(|(_, v)| v)
                        .unwrap_or(&0);
                    let new_sc = current_sc.saturating_add(stat_added);
                    
                    // Update or add the stat entry
                    if let Some(entry) = item.added_stats.entries.iter_mut()
                        .find(|(k, _)| *k == crystal_server_core::stats::Stat::MaxSC as u8) {
                        entry.1 = new_sc;
                    } else {
                        item.added_stats.entries.push((crystal_server_core::stats::Stat::MaxSC as u8, new_sc));
                    }
                }
                _ => {
                    // Unknown refined value - treat as failure
                    let item_name = match WorldProvider::get_item_info(&*self.world_db, item.item_index) {
                        Some(info) => info.name.clone(),
                        None => "Unknown Item".to_string(),
                    };
                    
                    self.send_system_chat(
                        &format!("Your {} smashed into a thousand pieces upon testing.", item_name),
                        out,
                    );
                    
                    use crystal_shared_proto::item::SRefineItem;
                    let pkt = SRefineItem {
                        unique_id: item.unique_id,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    
                    item.refine_success_chance = 0;
                    item_destroyed = true;
                }
            }
            
            // Reset refinement state on success
            item.refine_added = 0;
            item.refined_value = 0;
            item.refine_success_chance = 0;
        }

        // Update world state
        {
            let mut world = self.world.lock().unwrap();
            if item_destroyed {
                // Item was destroyed, don't put it back
                world.set_player_items(self.session_id, inv, eq);
            } else {
                // Put the updated item back
                inv.slots[item_index] = Some(item.clone());
                world.set_player_items(self.session_id, inv, eq);
                
                // Send ItemUpgraded packet
                use crystal_shared_proto::item::SItemUpgraded;
                // Serialize the item using the same method as SRefreshItem
                let item_bytes = match crystal_shared_proto::item::SRefreshItem::from_user_item(&item) {
                    Ok(refresh_item) => {
                        match refresh_item.encode() {
                            Ok(raw_packet) => raw_packet.payload,
                            Err(_) => {
                                // Fallback: empty bytes
                                Vec::new()
                            }
                        }
                    }
                    Err(_) => {
                        // Fallback: empty bytes
                        Vec::new()
                    }
                };
                
                let pkt = SItemUpgraded {
                    item_bytes,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }

    pub(crate) fn handle_replace_wed_ring(&mut self, msg: CReplaceWedRing, out: &mut Vec<Vec<u8>>) {
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

        // TODO: Check if NPC page is @REPLACEWEDRING (need to track current NPC page)
        // For now, we'll skip this check and allow the operation if other conditions are met

        // Get player equipment and inventory
        let (mut inv, mut eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        // Check if player is wearing a ring in RingL slot (index 7)
        const RING_L_SLOT: usize = 7;
        let current_ring = match eq.slots.get(RING_L_SLOT).and_then(|s| s.as_ref()) {
            Some(ring) => ring.clone(),
            None => {
                self.send_system_chat("您没有佩戴戒指。", out);
                return;
            }
        };

        // Check if current ring is a wedding ring
        if current_ring.wedding_ring == -1 {
            self.send_system_chat("您没有佩戴婚戒。", out);
            return;
        }

        // Find new ring in inventory
        let new_ring_index = match inv.slots.iter().position(|s| {
            s.as_ref().map(|i| i.unique_id == msg.unique_id).unwrap_or(false)
        }) {
            Some(idx) => idx,
            None => {
                self.send_system_chat("背包中未找到该物品。", out);
                return;
            }
        };

        let new_ring = match inv.slots[new_ring_index].as_ref() {
            Some(ring) => ring.clone(),
            None => return,
        };

        // Get item info for validation
        let new_ring_info = match WorldProvider::get_item_info(&*self.world_db, new_ring.item_index) {
            Some(info) => info,
            None => return,
        };

        // Validate new ring is a Ring type (ItemType.Ring = 3)
        if new_ring_info.item_type != 3 {
            self.send_system_chat("您不能使用此物品替换婚戒。", out);
            return;
        }

        // Check if new ring can be equipped (use equip_item_for_player to validate, but don't actually equip yet)
        // We'll validate by trying to get item info and check requirements
        let can_equip = {
            let world = self.world.lock().unwrap();
            // Get item from inventory to validate
            let item = match inv.slots[new_ring_index].as_ref() {
                Some(i) => i,
                None => return,
            };
            
            // Basic validation: check if it's a ring and can be equipped
            // The actual equip validation will be done by can_equip_item_for_player
            // For now, we'll do a simplified check
            let info = match WorldProvider::get_item_info(&*self.world_db, item.item_index) {
                Some(i) => i,
                None => return,
            };
            
            // Check item type is Ring (3)
            info.item_type == 3
        };
        
        if !can_equip {
            self.send_system_chat("您无法装备此物品。", out);
            return;
        }

        // Check bind flag: NoWeddingRing (need to check ItemInfoData.bind flags)
        // In C#, this is BindMode.NoWeddingRing, which is typically 0x0080
        const BIND_NO_WEDDING_RING: i16 = 0x0080;
        if (new_ring_info.bind & BIND_NO_WEDDING_RING) != 0 {
            self.send_system_chat("您不能使用此类型的戒指。", out);
            return;
        }

        // Calculate cost: (RequiredAmount * 10) * ReplaceWedRingCost (default 125)
        const REPLACE_WED_RING_COST: u32 = 125;
        let required_amount = new_ring_info.required_amount as u32;
        let cost = required_amount * 10 * REPLACE_WED_RING_COST;

        // Check if player has enough gold
        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => return,
        };

        if stats.gold < cost as i64 {
            self.send_system_chat("金币不足，无法替换婚戒。", out);
            return;
        }

        // Get player's married status (character index of spouse)
        // For now, we'll use the current ring's wedding_ring value as the married status
        // In a full implementation, this should come from player's marriage data
        let married = current_ring.wedding_ring;

        // Deduct gold
        let mut new_stats = stats.clone();
        new_stats.gold = new_stats.gold.saturating_sub(cost as i64);

        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let _ = self
                .store
                .save_character_stats(account_id, char_idx, &new_stats);
        }

        self.current_stats = Some(new_stats);

        // Prepare new ring with wedding ring property
        let mut new_ring_with_wedding = new_ring.clone();
        new_ring_with_wedding.wedding_ring = married;

        // Prepare old ring without wedding ring property
        let mut old_ring_without_wedding = current_ring.clone();
        old_ring_without_wedding.wedding_ring = -1;

        // Swap rings: new ring to equipment, old ring to inventory
        eq.slots[RING_L_SLOT] = Some(new_ring_with_wedding.clone());
        inv.slots[new_ring_index] = Some(old_ring_without_wedding.clone());

        // Update world
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
            world.recalc_player_equipment_stats(self.session_id);
        }

        // Send SLoseGold
        use crystal_shared_proto::user::status::SLoseGold;
        let lose_gold = SLoseGold { gold: cost };
        if let Ok(raw) = lose_gold.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Send SEquipItem
        use crystal_shared_proto::item::SEquipItem;
        let equip_item = SEquipItem {
            grid: 1, // Inventory
            unique_id: new_ring_with_wedding.unique_id,
            to: RING_L_SLOT as i32,
            success: true,
        };
        if let Ok(raw) = equip_item.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Send SRefreshItem for both rings
        use crystal_shared_proto::item::SRefreshItem;
        if let Ok(refresh_old) = SRefreshItem::from_user_item(&old_ring_without_wedding) {
            if let Ok(raw) = refresh_old.encode() {
                out.push(Self::encode_raw(raw));
            }
        }

        if let Ok(refresh_new) = SRefreshItem::from_user_item(&new_ring_with_wedding) {
            if let Ok(raw) = refresh_new.encode() {
                out.push(Self::encode_raw(raw));
            }
        }
    }
}