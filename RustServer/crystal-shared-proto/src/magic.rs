// Magic/skill-related packets compatible with the C# Crystal implementation.

use std::io::{self, Cursor, Read};

use crate::io::{
    read_bool, read_i32_le, read_i64_le, read_u16_le, read_u32_le, write_bool, write_i32_le,
    write_i64_le, write_u16_le, write_u32_le,
};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

/// Raw bytes representing ClientMagic.Save(writer) followed by Hero bool.
#[derive(Clone, Debug)]
pub struct SNewMagic {
    pub magic_bytes: Vec<u8>,
}

impl SNewMagic {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::NewMagic as i16,
            payload: self.magic_bytes.clone(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SNewMagic {
            magic_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SRemoveMagic {
    pub place_id: i32,
}

impl SRemoveMagic {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.place_id)?;
        Ok(RawPacket {
            id: ServerPacketId::RemoveMagic as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let place_id = read_i32_le(&mut c)?;
        Ok(SRemoveMagic { place_id })
    }
}

#[derive(Clone, Debug)]
pub struct SMagicLeveled {
    pub object_id: u32,
    pub spell: u8,
    pub level: u8,
    pub experience: u16,
}

impl SMagicLeveled {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        buf.push(self.spell);
        buf.push(self.level);
        write_u16_le(&mut buf, self.experience)?;
        Ok(RawPacket {
            id: ServerPacketId::MagicLeveled as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let spell = one[0];
        c.read_exact(&mut one)?;
        let level = one[0];
        let experience = read_u16_le(&mut c)?;
        Ok(SMagicLeveled {
            object_id,
            spell,
            level,
            experience,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SMagic {
    pub spell: u8,
    pub target_id: u32,
    pub target_x: i32,
    pub target_y: i32,
    pub cast: bool,
    pub level: u8,
    pub secondary_target_ids: Vec<u32>,
}

impl SMagic {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.spell);
        write_u32_le(&mut buf, self.target_id)?;
        write_i32_le(&mut buf, self.target_x)?;
        write_i32_le(&mut buf, self.target_y)?;
        write_bool(&mut buf, self.cast)?;
        buf.push(self.level);

        let count: i32 = self
            .secondary_target_ids
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many secondary ids"))?;
        write_i32_le(&mut buf, count)?;
        for id in &self.secondary_target_ids {
            write_u32_le(&mut buf, *id)?;
        }

        Ok(RawPacket {
            id: ServerPacketId::Magic as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let spell = one[0];
        let target_id = read_u32_le(&mut c)?;
        let target_x = read_i32_le(&mut c)?;
        let target_y = read_i32_le(&mut c)?;
        let cast = read_bool(&mut c)?;
        c.read_exact(&mut one)?;
        let level = one[0];

        let count = read_i32_le(&mut c)?;
        if count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative secondary target count",
            ));
        }
        let mut secondary_target_ids = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let id = read_u32_le(&mut c)?;
            secondary_target_ids.push(id);
        }

        Ok(SMagic {
            spell,
            target_id,
            target_x,
            target_y,
            cast,
            level,
            secondary_target_ids,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SMagicDelay {
    pub object_id: u32,
    pub spell: u8,
    pub delay: i64,
}

impl SMagicDelay {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        buf.push(self.spell);
        write_i64_le(&mut buf, self.delay)?;
        Ok(RawPacket {
            id: ServerPacketId::MagicDelay as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let spell = one[0];
        let delay = read_i64_le(&mut c)?;
        Ok(SMagicDelay {
            object_id,
            spell,
            delay,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SMagicCast {
    pub spell: u8,
}

impl SMagicCast {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.spell);
        RawPacket {
            id: ServerPacketId::MagicCast as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SMagicCast payload must be exactly 1 byte",
            ));
        }
        Ok(SMagicCast { spell: payload[0] })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectMagic {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
    pub spell: u8,
    pub target_id: u32,
    pub target_x: i32,
    pub target_y: i32,
    pub cast: bool,
    pub level: u8,
    pub self_broadcast: bool,
    pub secondary_target_ids: Vec<u32>,
}

impl SObjectMagic {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);

        buf.push(self.spell);
        write_u32_le(&mut buf, self.target_id)?;
        write_i32_le(&mut buf, self.target_x)?;
        write_i32_le(&mut buf, self.target_y)?;
        write_bool(&mut buf, self.cast)?;
        buf.push(self.level);
        write_bool(&mut buf, self.self_broadcast)?;

        let count: i32 = self
            .secondary_target_ids
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many secondary ids"))?;
        write_i32_le(&mut buf, count)?;
        for id in &self.secondary_target_ids {
            write_u32_le(&mut buf, *id)?;
        }

        Ok(RawPacket {
            id: ServerPacketId::ObjectMagic as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];

        c.read_exact(&mut one)?;
        let spell = one[0];
        let target_id = read_u32_le(&mut c)?;
        let target_x = read_i32_le(&mut c)?;
        let target_y = read_i32_le(&mut c)?;
        let cast = read_bool(&mut c)?;
        c.read_exact(&mut one)?;
        let level = one[0];
        let self_broadcast = read_bool(&mut c)?;

        let count = read_i32_le(&mut c)?;
        if count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative secondary target count",
            ));
        }
        let mut secondary_target_ids = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let id = read_u32_le(&mut c)?;
            secondary_target_ids.push(id);
        }

        Ok(SObjectMagic {
            object_id,
            location_x,
            location_y,
            direction,
            spell,
            target_id,
            target_x,
            target_y,
            cast,
            level,
            self_broadcast,
            secondary_target_ids,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectEffect {
    pub object_id: u32,
    pub effect: u8,
    pub effect_type: u32,
    pub delay_time: u32,
    pub time: u32,
}

impl SObjectEffect {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        buf.push(self.effect);
        write_u32_le(&mut buf, self.effect_type)?;
        write_u32_le(&mut buf, self.delay_time)?;
        write_u32_le(&mut buf, self.time)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectEffect as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let effect = one[0];
        let effect_type = read_u32_le(&mut c)?;
        let delay_time = read_u32_le(&mut c)?;
        let time = read_u32_le(&mut c)?;
        Ok(SObjectEffect {
            object_id,
            effect,
            effect_type,
            delay_time,
            time,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectProjectile {
    pub spell: u8,
    pub source: u32,
    pub destination: u32,
}

impl SObjectProjectile {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.spell);
        write_u32_le(&mut buf, self.source)?;
        write_u32_le(&mut buf, self.destination)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectProjectile as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let spell = one[0];
        let source = read_u32_le(&mut c)?;
        let destination = read_u32_le(&mut c)?;
        Ok(SObjectProjectile {
            spell,
            source,
            destination,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectSpell {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub spell: u8,
    pub direction: u8,
    pub param: bool,
}

impl SObjectSpell {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.spell);
        buf.push(self.direction);
        write_bool(&mut buf, self.param)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectSpell as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let spell = one[0];
        c.read_exact(&mut one)?;
        let direction = one[0];
        let param = read_bool(&mut c)?;
        Ok(SObjectSpell {
            object_id,
            location_x,
            location_y,
            spell,
            direction,
            param,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SRangeAttack {
    pub target_id: u32,
    pub target_x: i32,
    pub target_y: i32,
    pub spell: u8,
}

impl SRangeAttack {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.target_id)?;
        write_i32_le(&mut buf, self.target_x)?;
        write_i32_le(&mut buf, self.target_y)?;
        buf.push(self.spell);
        Ok(RawPacket {
            id: ServerPacketId::RangeAttack as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let target_id = read_u32_le(&mut c)?;
        let target_x = read_i32_le(&mut c)?;
        let target_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let spell = one[0];
        Ok(SRangeAttack {
            target_id,
            target_x,
            target_y,
            spell,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectRangeAttack {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
    pub target_id: u32,
    pub target_x: i32,
    pub target_y: i32,
    pub attack_type: u8,
    pub spell: u8,
    pub level: u8,
}

impl SObjectRangeAttack {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        write_u32_le(&mut buf, self.target_id)?;
        write_i32_le(&mut buf, self.target_x)?;
        write_i32_le(&mut buf, self.target_y)?;
        buf.push(self.attack_type);
        buf.push(self.spell);
        buf.push(self.level);
        Ok(RawPacket {
            id: ServerPacketId::ObjectRangeAttack as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        let target_id = read_u32_le(&mut c)?;
        let target_x = read_i32_le(&mut c)?;
        let target_y = read_i32_le(&mut c)?;
        c.read_exact(&mut one)?;
        let attack_type = one[0];
        c.read_exact(&mut one)?;
        let spell = one[0];
        c.read_exact(&mut one)?;
        let level = one[0];
        Ok(SObjectRangeAttack {
            object_id,
            location_x,
            location_y,
            direction,
            target_id,
            target_x,
            target_y,
            attack_type,
            spell,
            level,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SAddBuff {
    /// Raw bytes representing ClientBuff.Save(writer) payload.
    pub buff_bytes: Vec<u8>,
}

impl SAddBuff {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::AddBuff as i16,
            payload: self.buff_bytes.clone(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SAddBuff {
            buff_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SRemoveBuff {
    pub buff_type: u8,
    pub object_id: u32,
}

impl SRemoveBuff {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.buff_type);
        write_u32_le(&mut buf, self.object_id)?;
        Ok(RawPacket {
            id: ServerPacketId::RemoveBuff as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let buff_type = one[0];
        let object_id = read_u32_le(&mut c)?;
        Ok(SRemoveBuff { buff_type, object_id })
    }
}

#[derive(Clone, Debug)]
pub struct SPauseBuff {
    pub buff_type: u8,
    pub object_id: u32,
    pub paused: bool,
}

impl SPauseBuff {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.buff_type);
        write_u32_le(&mut buf, self.object_id)?;
        write_bool(&mut buf, self.paused)?;
        Ok(RawPacket {
            id: ServerPacketId::PauseBuff as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let buff_type = one[0];
        let object_id = read_u32_le(&mut c)?;
        let paused = read_bool(&mut c)?;
        Ok(SPauseBuff {
            buff_type,
            object_id,
            paused,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectHidden {
    pub object_id: u32,
    pub hidden: bool,
}

impl SObjectHidden {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_bool(&mut buf, self.hidden)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectHidden as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let hidden = read_bool(&mut c)?;
        Ok(SObjectHidden { object_id, hidden })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_magic_roundtrip() {
        let p = SNewMagic {
            magic_bytes: vec![1, 2, 3, 4],
        };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::NewMagic as i16);
        assert_eq!(raw.payload, p.magic_bytes);

        let decoded = SNewMagic::decode(&raw.payload).expect("decode SNewMagic");
        assert_eq!(decoded.magic_bytes, p.magic_bytes);
    }

    #[test]
    fn remove_magic_roundtrip() {
        let p = SRemoveMagic { place_id: 42 };

        let raw = p.encode().expect("encode SRemoveMagic");
        assert_eq!(raw.id, ServerPacketId::RemoveMagic as i16);

        let decoded = SRemoveMagic::decode(&raw.payload).expect("decode SRemoveMagic");
        assert_eq!(decoded.place_id, p.place_id);
    }

    #[test]
    fn magic_leveled_roundtrip() {
        let p = SMagicLeveled {
            object_id: 7,
            spell: 3,
            level: 2,
            experience: 999,
        };

        let raw = p.encode().expect("encode SMagicLeveled");
        assert_eq!(raw.id, ServerPacketId::MagicLeveled as i16);

        let decoded = SMagicLeveled::decode(&raw.payload).expect("decode SMagicLeveled");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.spell, p.spell);
        assert_eq!(decoded.level, p.level);
        assert_eq!(decoded.experience, p.experience);
    }

    #[test]
    fn magic_roundtrip() {
        let p = SMagic {
            spell: 5,
            target_id: 123,
            target_x: 10,
            target_y: 20,
            cast: true,
            level: 2,
            secondary_target_ids: vec![9, 8, 7],
        };

        let raw = p.encode().expect("encode SMagic");
        assert_eq!(raw.id, ServerPacketId::Magic as i16);

        let decoded = SMagic::decode(&raw.payload).expect("decode SMagic");
        assert_eq!(decoded.spell, p.spell);
        assert_eq!(decoded.target_id, p.target_id);
        assert_eq!(decoded.target_x, p.target_x);
        assert_eq!(decoded.target_y, p.target_y);
        assert_eq!(decoded.cast, p.cast);
        assert_eq!(decoded.level, p.level);
        assert_eq!(decoded.secondary_target_ids, p.secondary_target_ids);
    }

    #[test]
    fn magic_delay_roundtrip() {
        let p = SMagicDelay {
            object_id: 3,
            spell: 6,
            delay: 123456,
        };

        let raw = p.encode().expect("encode SMagicDelay");
        assert_eq!(raw.id, ServerPacketId::MagicDelay as i16);

        let decoded = SMagicDelay::decode(&raw.payload).expect("decode SMagicDelay");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.spell, p.spell);
        assert_eq!(decoded.delay, p.delay);
    }

    #[test]
    fn magic_cast_roundtrip() {
        let p = SMagicCast { spell: 9 };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::MagicCast as i16);

        let decoded = SMagicCast::decode(&raw.payload).expect("decode SMagicCast");
        assert_eq!(decoded.spell, p.spell);
    }

    #[test]
    fn object_magic_roundtrip() {
        let p = SObjectMagic {
            object_id: 1,
            location_x: 10,
            location_y: 20,
            direction: 3,
            spell: 5,
            target_id: 2,
            target_x: 30,
            target_y: 40,
            cast: true,
            level: 2,
            self_broadcast: false,
            secondary_target_ids: vec![7, 8],
        };

        let raw = p.encode().expect("encode SObjectMagic");
        assert_eq!(raw.id, ServerPacketId::ObjectMagic as i16);

        let decoded = SObjectMagic::decode(&raw.payload).expect("decode SObjectMagic");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.direction, p.direction);
        assert_eq!(decoded.spell, p.spell);
        assert_eq!(decoded.target_id, p.target_id);
        assert_eq!(decoded.target_x, p.target_x);
        assert_eq!(decoded.target_y, p.target_y);
        assert_eq!(decoded.cast, p.cast);
        assert_eq!(decoded.level, p.level);
        assert_eq!(decoded.self_broadcast, p.self_broadcast);
        assert_eq!(decoded.secondary_target_ids, p.secondary_target_ids);
    }

    #[test]
    fn object_effect_roundtrip() {
        let p = SObjectEffect {
            object_id: 3,
            effect: 4,
            effect_type: 10,
            delay_time: 100,
            time: 200,
        };

        let raw = p.encode().expect("encode SObjectEffect");
        assert_eq!(raw.id, ServerPacketId::ObjectEffect as i16);

        let decoded = SObjectEffect::decode(&raw.payload).expect("decode SObjectEffect");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.effect, p.effect);
        assert_eq!(decoded.effect_type, p.effect_type);
        assert_eq!(decoded.delay_time, p.delay_time);
        assert_eq!(decoded.time, p.time);
    }

    #[test]
    fn object_projectile_roundtrip() {
        let p = SObjectProjectile {
            spell: 1,
            source: 11,
            destination: 22,
        };

        let raw = p.encode().expect("encode SObjectProjectile");
        assert_eq!(raw.id, ServerPacketId::ObjectProjectile as i16);

        let decoded = SObjectProjectile::decode(&raw.payload).expect("decode SObjectProjectile");
        assert_eq!(decoded.spell, p.spell);
        assert_eq!(decoded.source, p.source);
        assert_eq!(decoded.destination, p.destination);
    }

    #[test]
    fn object_spell_roundtrip() {
        let p = SObjectSpell {
            object_id: 5,
            location_x: 10,
            location_y: 20,
            spell: 3,
            direction: 1,
            param: true,
        };

        let raw = p.encode().expect("encode SObjectSpell");
        assert_eq!(raw.id, ServerPacketId::ObjectSpell as i16);

        let decoded = SObjectSpell::decode(&raw.payload).expect("decode SObjectSpell");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.spell, p.spell);
        assert_eq!(decoded.direction, p.direction);
        assert_eq!(decoded.param, p.param);
    }

    #[test]
    fn range_attack_roundtrip() {
        let p = SRangeAttack {
            target_id: 9,
            target_x: 50,
            target_y: 60,
            spell: 2,
        };

        let raw = p.encode().expect("encode SRangeAttack");
        assert_eq!(raw.id, ServerPacketId::RangeAttack as i16);

        let decoded = SRangeAttack::decode(&raw.payload).expect("decode SRangeAttack");
        assert_eq!(decoded.target_id, p.target_id);
        assert_eq!(decoded.target_x, p.target_x);
        assert_eq!(decoded.target_y, p.target_y);
        assert_eq!(decoded.spell, p.spell);
    }

    #[test]
    fn object_range_attack_roundtrip() {
        let p = SObjectRangeAttack {
            object_id: 5,
            location_x: 70,
            location_y: 80,
            direction: 1,
            target_id: 6,
            target_x: 90,
            target_y: 100,
            attack_type: 3,
            spell: 4,
            level: 2,
        };

        let raw = p.encode().expect("encode SObjectRangeAttack");
        assert_eq!(raw.id, ServerPacketId::ObjectRangeAttack as i16);

        let decoded = SObjectRangeAttack::decode(&raw.payload)
            .expect("decode SObjectRangeAttack");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.direction, p.direction);
        assert_eq!(decoded.target_id, p.target_id);
        assert_eq!(decoded.target_x, p.target_x);
        assert_eq!(decoded.target_y, p.target_y);
        assert_eq!(decoded.attack_type, p.attack_type);
        assert_eq!(decoded.spell, p.spell);
        assert_eq!(decoded.level, p.level);
    }

    #[test]
    fn add_buff_roundtrip() {
        let p = SAddBuff {
            buff_bytes: vec![1, 2, 3, 4, 5],
        };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::AddBuff as i16);

        let decoded = SAddBuff::decode(&raw.payload).expect("decode SAddBuff");
        assert_eq!(decoded.buff_bytes, p.buff_bytes);
    }

    #[test]
    fn remove_buff_roundtrip() {
        let p = SRemoveBuff {
            buff_type: 7,
            object_id: 123,
        };

        let raw = p.encode().expect("encode SRemoveBuff");
        assert_eq!(raw.id, ServerPacketId::RemoveBuff as i16);

        let decoded = SRemoveBuff::decode(&raw.payload).expect("decode SRemoveBuff");
        assert_eq!(decoded.buff_type, p.buff_type);
        assert_eq!(decoded.object_id, p.object_id);
    }

    #[test]
    fn pause_buff_roundtrip() {
        let p = SPauseBuff {
            buff_type: 3,
            object_id: 456,
            paused: true,
        };

        let raw = p.encode().expect("encode SPauseBuff");
        assert_eq!(raw.id, ServerPacketId::PauseBuff as i16);

        let decoded = SPauseBuff::decode(&raw.payload).expect("decode SPauseBuff");
        assert_eq!(decoded.buff_type, p.buff_type);
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.paused, p.paused);
    }

    #[test]
    fn object_hidden_roundtrip() {
        let p = SObjectHidden {
            object_id: 999,
            hidden: true,
        };

        let raw = p.encode().expect("encode SObjectHidden");
        assert_eq!(raw.id, ServerPacketId::ObjectHidden as i16);

        let decoded = SObjectHidden::decode(&raw.payload).expect("decode SObjectHidden");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.hidden, p.hidden);
    }
}
