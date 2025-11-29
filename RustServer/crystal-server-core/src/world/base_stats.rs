use crate::stats::{Stat, Stats};
use once_cell::sync::Lazy;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::Job;

#[derive(Copy, Clone, Debug)]
enum StatFormula {
    Health,
    Mana,
    Weight,
    Stat,
}

#[derive(Copy, Clone, Debug)]
struct BaseStatConfig {
    stat: Stat,
    formula: StatFormula,
    base: i32,
    gain: f32,
    gain_rate: f32,
    max: i32,
}

impl BaseStatConfig {
    fn calculate(&self, job: Job, level: u16) -> i32 {
        if self.gain == 0.0 {
            return self.base;
        }

        let lvl = level as f32;
        let gain = self.gain;
        let gain_rate = self.gain_rate;
        let max_cap = if self.max > 0 {
            self.max as f32
        } else {
            i32::MAX as f32
        };

        let raw = match self.formula {
            StatFormula::Health => match job {
                Job::Warrior => {
                    self.base as f32 + ((lvl / gain) + gain_rate + (lvl / 20.0)) * lvl
                }
                _ => self.base as f32 + ((lvl / gain) + gain_rate) * lvl,
            },
            StatFormula::Mana => match job {
                Job::Wizard => {
                    self.base as f32 + (((lvl / gain) + 2.0) * 2.2 * lvl) + (lvl * gain_rate)
                }
                Job::Taoist => {
                    (self.base as f32 + (lvl / gain) * 2.2 * lvl) + (lvl * gain_rate)
                }
                _ => self.base as f32 + (lvl * gain) + (lvl * gain_rate),
            },
            StatFormula::Weight => self.base as f32 + ((lvl / gain) * lvl),
            StatFormula::Stat => self.base as f32 + (lvl / gain),
        };

        let clamped = raw.min(max_cap);
        clamped as i32
    }
}

// Hard-coded default BaseStats for each job. These mirror the defaults
// in Shared/BaseStats.cs and are used when no BaseStats*.ini files are
// present or when parsing fails. When possible we read ./Configs/
// BaseStats*.ini at runtime to match the live C# server configuration.
const WARRIOR_BASE_STATS: &[BaseStatConfig] = &[
    BaseStatConfig {
        stat: Stat::HP,
        formula: StatFormula::Health,
        base: 14,
        gain: 4.0,
        gain_rate: 4.5,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MP,
        formula: StatFormula::Mana,
        base: 11,
        gain: 3.5,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::BagWeight,
        formula: StatFormula::Weight,
        base: 50,
        gain: 3.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::WearWeight,
        formula: StatFormula::Weight,
        base: 15,
        gain: 20.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::HandWeight,
        formula: StatFormula::Weight,
        base: 12,
        gain: 13.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MinAC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 0.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MaxAC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 7.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MinDC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 5.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MaxDC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 5.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::Agility,
        formula: StatFormula::Stat,
        base: 15,
        gain: 0.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::Accuracy,
        formula: StatFormula::Stat,
        base: 5,
        gain: 0.0,
        gain_rate: 0.0,
        max: 0,
    },
];

const WIZARD_BASE_STATS: &[BaseStatConfig] = &[
    BaseStatConfig {
        stat: Stat::HP,
        formula: StatFormula::Health,
        base: 14,
        gain: 15.0,
        gain_rate: 1.8,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MP,
        formula: StatFormula::Mana,
        base: 13,
        gain: 5.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::BagWeight,
        formula: StatFormula::Weight,
        base: 50,
        gain: 5.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::WearWeight,
        formula: StatFormula::Weight,
        base: 15,
        gain: 100.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::HandWeight,
        formula: StatFormula::Weight,
        base: 12,
        gain: 90.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MinDC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 7.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MaxDC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 7.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MinMC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 7.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MaxMC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 7.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::Agility,
        formula: StatFormula::Stat,
        base: 15,
        gain: 0.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::Accuracy,
        formula: StatFormula::Stat,
        base: 5,
        gain: 0.0,
        gain_rate: 0.0,
        max: 0,
    },
];

