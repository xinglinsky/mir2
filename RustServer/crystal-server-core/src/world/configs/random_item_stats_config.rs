use once_cell::sync::Lazy;
use std::path::Path;

use crate::world::content;

#[derive(Clone, Debug, Default)]
pub struct RandomItemStat {
    pub max_dura_chance: u8,
    pub max_dura_stat_chance: u8,
    pub max_dura_max_stat: u8,
    pub max_ac_chance: u8,
    pub max_ac_stat_chance: u8,
    pub max_ac_max_stat: u8,
    pub max_mac_chance: u8,
    pub max_mac_stat_chance: u8,
    pub max_mac_max_stat: u8,
    pub max_dc_chance: u8,
    pub max_dc_stat_chance: u8,
    pub max_dc_max_stat: u8,
    pub max_mc_chance: u8,
    pub max_mc_stat_chance: u8,
    pub max_mc_max_stat: u8,
    pub max_sc_chance: u8,
    pub max_sc_stat_chance: u8,
    pub max_sc_max_stat: u8,
    pub accuracy_chance: u8,
    pub accuracy_stat_chance: u8,
    pub accuracy_max_stat: u8,
    pub agility_chance: u8,
    pub agility_stat_chance: u8,
    pub agility_max_stat: u8,
    pub hp_chance: u8,
    pub hp_stat_chance: u8,
    pub hp_max_stat: u8,
    pub mp_chance: u8,
    pub mp_stat_chance: u8,
    pub mp_max_stat: u8,
    pub strong_chance: u8,
    pub strong_stat_chance: u8,
    pub strong_max_stat: u8,
    pub magic_resist_chance: u8,
    pub magic_resist_stat_chance: u8,
    pub magic_resist_max_stat: u8,
    pub poison_resist_chance: u8,
    pub poison_resist_stat_chance: u8,
    pub poison_resist_max_stat: u8,
    pub hp_recov_chance: u8,
    pub hp_recov_stat_chance: u8,
    pub hp_recov_max_stat: u8,
    pub mp_recov_chance: u8,
    pub mp_recov_stat_chance: u8,
    pub mp_recov_max_stat: u8,
    pub poison_recov_chance: u8,
    pub poison_recov_stat_chance: u8,
    pub poison_recov_max_stat: u8,
    pub critical_rate_chance: u8,
    pub critical_rate_stat_chance: u8,
    pub critical_rate_max_stat: u8,
    pub critical_damage_chance: u8,
    pub critical_damage_stat_chance: u8,
    pub critical_damage_max_stat: u8,
    pub freeze_chance: u8,
    pub freeze_stat_chance: u8,
    pub freeze_max_stat: u8,
    pub poison_attack_chance: u8,
    pub poison_attack_stat_chance: u8,
    pub poison_attack_max_stat: u8,
    pub attack_speed_chance: u8,
    pub attack_speed_stat_chance: u8,
    pub attack_speed_max_stat: u8,
    pub luck_chance: u8,
    pub luck_stat_chance: u8,
    pub luck_max_stat: u8,
    pub curse_chance: u8,
    pub slot_chance: u8,
    pub slot_stat_chance: u8,
    pub slot_max_stat: u8,
}

#[derive(Clone, Debug, Default)]
pub struct RandomItemStatsConfig {
    pub stats: Vec<RandomItemStat>,
}

impl RandomItemStatsConfig {
    pub fn get(&self, id: u8) -> Option<&RandomItemStat> {
        self.stats.get(id as usize)
    }
}

pub static RANDOM_ITEM_STATS_CONFIG: Lazy<RandomItemStatsConfig> = Lazy::new(|| {
    load_random_item_stats_config(Path::new("./Configs/RandomItemStats.ini"))
});

pub fn random_item_stat_for_id(id: u8) -> Option<&'static RandomItemStat> {
    RANDOM_ITEM_STATS_CONFIG.get(id)
}

