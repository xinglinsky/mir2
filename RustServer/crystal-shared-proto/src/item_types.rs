use std::io::{self, Read, Write};

use crate::io::{
    read_bool, read_i16_le, read_i32_le, read_i64_le, read_string, read_u16_le, read_u32_le,
    read_u64_le, write_bool, write_i16_le, write_i32_le, write_i64_le, write_string,
    write_u16_le, write_u32_le, write_u64_le,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatsMap {
    pub entries: Vec<(u8, i32)>,
}

impl StatsMap {
    pub fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let count: i32 = self
            .entries
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many stats"))?;
        write_i32_le(w, count)?;
        for (stat, value) in &self.entries {
            w.write_all(&[*stat])?;
            write_i32_le(w, *value)?;
        }
        Ok(())
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
        let count = read_i32_le(r)?;
        if count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative stats count",
            ));
        }
        let mut entries = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let mut b = [0u8; 1];
            r.read_exact(&mut b)?;
            let stat = b[0];
            let value = read_i32_le(r)?;
            entries.push((stat, value));
        }
        Ok(StatsMap { entries })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemInfoData {
    pub index: i32,
    pub name: String,
    pub item_type: u8,
    pub grade: u8,
    pub required_type: u8,
    pub required_class: u8,
    pub required_gender: u8,
    pub set: u8,
    pub shape: i16,
    pub weight: u8,
    pub light: u8,
    pub required_amount: u8,
    pub image: u16,
    pub durability: u16,
    pub stack_size: u16,
    pub price: u32,
    pub start_item: bool,
    pub effect: u8,
    pub need_identify: bool,
    pub show_group_pickup: bool,
    pub class_based: bool,
    pub level_based: bool,
    pub can_mine: bool,
    pub global_drop_notify: bool,
    pub bind: i16,
    pub unique: i16,
    pub random_stats_id: u8,
    pub can_fast_run: bool,
    pub can_awakening: bool,
    pub slots: u8,
    pub stats: StatsMap,
    pub tooltip: Option<String>,
}

impl ItemInfoData {
    pub fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write_i32_le(w, self.index)?;
        write_string(w, &self.name)?;
        w.write_all(&[self.item_type])?;
        w.write_all(&[self.grade])?;
        w.write_all(&[self.required_type])?;
        w.write_all(&[self.required_class])?;
        w.write_all(&[self.required_gender])?;
        w.write_all(&[self.set])?;

        write_i16_le(w, self.shape)?;
        w.write_all(&[self.weight])?;
        w.write_all(&[self.light])?;
        w.write_all(&[self.required_amount])?;

        write_u16_le(w, self.image)?;
        write_u16_le(w, self.durability)?;
        write_u16_le(w, self.stack_size)?;
        write_u32_le(w, self.price)?;

        write_bool(w, self.start_item)?;

        w.write_all(&[self.effect])?;

        let mut bools = 0u8;
        if self.need_identify {
            bools |= 0x01;
        }
        if self.show_group_pickup {
            bools |= 0x02;
        }
        if self.class_based {
            bools |= 0x04;
        }
        if self.level_based {
            bools |= 0x08;
        }
        if self.can_mine {
            bools |= 0x10;
        }
        if self.global_drop_notify {
            bools |= 0x20;
        }
        w.write_all(&[bools])?;

        write_i16_le(w, self.bind)?;
        write_i16_le(w, self.unique)?;

        w.write_all(&[self.random_stats_id])?;

        write_bool(w, self.can_fast_run)?;
        write_bool(w, self.can_awakening)?;
        w.write_all(&[self.slots])?;

        self.stats.encode(w)?;

        write_bool(w, self.tooltip.is_some())?;
        if let Some(text) = &self.tooltip {
            write_string(w, text)?;
        }

        Ok(())
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
        let index = read_i32_le(r)?;
        let name = read_string(r)?;

        let mut b = [0u8; 1];
        r.read_exact(&mut b)?;
        let item_type = b[0];
        r.read_exact(&mut b)?;
        let grade = b[0];
        r.read_exact(&mut b)?;
        let required_type = b[0];
        r.read_exact(&mut b)?;
        let required_class = b[0];
        r.read_exact(&mut b)?;
        let required_gender = b[0];
        r.read_exact(&mut b)?;
        let set = b[0];

        let shape = read_i16_le(r)?;
        r.read_exact(&mut b)?;
        let weight = b[0];
        r.read_exact(&mut b)?;
        let light = b[0];
        r.read_exact(&mut b)?;
        let required_amount = b[0];

        let image = read_u16_le(r)?;
        let durability = read_u16_le(r)?;
        let stack_size = read_u16_le(r)?;
        let price = read_u32_le(r)?;

        let start_item = read_bool(r)?;

        r.read_exact(&mut b)?;
        let effect = b[0];

