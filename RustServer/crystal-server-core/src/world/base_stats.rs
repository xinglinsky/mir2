use crate::stats::{Stat, Stats};
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

fn configs_for_job(job: Job) -> &'static [BaseStatConfig] {
    match job {
        Job::Warrior => WARRIOR_BASE_STATS,
        Job::Wizard => WIZARD_BASE_STATS,
        Job::Taoist => TAOIST_BASE_STATS,
        Job::Assassin => ASSASSIN_BASE_STATS,
        Job::Archer => ARCHER_BASE_STATS,
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

pub fn base_caps_for_job(_job: Job) -> Stats {
    let mut caps = Stats::default();

    caps.set(Stat::MagicResist, 2);
    caps.set(Stat::PoisonResist, 6);
    caps.set(Stat::CriticalRate, 18);
    caps.set(Stat::CriticalDamage, 10);
    caps.set(Stat::Freezing, 6);
    caps.set(Stat::PoisonAttack, 6);
    caps.set(Stat::HealthRecovery, 8);
    caps.set(Stat::SpellRecovery, 8);
    caps.set(Stat::PoisonRecovery, 6);

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