const TAOIST_BASE_STATS: &[BaseStatConfig] = &[
    BaseStatConfig {
        stat: Stat::HP,
        formula: StatFormula::Health,
        base: 14,
        gain: 6.0,
        gain_rate: 2.5,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MP,
        formula: StatFormula::Mana,
        base: 13,
        gain: 8.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::BagWeight,
        formula: StatFormula::Weight,
        base: 50,
        gain: 4.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::WearWeight,
        formula: StatFormula::Weight,
        base: 15,
        gain: 50.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::HandWeight,
        formula: StatFormula::Weight,
        base: 12,
        gain: 42.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MinMAC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 12.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MaxMAC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 6.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MinDC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 7.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MaxDC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 7.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MinSC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 7.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MaxSC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 7.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::Agility,
        formula: StatFormula::Stat,
        base: 18,
        gain: 0.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::Accuracy,
        formula: StatFormula::Stat,
        base: 5,
        gain: 0.0,
        gain_rate: 0.0,
        max: 0,
    },
];

const ASSASSIN_BASE_STATS: &[BaseStatConfig] = &[
    BaseStatConfig {
        stat: Stat::HP,
        formula: StatFormula::Health,
        base: 14,
        gain: 4.0,
        gain_rate: 3.25,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MP,
        formula: StatFormula::Mana,
        base: 11,
        gain: 5.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::BagWeight,
        formula: StatFormula::Weight,
        base: 50,
        gain: 3.5,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::WearWeight,
        formula: StatFormula::Weight,
        base: 15,
        gain: 33.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::HandWeight,
        formula: StatFormula::Weight,
        base: 12,
        gain: 30.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MinDC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 8.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MaxDC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 8.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::Agility,
        formula: StatFormula::Stat,
        base: 20,
        gain: 0.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::Accuracy,
        formula: StatFormula::Stat,
        base: 5,
        gain: 0.0,
        gain_rate: 0.0,
        max: 0,
    },
];

const ARCHER_BASE_STATS: &[BaseStatConfig] = &[
    BaseStatConfig {
        stat: Stat::HP,
        formula: StatFormula::Health,
        base: 14,
        gain: 4.0,
        gain_rate: 3.25,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MP,
        formula: StatFormula::Mana,
        base: 11,
        gain: 4.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::BagWeight,
        formula: StatFormula::Weight,
        base: 50,
        gain: 4.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::WearWeight,
        formula: StatFormula::Weight,
        base: 15,
        gain: 33.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::HandWeight,
        formula: StatFormula::Weight,
        base: 12,
        gain: 30.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MinDC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 8.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MaxDC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 8.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MinMC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 8.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::MaxMC,
        formula: StatFormula::Stat,
        base: 0,
        gain: 8.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::Agility,
        formula: StatFormula::Stat,
        base: 15,
        gain: 0.0,
        gain_rate: 0.0,
        max: 0,
    },
    BaseStatConfig {
        stat: Stat::Accuracy,
        formula: StatFormula::Stat,
        base: 8,
        gain: 0.0,
        gain_rate: 0.0,
        max: 0,
    },
];

/// In-memory representation of the BaseStats for a single job, loaded
/// from the same ini layout as C# BaseStats*.ini. We keep this very
/// small and only support the fields actually used by the formulas.
#[derive(Clone, Debug)]
struct LoadedBaseStats {
    stats: Vec<BaseStatConfig>,
    caps: BTreeMap<Stat, i32>,
}

/// Lazily loaded BaseStats for all jobs. If the Configs directory or
/// any BaseStats*.ini files are missing, we fall back to the hard-coded
/// defaults above so that the server always starts.
static LOADED_BASE_STATS: Lazy<BTreeMap<Job, LoadedBaseStats>> = Lazy::new(|| {
    let mut map: BTreeMap<Job, LoadedBaseStats> = BTreeMap::new();

    // Helper: attempt to load a single job's ini file.
    fn load_job<P: AsRef<Path>>(
        job: Job,
        path: P,
        default_stats: &[BaseStatConfig],
    ) -> LoadedBaseStats {
        match parse_base_stats_ini(path.as_ref()) {
            Ok(loaded) => loaded,
            Err(_) => LoadedBaseStats {
                stats: default_stats.to_vec(),
                caps: default_caps_for_job(job),
            },
        }
    }

    let base_dir = Path::new("./Configs");

    let warrior_path = base_dir.join("BaseStatsWarrior.ini");
    map.insert(
        Job::Warrior,
        load_job(Job::Warrior, warrior_path, WARRIOR_BASE_STATS),
    );

    let wizard_path = base_dir.join("BaseStatsWizard.ini");
    map.insert(
        Job::Wizard,
        load_job(Job::Wizard, wizard_path, WIZARD_BASE_STATS),
    );

    let taoist_path = base_dir.join("BaseStatsTaoist.ini");
    map.insert(
        Job::Taoist,
        load_job(Job::Taoist, taoist_path, TAOIST_BASE_STATS),
    );

    let assassin_path = base_dir.join("BaseStatsAssassin.ini");
    map.insert(
        Job::Assassin,
        load_job(Job::Assassin, assassin_path, ASSASSIN_BASE_STATS),
    );

    let archer_path = base_dir.join("BaseStatsArcher.ini");
    map.insert(
        Job::Archer,
        load_job(Job::Archer, archer_path, ARCHER_BASE_STATS),
    );

    map
});

