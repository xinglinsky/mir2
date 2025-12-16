use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;

use crate::world::content;

const MAX_LEVEL: u16 = 500;

/// Experience table for normal characters, loaded from ExpList.ini.
/// Index 0 corresponds to Level 1, mirroring C# Settings.ExperienceList.
pub static EXP_TABLE: Lazy<Vec<i64>> = Lazy::new(|| load_exp_table(Path::new("./Configs/ExpList.ini")));

/// Get a shared reference to the global experience table.
pub fn exp_table() -> &'static Vec<i64> {
    &EXP_TABLE
}

/// Convenience helper to get MaxExperience for a given level.
pub fn max_experience_for_level(level: u16) -> i64 {
    if level == 0 || level > MAX_LEVEL {
        return 0;
    }
    // Vec index is level - 1.
    *EXP_TABLE.get(level as usize - 1).unwrap_or(&0)
}

fn load_exp_table(path: &Path) -> Vec<i64> {
    // Parse the ini once into a map of Level -> value, then simulate the
    // C# LoadEXP loop where missing entries fall back to the previous
    // level's required experience.
    let mut level_values: HashMap<u16, i64> = HashMap::new();

    if let Ok(text) = content::read_to_string(path) {
        let mut current_section: Option<String> = None;
        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with(';') || line.starts_with('#') {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                let name = &line[1..line.len() - 1];
                current_section = Some(name.to_string());
                continue;
            }

            let mut parts = line.splitn(2, '=');
            let key = parts.next().unwrap_or("").trim();
            let value = parts.next().unwrap_or("").trim();
            if key.is_empty() {
                continue;
            }

            if let Some(section) = &current_section {
                if section == "Exp" {
                    if let Some(lvl_str) = key.strip_prefix("Level") {
                        if let Ok(lvl) = lvl_str.parse::<u16>() {
                            if let Ok(v) = value.parse::<i64>() {
                                level_values.insert(lvl, v);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut table = Vec::with_capacity(MAX_LEVEL as usize);
    // C# default starting value when reading ExpList.ini.
    let mut last: i64 = 100;
    for lvl in 1..=MAX_LEVEL {
        if let Some(&v) = level_values.get(&lvl) {
            last = v;
        }
        table.push(last);
    }

    table
}
