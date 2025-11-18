use std::io;

use crate::io::{
    read_bool, read_i32_le, read_u16_le, read_u64_le, write_bool, write_i32_le, write_u16_le,
    write_u64_le,
};
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

#[derive(Clone, Debug)]
pub struct SChangeQuest {
    /// Raw bytes representing ClientQuestProgress.Save(writer) payload.
    pub quest_bytes: Vec<u8>,
    /// QuestState as a raw byte, matching the C# QuestState enum underlying value.
    pub quest_state: u8,
    pub track_quest: bool,
}

impl SChangeQuest {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.quest_bytes);
        buf.push(self.quest_state);
        write_bool(&mut buf, self.track_quest)?;
        Ok(RawPacket {
            id: ServerPacketId::ChangeQuest as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() < 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SChangeQuest payload too short",
            ));
        }
        let tail_start = payload.len() - 2;
        let quest_bytes = payload[..tail_start].to_vec();
        let quest_state = payload[tail_start];
        let track_quest = payload[tail_start + 1] != 0;
        Ok(SChangeQuest {
            quest_bytes,
            quest_state,
            track_quest,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SCompleteQuest {
    pub completed_quests: Vec<i32>,
}

impl SCompleteQuest {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        let count: i32 = self
            .completed_quests
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many quests"))?;
        write_i32_le(&mut buf, count)?;
        for q in &self.completed_quests {
            write_i32_le(&mut buf, *q)?;
        }
        Ok(RawPacket {
            id: ServerPacketId::CompleteQuest as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = std::io::Cursor::new(payload);
        let count = read_i32_le(&mut c)?;
        if count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative completed quest count",
            ));
        }
        let mut completed_quests = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let q = read_i32_le(&mut c)?;
            completed_quests.push(q);
        }
        Ok(SCompleteQuest { completed_quests })
    }
}

#[derive(Clone, Debug)]
pub struct SShareQuest {
    pub quest_index: i32,
    pub sharer_name: String,
}

impl SShareQuest {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.quest_index)?;
        crate::io::write_string(&mut buf, &self.sharer_name)?;
        Ok(RawPacket {
            id: ServerPacketId::ShareQuest as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = std::io::Cursor::new(payload);
        let quest_index = read_i32_le(&mut c)?;
        let sharer_name = crate::io::read_string(&mut c)?;
        Ok(SShareQuest {
            quest_index,
            sharer_name,
        })
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

    #[test]
    fn change_quest_roundtrip() {
        let p = SChangeQuest {
            quest_bytes: vec![1, 2, 3, 4],
            quest_state: 3,
            track_quest: true,
        };

        let raw = p.encode().expect("encode SChangeQuest");
        assert_eq!(raw.id, ServerPacketId::ChangeQuest as i16);

        let decoded = SChangeQuest::decode(&raw.payload).expect("decode SChangeQuest");
        assert_eq!(decoded.quest_bytes, p.quest_bytes);
        assert_eq!(decoded.quest_state, p.quest_state);
        assert_eq!(decoded.track_quest, p.track_quest);
    }

    #[test]
    fn complete_quest_roundtrip() {
        let p = SCompleteQuest {
            completed_quests: vec![10, 20, 30],
        };

        let raw = p.encode().expect("encode SCompleteQuest");
        assert_eq!(raw.id, ServerPacketId::CompleteQuest as i16);

        let decoded = SCompleteQuest::decode(&raw.payload).expect("decode SCompleteQuest");
        assert_eq!(decoded.completed_quests, p.completed_quests);
    }

    #[test]
    fn share_quest_roundtrip() {
        let p = SShareQuest {
            quest_index: 123,
            sharer_name: "Sharer".to_string(),
        };

        let raw = p.encode().expect("encode SShareQuest");
        assert_eq!(raw.id, ServerPacketId::ShareQuest as i16);

        let decoded = SShareQuest::decode(&raw.payload).expect("decode SShareQuest");
        assert_eq!(decoded.quest_index, p.quest_index);
        assert_eq!(decoded.sharer_name, p.sharer_name);
    }
}
