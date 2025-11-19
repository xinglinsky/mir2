// Social / trade related packets (MarriageRequest, Trade*, MentorRequest)
// compatible with the C# Crystal implementation.

use std::io::{self, Cursor};

use crate::io::{
    read_bool, read_i16_le, read_i64_le, read_string, read_u16_le, read_u32_le, write_bool,
    write_i16_le, write_i64_le, write_string, write_u16_le, write_u32_le,
};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct SMarriageRequest {
    pub name: String,
}

impl SMarriageRequest {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ServerPacketId::MarriageRequest as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        Ok(SMarriageRequest { name })
    }
}

#[derive(Clone, Debug)]
pub struct SDivorceRequest {
    pub name: String,
}

impl SDivorceRequest {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ServerPacketId::DivorceRequest as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        Ok(SDivorceRequest { name })
    }
}

#[derive(Clone, Debug)]
pub struct SMentorRequest {
    pub name: String,
    pub level: u16,
}

impl SMentorRequest {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        write_u16_le(&mut buf, self.level)?;
        Ok(RawPacket {
            id: ServerPacketId::MentorRequest as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        let level = read_u16_le(&mut c)?;
        Ok(SMentorRequest { name, level })
    }
}

#[derive(Clone, Debug)]
pub struct STradeRequest {
    pub name: String,
}

impl STradeRequest {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ServerPacketId::TradeRequest as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        Ok(STradeRequest { name })
    }
}

#[derive(Clone, Debug)]
pub struct STradeAccept {
    pub name: String,
}

impl STradeAccept {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ServerPacketId::TradeAccept as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        Ok(STradeAccept { name })
    }
}

#[derive(Clone, Debug)]
pub struct STradeGold {
    pub amount: u32,
}

impl STradeGold {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.amount)?;
        Ok(RawPacket {
            id: ServerPacketId::TradeGold as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let amount = read_u32_le(&mut c)?;
        Ok(STradeGold { amount })
    }
}

#[derive(Clone, Debug)]
pub struct STradeItem {
    /// Raw bytes representing the TradeItems array payload:
    /// count (i32) + (bool hasItem + UserItem.Save(writer)).
    pub items_bytes: Vec<u8>,
}

impl STradeItem {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::TradeItem as i16,
            payload: self.items_bytes.clone(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(STradeItem {
            items_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct STradeConfirm;

impl STradeConfirm {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::TradeConfirm as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "STradeConfirm payload must be empty",
            ));
        }
        Ok(STradeConfirm)
    }
}

#[derive(Clone, Debug)]
pub struct STradeCancel {
    pub unlock: bool,
}

impl STradeCancel {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.unlock)?;
        Ok(RawPacket {
            id: ServerPacketId::TradeCancel as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unlock = read_bool(&mut c)?;
        Ok(STradeCancel { unlock })
    }
}

#[derive(Clone, Debug)]
pub struct SFriendUpdate {
    /// Raw bytes representing ClientFriend list: count (i32) + repeated ClientFriend.Save.
    pub friends_bytes: Vec<u8>,
}

