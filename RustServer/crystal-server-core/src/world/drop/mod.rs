use std::{fs, io, path::Path};

use rand::Rng;

#[derive(Clone, Debug)]
pub struct DropInfo {
    pub chance: i32,
    /// If None, this drop is pure gold or a group-only marker.
    pub item_index: Option<i32>,
    pub gold: u32,
    /// Optional nested group of drops (mirrors C# GroupDropInfo usage).
    pub grouped_drop: Option<GroupDropInfo>,
    /// C# DropInfo.Type
    pub drop_type: u8,
    pub quest_required: bool,
}

#[derive(Clone, Debug)]
pub struct GroupDropInfo {
    pub random: bool,
    pub first: bool,
    pub drops: Vec<DropInfo>,
}

#[derive(Clone, Debug)]
pub struct DropRewardInfo {
    pub items: Vec<i32>,
    pub gold: u32,
}

impl DropInfo {
    pub fn attempt_drop<R: Rng + ?Sized>(
        &self,
        drop_rate: f32,
        item_drop_rate_percent_offset: i32,
        gold_drop_rate_percent_offset: i32,
        rng: &mut R,
    ) -> Option<DropRewardInfo> {
        if self.chance <= 0 {
            return None;
        }

        // In C#: int rate = (int)(Chance / Settings.DropRate);
        let effective_drop_rate = if drop_rate > 0.0 { drop_rate } else { 1.0 };
        let mut rate = (self.chance as f32 / effective_drop_rate) as i32;

        // Only apply positive item drop rate offsets, same as the C# code.
        if item_drop_rate_percent_offset > 0 && rate > 0 {
            rate -= (rate * item_drop_rate_percent_offset) / 100;
        }

        if rate < 1 {
            rate = 1;
        }

        // if (Envir.Random.Next(rate) != 0) return null;
        if rng.gen_range(0..rate) != 0 {
            return None;
        }

        let mut gold: u32 = 0;
        let mut items: Vec<i32> = Vec::new();

        if self.gold > 0 {
            // Gold range: [Gold / 2, Gold + Gold / 2)
            let half = self.gold / 2;
            let mut lower_gold_range: i64 = (self.gold / 2) as i64;
            let upper_gold_range: i64 = self.gold.saturating_add(half) as i64;

            // Optional percent boost on the lower bound only (C# behaviour).
            if gold_drop_rate_percent_offset > 0 && lower_gold_range > 0 {
                lower_gold_range +=
                    (lower_gold_range * gold_drop_rate_percent_offset as i64) / 100;
            }

            if lower_gold_range > upper_gold_range {
                lower_gold_range = upper_gold_range;
            }

            if upper_gold_range > 0 {
                let low_u = lower_gold_range.max(0) as u32;
                let up_u = upper_gold_range.max(low_u as i64) as u32;

                if up_u > low_u {
                    gold = rng.gen_range(low_u..up_u);
                } else {
                    gold = up_u;
                }
            }
        } else if let Some(item_index) = self.item_index {
            items.push(item_index);
        } else if let Some(group) = &self.grouped_drop {
            // Grouped drops: recurse into children and aggregate results,
            // respecting the Random and First flags the same way C# does.
            let mut temp_items: Vec<i32> = Vec::new();
            let mut total_gold: u32 = 0;

            for child in &group.drops {
                if let Some(reward) = child.attempt_drop(
                    drop_rate,
                    item_drop_rate_percent_offset,
                    gold_drop_rate_percent_offset,
                    rng,
                ) {
                    total_gold = total_gold.saturating_add(reward.gold);
                    if !reward.items.is_empty() {
                        temp_items.extend(reward.items);
                    }

                    if group.first {
                        break;
                    }
                }
            }

            gold = gold.saturating_add(total_gold);

            if group.random {
                if !temp_items.is_empty() {
                    let idx = rng.gen_range(0..temp_items.len());
                    items.push(temp_items[idx]);
                }
            } else {
                items.extend(temp_items);
            }
        }

        Some(DropRewardInfo { items, gold })
    }
}

fn parse_chance_token(token: &str) -> Option<i32> {
    if token.len() < 1 {
        return None;
    }

    let numeric = if let Some(pos) = token.find('/') {
        &token[pos + 1..]
    } else if let Some(pos) = token.find(':') {
        &token[pos + 1..]
    } else if token.len() > 2 {
        &token[2..]
    } else {
        token
    };

    numeric.trim().parse::<i32>().ok()
}

