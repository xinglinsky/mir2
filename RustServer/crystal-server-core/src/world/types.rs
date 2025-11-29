use std::fmt;
use serde::{Serialize, Deserialize};

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Job {
    Warrior = 0,
    Wizard = 1,
    Taoist = 2,
    Assassin = 3,
    Archer = 4,
}

impl Job {
    pub fn from_u8(id: u8) -> Option<Job> {
        match id {
            0 => Some(Job::Warrior),
            1 => Some(Job::Wizard),
            2 => Some(Job::Taoist),
            3 => Some(Job::Assassin),
            4 => Some(Job::Archer),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AttackMode {
    Peace = 0,
    Group = 1,
    Guild = 2,
    EnemyGuild = 3,
    RedBrown = 4,
    All = 5,
}

impl AttackMode {
    pub fn from_u8(id: u8) -> AttackMode {
        match id {
            0 => AttackMode::Peace,
            1 => AttackMode::Group,
            2 => AttackMode::Guild,
            3 => AttackMode::EnemyGuild,
            4 => AttackMode::RedBrown,
            5 => AttackMode::All,
            _ => AttackMode::All,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BuffProperty {
    None = 0,
    RemoveOnDeath = 1,
    RemoveOnExit = 2,
    Debuff = 4,
    PauseInSafeZone = 8,
}

impl BuffProperty {
    pub fn from_u8(id: u8) -> Option<BuffProperty> {
        match id {
            0 => Some(BuffProperty::None),
            1 => Some(BuffProperty::RemoveOnDeath),
            2 => Some(BuffProperty::RemoveOnExit),
            4 => Some(BuffProperty::Debuff),
            8 => Some(BuffProperty::PauseInSafeZone),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BuffStackType {
    None = 0,
    ResetDuration = 1,
    StackDuration = 2,
    StackStat = 3,
    StackStatAndDuration = 4,
    Infinite = 5,
    ResetStat = 6,
    ResetStatAndDuration = 7,
}

impl BuffStackType {
    pub fn from_u8(id: u8) -> Option<BuffStackType> {
        match id {
            0 => Some(BuffStackType::None),
            1 => Some(BuffStackType::ResetDuration),
            2 => Some(BuffStackType::StackDuration),
            3 => Some(BuffStackType::StackStat),
            4 => Some(BuffStackType::StackStatAndDuration),
            5 => Some(BuffStackType::Infinite),
            6 => Some(BuffStackType::ResetStat),
            7 => Some(BuffStackType::ResetStatAndDuration),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BuffType {
    None = 0,

    // Magics
    TemporalFlux = 1,
    Hiding = 2,
    Haste = 3,
    SwiftFeet = 4,
    Fury = 5,
    SoulShield = 6,
    BlessedArmour = 7,
    LightBody = 8,
    UltimateEnhancer = 9,
    ProtectionField = 10,
    Rage = 11,
    Curse = 12,
    MoonLight = 13,
    DarkBody = 14,
    Concentration = 15,
    VampireShot = 16,
    PoisonShot = 17,
    CounterAttack = 18,
    MentalState = 19,
    EnergyShield = 20,
    MagicBooster = 21,
    PetEnhancer = 22,
    ImmortalSkin = 23,
    MagicShield = 24,
    ElementalBarrier = 25,

    // Monster
    HornedArcherBuff = 50,
    ColdArcherBuff = 51,
    GeneralMeowMeowShield = 52,
    RhinoPriestDebuff = 53,
    PowerBeadBuff = 54,
    HornedWarriorShield = 55,
    HornedCommanderShield = 56,
    Blindness = 57,

    // Special
    GameMaster = 100,
    General = 101,
    Exp = 102,
    Drop = 103,
    Gold = 104,
    BagWeight = 105,
    Transform = 106,
    Lover = 107,
    Mentee = 108,
    Mentor = 109,
    Guild = 110,
    Prison = 111,
    Rested = 112,
    Skill = 113,
    ClearRing = 114,
    Newbie = 115,

    // Stats
    Impact = 200,
    Magic = 201,
    Taoist = 202,
    Storm = 203,
    HealthAid = 204,
    ManaAid = 205,
    Defence = 206,
    MagicDefence = 207,
    WonderDrug = 208,
    Knapsack = 209,
    FlamingSword = 210,
}

impl BuffType {
    pub fn from_u8(id: u8) -> Option<BuffType> {
        match id {
            0 => Some(BuffType::None),

            1 => Some(BuffType::TemporalFlux),
            2 => Some(BuffType::Hiding),
            3 => Some(BuffType::Haste),
            4 => Some(BuffType::SwiftFeet),
            5 => Some(BuffType::Fury),
            6 => Some(BuffType::SoulShield),
            7 => Some(BuffType::BlessedArmour),
            8 => Some(BuffType::LightBody),
            9 => Some(BuffType::UltimateEnhancer),
            10 => Some(BuffType::ProtectionField),
            11 => Some(BuffType::Rage),
            12 => Some(BuffType::Curse),
            13 => Some(BuffType::MoonLight),
            14 => Some(BuffType::DarkBody),
            15 => Some(BuffType::Concentration),
            16 => Some(BuffType::VampireShot),
            17 => Some(BuffType::PoisonShot),
            18 => Some(BuffType::CounterAttack),
            19 => Some(BuffType::MentalState),
            20 => Some(BuffType::EnergyShield),
            21 => Some(BuffType::MagicBooster),
            22 => Some(BuffType::PetEnhancer),
            23 => Some(BuffType::ImmortalSkin),
            24 => Some(BuffType::MagicShield),
            25 => Some(BuffType::ElementalBarrier),

            50 => Some(BuffType::HornedArcherBuff),
            51 => Some(BuffType::ColdArcherBuff),
            52 => Some(BuffType::GeneralMeowMeowShield),
            53 => Some(BuffType::RhinoPriestDebuff),
            54 => Some(BuffType::PowerBeadBuff),
            55 => Some(BuffType::HornedWarriorShield),
            56 => Some(BuffType::HornedCommanderShield),
            57 => Some(BuffType::Blindness),

            100 => Some(BuffType::GameMaster),
            101 => Some(BuffType::General),
            102 => Some(BuffType::Exp),
            103 => Some(BuffType::Drop),
            104 => Some(BuffType::Gold),
            105 => Some(BuffType::BagWeight),
            106 => Some(BuffType::Transform),
            107 => Some(BuffType::Lover),
            108 => Some(BuffType::Mentee),
            109 => Some(BuffType::Mentor),
            110 => Some(BuffType::Guild),
            111 => Some(BuffType::Prison),
            112 => Some(BuffType::Rested),
            113 => Some(BuffType::Skill),
            114 => Some(BuffType::ClearRing),
            115 => Some(BuffType::Newbie),

            200 => Some(BuffType::Impact),
            201 => Some(BuffType::Magic),
            202 => Some(BuffType::Taoist),
            203 => Some(BuffType::Storm),
            204 => Some(BuffType::HealthAid),
            205 => Some(BuffType::ManaAid),
            206 => Some(BuffType::Defence),
            207 => Some(BuffType::MagicDefence),
            208 => Some(BuffType::WonderDrug),
            209 => Some(BuffType::Knapsack),
            210 => Some(BuffType::FlamingSword),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}
impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Job::Warrior => "Warrior",
            Job::Wizard => "Wizard",
            Job::Taoist => "Taoist",
            Job::Assassin => "Assassin",
            Job::Archer => "Archer",
        };
        write!(f, "{}", s)
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ActorKind {
    None = 0,
    Player = 1,
    Item = 2,
    Merchant = 3,
    Spell = 4,
    Monster = 5,
    Deco = 6,
    Creature = 7,
    Hero = 8,
}

impl ActorKind {
    pub fn from_u8(id: u8) -> Option<ActorKind> {
        match id {
            0 => Some(ActorKind::None),
            1 => Some(ActorKind::Player),
            2 => Some(ActorKind::Item),
            3 => Some(ActorKind::Merchant),
            4 => Some(ActorKind::Spell),
            5 => Some(ActorKind::Monster),
            6 => Some(ActorKind::Deco),
            7 => Some(ActorKind::Creature),
            8 => Some(ActorKind::Hero),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PetKind {
    TaoistShinsu = 0,
    TaoistHolyDeva = 1,
    ArcherVampire = 2,
    ArcherToad = 3,
    ArcherSnakes = 4,
}

impl PetKind {
    pub fn from_u8(id: u8) -> Option<PetKind> {
        match id {
            0 => Some(PetKind::TaoistShinsu),
            1 => Some(PetKind::TaoistHolyDeva),
            2 => Some(PetKind::ArcherVampire),
            3 => Some(PetKind::ArcherToad),
            4 => Some(PetKind::ArcherSnakes),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Spell {
    None = 0,

    // Warrior
    Fencing = 1,
    Slaying = 2,
    Thrusting = 3,
    HalfMoon = 4,
    ShoulderDash = 5,
    TwinDrakeBlade = 6,
    Entrapment = 7,
    FlamingSword = 8,
    LionRoar = 9,
    CrossHalfMoon = 10,
    BladeAvalanche = 11,
    ProtectionField = 12,
    Rage = 13,
    CounterAttack = 14,
    SlashingBurst = 15,
    Fury = 16,
    ImmortalSkin = 17,

    // Wizard
    FireBall = 31,
    Repulsion = 32,
    ElectricShock = 33,
    GreatFireBall = 34,
    HellFire = 35,
    ThunderBolt = 36,
    Teleport = 37,
    FireBang = 38,
    FireWall = 39,
    Lightning = 40,
    FrostCrunch = 41,
    ThunderStorm = 42,
    MagicShield = 43,
    TurnUndead = 44,
    Vampirism = 45,
    IceStorm = 46,
    FlameDisruptor = 47,
    Mirroring = 48,
    FlameField = 49,
    Blizzard = 50,
    MagicBooster = 51,
    MeteorStrike = 52,
    IceThrust = 53,
    FastMove = 54,
    StormEscape = 55,

    // Taoist
    Healing = 61,
    SpiritSword = 62,
    Poisoning = 63,
    SoulFireBall = 64,
    SummonSkeleton = 65,
    Hiding = 67,
    MassHiding = 68,
    SoulShield = 69,
    Revelation = 70,
    BlessedArmour = 71,
    EnergyRepulsor = 72,
    TrapHexagon = 73,
    Purification = 74,
    MassHealing = 75,
    Hallucination = 76,
    UltimateEnhancer = 77,
    SummonShinsu = 78,
    Reincarnation = 79,
    SummonHolyDeva = 80,
    Curse = 81,
    Plague = 82,
    PoisonCloud = 83,
    EnergyShield = 84,
    PetEnhancer = 85,
    HealingCircle = 86,

    // Assassin
    FatalSword = 91,
    DoubleSlash = 92,
    Haste = 93,
    FlashDash = 94,
    LightBody = 95,
    HeavenlySword = 96,
    FireBurst = 97,
    Trap = 98,
    PoisonSword = 99,
    MoonLight = 100,
    MPEater = 101,
    SwiftFeet = 102,
    DarkBody = 103,
    Hemorrhage = 104,
    CrescentSlash = 105,
    MoonMist = 106,
    CatTongue = 107,

    // Archer
    Focus = 121,
    StraightShot = 122,
    DoubleShot = 123,
    ExplosiveTrap = 124,
    DelayedExplosion = 125,
    Meditation = 126,
    BackStep = 127,
    ElementalShot = 128,
    Concentration = 129,
    Stonetrap = 130,
    ElementalBarrier = 131,
    SummonVampire = 132,
    VampireShot = 133,
    SummonToad = 134,
    PoisonShot = 135,
    CrippleShot = 136,
    SummonSnakes = 137,
    NapalmShot = 138,
    OneWithNature = 139,
    BindingShot = 140,
    MentalState = 141,

    // Custom
    Blink = 151,
    Portal = 152,
    BattleCry = 153,
    FireBounce = 154,
    MeteorShower = 155,

    // Map Events
    DigOutZombie = 200,
    Rubble = 201,
    MapLightning = 202,
    MapLava = 203,
    MapQuake1 = 204,
    MapQuake2 = 205,
    DigOutArmadillo = 206,
    GeneralMeowMeowThunder = 207,
    StoneGolemQuake = 208,
    EarthGolemPile = 209,
    TreeQueenRoot = 210,
    TreeQueenMassRoots = 211,
    TreeQueenGroundRoots = 212,
    TucsonGeneralRock = 213,
    FlyingStatueIceTornado = 214,
    DarkOmaKingNuke = 215,
    HornedSorcererDustTornado = 216,
    HornedCommanderRockFall = 217,
    HornedCommanderRockSpike = 218,
}

impl Spell {
    pub fn from_u8(id: u8) -> Option<Spell> {
        match id {
            0 => Some(Spell::None),
            1 => Some(Spell::Fencing),
            2 => Some(Spell::Slaying),
            3 => Some(Spell::Thrusting),
            4 => Some(Spell::HalfMoon),
            5 => Some(Spell::ShoulderDash),
            6 => Some(Spell::TwinDrakeBlade),
            7 => Some(Spell::Entrapment),
            8 => Some(Spell::FlamingSword),
            9 => Some(Spell::LionRoar),
            10 => Some(Spell::CrossHalfMoon),
            11 => Some(Spell::BladeAvalanche),
            12 => Some(Spell::ProtectionField),
            13 => Some(Spell::Rage),
            14 => Some(Spell::CounterAttack),
            15 => Some(Spell::SlashingBurst),
            16 => Some(Spell::Fury),
            17 => Some(Spell::ImmortalSkin),

            31 => Some(Spell::FireBall),
            32 => Some(Spell::Repulsion),
            33 => Some(Spell::ElectricShock),
            34 => Some(Spell::GreatFireBall),
            35 => Some(Spell::HellFire),
            36 => Some(Spell::ThunderBolt),
            37 => Some(Spell::Teleport),
            38 => Some(Spell::FireBang),
            39 => Some(Spell::FireWall),
            40 => Some(Spell::Lightning),
            41 => Some(Spell::FrostCrunch),
            42 => Some(Spell::ThunderStorm),
            43 => Some(Spell::MagicShield),
            44 => Some(Spell::TurnUndead),
            45 => Some(Spell::Vampirism),
            46 => Some(Spell::IceStorm),
            47 => Some(Spell::FlameDisruptor),
            48 => Some(Spell::Mirroring),
            49 => Some(Spell::FlameField),
            50 => Some(Spell::Blizzard),
            51 => Some(Spell::MagicBooster),
            52 => Some(Spell::MeteorStrike),
            53 => Some(Spell::IceThrust),
            54 => Some(Spell::FastMove),
            55 => Some(Spell::StormEscape),

            61 => Some(Spell::Healing),
            62 => Some(Spell::SpiritSword),
            63 => Some(Spell::Poisoning),
            64 => Some(Spell::SoulFireBall),
            65 => Some(Spell::SummonSkeleton),
            67 => Some(Spell::Hiding),
            68 => Some(Spell::MassHiding),
            69 => Some(Spell::SoulShield),
            70 => Some(Spell::Revelation),
            71 => Some(Spell::BlessedArmour),
            72 => Some(Spell::EnergyRepulsor),
            73 => Some(Spell::TrapHexagon),
            74 => Some(Spell::Purification),
            75 => Some(Spell::MassHealing),
            76 => Some(Spell::Hallucination),
            77 => Some(Spell::UltimateEnhancer),
            78 => Some(Spell::SummonShinsu),
            79 => Some(Spell::Reincarnation),
            80 => Some(Spell::SummonHolyDeva),
            81 => Some(Spell::Curse),
            82 => Some(Spell::Plague),
            83 => Some(Spell::PoisonCloud),
            84 => Some(Spell::EnergyShield),
            85 => Some(Spell::PetEnhancer),
            86 => Some(Spell::HealingCircle),

            91 => Some(Spell::FatalSword),
            92 => Some(Spell::DoubleSlash),
            93 => Some(Spell::Haste),
            94 => Some(Spell::FlashDash),
            95 => Some(Spell::LightBody),
            96 => Some(Spell::HeavenlySword),
            97 => Some(Spell::FireBurst),
            98 => Some(Spell::Trap),
            99 => Some(Spell::PoisonSword),
            100 => Some(Spell::MoonLight),
            101 => Some(Spell::MPEater),
            102 => Some(Spell::SwiftFeet),
            103 => Some(Spell::DarkBody),
            104 => Some(Spell::Hemorrhage),
            105 => Some(Spell::CrescentSlash),
            106 => Some(Spell::MoonMist),
            107 => Some(Spell::CatTongue),

            121 => Some(Spell::Focus),
            122 => Some(Spell::StraightShot),
            123 => Some(Spell::DoubleShot),
            124 => Some(Spell::ExplosiveTrap),
            125 => Some(Spell::DelayedExplosion),
            126 => Some(Spell::Meditation),
            127 => Some(Spell::BackStep),
            128 => Some(Spell::ElementalShot),
            129 => Some(Spell::Concentration),
            130 => Some(Spell::Stonetrap),
            131 => Some(Spell::ElementalBarrier),
            132 => Some(Spell::SummonVampire),
            133 => Some(Spell::VampireShot),
            134 => Some(Spell::SummonToad),
            135 => Some(Spell::PoisonShot),
            136 => Some(Spell::CrippleShot),
            137 => Some(Spell::SummonSnakes),
            138 => Some(Spell::NapalmShot),
            139 => Some(Spell::OneWithNature),
            140 => Some(Spell::BindingShot),
            141 => Some(Spell::MentalState),

            151 => Some(Spell::Blink),
            152 => Some(Spell::Portal),
            153 => Some(Spell::BattleCry),
            154 => Some(Spell::FireBounce),
            155 => Some(Spell::MeteorShower),

            200 => Some(Spell::DigOutZombie),
            201 => Some(Spell::Rubble),
            202 => Some(Spell::MapLightning),
            203 => Some(Spell::MapLava),
            204 => Some(Spell::MapQuake1),
            205 => Some(Spell::MapQuake2),
            206 => Some(Spell::DigOutArmadillo),
            207 => Some(Spell::GeneralMeowMeowThunder),
            208 => Some(Spell::StoneGolemQuake),
            209 => Some(Spell::EarthGolemPile),
            210 => Some(Spell::TreeQueenRoot),
            211 => Some(Spell::TreeQueenMassRoots),
            212 => Some(Spell::TreeQueenGroundRoots),
            213 => Some(Spell::TucsonGeneralRock),
            214 => Some(Spell::FlyingStatueIceTornado),
            215 => Some(Spell::DarkOmaKingNuke),
            216 => Some(Spell::HornedSorcererDustTornado),
            217 => Some(Spell::HornedCommanderRockFall),
            218 => Some(Spell::HornedCommanderRockSpike),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}
