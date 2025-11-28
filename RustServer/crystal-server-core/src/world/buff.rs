use crate::stats::Stats;
use crate::world::types::{BuffType, BuffProperty, BuffStackType};

#[derive(Clone, Debug)]
pub struct PlayerBuff {
    pub buff_type: BuffType,
    pub caster_id: Option<u32>,
    pub visible: bool,
    pub expire_time_ms: i64,
    pub stats: Stats,
    pub values: Vec<i32>,
    pub infinite: bool,
    pub paused: bool,
    pub pause_remaining_ms: i64,
}

impl PlayerBuff {
    pub fn new(buff_type: BuffType, expire_time_ms: i64) -> Self {
        Self {
            buff_type,
            caster_id: None,
            visible: true,
            expire_time_ms,
            stats: Stats::default(),
            values: Vec::new(),
            infinite: false,
            paused: false,
            pause_remaining_ms: 0,
        }
    }

    pub fn encode(&self, buf: &mut Vec<u8>) {
        buf.push(self.buff_type.as_u8());
        buf.push(if self.visible { 1 } else { 0 });
        buf.extend_from_slice(&self.caster_id.unwrap_or(0).to_le_bytes());
        buf.extend_from_slice(&self.expire_time_ms.to_le_bytes());
        buf.push(if self.infinite { 1 } else { 0 });
        buf.push(if self.paused { 1 } else { 0 });

        self.stats.encode(buf);

        let count = self.values.len() as i32;
        buf.extend_from_slice(&count.to_le_bytes());
        for val in &self.values {
            buf.extend_from_slice(&val.to_le_bytes());
        }
    }
}

#[derive(Clone, Debug)]
pub struct BuffInfo {
    pub buff_type: BuffType,
    pub stack_type: BuffStackType,
    pub properties_mask: u8,
    pub icon: i32,
    pub visible: bool,
}

impl BuffInfo {
    pub fn has_property(&self, property: BuffProperty) -> bool {
        let mask = property.as_u8();
        if mask == 0 {
            self.properties_mask == 0
        } else {
            (self.properties_mask & mask) != 0
        }
    }
}

pub fn load_default_buff_infos(game_master_effect: bool) -> Vec<BuffInfo> {
    use BuffProperty as P;
    use BuffStackType as S;
    use BuffType as T;

    let mut list = Vec::new();

    // Magics
    list.push(BuffInfo { buff_type: T::TemporalFlux, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Hiding, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Haste, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::SwiftFeet, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::Fury, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::SoulShield, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::BlessedArmour, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::LightBody, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::UltimateEnhancer, stack_type: S::ResetStatAndDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::ProtectionField, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Rage, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Curse, stack_type: S::ResetDuration, properties_mask: P::RemoveOnDeath.as_u8() | P::Debuff.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::MoonLight, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::DarkBody, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::Concentration, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::VampireShot, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::PoisonShot, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::CounterAttack, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::MentalState, stack_type: S::Infinite, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::EnergyShield, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::MagicBooster, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::PetEnhancer, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::ImmortalSkin, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::MagicShield, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::ElementalBarrier, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });

    // Monsters
    list.push(BuffInfo { buff_type: T::HornedArcherBuff, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::ColdArcherBuff, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::GeneralMeowMeowShield, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::RhinoPriestDebuff, stack_type: S::ResetDuration, properties_mask: P::Debuff.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::PowerBeadBuff, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::HornedWarriorShield, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::HornedCommanderShield, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: true });
    list.push(BuffInfo { buff_type: T::Blindness, stack_type: S::ResetDuration, properties_mask: P::RemoveOnDeath.as_u8() | P::Debuff.as_u8(), icon: 0, visible: false });

    // Special
    list.push(BuffInfo { buff_type: T::GameMaster, stack_type: S::Infinite, properties_mask: P::None.as_u8(), icon: 0, visible: game_master_effect });
    list.push(BuffInfo { buff_type: T::Mentee, stack_type: S::Infinite, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Mentor, stack_type: S::Infinite, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Guild, stack_type: S::Infinite, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Skill, stack_type: S::Infinite, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::ClearRing, stack_type: S::Infinite, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Transform, stack_type: S::None, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Lover, stack_type: S::Infinite, properties_mask: P::RemoveOnExit.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Rested, stack_type: S::ResetDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Prison, stack_type: S::None, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::General, stack_type: S::None, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Newbie, stack_type: S::Infinite, properties_mask: P::RemoveOnExit.as_u8(), icon: 0, visible: false });

    // Stats
    list.push(BuffInfo { buff_type: T::Exp, stack_type: S::StackDuration, properties_mask: P::PauseInSafeZone.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Drop, stack_type: S::StackDuration, properties_mask: P::PauseInSafeZone.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Gold, stack_type: S::StackDuration, properties_mask: P::PauseInSafeZone.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::BagWeight, stack_type: S::StackDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Impact, stack_type: S::StackDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Magic, stack_type: S::StackDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Taoist, stack_type: S::StackDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Storm, stack_type: S::StackDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::HealthAid, stack_type: S::StackDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::ManaAid, stack_type: S::StackDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Defence, stack_type: S::StackDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::MagicDefence, stack_type: S::StackDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::WonderDrug, stack_type: S::StackDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });
    list.push(BuffInfo { buff_type: T::Knapsack, stack_type: S::StackDuration, properties_mask: P::None.as_u8(), icon: 0, visible: false });

    list
}
