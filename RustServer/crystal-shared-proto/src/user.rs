// User-related packets (e.g. UserInformation) compatible with the C# Crystal implementation.
//
// For now we implement a minimal SUserInformation that assumes empty inventory/equipment/
// quest inventory and no magics/creatures. This is sufficient for stub servers that
// only need to send basic player state.

use std::io::{self, Cursor, Read};

use crate::io::{
    read_bool, read_i32_le, read_i64_le, read_string, read_u16_le, read_u32_le, write_bool,
    write_i32_le, write_i64_le, write_string, write_u16_le, write_u32_le,
};
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
    pub summoned_creature_type: u8,
    pub creature_summoned: bool,
    pub allow_observe: bool,
    pub observer: bool,
}

impl SUserInformation {
    /// Encode using the exact layout of C# ServerPackets.UserInformation, but with
    /// Inventory/Equipment/QuestInventory all treated as null, and with empty Magics
    /// and IntelligentCreatures lists.
    pub fn encode(&self) -> io::Result<RawPacket> {
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

        // Inventory: present with fixed length 46, all slots empty.
        write_bool(&mut buf, true)?; // Inventory != null
        write_i32_le(&mut buf, 46)?; // Inventory.Length
        for _ in 0..46 {
            write_bool(&mut buf, false)?; // slot is null
        }

        // Equipment: present with fixed length 14, all slots empty.
        write_bool(&mut buf, true)?; // Equipment != null
        write_i32_le(&mut buf, 14)?; // Equipment.Length
        for _ in 0..14 {
            write_bool(&mut buf, false)?;
        }

        // QuestInventory: present with fixed length 40, all slots empty.
        write_bool(&mut buf, true)?; // QuestInventory != null
        write_i32_le(&mut buf, 40)?; // QuestInventory.Length
        for _ in 0..40 {
            write_bool(&mut buf, false)?;
        }

        write_u32_le(&mut buf, self.gold)?;
        write_u32_le(&mut buf, self.credit)?;

        write_bool(&mut buf, self.has_expanded_storage)?;
        write_i64_le(&mut buf, self.expanded_storage_expiry_binary)?;

        // Magics: count = 0
        write_i32_le(&mut buf, 0)?;

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
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "SUserInformation decode only supports empty inventory",
                ));
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
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "SUserInformation decode only supports empty equipment",
                ));
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
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "SUserInformation decode only supports empty quest inventory",
                ));
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
            summoned_creature_type,
            creature_summoned,
            allow_observe,
            observer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_information_roundtrip_empty_collections() {
        let info = SUserInformation {
            object_id: 1,
            real_id: 1,
            name: "TestUser".to_string(),
            guild_name: "".to_string(),
            guild_rank: "".to_string(),
            name_colour_argb: -1,
            class: 0,
            gender: 0,
            level: 1,
            location_x: 100,
            location_y: 200,
            direction: 0,
            hair: 0,
            hp: 50,
            mp: 30,
            experience: 123,
            max_experience: 456,
            level_effects: 0,
            has_hero: false,
            hero_behaviour: 0,
            gold: 1000,
            credit: 0,
            has_expanded_storage: false,
            expanded_storage_expiry_binary: 0,
            summoned_creature_type: 0,
            creature_summoned: false,
            allow_observe: false,
            observer: false,
        };

        let raw = info.encode().expect("encode SUserInformation");
        assert_eq!(raw.id, ServerPacketId::UserInformation as i16);

        let decoded = SUserInformation::decode(&raw.payload).expect("decode SUserInformation");
        assert_eq!(decoded.object_id, info.object_id);
        assert_eq!(decoded.real_id, info.real_id);
        assert_eq!(decoded.name, info.name);
        assert_eq!(decoded.guild_name, info.guild_name);
        assert_eq!(decoded.guild_rank, info.guild_rank);
        assert_eq!(decoded.name_colour_argb, info.name_colour_argb);
        assert_eq!(decoded.class, info.class);
        assert_eq!(decoded.gender, info.gender);
        assert_eq!(decoded.level, info.level);
        assert_eq!(decoded.location_x, info.location_x);
        assert_eq!(decoded.location_y, info.location_y);
        assert_eq!(decoded.direction, info.direction);
        assert_eq!(decoded.hair, info.hair);
        assert_eq!(decoded.hp, info.hp);
        assert_eq!(decoded.mp, info.mp);
        assert_eq!(decoded.experience, info.experience);
        assert_eq!(decoded.max_experience, info.max_experience);
        assert_eq!(decoded.level_effects, info.level_effects);
        assert_eq!(decoded.has_hero, info.has_hero);
        assert_eq!(decoded.hero_behaviour, info.hero_behaviour);
        assert_eq!(decoded.gold, info.gold);
        assert_eq!(decoded.credit, info.credit);
        assert_eq!(decoded.has_expanded_storage, info.has_expanded_storage);
        assert_eq!(
            decoded.expanded_storage_expiry_binary,
            info.expanded_storage_expiry_binary
        );
        assert_eq!(decoded.summoned_creature_type, info.summoned_creature_type);
        assert_eq!(decoded.creature_summoned, info.creature_summoned);
        assert_eq!(decoded.allow_observe, info.allow_observe);
        assert_eq!(decoded.observer, info.observer);
    }
}
