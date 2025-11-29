use once_cell::sync::Lazy;
use std::fs;
use std::path::Path;

fn parse_bool(val: &str, default: bool) -> bool {
    match val {
        "True" | "true" | "1" => true,
        "False" | "false" | "0" => false,
        _ => default,
    }
}

fn parse_i32(val: &str, default: i32) -> i32 {
    val.parse::<i32>().unwrap_or(default)
}

fn parse_i64(val: &str, default: i64) -> i64 {
    val.parse::<i64>().unwrap_or(default)
}

#[derive(Clone, Debug)]
pub struct FishingRatesConfig {
    pub attempts: i32,
    pub success_start: i32,
    pub success_multiplier: i32,
    pub delay_ms: i64,
    pub monster_spawn_chance: i32,
}

#[derive(Clone, Debug)]
pub struct FishingConfig {
    pub rates: FishingRatesConfig,
    pub monster: String,
}

impl Default for FishingConfig {
    fn default() -> Self {
        FishingConfig {
            rates: FishingRatesConfig {
                attempts: 30,
                success_start: 10,
                success_multiplier: 10,
                delay_ms: 0,
                monster_spawn_chance: 5,
            },
            monster: "GiantKeratoid".to_string(),
        }
    }
}

pub static FISHING_CONFIG: Lazy<FishingConfig> =
    Lazy::new(|| load_fishing_config(Path::new("./Configs/FishingSystem.ini")));

pub fn fishing_config() -> &'static FishingConfig {
    &FISHING_CONFIG
}

fn load_fishing_config(path: &Path) -> FishingConfig {
    let mut cfg = FishingConfig::default();

    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return cfg,
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
            current_section = Some(line[1..line.len() - 1].to_string());
            continue;
        }

        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();
        if key.is_empty() {
            continue;
        }

        match current_section.as_deref() {
            Some("Rates") => match key {
                "Attempts" => {
                    cfg.rates.attempts = parse_i32(value, cfg.rates.attempts);
                }
                "SuccessStart" => {
                    cfg.rates.success_start = parse_i32(value, cfg.rates.success_start);
                }
                "SuccessMultiplier" => {
                    cfg.rates.success_multiplier =
                        parse_i32(value, cfg.rates.success_multiplier);
                }
                "Delay" => {
                    cfg.rates.delay_ms = parse_i64(value, cfg.rates.delay_ms);
                }
                "MonsterSpawnChance" => {
                    cfg.rates.monster_spawn_chance =
                        parse_i32(value, cfg.rates.monster_spawn_chance);
                }
                _ => {}
            },
            Some("Game") => match key {
                "Monster" => {
                    cfg.monster = value.to_string();
                }
                _ => {}
            },
            _ => {}
        }
    }

    cfg
}
