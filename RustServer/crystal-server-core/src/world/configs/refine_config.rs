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

fn parse_u8(val: &str, default: u8) -> u8 {
    val.parse::<u8>().unwrap_or(default)
}

#[derive(Clone, Debug)]
pub struct RefineSettingsConfig {
    pub only_refine_weapon: bool,
    pub base_chance: u8,
    pub time: i32,
    pub stat_increase: u8,
    pub crit_chance: u8,
    pub crit_increase: u8,
    pub weapon_stat_reduced_chance: u8,
    pub item_stat_reduced_chance: u8,
    pub refine_cost: i32,
}

#[derive(Clone, Debug)]
pub struct RefineOreConfig {
    pub ore_name: String,
}

#[derive(Clone, Debug)]
pub struct RefineConfig {
    pub settings: RefineSettingsConfig,
    pub ore: RefineOreConfig,
}

impl Default for RefineConfig {
    fn default() -> Self {
        RefineConfig {
            settings: RefineSettingsConfig {
                only_refine_weapon: true,
                base_chance: 20,
                time: 20,
                stat_increase: 1,
                crit_chance: 10,
                crit_increase: 2,
                weapon_stat_reduced_chance: 6,
                item_stat_reduced_chance: 15,
                refine_cost: 125,
            },
            ore: RefineOreConfig {
                ore_name: "BlackIronOre".to_string(),
            },
        }
    }
}

pub static REFINE_CONFIG: Lazy<RefineConfig> =
    Lazy::new(|| load_refine_config(Path::new("./Configs/RefineSystem.ini")));

pub fn refine_config() -> &'static RefineConfig {
    &REFINE_CONFIG
}

fn load_refine_config(path: &Path) -> RefineConfig {
    let mut cfg = RefineConfig::default();

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
                "OnlyRefineWeapon" => {
                    cfg.settings.only_refine_weapon =
                        parse_bool(value, cfg.settings.only_refine_weapon);
                }
                "BaseChance" => {
                    cfg.settings.base_chance =
                        parse_u8(value, cfg.settings.base_chance);
                }
                "Time" => {
                    cfg.settings.time = parse_i32(value, cfg.settings.time);
                }
                "StatIncrease" => {
                    cfg.settings.stat_increase =
                        parse_u8(value, cfg.settings.stat_increase);
                }
                "CritChance" => {
                    cfg.settings.crit_chance =
                        parse_u8(value, cfg.settings.crit_chance);
                }
                "CritIncrease" => {
                    cfg.settings.crit_increase =
                        parse_u8(value, cfg.settings.crit_increase);
                }
                "WepStatReducedChance" => {
                    cfg.settings.weapon_stat_reduced_chance = parse_u8(
                        value,
                        cfg.settings.weapon_stat_reduced_chance,
                    );
                }
                "ItemStatReducedChance" => {
                    cfg.settings.item_stat_reduced_chance = parse_u8(
                        value,
                        cfg.settings.item_stat_reduced_chance,
                    );
                }
                "RefineCost" => {
                    cfg.settings.refine_cost =
                        parse_i32(value, cfg.settings.refine_cost);
                }
                _ => {}
            },
            Some("Ore") => match key {
                "OreName" => {
                    cfg.ore.ore_name = value.to_string();
                }
                _ => {}
            },
            _ => {}
        }
    }

    cfg
}