fn parse_drop_line<F>(
    line: &str,
    drop_type: u8,
    item_lookup: &F,
) -> Option<(DropInfo, bool)>
where
    F: Fn(&str) -> Option<i32>,
{
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with(';') {
        return None;
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let chance = parse_chance_token(parts[0])?;

    let mut info = DropInfo {
        chance,
        item_index: None,
        gold: 0,
        grouped_drop: None,
        drop_type,
        quest_required: false,
    };

    let second = parts[1];
    let mut has_group = false;

    if second.eq_ignore_ascii_case("Gold") {
        if parts.len() < 3 {
            return None;
        }
        let amount = parts[2].parse::<u32>().ok()?;
        if amount == 0 {
            return None;
        }
        info.gold = amount;
    } else if second.to_ascii_uppercase().starts_with("GROUP") {
        let s = second.as_bytes();
        let random = s.ends_with(&[b'*']);
        let first = s.ends_with(&[b'^']);
        info.grouped_drop = Some(GroupDropInfo {
            random,
            first,
            drops: Vec::new(),
        });
        has_group = true;
    } else {
        let name = second;
        let item_index = item_lookup(name)?;
        info.item_index = Some(item_index);

        if parts.len() > 2 {
            let req = parts[2];
            if req.eq_ignore_ascii_case("Q") {
                info.quest_required = true;
            }
        }
    }

    Some((info, has_group))
}

fn parse_group<F>(
    lines: &[String],
    mut index: usize,
    drop_type: u8,
    item_lookup: &F,
    parent_group: &mut GroupDropInfo,
) -> usize
where
    F: Fn(&str) -> Option<i32>,
{
    let mut started = false;

    while index < lines.len() {
        let line = lines[index].trim();

        if line == "{" {
            started = true;
            index += 1;
            continue;
        }

        if line == "}" {
            if started {
                index += 1;
                break;
            }
            index += 1;
            continue;
        }

        if line.is_empty() || line.starts_with(';') || line.starts_with("#INSERT") {
            index += 1;
            continue;
        }

        if let Some((mut drop, has_group)) = parse_drop_line(line, drop_type, item_lookup) {
            if has_group {
                if let Some(group) = &mut drop.grouped_drop {
                    let new_index = parse_group(lines, index + 1, drop_type, item_lookup, group);
                    index = new_index;
                } else {
                    index += 1;
                }
            } else {
                index += 1;
            }

            parent_group.drops.push(drop);
        } else {
            index += 1;
        }
    }

    index
}

fn expand_inserts<P>(lines: Vec<String>, root: P) -> io::Result<Vec<String>>
where
    P: AsRef<Path>,
{
    let root_path = root.as_ref();
    let mut out = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.to_ascii_uppercase().starts_with("#INSERT") {
            let start = trimmed.find('[').and_then(|s| {
                trimmed.find(']').map(|e| (s, e))
            });
            if let Some((s, e)) = start {
                if e > s + 1 {
                    let sub = &trimmed[s + 1..e];
                    let include_path = root_path.join(sub);
                    if let Ok(text) = fs::read_to_string(&include_path) {
                        out.extend(text.lines().map(|l| l.to_string()));
                    }
                }
            }
            continue;
        }

        out.push(line);
    }

    Ok(out)
}

pub fn load_drop_file<P, F>(
    path: P,
    drop_type: u8,
    item_lookup: &F,
) -> io::Result<Vec<DropInfo>>
where
    P: AsRef<Path>,
    F: Fn(&str) -> Option<i32>,
{
    let path_ref = path.as_ref();
    if !path_ref.exists() {
        return Ok(Vec::new());
    }

    let text = fs::read_to_string(path_ref)?;
    let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();

    let root = path_ref.parent().unwrap_or_else(|| Path::new("."));
    lines = expand_inserts(lines, root)?;

    let mut result: Vec<DropInfo> = Vec::new();
    let mut index: usize = 0;

    while index < lines.len() {
        let line = lines[index].trim();
        if line.is_empty() || line.starts_with(';') {
            index += 1;
            continue;
        }

        if line.to_ascii_uppercase().starts_with("#INSERT") {
            index += 1;
            continue;
        }

        if let Some((mut drop, has_group)) = parse_drop_line(line, drop_type, item_lookup) {
            if has_group {
                if let Some(group) = &mut drop.grouped_drop {
                    let new_index = parse_group(&lines, index + 1, drop_type, item_lookup, group);
                    index = new_index;
                } else {
                    index += 1;
                }
            } else {
                index += 1;
            }

            result.push(drop);
        } else {
            index += 1;
        }
    }

    Ok(result)
}