impl SFriendUpdate {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::FriendUpdate as i16,
            payload: self.friends_bytes.clone(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SFriendUpdate {
            friends_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SLoverUpdate {
    pub name: String,
    pub date_binary: i64,
    pub map_name: String,
    pub married_days: i16,
}

impl SLoverUpdate {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        write_i64_le(&mut buf, self.date_binary)?;
        write_string(&mut buf, &self.map_name)?;
        write_i16_le(&mut buf, self.married_days)?;
        Ok(RawPacket {
            id: ServerPacketId::LoverUpdate as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        let date_binary = read_i64_le(&mut c)?;
        let map_name = read_string(&mut c)?;
        let married_days = read_i16_le(&mut c)?;
        Ok(SLoverUpdate {
            name,
            date_binary,
            map_name,
            married_days,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SMentorUpdate {
    pub name: String,
    pub level: u16,
    pub online: bool,
    pub mentee_exp: i64,
}

impl SMentorUpdate {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        write_u16_le(&mut buf, self.level)?;
        write_bool(&mut buf, self.online)?;
        write_i64_le(&mut buf, self.mentee_exp)?;
        Ok(RawPacket {
            id: ServerPacketId::MentorUpdate as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        let level = read_u16_le(&mut c)?;
        let online = read_bool(&mut c)?;
        let mentee_exp = read_i64_le(&mut c)?;
        Ok(SMentorUpdate {
            name,
            level,
            online,
            mentee_exp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marriage_request_roundtrip() {
        let p = SMarriageRequest {
            name: "Spouse".to_string(),
        };

        let raw = p.encode().expect("encode SMarriageRequest");
        assert_eq!(raw.id, ServerPacketId::MarriageRequest as i16);

        let decoded = SMarriageRequest::decode(&raw.payload)
            .expect("decode SMarriageRequest");
        assert_eq!(decoded.name, p.name);
    }

    #[test]
    fn divorce_request_roundtrip() {
        let p = SDivorceRequest {
            name: "Ex".to_string(),
        };

        let raw = p.encode().expect("encode SDivorceRequest");
        assert_eq!(raw.id, ServerPacketId::DivorceRequest as i16);

        let decoded = SDivorceRequest::decode(&raw.payload)
            .expect("decode SDivorceRequest");
        assert_eq!(decoded.name, p.name);
    }

    #[test]
    fn mentor_request_roundtrip() {
        let p = SMentorRequest {
            name: "Student".to_string(),
            level: 45,
        };

        let raw = p.encode().expect("encode SMentorRequest");
        assert_eq!(raw.id, ServerPacketId::MentorRequest as i16);

        let decoded = SMentorRequest::decode(&raw.payload)
            .expect("decode SMentorRequest");
        assert_eq!(decoded.name, p.name);
        assert_eq!(decoded.level, p.level);
    }

    #[test]
    fn trade_request_roundtrip() {
        let p = STradeRequest {
            name: "Partner".to_string(),
        };

        let raw = p.encode().expect("encode STradeRequest");
        assert_eq!(raw.id, ServerPacketId::TradeRequest as i16);

        let decoded = STradeRequest::decode(&raw.payload)
            .expect("decode STradeRequest");
        assert_eq!(decoded.name, p.name);
    }

    #[test]
    fn trade_accept_roundtrip() {
        let p = STradeAccept {
            name: "Partner".to_string(),
        };

        let raw = p.encode().expect("encode STradeAccept");
        assert_eq!(raw.id, ServerPacketId::TradeAccept as i16);

        let decoded = STradeAccept::decode(&raw.payload)
            .expect("decode STradeAccept");
        assert_eq!(decoded.name, p.name);
    }

    #[test]
    fn trade_gold_roundtrip() {
        let p = STradeGold { amount: 12345 };

        let raw = p.encode().expect("encode STradeGold");
        assert_eq!(raw.id, ServerPacketId::TradeGold as i16);

        let decoded = STradeGold::decode(&raw.payload).expect("decode STradeGold");
        assert_eq!(decoded.amount, p.amount);
    }

    #[test]
    fn trade_item_roundtrip() {
        let p = STradeItem {
            items_bytes: vec![0, 0, 0, 0], // count = 0
        };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::TradeItem as i16);
        assert_eq!(raw.payload, p.items_bytes);

        let decoded = STradeItem::decode(&raw.payload).expect("decode STradeItem");
        assert_eq!(decoded.items_bytes, p.items_bytes);
    }

    #[test]
    fn trade_confirm_roundtrip() {
        let p = STradeConfirm;

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::TradeConfirm as i16);

        let decoded = STradeConfirm::decode(&raw.payload).expect("decode STradeConfirm");
        let _ = decoded;
    }

    #[test]
    fn trade_cancel_roundtrip() {
        let p = STradeCancel { unlock: true };

        let raw = p.encode().expect("encode STradeCancel");
        assert_eq!(raw.id, ServerPacketId::TradeCancel as i16);

        let decoded = STradeCancel::decode(&raw.payload).expect("decode STradeCancel");
        assert_eq!(decoded.unlock, p.unlock);
    }

    #[test]
    fn friend_update_roundtrip() {
        let p = SFriendUpdate {
            friends_bytes: vec![0, 0, 0, 0], // count = 0
        };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::FriendUpdate as i16);
        assert_eq!(raw.payload, p.friends_bytes);

        let decoded = SFriendUpdate::decode(&raw.payload).expect("decode SFriendUpdate");
        assert_eq!(decoded.friends_bytes, p.friends_bytes);
    }

    #[test]
    fn lover_update_roundtrip() {
        let p = SLoverUpdate {
            name: "Spouse".to_string(),
            date_binary: 123456789,
            map_name: "Town".to_string(),
            married_days: 100,
        };

        let raw = p.encode().expect("encode SLoverUpdate");
        assert_eq!(raw.id, ServerPacketId::LoverUpdate as i16);

        let decoded = SLoverUpdate::decode(&raw.payload).expect("decode SLoverUpdate");
        assert_eq!(decoded.name, p.name);
        assert_eq!(decoded.date_binary, p.date_binary);
        assert_eq!(decoded.map_name, p.map_name);
        assert_eq!(decoded.married_days, p.married_days);
    }

    #[test]
    fn mentor_update_roundtrip() {
        let p = SMentorUpdate {
            name: "Teacher".to_string(),
            level: 50,
            online: true,
            mentee_exp: 999999,
        };

        let raw = p.encode().expect("encode SMentorUpdate");
        assert_eq!(raw.id, ServerPacketId::MentorUpdate as i16);

        let decoded = SMentorUpdate::decode(&raw.payload).expect("decode SMentorUpdate");
        assert_eq!(decoded.name, p.name);
        assert_eq!(decoded.level, p.level);
        assert_eq!(decoded.online, p.online);
        assert_eq!(decoded.mentee_exp, p.mentee_exp);
    }
}
