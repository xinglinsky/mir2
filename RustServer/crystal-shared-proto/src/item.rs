// Item-related packets (e.g. NewItemInfo/NewChatItem) compatible with the C# Crystal implementation.
//
// For now we treat the inner ItemInfo/UserItem payloads as opaque byte blobs; this is
// sufficient to roundtrip data and to plug in real ItemInfo/UserItem encoders later.

use std::io::{self, Read};

use crate::io::{
    read_bool, read_i32_le, read_i64_le, read_u16_le, read_u64_le, write_bool, write_i32_le,
    write_i64_le, write_u16_le, write_u64_le,
};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;
use crate::item_types::{ItemInfoData, UserItemData};

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

impl SNewItemInfo {
    pub fn decode_item_info(&self) -> io::Result<ItemInfoData> {
        ItemInfoData::decode_from_bytes(&self.info_bytes)
    }

    pub fn from_item_info(info: &ItemInfoData) -> io::Result<Self> {
        let bytes = info.encode_to_bytes()?;
        Ok(SNewItemInfo { info_bytes: bytes })
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

impl SNewChatItem {
    pub fn decode_user_item(&self) -> io::Result<UserItemData> {
        UserItemData::decode_from_bytes(&self.item_bytes)
    }

    pub fn from_user_item(item: &UserItemData) -> io::Result<Self> {
        let bytes = item.encode_to_bytes()?;
        Ok(SNewChatItem { item_bytes: bytes })
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

impl SRefreshItem {
    pub fn decode_user_item(&self) -> io::Result<UserItemData> {
        UserItemData::decode_from_bytes(&self.item_bytes)
    }

    pub fn from_user_item(item: &UserItemData) -> io::Result<Self> {
        let bytes = item.encode_to_bytes()?;
        Ok(SRefreshItem { item_bytes: bytes })
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

#[derive(Clone, Debug)]
pub struct SChatItemStats {
    pub chat_item_id: u64,
    /// Raw bytes representing UserItem.Save(writer) as written by the C# server.
    pub stats_bytes: Vec<u8>,
}

impl SChatItemStats {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.chat_item_id)?;
        buf.extend_from_slice(&self.stats_bytes);
        Ok(RawPacket {
            id: ServerPacketId::ChatItemStats as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = std::io::Cursor::new(payload);
        let chat_item_id = read_u64_le(&mut c)?;
        let pos = c.position() as usize;
        let stats_bytes = payload[pos..].to_vec();

        Ok(SChatItemStats {
            chat_item_id,
            stats_bytes,
        })
    }
}

impl SChatItemStats {
    pub fn decode_user_item(&self) -> io::Result<UserItemData> {
        UserItemData::decode_from_bytes(&self.stats_bytes)
    }

    pub fn from_user_item(chat_item_id: u64, item: &UserItemData) -> io::Result<Self> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, chat_item_id)?;
        let item_bytes = item.encode_to_bytes()?;
        buf.extend_from_slice(&item_bytes);
        Ok(SChatItemStats {
            chat_item_id,
            stats_bytes: item_bytes,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SUserStorage {
    /// Raw bytes representing the serialized UserItem[] Storage:
    /// bool hasStorage + (if true) count (i32) + per-slot (bool hasItem + UserItem.Save).
    pub storage_bytes: Vec<u8>,
}

impl SUserStorage {
    pub fn encode(&self) -> io::Result<RawPacket> {
        Ok(RawPacket {
            id: ServerPacketId::UserStorage as i16,
            payload: self.storage_bytes.clone(),
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SUserStorage {
            storage_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SResizeInventory {
    pub size: i32,
}

impl SResizeInventory {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.size)?;
        Ok(RawPacket {
            id: ServerPacketId::ResizeInventory as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = std::io::Cursor::new(payload);
        let size = read_i32_le(&mut c)?;
        Ok(SResizeInventory { size })
    }
}

#[derive(Clone, Debug)]
pub struct SResizeStorage {
    pub size: i32,
    pub has_expanded_storage: bool,
    /// Expiry time stored as DateTime.ToBinary() i64.
    pub expiry_time_binary: i64,
}

impl SResizeStorage {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.size)?;
        write_bool(&mut buf, self.has_expanded_storage)?;
        write_i64_le(&mut buf, self.expiry_time_binary)?;
        Ok(RawPacket {
            id: ServerPacketId::ResizeStorage as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = std::io::Cursor::new(payload);
        let size = read_i32_le(&mut c)?;
        let has_expanded_storage = read_bool(&mut c)?;
        let expiry_time_binary = read_i64_le(&mut c)?;
        Ok(SResizeStorage {
            size,
            has_expanded_storage,
            expiry_time_binary,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SCombineItem {
    pub grid: u8,
    pub id_from: u64,
    pub id_to: u64,
    pub success: bool,
    pub destroy: bool,
}

impl SCombineItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.grid);
        write_u64_le(&mut buf, self.id_from)?;
        write_u64_le(&mut buf, self.id_to)?;
        write_bool(&mut buf, self.success)?;
        write_bool(&mut buf, self.destroy)?;
        Ok(RawPacket {
            id: ServerPacketId::CombineItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = std::io::Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let grid = one[0];
        let id_from = read_u64_le(&mut c)?;
        let id_to = read_u64_le(&mut c)?;
        let success = read_bool(&mut c)?;
        let destroy = read_bool(&mut c)?;
        Ok(SCombineItem {
            grid,
            id_from,
            id_to,
            success,
            destroy,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SItemUpgraded {
    /// Raw bytes representing UserItem.Save(writer) as written by the C# server.
    pub item_bytes: Vec<u8>,
}

impl SItemUpgraded {
    pub fn encode(&self) -> io::Result<RawPacket> {
        Ok(RawPacket {
            id: ServerPacketId::ItemUpgraded as i16,
            payload: self.item_bytes.clone(),
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SItemUpgraded {
            item_bytes: payload.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_types::{AwakeData, StatsMap, ItemInfoData, UserItemData};

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

    #[test]
    fn chat_item_stats_roundtrip() {
        let p = SChatItemStats {
            chat_item_id: 0xAABB_CCDD_EEFF_0011,
            stats_bytes: vec![1, 2, 3, 4, 5],
        };

        let raw = p.encode().expect("encode SChatItemStats");
        assert_eq!(raw.id, ServerPacketId::ChatItemStats as i16);

        let decoded = SChatItemStats::decode(&raw.payload).expect("decode SChatItemStats");
        assert_eq!(decoded.chat_item_id, p.chat_item_id);
        assert_eq!(decoded.stats_bytes, p.stats_bytes);
    }

    #[test]
    fn user_storage_roundtrip() {
        let p = SUserStorage {
            storage_bytes: vec![1, 0, 0, 0, 1, 2, 3],
        };

        let raw = p.encode().expect("encode SUserStorage");
        assert_eq!(raw.id, ServerPacketId::UserStorage as i16);
        assert_eq!(raw.payload, p.storage_bytes);

        let decoded = SUserStorage::decode(&raw.payload).expect("decode SUserStorage");
        assert_eq!(decoded.storage_bytes, p.storage_bytes);
    }

    #[test]
    fn resize_inventory_roundtrip() {
        let p = SResizeInventory { size: 120 };

        let raw = p.encode().expect("encode SResizeInventory");
        assert_eq!(raw.id, ServerPacketId::ResizeInventory as i16);

        let decoded = SResizeInventory::decode(&raw.payload).expect("decode SResizeInventory");
        assert_eq!(decoded.size, p.size);
    }

    #[test]
    fn resize_storage_roundtrip() {
        let p = SResizeStorage {
            size: 240,
            has_expanded_storage: true,
            expiry_time_binary: 1234567890,
        };

        let raw = p.encode().expect("encode SResizeStorage");
        assert_eq!(raw.id, ServerPacketId::ResizeStorage as i16);

        let decoded = SResizeStorage::decode(&raw.payload).expect("decode SResizeStorage");
        assert_eq!(decoded.size, p.size);
        assert_eq!(decoded.has_expanded_storage, p.has_expanded_storage);
        assert_eq!(decoded.expiry_time_binary, p.expiry_time_binary);
    }

    #[test]
    fn combine_item_roundtrip() {
        let p = SCombineItem {
            grid: 2,
            id_from: 0x1111_2222_3333_4444,
            id_to: 0xAAAA_BBBB_CCCC_DDDD,
            success: true,
            destroy: false,
        };

        let raw = p.encode().expect("encode SCombineItem");
        assert_eq!(raw.id, ServerPacketId::CombineItem as i16);

        let decoded = SCombineItem::decode(&raw.payload).expect("decode SCombineItem");
        assert_eq!(decoded.grid, p.grid);
        assert_eq!(decoded.id_from, p.id_from);
        assert_eq!(decoded.id_to, p.id_to);
        assert_eq!(decoded.success, p.success);
        assert_eq!(decoded.destroy, p.destroy);
    }

    #[test]
    fn item_upgraded_roundtrip() {
        let p = SItemUpgraded {
            item_bytes: vec![9, 9, 9, 9],
        };

        let raw = p.encode().expect("encode SItemUpgraded");
        assert_eq!(raw.id, ServerPacketId::ItemUpgraded as i16);
        assert_eq!(raw.payload, p.item_bytes);

        let decoded = SItemUpgraded::decode(&raw.payload).expect("decode SItemUpgraded");
        assert_eq!(decoded.item_bytes, p.item_bytes);
    }

    #[test]
    fn new_item_info_structured_roundtrip() {
        let info = ItemInfoData {
            index: 1,
            name: "Sword".to_string(),
            item_type: 2,
            grade: 1,
            required_type: 0,
            required_class: 0,
            required_gender: 0,
            set: 0,
            shape: 10,
            weight: 5,
            light: 0,
            required_amount: 0,
            image: 100,
            durability: 200,
            stack_size: 50,
            price: 1000,
            start_item: false,
            effect: 0,
            need_identify: false,
            show_group_pickup: false,
            class_based: false,
            level_based: false,
            can_mine: false,
            global_drop_notify: false,
            bind: 0,
            unique: 0,
            random_stats_id: 0,
            can_fast_run: false,
            can_awakening: false,
            slots: 0,
            stats: StatsMap { entries: vec![] },
            tooltip: None,
        };

        let pkt = SNewItemInfo::from_item_info(&info).expect("from_item_info");
        let raw = pkt.encode().expect("encode SNewItemInfo");
        assert_eq!(raw.id, ServerPacketId::NewItemInfo as i16);

        let decoded_pkt = SNewItemInfo::decode(&raw.payload).expect("decode SNewItemInfo");
        let decoded_info = decoded_pkt
            .decode_item_info()
            .expect("decode_item_info");
        assert_eq!(decoded_info, info);
    }

    #[test]
    fn new_chat_item_structured_roundtrip() {
        let item = UserItemData {
            unique_id: 1,
            item_index: 10,
            current_dura: 50,
            max_dura: 100,
            count: 1,
            soul_bound_id: -1,
            identified: true,
            cursed: false,
            slots: Vec::new(),
            gem_count: 0,
            added_stats: StatsMap { entries: vec![] },
            awake: AwakeData {
                awake_type: 0,
                values: Vec::new(),
            },
            refined_value: 0,
            refine_added: 0,
            refine_success_chance: 0,
            wedding_ring: -1,
            expire_info: None,
            rental_information: None,
            is_shop_item: false,
            sealed_info: None,
            gm_made: false,
        };

        let pkt = SNewChatItem::from_user_item(&item).expect("from_user_item");
        let raw = pkt.encode().expect("encode SNewChatItem");

        let decoded_pkt = SNewChatItem::decode(&raw.payload).expect("decode SNewChatItem");
        let decoded_item = decoded_pkt
            .decode_user_item()
            .expect("decode_user_item");
        assert_eq!(decoded_item, item);
    }

    #[test]
    fn refresh_item_structured_roundtrip() {
        let item = UserItemData {
            unique_id: 2,
            item_index: 20,
            current_dura: 10,
            max_dura: 10,
            count: 5,
            soul_bound_id: 0,
            identified: false,
            cursed: false,
            slots: Vec::new(),
            gem_count: 0,
            added_stats: StatsMap { entries: vec![] },
            awake: AwakeData {
                awake_type: 0,
                values: Vec::new(),
            },
            refined_value: 0,
            refine_added: 0,
            refine_success_chance: 0,
            wedding_ring: -1,
            expire_info: None,
            rental_information: None,
            is_shop_item: false,
            sealed_info: None,
            gm_made: false,
        };

        let pkt = SRefreshItem::from_user_item(&item).expect("from_user_item");
        let raw = pkt.encode().expect("encode SRefreshItem");

        let decoded_pkt = SRefreshItem::decode(&raw.payload).expect("decode SRefreshItem");
        let decoded_item = decoded_pkt
            .decode_user_item()
            .expect("decode_user_item");
        assert_eq!(decoded_item, item);
    }

    #[test]
    fn chat_item_stats_structured_roundtrip() {
        let item = UserItemData {
            unique_id: 3,
            item_index: 30,
            current_dura: 1,
            max_dura: 1,
            count: 1,
            soul_bound_id: -1,
            identified: true,
            cursed: true,
            slots: Vec::new(),
            gem_count: 0,
            added_stats: StatsMap { entries: vec![] },
            awake: AwakeData {
                awake_type: 0,
                values: Vec::new(),
            },
            refined_value: 0,
            refine_added: 0,
            refine_success_chance: 0,
            wedding_ring: -1,
            expire_info: None,
            rental_information: None,
            is_shop_item: false,
            sealed_info: None,
            gm_made: false,
        };

        let chat_item_id = 0xAABB_CCDD_EEFF_0011;
        let pkt = SChatItemStats::from_user_item(chat_item_id, &item)
            .expect("from_user_item");
        let raw = pkt.encode().expect("encode SChatItemStats");

        let decoded_pkt = SChatItemStats::decode(&raw.payload).expect("decode SChatItemStats");
        assert_eq!(decoded_pkt.chat_item_id, chat_item_id);
        let decoded_item = decoded_pkt
            .decode_user_item()
            .expect("decode_user_item");
        assert_eq!(decoded_item, item);
    }
}