fn default_caps_for_job(_job: Job) -> BTreeMap<Stat, i32> {
    let mut caps = BTreeMap::new();
    caps.insert(Stat::MagicResist, 2);
    caps.insert(Stat::PoisonResist, 6);
    caps.insert(Stat::CriticalRate, 18);
    caps.insert(Stat::CriticalDamage, 10);
    caps.insert(Stat::Freezing, 6);
    caps.insert(Stat::PoisonAttack, 6);
    caps.insert(Stat::HealthRecovery, 8);
    caps.insert(Stat::SpellRecovery, 8);
    caps.insert(Stat::PoisonRecovery, 6);
    caps
}

fn configs_for_job(job: Job) -> &'static [BaseStatConfig] {
    if let Some(loaded) = LOADED_BASE_STATS.get(&job) {
        &loaded.stats
    } else {
        match job {
            Job::Warrior => WARRIOR_BASE_STATS,
            Job::Wizard => WIZARD_BASE_STATS,
            Job::Taoist => TAOIST_BASE_STATS,
            Job::Assassin => ASSASSIN_BASE_STATS,
            Job::Archer => ARCHER_BASE_STATS,
        }
    }
}

pub fn calc_base_stats_for_level(job: Job, level: u16) -> Stats {
    let mut stats = Stats::default();
    let configs = configs_for_job(job);

    for cfg in configs {
        let value = cfg.calculate(job, level);
        if value != 0 {
            stats.set(cfg.stat, value);
        }
    }

    stats
}

pub fn base_caps_for_job(job: Job) -> Stats {
    let mut caps = Stats::default();

    if let Some(loaded) = LOADED_BASE_STATS.get(&job) {
        for (stat, value) in &loaded.caps {
            caps.set(*stat, *value);
        }
        return caps;
    }

    // Fallback to default caps if the job wasn't present in the
    // loaded map for some reason.
    for (stat, value) in default_caps_for_job(job) {
        caps.set(stat, value);
    }

    caps
}

pub fn encode_base_stats_for_job(job: Job) -> Vec<u8> {
    let mut buf = Vec::new();

    // Job byte (MirClass).
    buf.push(job.as_u8());

    // BaseStats.Stats count.
    let configs = configs_for_job(job);
    let count: i32 = configs.len() as i32;
    buf.extend_from_slice(&count.to_le_bytes());

    // Serialize each BaseStat entry in the same layout as C# BaseStat.Save.
    for cfg in configs {
        // Stat type.
        buf.push(cfg.stat as u8);

        // StatFormula as byte.
        let formula_id = match cfg.formula {
            StatFormula::Health => 0u8,
            StatFormula::Mana => 1u8,
            StatFormula::Weight => 2u8,
            StatFormula::Stat => 3u8,
        };
        buf.push(formula_id);

        // Base / Gain / GainRate / Max.
        buf.extend_from_slice(&cfg.base.to_le_bytes());
        buf.extend_from_slice(&cfg.gain.to_le_bytes());
        buf.extend_from_slice(&cfg.gain_rate.to_le_bytes());
        buf.extend_from_slice(&cfg.max.to_le_bytes());
    }

    // Caps: serialize Stats in the same way as C# Stats.Save.
    let caps = base_caps_for_job(job);
    let caps_count: i32 = caps.values.len() as i32;
    buf.extend_from_slice(&caps_count.to_le_bytes());
    for (stat, value) in &caps.values {
        buf.push(*stat as u8);
        buf.extend_from_slice(&value.to_le_bytes());
    }

    buf
}

