use std::io::{self, Cursor, Read};

use crate::io::{
    read_bool, read_i32_le, read_i64_le, read_string, read_u16_le, read_u32_le, write_bool,
    write_i32_le, write_i64_le, write_string, write_u16_le, write_u32_le,
};
use crate::item_types::UserItemData;
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct SUserInformation {
    pub object_id: u32,
    pub real_id: u32,
    pub name: String,
    pub guild_name: String,
    pub guild_rank: String,
    /// ARGB color as i32, matches System.Drawing.Color.ToArgb
    pub name_colour_argb: i32,
    pub class: u8,
    pub gender: u8,
    pub level: u16,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
    pub hair: u8,
    pub hp: i32,
    pub mp: i32,
    pub experience: i64,
    pub max_experience: i64,
    pub level_effects: u16,
    pub has_hero: bool,
    pub hero_behaviour: u8,
    pub gold: u32,
    pub credit: u32,
    pub has_expanded_storage: bool,
    pub expanded_storage_expiry_binary: i64,
    /// Raw ClientMagic.Save(writer) bytes for each learned magic.
    pub magics: Vec<Vec<u8>>,
    pub summoned_creature_type: u8,
    pub creature_summoned: bool,
    pub allow_observe: bool,
    pub observer: bool,
}

#[derive(Clone, Debug)]
pub struct SUserSlotsRefresh {
    pub inventory: Vec<Option<UserItemData>>,
    pub equipment: Vec<Option<UserItemData>>,
}

impl SUserSlotsRefresh {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();

        write_bool(&mut buf, true)?;
        let inv_len: i32 = self
            .inventory
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many inventory slots"))?;
        write_i32_le(&mut buf, inv_len)?;
        for slot in &self.inventory {
            match slot {
                None => write_bool(&mut buf, false)?,
                Some(item) => {
                    write_bool(&mut buf, true)?;
                    item.encode(&mut buf)?;
                }
            }
        }

        write_bool(&mut buf, true)?;
        let eq_len: i32 = self
            .equipment
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many equipment slots"))?;
        write_i32_le(&mut buf, eq_len)?;
        for slot in &self.equipment {
            match slot {
                None => write_bool(&mut buf, false)?,
                Some(item) => {
                    write_bool(&mut buf, true)?;
                    item.encode(&mut buf)?;
                }
            }
        }

        Ok(RawPacket {
            id: ServerPacketId::UserSlotsRefresh as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);

        let has_inventory = read_bool(&mut c)?;
        let inventory = if has_inventory {
            let inv_len = read_i32_le(&mut c)?;
            if inv_len < 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "negative inventory length",
                ));
            }
            let mut slots = Vec::with_capacity(inv_len as usize);
            for _ in 0..inv_len {
                let has_item = read_bool(&mut c)?;
                if has_item {
                    let item = UserItemData::decode(&mut c)?;
                    slots.push(Some(item));
                } else {
                    slots.push(None);
                }
            }
            slots
        } else {
            Vec::new()
        };

        let has_equipment = read_bool(&mut c)?;
        let equipment = if has_equipment {
            let eq_len = read_i32_le(&mut c)?;
            if eq_len < 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "negative equipment length",
                ));
            }
            let mut slots = Vec::with_capacity(eq_len as usize);
            for _ in 0..eq_len {
                let has_item = read_bool(&mut c)?;
                if has_item {
                    let item = UserItemData::decode(&mut c)?;
                    slots.push(Some(item));
                } else {
                    slots.push(None);
                }
            }
            slots
        } else {
            Vec::new()
        };

        Ok(SUserSlotsRefresh {
            inventory,
            equipment,
        })
    }
}

impl SUserInformation {
    /// Encode using the exact layout of C# ServerPackets.UserInformation, but with
    /// Inventory/Equipment/QuestInventory all treated as null, and with empty Magics
    /// and IntelligentCreatures lists.
    pub fn encode(&self) -> io::Result<RawPacket> {
        let empty_inv: Vec<Option<UserItemData>> = Vec::new();
        let empty_eq: Vec<Option<UserItemData>> = Vec::new();
        let empty_quest: Vec<Option<UserItemData>> = Vec::new();
        self.encode_with_items(&empty_inv, &empty_eq, &empty_quest)
    }

