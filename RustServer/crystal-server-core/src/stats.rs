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

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn encode(&self, buf: &mut Vec<u8>) {
        let count = self.values.len() as i32;
        buf.extend_from_slice(&count.to_le_bytes());

        for (stat, value) in &self.values {
            buf.push(*stat as u8);
            buf.extend_from_slice(&value.to_le_bytes());
        }
    }
}

impl Stat {
    pub fn from_u8(id: u8) -> Stat {
        match id {
            0 => Stat::MinAC,
            1 => Stat::MaxAC,
            2 => Stat::MinMAC,
            3 => Stat::MaxMAC,
            4 => Stat::MinDC,
            5 => Stat::MaxDC,
            6 => Stat::MinMC,
            7 => Stat::MaxMC,
            8 => Stat::MinSC,
            9 => Stat::MaxSC,
            10 => Stat::Accuracy,
            11 => Stat::Agility,
            12 => Stat::HP,
            13 => Stat::MP,
            14 => Stat::AttackSpeed,
            15 => Stat::Luck,
            16 => Stat::BagWeight,
            17 => Stat::HandWeight,
            18 => Stat::WearWeight,
            19 => Stat::Reflect,
            20 => Stat::Strong,
            21 => Stat::Holy,
            22 => Stat::Freezing,
            23 => Stat::PoisonAttack,
            30 => Stat::MagicResist,
            31 => Stat::PoisonResist,
            32 => Stat::HealthRecovery,
            33 => Stat::SpellRecovery,
            34 => Stat::PoisonRecovery,
            35 => Stat::CriticalRate,
            36 => Stat::CriticalDamage,
            40 => Stat::MaxACRatePercent,
            41 => Stat::MaxMACRatePercent,
            42 => Stat::MaxDCRatePercent,
            43 => Stat::MaxMCRatePercent,
            44 => Stat::MaxSCRatePercent,
            45 => Stat::AttackSpeedRatePercent,
            46 => Stat::HPRatePercent,
            47 => Stat::MPRatePercent,
            48 => Stat::HPDrainRatePercent,
            100 => Stat::ExpRatePercent,
            101 => Stat::ItemDropRatePercent,
            102 => Stat::GoldDropRatePercent,
            103 => Stat::MineRatePercent,
            104 => Stat::GemRatePercent,
            105 => Stat::FishRatePercent,
            106 => Stat::CraftRatePercent,
            107 => Stat::SkillGainMultiplier,
            108 => Stat::AttackBonus,
            120 => Stat::LoverExpRatePercent,
            121 => Stat::MentorDamageRatePercent,
            123 => Stat::MentorExpRatePercent,
            124 => Stat::DamageReductionPercent,
            125 => Stat::EnergyShieldPercent,
            126 => Stat::EnergyShieldHPGain,
            127 => Stat::ManaPenaltyPercent,
            128 => Stat::TeleportManaPenaltyPercent,
            129 => Stat::Hero,
            _ => Stat::Unknown,
        }
    }
}