/// Very small ini parser specialised for the BaseStats*.ini layout used
/// by the legacy C# server. We only support the sections and keys that
/// appear in those files.
fn parse_base_stats_ini(path: &Path) -> Result<LoadedBaseStats, ()> {
    let text = fs::read_to_string(path).map_err(|_| ())?;

    let mut stats: BTreeMap<String, BaseStatConfig> = BTreeMap::new();
    let mut caps: BTreeMap<Stat, i32> = BTreeMap::new();

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
            if section == "Caps" {
                if let Ok(v) = value.parse::<i32>() {
                    // Map stat name string to Stat enum using the same
                    // textual names as the C# config (case-sensitive).
                    if let Some(stat) = stat_from_name(key) {
                        caps.insert(stat, v);
                    }
                }
                continue;
            }

            // Normal BaseStat section such as [HP], [MP], etc.
            let entry = stats.entry(section.clone()).or_insert(BaseStatConfig {
                stat: stat_from_name(section).unwrap_or(Stat::Unknown),
                formula: StatFormula::Stat,
                base: 0,
                gain: 0.0,
                gain_rate: 0.0,
                max: 0,
            });

            match key {
                "Formula" => {
                    entry.formula = match value {
                        "Health" => StatFormula::Health,
                        "Mana" => StatFormula::Mana,
                        "Weight" => StatFormula::Weight,
                        _ => StatFormula::Stat,
                    };
                }
                "Base" => {
                    if let Ok(v) = value.parse::<i32>() {
                        entry.base = v;
                    }
                }
                "Gain" => {
                    if let Ok(v) = value.parse::<f32>() {
                        entry.gain = v;
                    }
                }
                "GainRate" => {
                    if let Ok(v) = value.parse::<f32>() {
                        entry.gain_rate = v;
                    }
                }
                "Max" => {
                    if let Ok(v) = value.parse::<i32>() {
                        entry.max = v;
                    }
                }
                _ => {}
            }
        }
    }

    let mut ordered_stats: Vec<BaseStatConfig> = Vec::new();

    // Preserve roughly the same ordering as the hard-coded defaults by
    // iterating over known Stat values.
    for stat in [
        Stat::MinAC,
        Stat::MaxAC,
        Stat::MinMAC,
        Stat::MaxMAC,
        Stat::MinDC,
        Stat::MaxDC,
        Stat::MinMC,
        Stat::MaxMC,
        Stat::MinSC,
        Stat::MaxSC,
        Stat::Accuracy,
        Stat::Agility,
        Stat::HP,
        Stat::MP,
        Stat::BagWeight,
        Stat::HandWeight,
        Stat::WearWeight,
    ] {
        let name = stat_name(stat);
        if let Some(cfg) = stats.get(&name) {
            ordered_stats.push(*cfg);
        }
    }

    if ordered_stats.is_empty() {
        return Err(());
    }

    Ok(LoadedBaseStats {
        stats: ordered_stats,
        caps,
    })
}

fn stat_from_name(name: &str) -> Option<Stat> {
    match name {
        "MinAC" => Some(Stat::MinAC),
        "MaxAC" => Some(Stat::MaxAC),
        "MinMAC" => Some(Stat::MinMAC),
        "MaxMAC" => Some(Stat::MaxMAC),
        "MinDC" => Some(Stat::MinDC),
        "MaxDC" => Some(Stat::MaxDC),
        "MinMC" => Some(Stat::MinMC),
        "MaxMC" => Some(Stat::MaxMC),
        "MinSC" => Some(Stat::MinSC),
        "MaxSC" => Some(Stat::MaxSC),
        "Accuracy" => Some(Stat::Accuracy),
        "Agility" => Some(Stat::Agility),
        "HP" => Some(Stat::HP),
        "MP" => Some(Stat::MP),
        "BagWeight" => Some(Stat::BagWeight),
        "HandWeight" => Some(Stat::HandWeight),
        "WearWeight" => Some(Stat::WearWeight),
        _ => None,
    }
}

fn stat_name(stat: Stat) -> String {
    match stat {
        Stat::MinAC => "MinAC",
        Stat::MaxAC => "MaxAC",
        Stat::MinMAC => "MinMAC",
        Stat::MaxMAC => "MaxMAC",
        Stat::MinDC => "MinDC",
        Stat::MaxDC => "MaxDC",
        Stat::MinMC => "MinMC",
        Stat::MaxMC => "MaxMC",
        Stat::MinSC => "MinSC",
        Stat::MaxSC => "MaxSC",
        Stat::Accuracy => "Accuracy",
        Stat::Agility => "Agility",
        Stat::HP => "HP",
        Stat::MP => "MP",
        Stat::BagWeight => "BagWeight",
        Stat::HandWeight => "HandWeight",
        Stat::WearWeight => "WearWeight",
        _ => "Unknown",
    }
    .to_string()
}
