use once_cell::sync::Lazy;
use std::fs;
use std::path::Path;

/// Simple representation of a required item volume for guild creation,
/// mirroring C# GuildItemVolume (ItemName + Amount).
#[derive(Clone, Debug, Default)]
pub struct GuildCreationCost {
    pub item_name: String,
    pub amount: u32,
}

/// Guild settings loaded from Configs/GuildSettings.ini. This mirrors the
/// data read by C# Settings.LoadGuildSettings: required level, exp rate,
/// points per level, war time/cost, newbie guild buffs, guild experience
/// per level, member caps per level and creation cost requirements. Guild
/// buff definitions are currently ignored and can be added later.
#[derive(Clone, Debug)]
pub struct GuildSettings {
    pub required_level: u8,
    pub exp_rate: f32,
    pub points_per_level: u8,
    pub war_time: i64,
    pub war_cost: u32,
    pub newbie_guild_buff_enabled: bool,
    pub newbie_guild_exp_buff: i32,
    pub exp_by_level: Vec<i64>,
    pub member_cap_by_level: Vec<i32>,
    pub creation_cost: Vec<GuildCreationCost>,
}

impl Default for GuildSettings {
    fn default() -> Self {
        Self {
            // Defaults taken from C# Settings fields.
            required_level: 22,
            exp_rate: 0.01,
            points_per_level: 0,
            war_time: 180,
            war_cost: 3000,
            newbie_guild_buff_enabled: true,
            newbie_guild_exp_buff: 5,
            exp_by_level: Vec::new(),
            member_cap_by_level: Vec::new(),
            creation_cost: Vec::new(),
        }
    }
}

pub static GUILD_SETTINGS: Lazy<GuildSettings> = Lazy::new(|| {
    load_guild_settings(Path::new("./Configs/GuildSettings.ini"))
});

pub fn guild_settings() -> &'static GuildSettings {
    &GUILD_SETTINGS
}

pub fn guild_max_experience_for_level(level: u8) -> i64 {
    let s = guild_settings();
    s.exp_by_level
        .get(level as usize)
        .copied()
        .unwrap_or(0)
}

pub fn guild_member_cap_for_level(level: u8) -> i32 {
    let s = guild_settings();
    s.member_cap_by_level
        .get(level as usize)
        .copied()
        .unwrap_or(0)
}

fn load_guild_settings(path: &Path) -> GuildSettings {
    let mut cfg = GuildSettings::default();

    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => {
            // Mirror C# behaviour when GuildSettings.ini is missing: provide
            // default guild creation cost (1,000,000 gold + 1 WoomaHorn).
            cfg.creation_cost.push(GuildCreationCost {
                item_name: String::new(),
                amount: 1_000_000,
            });
            cfg.creation_cost.push(GuildCreationCost {
                item_name: "WoomaHorn".to_string(),
                amount: 1,
            });
            return cfg;
        }
    };

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
            match section.as_str() {
                "Guilds" => {
                    match key {
                        "MinimumLevel" => {
                            if let Ok(v) = value.parse::<u8>() {
                                cfg.required_level = v;
                            }
                        }
                        "ExpRate" => {
                            if let Ok(v) = value.parse::<f32>() {
                                cfg.exp_rate = v;
                            }
                        }
                        "PointPerLevel" => {
                            if let Ok(v) = value.parse::<u8>() {
                                cfg.points_per_level = v;
                            }
                        }
                        "WarTime" => {
                            if let Ok(v) = value.parse::<i64>() {
                                cfg.war_time = v;
                            }
                        }
                        "WarCost" => {
                            if let Ok(v) = value.parse::<u32>() {
                                cfg.war_cost = v;
                            }
                        }
                        "NewbieGuildBuffEnabled" => {
                            cfg.newbie_guild_buff_enabled = matches!(
                                value,
                                "True" | "true" | "1"
                            );
                        }
                        "NewbieGuildExpBuff" => {
                            if let Ok(v) = value.parse::<i32>() {
                                cfg.newbie_guild_exp_buff = v;
                            }
                        }
                        _ => {}
                    }
                }
                "Exp" => {
                    if let Some(idx_str) = key.strip_prefix("Level-") {
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            ensure_len_i64(&mut cfg.exp_by_level, idx + 1);
                            if let Ok(v) = value.parse::<i64>() {
                                cfg.exp_by_level[idx] = v;
                            }
                        }
                    }
                }
                "Cap" => {
                    if let Some(idx_str) = key.strip_prefix("Level-") {
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            ensure_len_i32(&mut cfg.member_cap_by_level, idx + 1);
                            if let Ok(v) = value.parse::<i32>() {
                                cfg.member_cap_by_level[idx] = v;
                            }
                        }
                    }
                }
                _ if section.starts_with("Required-") => {
                    // Required-X sections: ItemName / Amount.
                    let idx = section["Required-".len()..].parse::<usize>().unwrap_or(0);
                    ensure_len_cost(&mut cfg.creation_cost, idx + 1);
                    let cost = &mut cfg.creation_cost[idx];
                    match key {
                        "ItemName" => {
                            cost.item_name = value.to_string();
                        }
                        "Amount" => {
                            if let Ok(v) = value.parse::<u32>() {
                                cost.amount = v;
                            }
                        }
                        _ => {}
                    }
                }
                _ => {
                    // Buff-* sections are currently ignored; they can be wired
                    // into a future GuildBuffConfig when needed.
                }
            }
        }
    }

    cfg
}

fn ensure_len_i64(v: &mut Vec<i64>, len: usize) {
    if v.len() < len {
        v.resize(len, 0);
    }
}

fn ensure_len_i32(v: &mut Vec<i32>, len: usize) {
    if v.len() < len {
        v.resize(len, 0);
    }
}

fn ensure_len_cost(v: &mut Vec<GuildCreationCost>, len: usize) {
    if v.len() < len {
        v.resize_with(len, GuildCreationCost::default);
    }
}
