use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crystal_shared_proto::io::{write_bool, write_f32_le, write_i32_le};
use crystal_shared_proto::item_types::UserItemData;

use super::LoginConnection;

impl LoginConnection {
    pub(crate) fn normalize_npc_key(key: &str) -> String {
        let mut s = key.trim().to_ascii_uppercase();
        if s.starts_with('[') && s.ends_with(']') && s.len() >= 3 {
            s = s[1..s.len() - 1].to_string();
        }
        if !s.starts_with('@') {
            s = format!("@{}", s);
        }
        if s == "@MAIN" {
            "@MAIN-1".to_string()
        } else {
            s
        }
    }

    pub(crate) fn find_npc_script_path(root: &Path, file_name: &str) -> Option<PathBuf> {
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
}
