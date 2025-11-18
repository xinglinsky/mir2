use std::io;

use crate::io::{read_u16_le, read_u64_le, write_u16_le, write_u64_le};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct SNewQuestInfo {
    /// Raw bytes representing ClientQuestInfo.Save(writer) payload.
    pub quest_bytes: Vec<u8>,
}

impl SNewQuestInfo {
    pub fn encode(&self) -> io::Result<RawPacket> {
        Ok(RawPacket {
            id: ServerPacketId::NewQuestInfo as i16,
            payload: self.quest_bytes.clone(),
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SNewQuestInfo {
            quest_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SGainedQuestItem {
    /// Raw bytes representing UserItem.Save(writer) payload.
    pub item_bytes: Vec<u8>,
}

impl SGainedQuestItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        Ok(RawPacket {
            id: ServerPacketId::GainedQuestItem as i16,
            payload: self.item_bytes.clone(),
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SGainedQuestItem {
            item_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SDeleteQuestItem {
    pub unique_id: u64,
    pub count: u16,
}

impl SDeleteQuestItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        write_u16_le(&mut buf, self.count)?;
        Ok(RawPacket {
            id: ServerPacketId::DeleteQuestItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = std::io::Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        let count = read_u16_le(&mut c)?;
        Ok(SDeleteQuestItem { unique_id, count })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_quest_info_roundtrip() {
        let p = SNewQuestInfo {
            quest_bytes: vec![1, 2, 3, 4, 5],
        };

        let raw = p.encode().expect("encode SNewQuestInfo");
        assert_eq!(raw.id, ServerPacketId::NewQuestInfo as i16);
        assert_eq!(raw.payload, p.quest_bytes);

        let decoded = SNewQuestInfo::decode(&raw.payload).expect("decode SNewQuestInfo");
        assert_eq!(decoded.quest_bytes, p.quest_bytes);
    }

    #[test]
    fn gained_quest_item_roundtrip() {
        let p = SGainedQuestItem {
            item_bytes: vec![9, 8, 7, 6],
        };

        let raw = p.encode().expect("encode SGainedQuestItem");
        assert_eq!(raw.id, ServerPacketId::GainedQuestItem as i16);
        assert_eq!(raw.payload, p.item_bytes);

        let decoded = SGainedQuestItem::decode(&raw.payload).expect("decode SGainedQuestItem");
        assert_eq!(decoded.item_bytes, p.item_bytes);
    }

    #[test]
    fn delete_quest_item_roundtrip() {
        let p = SDeleteQuestItem {
            unique_id: 0x1122_3344_5566_7788,
            count: 42,
        };

        let raw = p.encode().expect("encode SDeleteQuestItem");
        assert_eq!(raw.id, ServerPacketId::DeleteQuestItem as i16);

        let decoded = SDeleteQuestItem::decode(&raw.payload).expect("decode SDeleteQuestItem");
        assert_eq!(decoded.unique_id, p.unique_id);
        assert_eq!(decoded.count, p.count);
    }
}
