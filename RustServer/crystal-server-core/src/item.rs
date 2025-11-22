use std::io;
use crystal_shared_proto::item_types::{AwakeData, ItemInfoData, StatsMap, UserItemData};

pub const INVENTORY_SIZE: usize = 46;
pub const EQUIPMENT_SIZE: usize = 14;
pub const QUEST_INVENTORY_SIZE: usize = 40;

#[derive(Clone, Debug, Default)]
pub struct Inventory {
    pub slots: Vec<Option<UserItemData>>,
}

impl Inventory {
    pub fn new_default() -> Self {
        Self {
            slots: vec![None; INVENTORY_SIZE],
        }
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn get(&self, index: usize) -> Option<&UserItemData> {
        self.slots.get(index).and_then(|s| s.as_ref())
    }

    pub fn set(&mut self, index: usize, item: Option<UserItemData>) {
        if index < self.slots.len() {
            self.slots[index] = item;
        }
    }
}

pub fn encode_item_slots(slots: &[Option<UserItemData>]) -> io::Result<Vec<Option<Vec<u8>>>> {
    let mut out = Vec::with_capacity(slots.len());
    for slot in slots {
        match slot {
            None => out.push(None),
            Some(item) => out.push(Some(item.encode_to_bytes()?)),
        }
    }
    Ok(out)
}

pub fn decode_item_slots(data: Vec<Option<Vec<u8>>>) -> io::Result<Vec<Option<UserItemData>>> {
    let mut out = Vec::with_capacity(data.len());
    for slot in data {
        match slot {
            None => out.push(None),
            Some(bytes) => {
                let item = UserItemData::decode_from_bytes(&bytes)?;
                out.push(Some(item));
            }
        }
    }
    Ok(out)
}

pub fn create_fresh_user_item(info: &ItemInfoData, unique_id: u64, count: u16) -> UserItemData {
    UserItemData {
        unique_id,
        item_index: info.index,
        current_dura: info.durability,
        max_dura: info.durability,
        count,
        soul_bound_id: -1,
        identified: !info.need_identify,
        cursed: false,
        slots: Vec::new(),
        gem_count: 0,
        added_stats: StatsMap { entries: Vec::new() },
        awake: AwakeData {
            awake_type: 0,
            values: Vec::new(),
        },
        refined_value: 0,
        refine_added: 0,
        refine_success_chance: 0,
        wedding_ring: 0,
        expire_info: None,
        rental_information: None,
        is_shop_item: false,
        sealed_info: None,
        gm_made: false,
    }
}

#[derive(Clone, Debug, Default)]
pub struct Equipment {
    pub slots: Vec<Option<UserItemData>>,
}

impl Equipment {
    pub fn new_default() -> Self {
        Self {
            slots: vec![None; EQUIPMENT_SIZE],
        }
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn get(&self, index: usize) -> Option<&UserItemData> {
        self.slots.get(index).and_then(|s| s.as_ref())
    }

    pub fn set(&mut self, index: usize, item: Option<UserItemData>) {
        if index < self.slots.len() {
            self.slots[index] = item;
        }
    }
}
