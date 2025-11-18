// Stats/name related packets (BaseStatsInfo, HeroBaseStatsInfo, UserName)
// compatible with the C# Crystal implementation.

use std::io::{self, Cursor};

use crate::io::{read_string, read_u32_le, write_string, write_u32_le};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct SBaseStatsInfo {
    /// Raw bytes representing BaseStats.Save(writer) as written by the C# server.
    pub stats_bytes: Vec<u8>,
}

impl SBaseStatsInfo {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::BaseStatsInfo as i16,
            payload: self.stats_bytes.clone(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SBaseStatsInfo {
            stats_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SHeroBaseStatsInfo {
    /// Raw bytes representing BaseStats.Save(writer) for the hero.
    pub stats_bytes: Vec<u8>,
}

impl SHeroBaseStatsInfo {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::HeroBaseStatsInfo as i16,
            payload: self.stats_bytes.clone(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SHeroBaseStatsInfo {
            stats_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SUserName {
    pub id: u32,
    pub name: String,
}

impl SUserName {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.id)?;
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ServerPacketId::UserName as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let id = read_u32_le(&mut c)?;
        let name = read_string(&mut c)?;
        Ok(SUserName { id, name })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_stats_info_roundtrip() {
        let p = SBaseStatsInfo {
            stats_bytes: vec![1, 2, 3, 4, 5],
        };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::BaseStatsInfo as i16);
        assert_eq!(raw.payload, p.stats_bytes);

        let decoded = SBaseStatsInfo::decode(&raw.payload).expect("decode SBaseStatsInfo");
        assert_eq!(decoded.stats_bytes, p.stats_bytes);
    }

    #[test]
    fn hero_base_stats_info_roundtrip() {
        let p = SHeroBaseStatsInfo {
            stats_bytes: vec![9, 8, 7, 6],
        };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::HeroBaseStatsInfo as i16);
        assert_eq!(raw.payload, p.stats_bytes);

        let decoded = SHeroBaseStatsInfo::decode(&raw.payload)
            .expect("decode SHeroBaseStatsInfo");
        assert_eq!(decoded.stats_bytes, p.stats_bytes);
    }

    #[test]
    fn user_name_roundtrip() {
        let p = SUserName {
            id: 42,
            name: "PlayerName".to_string(),
        };

        let raw = p.encode().expect("encode SUserName");
        assert_eq!(raw.id, ServerPacketId::UserName as i16);

        let decoded = SUserName::decode(&raw.payload).expect("decode SUserName");
        assert_eq!(decoded.id, p.id);
        assert_eq!(decoded.name, p.name);
    }
}
