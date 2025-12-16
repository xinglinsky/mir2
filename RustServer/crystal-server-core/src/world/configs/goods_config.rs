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

fn parse_u32(val: &str, default: u32) -> u32 {
    val.parse::<u32>().unwrap_or(default)
}

#[derive(Clone, Debug)]
pub struct GoodsConfig {
    pub on: bool,
    pub max_stored: u32,
    pub buy_back_time: u32,
    pub buy_back_max_stored: u32,
    pub hide_added_stats: bool,
}

impl Default for GoodsConfig {
    fn default() -> Self {
        GoodsConfig {
            on: true,
            max_stored: 15,
            buy_back_time: 60,
            buy_back_max_stored: 20,
            hide_added_stats: true,
        }
    }
}

pub static GOODS_CONFIG: Lazy<GoodsConfig> =
    Lazy::new(|| load_goods_config(Path::new("./Configs/GoodsSystem.ini")));

pub fn goods_config() -> &'static GoodsConfig {
    &GOODS_CONFIG
}

fn load_goods_config(path: &Path) -> GoodsConfig {
    let mut cfg = GoodsConfig::default();

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
            Some("Goods") => match key {
                "On" => {
                    cfg.on = parse_bool(value, cfg.on);
                }
                "MaxStored" => {
                    cfg.max_stored = parse_u32(value, cfg.max_stored);
                }
                "BuyBackTime" => {
                    cfg.buy_back_time = parse_u32(value, cfg.buy_back_time);
                }
                "BuyBackMaxStored" => {
                    cfg.buy_back_max_stored =
                        parse_u32(value, cfg.buy_back_max_stored);
                }
                "HideAddedStats" => {
                    cfg.hide_added_stats =
                        parse_bool(value, cfg.hide_added_stats);
                }
                _ => {}
            },
            _ => {}
        }
    }

    cfg
}
