// Character selection related structures and packets.
// Mirrors C# SelectInfo and ServerPackets.LoginSuccess/NewCharacterSuccess.

use std::io::{self, Cursor, Read, Write};

use crate::io::{read_i32_le, read_string, write_i32_le, write_string};
use crate::packet::RawPacket;
use crate::login::ServerPacketId;

#[derive(Clone, Debug)]
pub struct SelectInfo {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: u8,
    pub gender: u8,
    /// .NET DateTime::ToBinary() value for LastAccess
    pub last_access_binary: i64,
}

impl SelectInfo {
    pub fn decode_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let index = read_i32_le(r)?;
        let name = read_string(r)?;
        let level = crate::io::read_u16_le(r)?;

        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;
        let class = buf[0];
        r.read_exact(&mut buf)?;
        let gender = buf[0];

        let last_access_binary = crate::io::read_i64_le(r)?;

        Ok(SelectInfo {
            index,
            name,
            level,
            class,
            gender,
            last_access_binary,
        })
    }

    pub fn encode_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write_i32_le(w, self.index)?;
        write_string(w, &self.name)?;
        crate::io::write_u16_le(w, self.level)?;
        w.write_all(&[self.class])?;
        w.write_all(&[self.gender])?;
        crate::io::write_i64_le(w, self.last_access_binary)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct SLoginSuccess {
    pub characters: Vec<SelectInfo>,
}

impl SLoginSuccess {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        let count: i32 = self
            .characters
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many characters"))?;
        write_i32_le(&mut buf, count)?;
        for ch in &self.characters {
            ch.encode_to(&mut buf)?;
        }

        Ok(RawPacket {
            id: ServerPacketId::LoginSuccess as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let count = read_i32_le(&mut c)?;
        if count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative character count",
            ));
        }

        let mut characters = Vec::with_capacity(count as usize);
        for _ in 0..count {
            characters.push(SelectInfo::decode_from(&mut c)?);
        }

        Ok(SLoginSuccess { characters })
    }
}

#[derive(Clone, Debug)]
pub struct SNewCharacterSuccess {
    pub char_info: SelectInfo,
}

impl SNewCharacterSuccess {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        self.char_info.encode_to(&mut buf)?;
        Ok(RawPacket {
            id: ServerPacketId::NewCharacterSuccess as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let char_info = SelectInfo::decode_from(&mut c)?;
        Ok(SNewCharacterSuccess { char_info })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_info_roundtrip() {
        let info = SelectInfo {
            index: 1,
            name: "TestChar".to_string(),
            level: 55,
            class: 1,
            gender: 0,
            last_access_binary: 123456789,
        };

        let mut buf = Vec::new();
        info.encode_to(&mut buf).expect("encode SelectInfo");

        let mut cursor = Cursor::new(&buf[..]);
        let decoded = SelectInfo::decode_from(&mut cursor).expect("decode SelectInfo");

        assert_eq!(decoded.index, info.index);
        assert_eq!(decoded.name, info.name);
        assert_eq!(decoded.level, info.level);
        assert_eq!(decoded.class, info.class);
        assert_eq!(decoded.gender, info.gender);
        assert_eq!(decoded.last_access_binary, info.last_access_binary);
    }

    #[test]
    fn login_success_roundtrip() {
        let characters = vec![SelectInfo {
            index: 1,
            name: "TestChar".to_string(),
            level: 55,
            class: 1,
            gender: 0,
            last_access_binary: 123456789,
        }];

        let resp = SLoginSuccess { characters };
        let raw = resp.encode().expect("encode SLoginSuccess");

        assert_eq!(raw.id, ServerPacketId::LoginSuccess as i16);

        let decoded = SLoginSuccess::decode(&raw.payload).expect("decode SLoginSuccess");
        assert_eq!(decoded.characters.len(), 1);
        assert_eq!(decoded.characters[0].name, "TestChar");
    }

    #[test]
    fn new_character_success_roundtrip() {
        let info = SelectInfo {
            index: 1,
            name: "NewChar".to_string(),
            level: 1,
            class: 0,
            gender: 1,
            last_access_binary: 987654321,
        };

        let resp = SNewCharacterSuccess { char_info: info };
        let raw = resp.encode().expect("encode SNewCharacterSuccess");

        assert_eq!(raw.id, ServerPacketId::NewCharacterSuccess as i16);

        let decoded = SNewCharacterSuccess::decode(&raw.payload).expect("decode SNewCharacterSuccess");
        assert_eq!(decoded.char_info.name, "NewChar");
        assert_eq!(decoded.char_info.level, 1);
    }
}
