use std::io::{self, Cursor, Read};

use crate::io::{
    read_bool,
    read_i32_le,
    read_i64_le,
    read_u16_le,
    read_u32_le,
    write_bool,
    write_i32_le,
    write_i64_le,
    write_u16_le,
    write_u32_le,
};
use crate::item_types::UserItemData;
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

use super::flat;

#[derive(Clone, Debug)]
pub struct SStruck {
    pub attacker_id: u32,
}

impl SStruck {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.attacker_id)?;
        Ok(RawPacket {
            id: ServerPacketId::Struck as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let attacker_id = read_u32_le(&mut c)?;
        Ok(SStruck { attacker_id })
    }
}

#[derive(Clone, Debug)]
pub struct SHealthChanged {
    pub hp: i32,
    pub mp: i32,
}

impl SHealthChanged {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.hp)?;
        write_i32_le(&mut buf, self.mp)?;
        Ok(RawPacket {
            id: ServerPacketId::HealthChanged as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let hp = read_i32_le(&mut c)?;
        let mp = read_i32_le(&mut c)?;
        Ok(SHealthChanged { hp, mp })
    }
}

#[derive(Clone, Debug)]
pub struct SHeroHealthChanged {
    pub hp: i32,
    pub mp: i32,
}

impl SHeroHealthChanged {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.hp)?;
        write_i32_le(&mut buf, self.mp)?;
        Ok(RawPacket {
            id: ServerPacketId::HeroHealthChanged as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let hp = read_i32_le(&mut c)?;
        let mp = read_i32_le(&mut c)?;
        Ok(SHeroHealthChanged { hp, mp })
    }
}

#[derive(Clone, Debug)]
pub struct SDamageIndicator {
    pub damage: i32,
    pub damage_type: u8,
    pub object_id: u32,
}

