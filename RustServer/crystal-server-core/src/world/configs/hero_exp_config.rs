use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;

use crate::world::content;

const HERO_MAX_LEVEL: u16 = 500;

#[derive(Clone, Debug)]
pub struct HeroExpConfig {
    /// Experience required per hero level. Index 0 is unused; index i
    /// corresponds to Level i as in the C# HeroExpList.ini.
    pub exp_by_level: Vec<i64>,
}

impl HeroExpConfig {
    pub fn max_experience_for_level(&self, level: u16) -> i64 {
        let idx = level as usize;
        if idx == 0 || idx >= self.exp_by_level.len() {
            0
        } else {
            self.exp_by_level[idx]
        }
    }
}

pub static HERO_EXP_CONFIG: Lazy<HeroExpConfig> = Lazy::new(|| {
    load_hero_exp_config(Path::new("./Configs/HeroExpList.ini"))
});

pub fn hero_max_experience_for_level(level: u16) -> i64 {
    HERO_EXP_CONFIG.max_experience_for_level(level)
}

fn load_hero_exp_config(path: &Path) -> HeroExpConfig {
    // Parse the ini once into a map of Level -> value, then simulate the
    // C# LoadHeroEXP loop where missing entries fall back to the previous
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

    let mut exp_by_level = Vec::with_capacity(HERO_MAX_LEVEL as usize + 1);
    // index 0 unused to keep indexing consistent with Level numbers.
    exp_by_level.push(0);

    // C# default starting value when reading HeroExpList.ini.
    let mut last: i64 = 100;
    for lvl in 1..=HERO_MAX_LEVEL {
        if let Some(&v) = level_values.get(&lvl) {
            last = v;
        }
        exp_by_level.push(last);
    }

    HeroExpConfig { exp_by_level }
}
