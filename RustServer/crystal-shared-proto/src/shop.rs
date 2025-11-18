use std::io;

use crate::io::{read_i32_le, write_i32_le};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct SGameShopInfo {
    /// Raw bytes representing GameShopItem.Save(writer, true) followed by StockLevel.
    pub info_bytes: Vec<u8>,
}

impl SGameShopInfo {
    pub fn encode(&self) -> io::Result<RawPacket> {
        Ok(RawPacket {
            id: ServerPacketId::GameShopInfo as i16,
            payload: self.info_bytes.clone(),
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SGameShopInfo {
            info_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SGameShopStock {
    pub gindex: i32,
    pub stock_level: i32,
}

impl SGameShopStock {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.gindex)?;
        write_i32_le(&mut buf, self.stock_level)?;
        Ok(RawPacket {
            id: ServerPacketId::GameShopStock as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = std::io::Cursor::new(payload);
        let gindex = read_i32_le(&mut c)?;
        let stock_level = read_i32_le(&mut c)?;
        Ok(SGameShopStock { gindex, stock_level })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_shop_info_roundtrip() {
        let p = SGameShopInfo {
            info_bytes: vec![1, 2, 3, 4],
        };

        let raw = p.encode().expect("encode SGameShopInfo");
        assert_eq!(raw.id, ServerPacketId::GameShopInfo as i16);
        assert_eq!(raw.payload, p.info_bytes);

        let decoded = SGameShopInfo::decode(&raw.payload).expect("decode SGameShopInfo");
        assert_eq!(decoded.info_bytes, p.info_bytes);
    }

    #[test]
    fn game_shop_stock_roundtrip() {
        let p = SGameShopStock {
            gindex: 123,
            stock_level: 456,
        };

        let raw = p.encode().expect("encode SGameShopStock");
        assert_eq!(raw.id, ServerPacketId::GameShopStock as i16);

        let decoded = SGameShopStock::decode(&raw.payload).expect("decode SGameShopStock");
        assert_eq!(decoded.gindex, p.gindex);
        assert_eq!(decoded.stock_level, p.stock_level);
    }
}
