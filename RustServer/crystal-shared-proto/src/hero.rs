// Hero-related packets compatible with the C# Crystal implementation.
// For complex nested types like ClientHeroInformation and UserItem arrays, we
// currently use a blob-first representation to preserve byte-level
// compatibility while keeping the Rust side simple. This can be refined later
// if we need structured access.

use std::io::{self, Cursor, Read};

use crate::io::{
    read_bool, read_i32_le, read_string, read_u32_le, read_u16_le, write_bool, write_i32_le,
    write_string, write_u32_le, write_u16_le,
};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct SHeroCreateRequest {
    pub can_create_class: Vec<bool>,
}

impl SHeroCreateRequest {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        let count: i32 = self
            .can_create_class
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many classes"))?;
        write_i32_le(&mut buf, count)?;
        for b in &self.can_create_class {
            write_bool(&mut buf, *b)?;
        }
        Ok(RawPacket {
            id: ServerPacketId::HeroCreateRequest as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let count = read_i32_le(&mut c)?;
        if count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative CanCreateClass length",
            ));
        }
        let mut can_create_class = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let b = read_bool(&mut c)?;
            can_create_class.push(b);
        }
        Ok(SHeroCreateRequest { can_create_class })
    }
}

#[derive(Clone, Debug)]
pub struct SNewHero {
    pub result: u8,
}

impl SNewHero {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::new();
        buf.push(self.result);
        RawPacket {
            id: ServerPacketId::NewHero as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SNewHero payload must be exactly 1 byte",
            ));
        }
        Ok(SNewHero { result: payload[0] })
    }
}

#[derive(Clone, Debug)]
pub struct SHeroInformation {
    /// Raw bytes representing the UserInformation + inventory/equipment/magic
    /// portion of the HeroInformation packet, up to but excluding the
    /// AutoPot/AutoHPPercent/AutoMPPercent/HPItemIndex/MPItemIndex tail.
    pub core_bytes: Vec<u8>,
    pub auto_pot: bool,
    pub auto_hp_percent: u8,
    pub auto_mp_percent: u8,
    pub hp_item_index: i32,
    pub mp_item_index: i32,
}

impl SHeroInformation {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.core_bytes);
        write_bool(&mut buf, self.auto_pot)?;
        buf.push(self.auto_hp_percent);
        buf.push(self.auto_mp_percent);
        write_i32_le(&mut buf, self.hp_item_index)?;
        write_i32_le(&mut buf, self.mp_item_index)?;
        Ok(RawPacket {
            id: ServerPacketId::HeroInformation as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        // Tail is: bool (1) + byte + byte + i32 + i32 = 11 bytes.
        if payload.len() < 11 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SHeroInformation payload too short",
            ));
        }
        let tail_start = payload.len() - (1 + 1 + 1 + 4 + 4);
        let core_bytes = payload[..tail_start].to_vec();
        let mut c = Cursor::new(&payload[tail_start..]);
        let auto_pot = read_bool(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let auto_hp_percent = one[0];
        c.read_exact(&mut one)?;
        let auto_mp_percent = one[0];
        let hp_item_index = read_i32_le(&mut c)?;
        let mp_item_index = read_i32_le(&mut c)?;
        Ok(SHeroInformation {
            core_bytes,
            auto_pot,
            auto_hp_percent,
            auto_mp_percent,
            hp_item_index,
            mp_item_index,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SUpdateHeroSpawnState {
    pub state: u8,
}

impl SUpdateHeroSpawnState {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::UpdateHeroSpawnState as i16,
            payload: vec![self.state],
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SUpdateHeroSpawnState payload must be exactly 1 byte",
            ));
        }
        Ok(SUpdateHeroSpawnState { state: payload[0] })
    }
}

#[derive(Clone, Debug)]
pub struct SUnlockHeroAutoPot;

impl SUnlockHeroAutoPot {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::UnlockHeroAutoPot as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SUnlockHeroAutoPot payload must be empty",
            ));
        }
        Ok(SUnlockHeroAutoPot)
    }
}

#[derive(Clone, Debug)]
pub struct SSetAutoPotValue {
    pub stat: u8,
    pub value: u32,
}

impl SSetAutoPotValue {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.stat);
        write_u32_le(&mut buf, self.value)?;
        Ok(RawPacket {
            id: ServerPacketId::SetAutoPotValue as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let stat = one[0];
        let value = read_u32_le(&mut c)?;
        Ok(SSetAutoPotValue { stat, value })
    }
}

#[derive(Clone, Debug)]
pub struct SSetAutoPotItem {
    pub grid: u8,
    pub item_index: i32,
}

impl SSetAutoPotItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.grid);
        write_i32_le(&mut buf, self.item_index)?;
        Ok(RawPacket {
            id: ServerPacketId::SetAutoPotItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let grid = one[0];
        let item_index = read_i32_le(&mut c)?;
        Ok(SSetAutoPotItem { grid, item_index })
    }
}

#[derive(Clone, Debug)]
pub struct SSetHeroBehaviour {
    pub behaviour: u8,
}

impl SSetHeroBehaviour {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::SetHeroBehaviour as i16,
            payload: vec![self.behaviour],
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SSetHeroBehaviour payload must be exactly 1 byte",
            ));
        }
        Ok(SSetHeroBehaviour {
            behaviour: payload[0],
        })
    }
}

#[derive(Clone, Debug)]
pub struct SManageHeroes {
    pub maximum_count: i32,
    /// Raw bytes representing the ClientHeroInformation portion:
    ///   CurrentHero presence + payload, Heroes presence + count + entries.
    pub heroes_bytes: Vec<u8>,
}