        r.read_exact(&mut b)?;
        let bools = b[0];
        let need_identify = (bools & 0x01) != 0;
        let show_group_pickup = (bools & 0x02) != 0;
        let class_based = (bools & 0x04) != 0;
        let level_based = (bools & 0x08) != 0;
        let can_mine = (bools & 0x10) != 0;
        let global_drop_notify = (bools & 0x20) != 0;

        let bind = read_i16_le(r)?;
        let unique = read_i16_le(r)?;

        r.read_exact(&mut b)?;
        let random_stats_id = b[0];

        let can_fast_run = read_bool(r)?;
        let can_awakening = read_bool(r)?;
        r.read_exact(&mut b)?;
        let slots = b[0];

        let stats = StatsMap::decode(r)?;

        let has_tooltip = read_bool(r)?;
        let tooltip = if has_tooltip {
            Some(read_string(r)?)
        } else {
            None
        };

        Ok(ItemInfoData {
            index,
            name,
            item_type,
            grade,
            required_type,
            required_class,
            required_gender,
            set,
            shape,
            weight,
            light,
            required_amount,
            image,
            durability,
            stack_size,
            price,
            start_item,
            effect,
            need_identify,
            show_group_pickup,
            class_based,
            level_based,
            can_mine,
            global_drop_notify,
            bind,
            unique,
            random_stats_id,
            can_fast_run,
            can_awakening,
            slots,
            stats,
            tooltip,
        })
    }

    pub fn encode_to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.encode(&mut buf)?;
        Ok(buf)
    }

    pub fn decode_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let mut c = std::io::Cursor::new(bytes);
        Self::decode(&mut c)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpireInfoData {
    pub expiry_binary: i64,
}

impl ExpireInfoData {
    pub fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write_i64_le(w, self.expiry_binary)
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
        let expiry_binary = read_i64_le(r)?;
        Ok(ExpireInfoData { expiry_binary })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealedInfoData {
    pub expiry_binary: i64,
    pub next_seal_binary: i64,
}

impl SealedInfoData {
    pub fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write_i64_le(w, self.expiry_binary)?;
        write_i64_le(w, self.next_seal_binary)
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
        let expiry_binary = read_i64_le(r)?;
        let next_seal_binary = read_i64_le(r)?;
        Ok(SealedInfoData {
            expiry_binary,
            next_seal_binary,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RentalInformationData {
    pub owner_name: String,
    pub binding_flags: i16,
    pub expiry_binary: i64,
    pub rental_locked: bool,
}

impl RentalInformationData {
    pub fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write_string(w, &self.owner_name)?;
        write_i16_le(w, self.binding_flags)?;
        write_i64_le(w, self.expiry_binary)?;
        write_bool(w, self.rental_locked)
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
        let owner_name = read_string(r)?;
        let binding_flags = read_i16_le(r)?;
        let expiry_binary = read_i64_le(r)?;
        let rental_locked = read_bool(r)?;
        Ok(RentalInformationData {
            owner_name,
            binding_flags,
            expiry_binary,
            rental_locked,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AwakeData {
    pub awake_type: u8,
    pub values: Vec<u8>,
}

impl AwakeData {
    pub fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&[self.awake_type])?;
        let count: i32 = self
            .values
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many awake values"))?;
        write_i32_le(w, count)?;
        for v in &self.values {
            w.write_all(&[*v])?;
        }
        Ok(())
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut b = [0u8; 1];
        r.read_exact(&mut b)?;
        let awake_type = b[0];
        let count = read_i32_le(r)?;
        if count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative awake count",
            ));
        }
        let mut values = Vec::with_capacity(count as usize);
        for _ in 0..count {
            r.read_exact(&mut b)?;
            values.push(b[0]);
        }
        Ok(AwakeData { awake_type, values })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserItemData {
    pub unique_id: u64,
    pub item_index: i32,
    pub current_dura: u16,
    pub max_dura: u16,
    pub count: u16,
    pub soul_bound_id: i32,
    pub identified: bool,
    pub cursed: bool,
    pub slots: Vec<Option<Box<UserItemData>>>,
    pub gem_count: u16,
    pub added_stats: StatsMap,
    pub awake: AwakeData,
    pub refined_value: u8,
    pub refine_added: u8,
    pub refine_success_chance: i32,
    pub wedding_ring: i32,
    pub expire_info: Option<ExpireInfoData>,
    pub rental_information: Option<RentalInformationData>,
    pub is_shop_item: bool,
    pub sealed_info: Option<SealedInfoData>,
    pub gm_made: bool,
}

impl UserItemData {
    pub fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write_u64_le(w, self.unique_id)?;
        write_i32_le(w, self.item_index)?;
        write_u16_le(w, self.current_dura)?;
        write_u16_le(w, self.max_dura)?;
        write_u16_le(w, self.count)?;
        write_i32_le(w, self.soul_bound_id)?;

        let mut b = 0u8;
        if self.identified {
            b |= 0x01;
        }
        if self.cursed {
            b |= 0x02;
        }
        w.write_all(&[b])?;

        let slots_len: i32 = self
            .slots
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many slots"))?;
        write_i32_le(w, slots_len)?;
        for slot in &self.slots {
            match slot {
                None => {
                    write_bool(w, true)?;
                }
                Some(item) => {
                    write_bool(w, false)?;
                    item.encode(w)?;
                }
            }
        }

        write_u16_le(w, self.gem_count)?;

        self.added_stats.encode(w)?;
        self.awake.encode(w)?;

        w.write_all(&[self.refined_value])?;
        w.write_all(&[self.refine_added])?;
        write_i32_le(w, self.refine_success_chance)?;

        write_i32_le(w, self.wedding_ring)?;

        write_bool(w, self.expire_info.is_some())?;
        if let Some(info) = &self.expire_info {
            info.encode(w)?;
        }

        write_bool(w, self.rental_information.is_some())?;
        if let Some(info) = &self.rental_information {
            info.encode(w)?;
        }

        write_bool(w, self.is_shop_item)?;

        write_bool(w, self.sealed_info.is_some())?;
        if let Some(info) = &self.sealed_info {
            info.encode(w)?;
        }

        write_bool(w, self.gm_made)?;

        Ok(())
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
        let unique_id = read_u64_le(r)?;
        let item_index = read_i32_le(r)?;
        let current_dura = read_u16_le(r)?;
        let max_dura = read_u16_le(r)?;
        let count = read_u16_le(r)?;
        let soul_bound_id = read_i32_le(r)?;

        let mut b1 = [0u8; 1];
        r.read_exact(&mut b1)?;
        let bools = b1[0];
        let identified = (bools & 0x01) != 0;
        let cursed = (bools & 0x02) != 0;

        let slot_count = read_i32_le(r)?;
        if slot_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative slot count",
            ));
        }
        let mut slots = Vec::with_capacity(slot_count as usize);
        for _ in 0..slot_count {
            let has_null = read_bool(r)?;
            if has_null {
                slots.push(None);
            } else {
                let item = UserItemData::decode(r)?;
                slots.push(Some(Box::new(item)));
            }
        }

