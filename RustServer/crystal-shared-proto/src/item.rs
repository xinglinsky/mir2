// Item-related packets (e.g. NewItemInfo/NewChatItem) compatible with the C# Crystal implementation.
//
// For now we treat the inner ItemInfo/UserItem payloads as opaque byte blobs; this is
// sufficient to roundtrip data and to plug in real ItemInfo/UserItem encoders later.

use std::io;

use crate::io::{read_u16_le, read_u64_le, write_u16_le, write_u64_le};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct SNewItemInfo {
    /// Raw bytes representing the serialized ItemInfo (as written by C# ItemInfo.Save).
    pub info_bytes: Vec<u8>,
}

impl SNewItemInfo {
    pub fn encode(&self) -> io::Result<RawPacket> {
        Ok(RawPacket {
            id: ServerPacketId::NewItemInfo as i16,
            payload: self.info_bytes.clone(),
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SNewItemInfo {
            info_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SNewHeroInfo {
    /// Raw bytes representing the serialized ClientHeroInformation followed by
    /// the StorageIndex (as written by C# ClientHeroInformation.Save and WritePacket).
    pub hero_bytes: Vec<u8>,
}

impl SNewHeroInfo {
    pub fn encode(&self) -> io::Result<RawPacket> {
        Ok(RawPacket {
            id: ServerPacketId::NewHeroInfo as i16,
            payload: self.hero_bytes.clone(),
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SNewHeroInfo {
            hero_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SNewChatItem {
    /// Raw bytes representing the serialized UserItem (as written by C# UserItem.Save).
    pub item_bytes: Vec<u8>,
}

impl SNewChatItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        Ok(RawPacket {
            id: ServerPacketId::NewChatItem as i16,
            payload: self.item_bytes.clone(),
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SNewChatItem {
            item_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SRefreshItem {
    pub item_bytes: Vec<u8>,
}

impl SRefreshItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        Ok(RawPacket {
            id: ServerPacketId::RefreshItem as i16,
            payload: self.item_bytes.clone(),
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SRefreshItem {
            item_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SDuraChanged {
    pub unique_id: u64,
    pub current_dura: u16,
}

impl SDuraChanged {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        write_u16_le(&mut buf, self.current_dura)?;
        Ok(RawPacket {
            id: ServerPacketId::DuraChanged as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = std::io::Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        let current_dura = read_u16_le(&mut c)?;
        Ok(SDuraChanged {
            unique_id,
            current_dura,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SDeleteItem {
    pub unique_id: u64,
    pub count: u16,
}

impl SDeleteItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        write_u16_le(&mut buf, self.count)?;
        Ok(RawPacket {
            id: ServerPacketId::DeleteItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = std::io::Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        let count = read_u16_le(&mut c)?;
        Ok(SDeleteItem { unique_id, count })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_item_info_roundtrip() {
        let p = SNewItemInfo {
            info_bytes: vec![1, 2, 3, 4],
        };

        let raw = p.encode().expect("encode SNewItemInfo");
        assert_eq!(raw.id, ServerPacketId::NewItemInfo as i16);
        assert_eq!(raw.payload, p.info_bytes);

        let decoded = SNewItemInfo::decode(&raw.payload).expect("decode SNewItemInfo");
        assert_eq!(decoded.info_bytes, p.info_bytes);
    }

    #[test]
    fn new_hero_info_roundtrip() {
        let p = SNewHeroInfo {
            hero_bytes: vec![5, 6, 7, 8],
        };

        let raw = p.encode().expect("encode SNewHeroInfo");
        assert_eq!(raw.id, ServerPacketId::NewHeroInfo as i16);
        assert_eq!(raw.payload, p.hero_bytes);

        let decoded = SNewHeroInfo::decode(&raw.payload).expect("decode SNewHeroInfo");
        assert_eq!(decoded.hero_bytes, p.hero_bytes);
    }

    #[test]
    fn new_chat_item_roundtrip() {
        let p = SNewChatItem {
            item_bytes: vec![9, 8, 7],
        };

        let raw = p.encode().expect("encode SNewChatItem");
        assert_eq!(raw.id, ServerPacketId::NewChatItem as i16);
        assert_eq!(raw.payload, p.item_bytes);

        let decoded = SNewChatItem::decode(&raw.payload).expect("decode SNewChatItem");
        assert_eq!(decoded.item_bytes, p.item_bytes);
    }

    #[test]
    fn refresh_item_roundtrip() {
        let p = SRefreshItem {
            item_bytes: vec![1, 2, 3, 4, 5],
        };

        let raw = p.encode().expect("encode SRefreshItem");
        assert_eq!(raw.id, ServerPacketId::RefreshItem as i16);

        let decoded = SRefreshItem::decode(&raw.payload).expect("decode SRefreshItem");
        assert_eq!(decoded.item_bytes, p.item_bytes);
    }

    #[test]
    fn dura_changed_roundtrip() {
        let p = SDuraChanged {
            unique_id: 0x1122_3344_5566_7788,
            current_dura: 123,
        };

        let raw = p.encode().expect("encode SDuraChanged");
        assert_eq!(raw.id, ServerPacketId::DuraChanged as i16);

        let decoded = SDuraChanged::decode(&raw.payload).expect("decode SDuraChanged");
        assert_eq!(decoded.unique_id, p.unique_id);
        assert_eq!(decoded.current_dura, p.current_dura);
    }

    #[test]
    fn delete_item_roundtrip() {
        let p = SDeleteItem {
            unique_id: 0xAABB_CCDD_EEFF_0011,
            count: 10,
        };

        let raw = p.encode().expect("encode SDeleteItem");
        assert_eq!(raw.id, ServerPacketId::DeleteItem as i16);

        let decoded = SDeleteItem::decode(&raw.payload).expect("decode SDeleteItem");
        assert_eq!(decoded.unique_id, p.unique_id);
        assert_eq!(decoded.count, p.count);
    }
}