fn load_random_item_stats_config(path: &Path) -> RandomItemStatsConfig {
    let text = match content::read_to_string(path) {
        Ok(t) => t,
        Err(_) => {
            // If the file is missing or unreadable, return an empty config.
            return RandomItemStatsConfig::default();
        }
    };

    let mut config = RandomItemStatsConfig::default();
    let mut current_index: Option<usize> = None;

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
            if let Some(idx_str) = name.strip_prefix("Item") {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    if config.stats.len() <= idx {
                        config.stats.resize_with(idx + 1, RandomItemStat::default);
                    }
                    current_index = Some(idx);
                } else {
                    current_index = None;
                }
            } else {
                current_index = None;
            }
            continue;
        }

        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();
        if key.is_empty() {
            continue;
        }

        let Some(idx) = current_index else { continue; };
        let Some(stat) = config.stats.get_mut(idx) else { continue; };

        let parse_u8 = |s: &str| -> Option<u8> { s.parse::<u8>().ok() };

        match key {
            "MaxDuraChance" => if let Some(v) = parse_u8(value) { stat.max_dura_chance = v; },
            "MaxDuraStatChance" => if let Some(v) = parse_u8(value) { stat.max_dura_stat_chance = v; },
            "MaxDuraMaxStat" => if let Some(v) = parse_u8(value) { stat.max_dura_max_stat = v; },
            "MaxAcChance" => if let Some(v) = parse_u8(value) { stat.max_ac_chance = v; },
            "MaxAcStatChance" => if let Some(v) = parse_u8(value) { stat.max_ac_stat_chance = v; },
            "MaxAcMaxStat" => if let Some(v) = parse_u8(value) { stat.max_ac_max_stat = v; },
            "MaxMacChance" => if let Some(v) = parse_u8(value) { stat.max_mac_chance = v; },
            "MaxMacStatChance" => if let Some(v) = parse_u8(value) { stat.max_mac_stat_chance = v; },
            // Ini key is MaxMACMaxStat (MAC in caps) in the original C# server.
            "MaxMACMaxStat" => if let Some(v) = parse_u8(value) { stat.max_mac_max_stat = v; },
            "MaxDcChance" => if let Some(v) = parse_u8(value) { stat.max_dc_chance = v; },
            "MaxDcStatChance" => if let Some(v) = parse_u8(value) { stat.max_dc_stat_chance = v; },
            "MaxDcMaxStat" => if let Some(v) = parse_u8(value) { stat.max_dc_max_stat = v; },
            "MaxMcChance" => if let Some(v) = parse_u8(value) { stat.max_mc_chance = v; },
            "MaxMcStatChance" => if let Some(v) = parse_u8(value) { stat.max_mc_stat_chance = v; },
            "MaxMcMaxStat" => if let Some(v) = parse_u8(value) { stat.max_mc_max_stat = v; },
            "MaxScChance" => if let Some(v) = parse_u8(value) { stat.max_sc_chance = v; },
            "MaxScStatChance" => if let Some(v) = parse_u8(value) { stat.max_sc_stat_chance = v; },
            "MaxScMaxStat" => if let Some(v) = parse_u8(value) { stat.max_sc_max_stat = v; },
            "AccuracyChance" => if let Some(v) = parse_u8(value) { stat.accuracy_chance = v; },
            "AccuracyStatChance" => if let Some(v) = parse_u8(value) { stat.accuracy_stat_chance = v; },
            "AccuracyMaxStat" => if let Some(v) = parse_u8(value) { stat.accuracy_max_stat = v; },
            "AgilityChance" => if let Some(v) = parse_u8(value) { stat.agility_chance = v; },
            "AgilityStatChance" => if let Some(v) = parse_u8(value) { stat.agility_stat_chance = v; },
            "AgilityMaxStat" => if let Some(v) = parse_u8(value) { stat.agility_max_stat = v; },
            "HpChance" => if let Some(v) = parse_u8(value) { stat.hp_chance = v; },
            "HpStatChance" => if let Some(v) = parse_u8(value) { stat.hp_stat_chance = v; },
            "HpMaxStat" => if let Some(v) = parse_u8(value) { stat.hp_max_stat = v; },
            "MpChance" => if let Some(v) = parse_u8(value) { stat.mp_chance = v; },
            "MpStatChance" => if let Some(v) = parse_u8(value) { stat.mp_stat_chance = v; },
            "MpMaxStat" => if let Some(v) = parse_u8(value) { stat.mp_max_stat = v; },
            "StrongChance" => if let Some(v) = parse_u8(value) { stat.strong_chance = v; },
            "StrongStatChance" => if let Some(v) = parse_u8(value) { stat.strong_stat_chance = v; },
            "StrongMaxStat" => if let Some(v) = parse_u8(value) { stat.strong_max_stat = v; },
            "MagicResistChance" => if let Some(v) = parse_u8(value) { stat.magic_resist_chance = v; },
            "MagicResistStatChance" => if let Some(v) = parse_u8(value) { stat.magic_resist_stat_chance = v; },
            "MagicResistMaxStat" => if let Some(v) = parse_u8(value) { stat.magic_resist_max_stat = v; },
            "PoisonResistChance" => if let Some(v) = parse_u8(value) { stat.poison_resist_chance = v; },
            "PoisonResistStatChance" => if let Some(v) = parse_u8(value) { stat.poison_resist_stat_chance = v; },
            "PoisonResistMaxStat" => if let Some(v) = parse_u8(value) { stat.poison_resist_max_stat = v; },
            "HpRecovChance" => if let Some(v) = parse_u8(value) { stat.hp_recov_chance = v; },
            "HpRecovStatChance" => if let Some(v) = parse_u8(value) { stat.hp_recov_stat_chance = v; },
            "HpRecovMaxStat" => if let Some(v) = parse_u8(value) { stat.hp_recov_max_stat = v; },
            "MpRecovChance" => if let Some(v) = parse_u8(value) { stat.mp_recov_chance = v; },
            "MpRecovStatChance" => if let Some(v) = parse_u8(value) { stat.mp_recov_stat_chance = v; },
            "MpRecovMaxStat" => if let Some(v) = parse_u8(value) { stat.mp_recov_max_stat = v; },
            "PoisonRecovChance" => if let Some(v) = parse_u8(value) { stat.poison_recov_chance = v; },
            "PoisonRecovStatChance" => if let Some(v) = parse_u8(value) { stat.poison_recov_stat_chance = v; },
            "PoisonRecovMaxStat" => if let Some(v) = parse_u8(value) { stat.poison_recov_max_stat = v; },
            "CriticalRateChance" => if let Some(v) = parse_u8(value) { stat.critical_rate_chance = v; },
            "CriticalRateStatChance" => if let Some(v) = parse_u8(value) { stat.critical_rate_stat_chance = v; },
            "CriticalRateMaxStat" => if let Some(v) = parse_u8(value) { stat.critical_rate_max_stat = v; },
            "CriticalDamageChance" => if let Some(v) = parse_u8(value) { stat.critical_damage_chance = v; },
            "CriticalDamageStatChance" => if let Some(v) = parse_u8(value) { stat.critical_damage_stat_chance = v; },
            "CriticalDamageMaxStat" => if let Some(v) = parse_u8(value) { stat.critical_damage_max_stat = v; },
            "FreezeChance" => if let Some(v) = parse_u8(value) { stat.freeze_chance = v; },
            "FreezeStatChance" => if let Some(v) = parse_u8(value) { stat.freeze_stat_chance = v; },
            "FreezeMaxStat" => if let Some(v) = parse_u8(value) { stat.freeze_max_stat = v; },
            "PoisonAttackChance" => if let Some(v) = parse_u8(value) { stat.poison_attack_chance = v; },
            "PoisonAttackStatChance" => if let Some(v) = parse_u8(value) { stat.poison_attack_stat_chance = v; },
            "PoisonAttackMaxStat" => if let Some(v) = parse_u8(value) { stat.poison_attack_max_stat = v; },
            "AttackSpeedChance" => if let Some(v) = parse_u8(value) { stat.attack_speed_chance = v; },
            "AttackSpeedStatChance" => if let Some(v) = parse_u8(value) { stat.attack_speed_stat_chance = v; },
            "AttackSpeedMaxStat" => if let Some(v) = parse_u8(value) { stat.attack_speed_max_stat = v; },
            "LuckChance" => if let Some(v) = parse_u8(value) { stat.luck_chance = v; },
            "LuckStatChance" => if let Some(v) = parse_u8(value) { stat.luck_stat_chance = v; },
            "LuckMaxStat" => if let Some(v) = parse_u8(value) { stat.luck_max_stat = v; },
            "CurseChance" => if let Some(v) = parse_u8(value) { stat.curse_chance = v; },
            "SlotChance" => if let Some(v) = parse_u8(value) { stat.slot_chance = v; },
            "SlotStatChance" => if let Some(v) = parse_u8(value) { stat.slot_stat_chance = v; },
            "SlotMaxStat" => if let Some(v) = parse_u8(value) { stat.slot_max_stat = v; },
            _ => {}
        }
    }

    config
}
