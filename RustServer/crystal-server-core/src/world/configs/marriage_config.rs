use once_cell::sync::Lazy;
use std::path::Path;

use crate::world::content;

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

#[derive(Clone, Debug)]
pub struct MarriageConfig {
    pub lover_exp_bonus: i32,
    pub marriage_cooldown: i32,
    pub wedding_ring_recall: bool,
    pub marriage_level_required: i32,
    pub replace_ring_cost: i32,
}

impl Default for MarriageConfig {
    fn default() -> Self {
        MarriageConfig {
            lover_exp_bonus: 5,
            marriage_cooldown: 7,
            wedding_ring_recall: true,
            marriage_level_required: 10,
            replace_ring_cost: 125,
        }
    }
}

pub static MARRIAGE_CONFIG: Lazy<MarriageConfig> =
    Lazy::new(|| load_marriage_config(Path::new("./Configs/MarriageSystem.ini")));

pub fn marriage_config() -> &'static MarriageConfig {
    &MARRIAGE_CONFIG
}

fn load_marriage_config(path: &Path) -> MarriageConfig {
    let mut cfg = MarriageConfig::default();

    let text = match content::read_to_string(path) {
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
            Some("Config") => match key {
                "EXPBonus" => {
                    cfg.lover_exp_bonus =
                        parse_i32(value, cfg.lover_exp_bonus);
                }
                "MarriageCooldown" => {
                    cfg.marriage_cooldown =
                        parse_i32(value, cfg.marriage_cooldown);
                }
                "AllowLoverRecall" => {
                    cfg.wedding_ring_recall =
                        parse_bool(value, cfg.wedding_ring_recall);
                }
                "MinimumLevel" => {
                    cfg.marriage_level_required =
                        parse_i32(value, cfg.marriage_level_required);
                }
                "ReplaceRingCost" => {
                    cfg.replace_ring_cost =
                        parse_i32(value, cfg.replace_ring_cost);
                }
                _ => {}
            },
            _ => {}
        }
    }

    cfg
}