    pub fn encode_with_items(
        &self,
        inventory: &[Option<UserItemData>],
        equipment: &[Option<UserItemData>],
        quest_inventory: &[Option<UserItemData>],
    ) -> io::Result<RawPacket> {
        let mut buf = Vec::new();

        // Header fields
        write_u32_le(&mut buf, self.object_id)?;
        write_u32_le(&mut buf, self.real_id)?;
        write_string(&mut buf, &self.name)?;
        write_string(&mut buf, &self.guild_name)?;
        write_string(&mut buf, &self.guild_rank)?;
        write_i32_le(&mut buf, self.name_colour_argb)?;
        buf.push(self.class);
        buf.push(self.gender);
        write_u16_le(&mut buf, self.level)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        buf.push(self.hair);
        write_i32_le(&mut buf, self.hp)?;
        write_i32_le(&mut buf, self.mp)?;

        write_i64_le(&mut buf, self.experience)?;
        write_i64_le(&mut buf, self.max_experience)?;

        write_u16_le(&mut buf, self.level_effects)?;
        write_bool(&mut buf, self.has_hero)?;
        buf.push(self.hero_behaviour);

        // Inventory: present with fixed length 46.
        write_bool(&mut buf, true)?; // Inventory != null
        let inv_len = if inventory.is_empty() { 46 } else { inventory.len() };
        write_i32_le(&mut buf, inv_len as i32)?;
        if inventory.is_empty() {
            for _ in 0..inv_len {
                write_bool(&mut buf, false)?;
            }
        } else {
            if inv_len != 46 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "SUserInformation inventory length must be 46",
                ));
            }
            for slot in inventory {
                match slot {
                    None => write_bool(&mut buf, false)?,
                    Some(item) => {
                        write_bool(&mut buf, true)?;
                        item.encode(&mut buf)?;
                    }
                }
            }
        }

        // Equipment: present with fixed length 14.
        write_bool(&mut buf, true)?; // Equipment != null
        let eq_len = if equipment.is_empty() { 14 } else { equipment.len() };
        write_i32_le(&mut buf, eq_len as i32)?;
        if equipment.is_empty() {
            for _ in 0..eq_len {
                write_bool(&mut buf, false)?;
            }
        } else {
            if eq_len != 14 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "SUserInformation equipment length must be 14",
                ));
            }
            for slot in equipment {
                match slot {
                    None => write_bool(&mut buf, false)?,
                    Some(item) => {
                        write_bool(&mut buf, true)?;
                        item.encode(&mut buf)?;
                    }
                }
            }
        }

        // QuestInventory: present with fixed length 40.
        write_bool(&mut buf, true)?; // QuestInventory != null
        let quest_len = if quest_inventory.is_empty() {
            40
        } else {
            quest_inventory.len()
        };
        write_i32_le(&mut buf, quest_len as i32)?;
        if quest_inventory.is_empty() {
            for _ in 0..quest_len {
                write_bool(&mut buf, false)?;
            }
        } else {
            if quest_len != 40 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "SUserInformation quest inventory length must be 40",
                ));
            }
            for slot in quest_inventory {
                match slot {
                    None => write_bool(&mut buf, false)?,
                    Some(item) => {
                        write_bool(&mut buf, true)?;
                        item.encode(&mut buf)?;
                    }
                }
            }
        }

        write_u32_le(&mut buf, self.gold)?;
        write_u32_le(&mut buf, self.credit)?;

        write_bool(&mut buf, self.has_expanded_storage)?;
        write_i64_le(&mut buf, self.expanded_storage_expiry_binary)?;

        // Magics: variable-length list of ClientMagic.Save(writer) payloads.
        let magics_count: i32 = self
            .magics
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many magics"))?;
        write_i32_le(&mut buf, magics_count)?;
        for magic in &self.magics {
            buf.extend_from_slice(magic);
        }

        // IntelligentCreatures: count = 0
        write_i32_le(&mut buf, 0)?;

        buf.push(self.summoned_creature_type);
        write_bool(&mut buf, self.creature_summoned)?;
        write_bool(&mut buf, self.allow_observe)?;
        write_bool(&mut buf, self.observer)?;

        Ok(RawPacket {
            id: ServerPacketId::UserInformation as i16,
            payload: buf,
        })
    }

    /// Decode only the subset that this module knows how to encode (no inventory/equipment,
    /// no magics/creatures). This is primarily for roundtrip tests of the Rust implementation.
    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);

        let object_id = read_u32_le(&mut c)?;
        let real_id = read_u32_le(&mut c)?;
        let name = read_string(&mut c)?;
        let guild_name = read_string(&mut c)?;
        let guild_rank = read_string(&mut c)?;
        let name_colour_argb = read_i32_le(&mut c)?;

        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let class = one[0];
        c.read_exact(&mut one)?;
        let gender = one[0];

        let level = read_u16_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        c.read_exact(&mut one)?;
        let direction = one[0];
        c.read_exact(&mut one)?;
        let hair = one[0];
        let hp = read_i32_le(&mut c)?;
        let mp = read_i32_le(&mut c)?;

        let experience = read_i64_le(&mut c)?;
        let max_experience = read_i64_le(&mut c)?;

        let level_effects = read_u16_le(&mut c)?;
        let has_hero = read_bool(&mut c)?;
        c.read_exact(&mut one)?;
        let hero_behaviour = one[0];

        // Inventory: expect present, length 46, all slots null.
        let has_inventory = read_bool(&mut c)?;
        if !has_inventory {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SUserInformation decode expects inventory to be present",
            ));
        }
        let inv_len = read_i32_le(&mut c)?;
        if inv_len != 46 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SUserInformation inventory length must be 46",
            ));
        }
        for _ in 0..inv_len {
            let has_item = read_bool(&mut c)?;
            if has_item {
                let _ = UserItemData::decode(&mut c)?;
            }
        }

        // Equipment: expect present, length 14, all slots null.
        let has_equipment = read_bool(&mut c)?;
        if !has_equipment {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SUserInformation decode expects equipment to be present",
            ));
        }
        let eq_len = read_i32_le(&mut c)?;
        if eq_len != 14 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SUserInformation equipment length must be 14",
            ));
        }
        for _ in 0..eq_len {
            let has_item = read_bool(&mut c)?;
            if has_item {
                let _ = UserItemData::decode(&mut c)?;
            }
        }

        // QuestInventory: expect present, length 40, all slots null.
        let has_quest_inventory = read_bool(&mut c)?;
        if !has_quest_inventory {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SUserInformation decode expects quest inventory to be present",
            ));
        }
        let quest_len = read_i32_le(&mut c)?;
        if quest_len != 40 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SUserInformation quest inventory length must be 40",
            ));
        }
        for _ in 0..quest_len {
            let has_item = read_bool(&mut c)?;
            if has_item {
                let _ = UserItemData::decode(&mut c)?;
            }
        }

        let gold = read_u32_le(&mut c)?;
        let credit = read_u32_le(&mut c)?;

        let has_expanded_storage = read_bool(&mut c)?;
        let expanded_storage_expiry_binary = read_i64_le(&mut c)?;

        let magics_count = read_i32_le(&mut c)?;
        if magics_count != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SUserInformation decode only supports zero magics",
            ));
        }

        let creatures_count = read_i32_le(&mut c)?;
        if creatures_count != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SUserInformation decode only supports zero intelligent creatures",
            ));
        }

        c.read_exact(&mut one)?;
        let summoned_creature_type = one[0];
        let creature_summoned = read_bool(&mut c)?;
        let allow_observe = read_bool(&mut c)?;
        let observer = read_bool(&mut c)?;

        Ok(SUserInformation {
            object_id,
            real_id,
            name,
            guild_name,
            guild_rank,
            name_colour_argb,
            class,
            gender,
            level,
            location_x,
            location_y,
            direction,
            hair,
            hp,
            mp,
            experience,
            max_experience,
            level_effects,
            has_hero,
            hero_behaviour,
            gold,
            credit,
            has_expanded_storage,
            expanded_storage_expiry_binary,
            magics: Vec::new(),
            summoned_creature_type,
            creature_summoned,
            allow_observe,
            observer,
        })
    }
}

