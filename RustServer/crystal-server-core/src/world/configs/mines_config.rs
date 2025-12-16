use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;

use crate::world::content;

#[derive(Clone, Debug)]
pub struct MineDropConfig {
    pub item_name: String,
    pub min_slot: u8,
    pub max_slot: u8,
    pub min_dura: u8,
    pub max_dura: u8,
    pub bonus_chance: u8,
    pub max_bonus_dura: u8,
}

#[derive(Clone, Debug)]
pub struct MineSetConfig {
    pub name: String,
    pub spot_regen_rate: u8,
    pub max_stones: u8,
    pub hit_rate: u8,
    pub drop_rate: u8,
    pub total_slots: u8,
    pub drops: Vec<MineDropConfig>,
}

#[derive(Clone, Debug, Default)]
pub struct MinesConfig {
    pub mines: Vec<MineSetConfig>,
}

pub static MINES_CONFIG: Lazy<MinesConfig> =
    Lazy::new(|| load_mines_config(Path::new("./Configs/Mines.ini")));

pub fn mines_config() -> &'static MinesConfig {
    &MINES_CONFIG
}

fn parse_ini_map(text: &str) -> HashMap<String, HashMap<String, String>> {
    let mut map: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current = String::new();

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current = line[1..line.len() - 1].to_string();
            continue;
        }
        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();
        if key.is_empty() {
            continue;
        }
        let section = map.entry(current.clone()).or_insert_with(HashMap::new);
        section.insert(key.to_string(), value.to_string());
    }

    map
}

fn read_u8(
    sections: &HashMap<String, HashMap<String, String>>,
    section: &str,
    key: &str,
    default: u8,
) -> u8 {
    sections
        .get(section)
        .and_then(|m| m.get(key))
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(default)
}

fn read_string(
    sections: &HashMap<String, HashMap<String, String>>,
    section: &str,
    key: &str,
    default: &str,
) -> String {
    sections
        .get(section)
        .and_then(|m| m.get(key))
        .map(|s| s.as_str())
        .unwrap_or(default)
        .to_string()
}

fn default_mines() -> MinesConfig {
    let mut mines = Vec::new();

    mines.push(MineSetConfig {
        name: String::new(),
        spot_regen_rate: 5,
        max_stones: 80,
        hit_rate: 25,
        drop_rate: 10,
        total_slots: 120,
        drops: vec![
            MineDropConfig {
                item_name: "GoldOre".to_string(),
                min_slot: 1,
                max_slot: 2,
                min_dura: 3,
                max_dura: 16,
                bonus_chance: 20,
                max_bonus_dura: 10,
            },
            MineDropConfig {
                item_name: "SilverOre".to_string(),
                min_slot: 3,
                max_slot: 20,
                min_dura: 3,
                max_dura: 16,
                bonus_chance: 20,
                max_bonus_dura: 10,
            },
            MineDropConfig {
                item_name: "CopperOre".to_string(),
                min_slot: 21,
                max_slot: 45,
                min_dura: 3,
                max_dura: 16,
                bonus_chance: 20,
                max_bonus_dura: 10,
            },
            MineDropConfig {
                item_name: "BlackIronOre".to_string(),
                min_slot: 46,
                max_slot: 56,
                min_dura: 3,
                max_dura: 16,
                bonus_chance: 20,
                max_bonus_dura: 10,
            },
        ],
    });

    mines.push(MineSetConfig {
        name: String::new(),
        spot_regen_rate: 5,
        max_stones: 80,
        hit_rate: 25,
        drop_rate: 10,
        total_slots: 100,
        drops: vec![
            MineDropConfig {
                item_name: "PlatinumOre".to_string(),
                min_slot: 1,
                max_slot: 2,
                min_dura: 3,
                max_dura: 16,
                bonus_chance: 20,
                max_bonus_dura: 10,
            },
            MineDropConfig {
                item_name: "RubyOre".to_string(),
                min_slot: 3,
                max_slot: 20,
                min_dura: 3,
                max_dura: 16,
                bonus_chance: 20,
                max_bonus_dura: 10,
            },
            MineDropConfig {
                item_name: "NephriteOre".to_string(),
                min_slot: 21,
                max_slot: 45,
                min_dura: 3,
                max_dura: 16,
                bonus_chance: 20,
                max_bonus_dura: 10,
            },
            MineDropConfig {
                item_name: "AmethystOre".to_string(),
                min_slot: 46,
                max_slot: 56,
                min_dura: 3,
                max_dura: 16,
                bonus_chance: 20,
                max_bonus_dura: 10,
            },
        ],
    });

    MinesConfig { mines }
}

fn load_mines_config(path: &Path) -> MinesConfig {
    let text = match content::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return default_mines(),
    };

    let sections = parse_ini_map(&text);
    let mut mines = Vec::new();
    let mut i = 0;

    loop {
        let section = format!("Mine{}", i);
        let spot = read_u8(&sections, &section, "SpotRegenRate", 255);
        if spot == 255 {
            break;
        }

        let mut mine = MineSetConfig {
            name: read_string(&sections, &section, "Name", ""),
            spot_regen_rate: spot,
            max_stones: read_u8(&sections, &section, "MaxStones", 80),
            hit_rate: read_u8(&sections, &section, "HitRate", 25),
            drop_rate: read_u8(&sections, &section, "DropRate", 10),
            total_slots: read_u8(&sections, &section, "TotalSlots", 100),
            drops: Vec::new(),
        };

        let mut j = 0;
        loop {
            let min_slot_key = format!("D{}-MinSlot", j);
            let min_slot = read_u8(&sections, &section, &min_slot_key, 255);
            if min_slot == 255 {
                break;
            }

            let item_name_key = format!("D{}-ItemName", j);
            let max_slot_key = format!("D{}-MaxSlot", j);
            let min_dura_key = format!("D{}-MinDura", j);
            let max_dura_key = format!("D{}-MaxDura", j);
            let bonus_chance_key = format!("D{}-BonusChance", j);
            let max_bonus_dura_key = format!("D{}-MaxBonusDura", j);

            let drop = MineDropConfig {
                item_name: read_string(&sections, &section, &item_name_key, ""),
                min_slot,
                max_slot: read_u8(&sections, &section, &max_slot_key, 255),
                min_dura: read_u8(&sections, &section, &min_dura_key, 255),
                max_dura: read_u8(&sections, &section, &max_dura_key, 255),
                bonus_chance: read_u8(&sections, &section, &bonus_chance_key, 255),
                max_bonus_dura: read_u8(&sections, &section, &max_bonus_dura_key, 255),
            };

            mine.drops.push(drop);
            j += 1;
        }

        mines.push(mine);
        i += 1;
    }

    MinesConfig { mines }
}
