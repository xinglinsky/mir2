// Intelligent Creature related packets compatible with the C# Crystal implementation.

use std::io::{self, Cursor};

use crate::io::{read_bool, read_i32_le, read_u32_le, write_bool, write_i32_le, write_u32_le};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct SNewIntelligentCreature {
    /// Raw bytes representing ClientIntelligentCreature.Save(writer).
    pub creature_bytes: Vec<u8>,
}

impl SNewIntelligentCreature {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::NewIntelligentCreature as i16,
            payload: self.creature_bytes.clone(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SNewIntelligentCreature {
            creature_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SUpdateIntelligentCreatureList {
    /// Raw bytes representing the creature list: count (i32) + repeated ClientIntelligentCreature.Save.
    pub creatures_bytes: Vec<u8>,
    pub creature_summoned: bool,
    pub summoned_creature_type: u8,
    pub pearl_count: i32,
}

impl SUpdateIntelligentCreatureList {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.creatures_bytes);
        write_bool(&mut buf, self.creature_summoned)?;
        buf.push(self.summoned_creature_type);
        write_i32_le(&mut buf, self.pearl_count)?;
        Ok(RawPacket {
            id: ServerPacketId::UpdateIntelligentCreatureList as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() < 6 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SUpdateIntelligentCreatureList payload too short",
            ));
        }
        let tail_start = payload.len() - 6;
        let creatures_bytes = payload[..tail_start].to_vec();
        let mut c = Cursor::new(&payload[tail_start..]);
        let creature_summoned = read_bool(&mut c)?;
        let mut one = [0u8; 1];
        use std::io::Read;
        c.read_exact(&mut one)?;
        let summoned_creature_type = one[0];
        let pearl_count = read_i32_le(&mut c)?;
        Ok(SUpdateIntelligentCreatureList {
            creatures_bytes,
            creature_summoned,
            summoned_creature_type,
            pearl_count,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SIntelligentCreatureEnableRename;

impl SIntelligentCreatureEnableRename {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::IntelligentCreatureEnableRename as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SIntelligentCreatureEnableRename payload must be empty",
            ));
        }
        Ok(SIntelligentCreatureEnableRename)
    }
}

#[derive(Clone, Debug)]
pub struct SIntelligentCreaturePickup {
    pub object_id: u32,
}

impl SIntelligentCreaturePickup {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        Ok(RawPacket {
            id: ServerPacketId::IntelligentCreaturePickup as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        Ok(SIntelligentCreaturePickup { object_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_intelligent_creature_roundtrip() {
        let p = SNewIntelligentCreature {
            creature_bytes: vec![1, 2, 3, 4, 5],
        };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::NewIntelligentCreature as i16);
        assert_eq!(raw.payload, p.creature_bytes);

        let decoded = SNewIntelligentCreature::decode(&raw.payload)
            .expect("decode SNewIntelligentCreature");
        assert_eq!(decoded.creature_bytes, p.creature_bytes);
    }

    #[test]
    fn update_intelligent_creature_list_roundtrip() {
        let p = SUpdateIntelligentCreatureList {
            creatures_bytes: vec![0, 0, 0, 0], // count = 0
            creature_summoned: true,
            summoned_creature_type: 2,
            pearl_count: 10,
        };

        let raw = p.encode().expect("encode SUpdateIntelligentCreatureList");
        assert_eq!(
            raw.id,
            ServerPacketId::UpdateIntelligentCreatureList as i16
        );

        let decoded = SUpdateIntelligentCreatureList::decode(&raw.payload)
            .expect("decode SUpdateIntelligentCreatureList");
        assert_eq!(decoded.creatures_bytes, p.creatures_bytes);
        assert_eq!(decoded.creature_summoned, p.creature_summoned);
        assert_eq!(decoded.summoned_creature_type, p.summoned_creature_type);
        assert_eq!(decoded.pearl_count, p.pearl_count);
    }

    #[test]
    fn intelligent_creature_enable_rename_roundtrip() {
        let p = SIntelligentCreatureEnableRename;

        let raw = p.encode();
        assert_eq!(
            raw.id,
            ServerPacketId::IntelligentCreatureEnableRename as i16
        );

        let _ = SIntelligentCreatureEnableRename::decode(&raw.payload)
            .expect("decode SIntelligentCreatureEnableRename");
    }

    #[test]
    fn intelligent_creature_pickup_roundtrip() {
        let p = SIntelligentCreaturePickup { object_id: 123 };

        let raw = p.encode().expect("encode SIntelligentCreaturePickup");
        assert_eq!(raw.id, ServerPacketId::IntelligentCreaturePickup as i16);

        let decoded = SIntelligentCreaturePickup::decode(&raw.payload)
            .expect("decode SIntelligentCreaturePickup");
        assert_eq!(decoded.object_id, p.object_id);
    }
}