        let gem_count = read_u16_le(r)?;

        let added_stats = StatsMap::decode(r)?;
        let awake = AwakeData::decode(r)?;

        r.read_exact(&mut b1)?;
        let refined_value = b1[0];
        r.read_exact(&mut b1)?;
        let refine_added = b1[0];
        let refine_success_chance = read_i32_le(r)?;

        let wedding_ring = read_i32_le(r)?;

        let has_expire = read_bool(r)?;
        let expire_info = if has_expire {
            Some(ExpireInfoData::decode(r)?)
        } else {
            None
        };

        let has_rental = read_bool(r)?;
        let rental_information = if has_rental {
            Some(RentalInformationData::decode(r)?)
        } else {
            None
        };

        let is_shop_item = read_bool(r)?;

        let has_sealed = read_bool(r)?;
        let sealed_info = if has_sealed {
            Some(SealedInfoData::decode(r)?)
        } else {
            None
        };

        let gm_made = read_bool(r)?;

        Ok(UserItemData {
            unique_id,
            item_index,
            current_dura,
            max_dura,
            count,
            soul_bound_id,
            identified,
            cursed,
            slots,
            gem_count,
            added_stats,
            awake,
            refined_value,
            refine_added,
            refine_success_chance,
            wedding_ring,
            expire_info,
            rental_information,
            is_shop_item,
            sealed_info,
            gm_made,
        })
    }

    pub fn encode_to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.encode(&mut buf)?;
        Ok(buf)
    }

    pub fn decode_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let mut c = std::io::Cursor::new(bytes);
        Self::decode(&mut c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_map_roundtrip() {
        let stats = StatsMap {
            entries: vec![(1, 10), (5, -3)],
        };
        let mut buf = Vec::new();
        stats.encode(&mut buf).unwrap();
        let mut c = std::io::Cursor::new(&buf);
        let decoded = StatsMap::decode(&mut c).unwrap();
        assert_eq!(decoded, stats);
    }

    #[test]
    fn item_info_roundtrip() {
        let item = ItemInfoData {
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
        let bytes = item.encode_to_bytes().unwrap();
        let decoded = ItemInfoData::decode_from_bytes(&bytes).unwrap();
        assert_eq!(decoded, item);
    }

    #[test]
    fn user_item_roundtrip_simple() {
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
        let bytes = item.encode_to_bytes().unwrap();
        let decoded = UserItemData::decode_from_bytes(&bytes).unwrap();
        assert_eq!(decoded, item);
    }
}