impl SDamageIndicator {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.damage)?;
        buf.push(self.damage_type);
        write_u32_le(&mut buf, self.object_id)?;
        Ok(RawPacket {
            id: ServerPacketId::DamageIndicator as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let damage = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let damage_type = one[0];
        let object_id = read_u32_le(&mut c)?;
        Ok(SDamageIndicator {
            damage,
            damage_type,
            object_id,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SColourChanged {
    pub name_colour_argb: i32,
}

impl SColourChanged {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.name_colour_argb)?;
        Ok(RawPacket {
            id: ServerPacketId::ColourChanged as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name_colour_argb = read_i32_le(&mut c)?;
        Ok(SColourChanged { name_colour_argb })
    }
}

#[derive(Clone, Debug)]
pub struct SGainedItem {
    /// Raw bytes representing UserItem.Save(writer) payload.
    pub item_bytes: Vec<u8>,
}

impl SGainedItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        Ok(RawPacket {
            id: ServerPacketId::GainedItem as i16,
            payload: self.item_bytes.clone(),
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SGainedItem {
            item_bytes: payload.to_vec(),
        })
    }
}

impl SGainedItem {
    pub fn decode_user_item(&self) -> io::Result<UserItemData> {
        UserItemData::decode_from_bytes(&self.item_bytes)
    }

    pub fn from_user_item(item: &UserItemData) -> io::Result<Self> {
        let bytes = item.encode_to_bytes()?;
        Ok(SGainedItem { item_bytes: bytes })
    }
}

#[derive(Clone, Debug)]
pub struct SGainedGold {
    pub gold: u32,
}

impl SGainedGold {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.gold)?;
        Ok(RawPacket {
            id: ServerPacketId::GainedGold as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let gold = read_u32_le(&mut c)?;
        Ok(SGainedGold { gold })
    }
}

#[derive(Clone, Debug)]
pub struct SLoseGold {
    pub gold: u32,
}

impl SLoseGold {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.gold)?;
        Ok(RawPacket {
            id: ServerPacketId::LoseGold as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let gold = read_u32_le(&mut c)?;
        Ok(SLoseGold { gold })
    }
}

#[derive(Clone, Debug)]
pub struct SGainedCredit {
    pub credit: u32,
}

impl SGainedCredit {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.credit)?;
        Ok(RawPacket {
            id: ServerPacketId::GainedCredit as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let credit = read_u32_le(&mut c)?;
        Ok(SGainedCredit { credit })
    }
}

#[derive(Clone, Debug)]
pub struct SLoseCredit {
    pub credit: u32,
}

impl SLoseCredit {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.credit)?;
        Ok(RawPacket {
            id: ServerPacketId::LoseCredit as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let credit = read_u32_le(&mut c)?;
        Ok(SLoseCredit { credit })
    }
}

#[derive(Clone, Debug)]
pub struct SGainExperience {
    pub amount: u32,
}

impl SGainExperience {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.amount)?;
        Ok(RawPacket {
            id: ServerPacketId::GainExperience as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let amount = read_u32_le(&mut c)?;
        Ok(SGainExperience { amount })
    }
}

#[derive(Clone, Debug)]
pub struct SGainHeroExperience {
    pub amount: u32,
}

impl SGainHeroExperience {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.amount)?;
        Ok(RawPacket {
            id: ServerPacketId::GainHeroExperience as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let amount = read_u32_le(&mut c)?;
        Ok(SGainHeroExperience { amount })
    }
}

#[derive(Clone, Debug)]
pub struct SLevelChanged {
    pub level: u16,
    pub experience: i64,
    pub max_experience: i64,
}

impl SLevelChanged {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u16_le(&mut buf, self.level)?;
        write_i64_le(&mut buf, self.experience)?;
        write_i64_le(&mut buf, self.max_experience)?;
        Ok(RawPacket {
            id: ServerPacketId::LevelChanged as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let level = read_u16_le(&mut c)?;
        let experience = read_i64_le(&mut c)?;
        let max_experience = read_i64_le(&mut c)?;
        Ok(SLevelChanged {
            level,
            experience,
            max_experience,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SHeroLevelChanged {
    pub level: u16,
    pub experience: i64,
    pub max_experience: i64,
}

impl SHeroLevelChanged {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u16_le(&mut buf, self.level)?;
        write_i64_le(&mut buf, self.experience)?;
        write_i64_le(&mut buf, self.max_experience)?;
        Ok(RawPacket {
            id: ServerPacketId::HeroLevelChanged as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let level = read_u16_le(&mut c)?;
        let experience = read_i64_le(&mut c)?;
        let max_experience = read_i64_le(&mut c)?;
        Ok(SHeroLevelChanged {
            level,
            experience,
            max_experience,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SDeath {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SDeath {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::Death as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SDeath {
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SRevived;

impl SRevived {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::Revived as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SRevived payload must be empty",
            ));
        }
        Ok(SRevived)
    }
}

#[derive(Clone, Debug)]
pub struct SObjectRevived {
    pub object_id: u32,
    pub effect: bool,
}

impl SObjectRevived {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_bool(&mut buf, self.effect)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectRevived as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let effect = read_bool(&mut c)?;
        Ok(SObjectRevived { object_id, effect })
    }
}

// Re-export remaining status-related packets from the legacy flat module.
pub use flat::SPoisoned;
pub use flat::SAllowObserve;
pub use flat::STimeOfDay;
pub use flat::SSendOutputMessage;
pub use flat::SRemoveDelayedExplosion;
pub use flat::SInTrapRock;
pub use flat::SSetConcentration;
pub use flat::SSetElemental;
pub use flat::SMountUpdate;
pub use flat::STransformUpdate;
pub use flat::SEquipSlotItem;
pub use flat::SFishingUpdate;
pub use flat::SSetBindingShot;
pub use crate::user::group::{
    SSwitchGroup,
    SDeleteGroup,
    SCancelReincarnation,
    SRequestReincarnation,
    SDeleteMember,
    SGroupInvite,
    SAddMember,
};