impl SManageHeroes {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.maximum_count)?;
        buf.extend_from_slice(&self.heroes_bytes);
        Ok(RawPacket {
            id: ServerPacketId::ManageHeroes as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let maximum_count = read_i32_le(&mut c)?;
        let pos = c.position() as usize;
        let heroes_bytes = payload[pos..].to_vec();
        Ok(SManageHeroes {
            maximum_count,
            heroes_bytes,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SChangeHero {
    pub from_index: i32,
}

impl SChangeHero {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.from_index)?;
        Ok(RawPacket {
            id: ServerPacketId::ChangeHero as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let from_index = read_i32_le(&mut c)?;
        Ok(SChangeHero { from_index })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hero_create_request_roundtrip() {
        let p = SHeroCreateRequest {
            can_create_class: vec![true, false, true],
        };

        let raw = p
            .encode()
            .expect("encode SHeroCreateRequest");
        assert_eq!(raw.id, ServerPacketId::HeroCreateRequest as i16);

        let decoded = SHeroCreateRequest::decode(&raw.payload)
            .expect("decode SHeroCreateRequest");
        assert_eq!(decoded.can_create_class, p.can_create_class);
    }

    #[test]
    fn new_hero_roundtrip() {
        let p = SNewHero { result: 3 };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::NewHero as i16);

        let decoded = SNewHero::decode(&raw.payload).expect("decode SNewHero");
        assert_eq!(decoded.result, p.result);
    }

    #[test]
    fn hero_information_roundtrip() {
        let p = SHeroInformation {
            core_bytes: vec![1, 2, 3, 4, 5],
            auto_pot: true,
            auto_hp_percent: 50,
            auto_mp_percent: 60,
            hp_item_index: 100,
            mp_item_index: 200,
        };

        let raw = p
            .encode()
            .expect("encode SHeroInformation");
        assert_eq!(raw.id, ServerPacketId::HeroInformation as i16);

        let decoded = SHeroInformation::decode(&raw.payload)
            .expect("decode SHeroInformation");
        assert_eq!(decoded.core_bytes, p.core_bytes);
        assert_eq!(decoded.auto_pot, p.auto_pot);
        assert_eq!(decoded.auto_hp_percent, p.auto_hp_percent);
        assert_eq!(decoded.auto_mp_percent, p.auto_mp_percent);
        assert_eq!(decoded.hp_item_index, p.hp_item_index);
        assert_eq!(decoded.mp_item_index, p.mp_item_index);
    }

    #[test]
    fn update_hero_spawn_state_roundtrip() {
        let p = SUpdateHeroSpawnState { state: 2 };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::UpdateHeroSpawnState as i16);

        let decoded = SUpdateHeroSpawnState::decode(&raw.payload)
            .expect("decode SUpdateHeroSpawnState");
        assert_eq!(decoded.state, p.state);
    }

    #[test]
    fn unlock_hero_auto_pot_roundtrip() {
        let p = SUnlockHeroAutoPot;

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::UnlockHeroAutoPot as i16);

        let decoded = SUnlockHeroAutoPot::decode(&raw.payload)
            .expect("decode SUnlockHeroAutoPot");
        let _ = decoded;
    }

    #[test]
    fn set_auto_pot_value_roundtrip() {
        let p = SSetAutoPotValue {
            stat: 7,
            value: 12345,
        };

        let raw = p
            .encode()
            .expect("encode SSetAutoPotValue");
        assert_eq!(raw.id, ServerPacketId::SetAutoPotValue as i16);

        let decoded = SSetAutoPotValue::decode(&raw.payload)
            .expect("decode SSetAutoPotValue");
        assert_eq!(decoded.stat, p.stat);
        assert_eq!(decoded.value, p.value);
    }

    #[test]
    fn set_auto_pot_item_roundtrip() {
        let p = SSetAutoPotItem {
            grid: 2,
            item_index: 99,
        };

        let raw = p
            .encode()
            .expect("encode SSetAutoPotItem");
        assert_eq!(raw.id, ServerPacketId::SetAutoPotItem as i16);

        let decoded = SSetAutoPotItem::decode(&raw.payload)
            .expect("decode SSetAutoPotItem");
        assert_eq!(decoded.grid, p.grid);
        assert_eq!(decoded.item_index, p.item_index);
    }

    #[test]
    fn set_hero_behaviour_roundtrip() {
        let p = SSetHeroBehaviour { behaviour: 1 };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::SetHeroBehaviour as i16);

        let decoded = SSetHeroBehaviour::decode(&raw.payload)
            .expect("decode SSetHeroBehaviour");
        assert_eq!(decoded.behaviour, p.behaviour);
    }

    #[test]
    fn manage_heroes_roundtrip() {
        let p = SManageHeroes {
            maximum_count: 3,
            heroes_bytes: vec![1, 0, 0, 0],
        };

        let raw = p
            .encode()
            .expect("encode SManageHeroes");
        assert_eq!(raw.id, ServerPacketId::ManageHeroes as i16);

        let decoded = SManageHeroes::decode(&raw.payload)
            .expect("decode SManageHeroes");
        assert_eq!(decoded.maximum_count, p.maximum_count);
        assert_eq!(decoded.heroes_bytes, p.heroes_bytes);
    }

    #[test]
    fn change_hero_roundtrip() {
        let p = SChangeHero { from_index: 5 };

        let raw = p.encode().expect("encode SChangeHero");
        assert_eq!(raw.id, ServerPacketId::ChangeHero as i16);

        let decoded = SChangeHero::decode(&raw.payload).expect("decode SChangeHero");
        assert_eq!(decoded.from_index, p.from_index);
    }
}
