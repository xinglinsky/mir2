use std::collections::BTreeMap;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stat {
    MinAC = 0,
    MaxAC = 1,
    MinMAC = 2,
    MaxMAC = 3,
    MinDC = 4,
    MaxDC = 5,
    MinMC = 6,
    MaxMC = 7,
    MinSC = 8,
    MaxSC = 9,

    Accuracy = 10,
    Agility = 11,
    HP = 12,
    MP = 13,
    AttackSpeed = 14,
    Luck = 15,
    BagWeight = 16,
    HandWeight = 17,
    WearWeight = 18,
    Reflect = 19,
    Strong = 20,
    Holy = 21,
    Freezing = 22,
    PoisonAttack = 23,

    MagicResist = 30,
    PoisonResist = 31,
    HealthRecovery = 32,
    SpellRecovery = 33,
    PoisonRecovery = 34,
    CriticalRate = 35,
    CriticalDamage = 36,

    MaxACRatePercent = 40,
    MaxMACRatePercent = 41,
    MaxDCRatePercent = 42,
    MaxMCRatePercent = 43,
    MaxSCRatePercent = 44,
    AttackSpeedRatePercent = 45,
    HPRatePercent = 46,
    MPRatePercent = 47,
    HPDrainRatePercent = 48,

    ExpRatePercent = 100,
    ItemDropRatePercent = 101,
    GoldDropRatePercent = 102,
    MineRatePercent = 103,
    GemRatePercent = 104,
    FishRatePercent = 105,
    CraftRatePercent = 106,
    SkillGainMultiplier = 107,
    AttackBonus = 108,

    LoverExpRatePercent = 120,
    MentorDamageRatePercent = 121,
    MentorExpRatePercent = 123,
    DamageReductionPercent = 124,
    EnergyShieldPercent = 125,
    EnergyShieldHPGain = 126,
    ManaPenaltyPercent = 127,
    TeleportManaPenaltyPercent = 128,
    Hero = 129,

    Unknown = 255,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub values: BTreeMap<Stat, i32>,
}

impl Stats {
    pub fn get(&self, stat: Stat) -> i32 {
        *self.values.get(&stat).unwrap_or(&0)
    }

    pub fn set(&mut self, stat: Stat, value: i32) {
        if value == 0 {
            self.values.remove(&stat);
        } else {
            self.values.insert(stat, value);
        }
    }

    pub fn add(&mut self, other: &Stats) {
        for (stat, value) in &other.values {
            let current = self.get(*stat);
            self.set(*stat, current + value);
        }
    }
}
