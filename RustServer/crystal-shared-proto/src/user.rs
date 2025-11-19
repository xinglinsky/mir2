// User-related packets (e.g. UserInformation) compatible with the C# Crystal implementation.
//
// For now we implement a minimal SUserInformation that assumes empty inventory/equipment/
// quest inventory and no magics/creatures. This is sufficient for stub servers that
// only need to send basic player state.

use std::io::{self, Cursor, Read};

use crate::io::{
    read_bool, read_i16_le, read_i32_le, read_i64_le, read_string, read_u16_le, read_u32_le,
    read_u64_le, write_bool, write_i16_le, write_i32_le, write_i64_le, write_string,
    write_u16_le, write_u32_le, write_u64_le,
};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;
use crate::item_types::UserItemData;

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

#[derive(Clone, Debug)]
pub struct SUserLocation {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SUserLocation {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::UserLocation as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SUserLocation {
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectPlayer {
    pub object_id: u32,
    pub name: String,
    pub guild_name: String,
    pub guild_rank_name: String,
    pub name_colour_argb: i32,
    pub class: u8,
    pub gender: u8,
    pub level: u16,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
    pub hair: u8,
    pub light: u8,
    pub weapon: i16,
    pub weapon_effect: i16,
    pub armour: i16,
    pub poison: u16,
    pub dead: bool,
    pub hidden: bool,
    pub effect: u8,
    pub wing_effect: u8,
    pub extra: bool,
    pub mount_type: i16,
    pub riding_mount: bool,
    pub fishing: bool,
    pub transform_type: i16,
    pub element_orb_effect: u32,
    pub element_orb_lvl: u32,
    pub element_orb_max: u32,
    pub buffs: Vec<u8>,
    pub level_effects: u16,
}

impl SObjectPlayer {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();

        write_u32_le(&mut buf, self.object_id)?;
        write_string(&mut buf, &self.name)?;
        write_string(&mut buf, &self.guild_name)?;
        write_string(&mut buf, &self.guild_rank_name)?;
        write_i32_le(&mut buf, self.name_colour_argb)?;
        buf.push(self.class);
        buf.push(self.gender);
        write_u16_le(&mut buf, self.level)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        buf.push(self.hair);
        buf.push(self.light);
        write_i16_le(&mut buf, self.weapon)?;
        write_i16_le(&mut buf, self.weapon_effect)?;
        write_i16_le(&mut buf, self.armour)?;
        write_u16_le(&mut buf, self.poison)?;
        write_bool(&mut buf, self.dead)?;
        write_bool(&mut buf, self.hidden)?;
        buf.push(self.effect);
        buf.push(self.wing_effect);
        write_bool(&mut buf, self.extra)?;
        write_i16_le(&mut buf, self.mount_type)?;
        write_bool(&mut buf, self.riding_mount)?;
        write_bool(&mut buf, self.fishing)?;
        write_i16_le(&mut buf, self.transform_type)?;
        write_u32_le(&mut buf, self.element_orb_effect)?;
        write_u32_le(&mut buf, self.element_orb_lvl)?;
        write_u32_le(&mut buf, self.element_orb_max)?;

        let count: i32 = self
            .buffs
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many buffs"))?;
        write_i32_le(&mut buf, count)?;
        buf.extend_from_slice(&self.buffs);

        write_u16_le(&mut buf, self.level_effects)?;

        Ok(RawPacket {
            id: ServerPacketId::ObjectPlayer as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);

        let object_id = read_u32_le(&mut c)?;
        let name = read_string(&mut c)?;
        let guild_name = read_string(&mut c)?;
        let guild_rank_name = read_string(&mut c)?;
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
        c.read_exact(&mut one)?;
        let light = one[0];

        let weapon = read_i16_le(&mut c)?;
        let weapon_effect = read_i16_le(&mut c)?;
        let armour = read_i16_le(&mut c)?;
        let poison = read_u16_le(&mut c)?;
        let dead = read_bool(&mut c)?;
        let hidden = read_bool(&mut c)?;
        c.read_exact(&mut one)?;
        let effect = one[0];
        c.read_exact(&mut one)?;
        let wing_effect = one[0];
        let extra = read_bool(&mut c)?;
        let mount_type = read_i16_le(&mut c)?;
        let riding_mount = read_bool(&mut c)?;
        let fishing = read_bool(&mut c)?;
        let transform_type = read_i16_le(&mut c)?;
        let element_orb_effect = read_u32_le(&mut c)?;
        let element_orb_lvl = read_u32_le(&mut c)?;
        let element_orb_max = read_u32_le(&mut c)?;

        let buff_count = read_i32_le(&mut c)?;
        if buff_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative buff count",
            ));
        }
        let mut buffs = vec![0u8; buff_count as usize];
        c.read_exact(&mut buffs)?;

        let level_effects = read_u16_le(&mut c)?;

        Ok(SObjectPlayer {
            object_id,
            name,
            guild_name,
            guild_rank_name,
            name_colour_argb,
            class,
            gender,
            level,
            location_x,
            location_y,
            direction,
            hair,
            light,
            weapon,
            weapon_effect,
            armour,
            poison,
            dead,
            hidden,
            effect,
            wing_effect,
            extra,
            mount_type,
            riding_mount,
            fishing,
            transform_type,
            element_orb_effect,
            element_orb_lvl,
            element_orb_max,
            buffs,
            level_effects,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectHero {
    pub base: SObjectPlayer,
    pub owner_name: String,
}

impl SObjectHero {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let b = &self.base;
        let mut buf = Vec::new();

        write_u32_le(&mut buf, b.object_id)?;
        write_string(&mut buf, &b.name)?;
        write_string(&mut buf, &b.guild_name)?;
        write_string(&mut buf, &b.guild_rank_name)?;
        write_i32_le(&mut buf, b.name_colour_argb)?;
        buf.push(b.class);
        buf.push(b.gender);
        write_u16_le(&mut buf, b.level)?;
        write_i32_le(&mut buf, b.location_x)?;
        write_i32_le(&mut buf, b.location_y)?;
        buf.push(b.direction);
        buf.push(b.hair);
        buf.push(b.light);
        write_i16_le(&mut buf, b.weapon)?;
        write_i16_le(&mut buf, b.weapon_effect)?;
        write_i16_le(&mut buf, b.armour)?;
        write_u16_le(&mut buf, b.poison)?;
        write_bool(&mut buf, b.dead)?;
        write_bool(&mut buf, b.hidden)?;
        buf.push(b.effect);
        buf.push(b.wing_effect);
        write_bool(&mut buf, b.extra)?;
        write_i16_le(&mut buf, b.mount_type)?;
        write_bool(&mut buf, b.riding_mount)?;
        write_bool(&mut buf, b.fishing)?;
        write_i16_le(&mut buf, b.transform_type)?;
        write_u32_le(&mut buf, b.element_orb_effect)?;
        write_u32_le(&mut buf, b.element_orb_lvl)?;
        write_u32_le(&mut buf, b.element_orb_max)?;

        let count: i32 = b
            .buffs
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many buffs"))?;
        write_i32_le(&mut buf, count)?;
        buf.extend_from_slice(&b.buffs);

        write_u16_le(&mut buf, b.level_effects)?;

        // Extra field for hero
        write_string(&mut buf, &self.owner_name)?;

        Ok(RawPacket {
            id: ServerPacketId::ObjectHero as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);

        let object_id = read_u32_le(&mut c)?;
        let name = read_string(&mut c)?;
        let guild_name = read_string(&mut c)?;
        let guild_rank_name = read_string(&mut c)?;
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
        c.read_exact(&mut one)?;
        let light = one[0];

        let weapon = read_i16_le(&mut c)?;
        let weapon_effect = read_i16_le(&mut c)?;
        let armour = read_i16_le(&mut c)?;
        let poison = read_u16_le(&mut c)?;
        let dead = read_bool(&mut c)?;
        let hidden = read_bool(&mut c)?;
        c.read_exact(&mut one)?;
        let effect = one[0];
        c.read_exact(&mut one)?;
        let wing_effect = one[0];
        let extra = read_bool(&mut c)?;
        let mount_type = read_i16_le(&mut c)?;
        let riding_mount = read_bool(&mut c)?;
        let fishing = read_bool(&mut c)?;
        let transform_type = read_i16_le(&mut c)?;
        let element_orb_effect = read_u32_le(&mut c)?;
        let element_orb_lvl = read_u32_le(&mut c)?;
        let element_orb_max = read_u32_le(&mut c)?;

        let buff_count = read_i32_le(&mut c)?;
        if buff_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative buff count",
            ));
        }
        let mut buffs = vec![0u8; buff_count as usize];
        c.read_exact(&mut buffs)?;

        let level_effects = read_u16_le(&mut c)?;

        let owner_name = read_string(&mut c)?;

        let base = SObjectPlayer {
            object_id,
            name,
            guild_name,
            guild_rank_name,
            name_colour_argb,
            class,
            gender,
            level,
            location_x,
            location_y,
            direction,
            hair,
            light,
            weapon,
            weapon_effect,
            armour,
            poison,
            dead,
            hidden,
            effect,
            wing_effect,
            extra,
            mount_type,
            riding_mount,
            fishing,
            transform_type,
            element_orb_effect,
            element_orb_lvl,
            element_orb_max,
            buffs,
            level_effects,
        };

        Ok(SObjectHero { base, owner_name })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectRemove {
    pub object_id: u32,
}

impl SObjectRemove {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectRemove as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        Ok(SObjectRemove { object_id })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectTurnWalkRun {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SObjectTurnWalkRun {
    fn encode_with_id(&self, id: ServerPacketId) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: id as i16,
            payload: buf,
        })
    }

    fn decode_from(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SObjectTurnWalkRun {
            object_id,
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectTurn(pub SObjectTurnWalkRun);

impl SObjectTurn {
    pub fn encode(&self) -> io::Result<RawPacket> {
        self.0.encode_with_id(ServerPacketId::ObjectTurn)
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SObjectTurn(SObjectTurnWalkRun::decode_from(payload)?))
    }
}

#[derive(Clone, Debug)]
pub struct SObjectWalk(pub SObjectTurnWalkRun);

impl SObjectWalk {
    pub fn encode(&self) -> io::Result<RawPacket> {
        self.0.encode_with_id(ServerPacketId::ObjectWalk)
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SObjectWalk(SObjectTurnWalkRun::decode_from(payload)?))
    }
}

#[derive(Clone, Debug)]
pub struct SObjectRun(pub SObjectTurnWalkRun);

impl SObjectRun {
    pub fn encode(&self) -> io::Result<RawPacket> {
        self.0.encode_with_id(ServerPacketId::ObjectRun)
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SObjectRun(SObjectTurnWalkRun::decode_from(payload)?))
    }
}

#[derive(Clone, Debug)]
pub struct SPushed {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SPushed {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::Pushed as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SPushed {
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectPushed(pub SObjectTurnWalkRun);

impl SObjectPushed {
    pub fn encode(&self) -> io::Result<RawPacket> {
        self.0.encode_with_id(ServerPacketId::ObjectPushed)
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SObjectPushed(SObjectTurnWalkRun::decode_from(payload)?))
    }
}

#[derive(Clone, Debug)]
pub struct SUserDash {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SUserDash {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::UserDash as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SUserDash {
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectDash(pub SObjectTurnWalkRun);

impl SObjectDash {
    pub fn encode(&self) -> io::Result<RawPacket> {
        self.0.encode_with_id(ServerPacketId::ObjectDash)
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SObjectDash(SObjectTurnWalkRun::decode_from(payload)?))
    }
}

#[derive(Clone, Debug)]
pub struct SUserDashFail {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SUserDashFail {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::UserDashFail as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SUserDashFail {
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectDashFail(pub SObjectTurnWalkRun);

impl SObjectDashFail {
    pub fn encode(&self) -> io::Result<RawPacket> {
        self.0.encode_with_id(ServerPacketId::ObjectDashFail)
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SObjectDashFail(SObjectTurnWalkRun::decode_from(payload)?))
    }
}

#[derive(Clone, Debug)]
pub struct SUserBackStep {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SUserBackStep {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::UserBackStep as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SUserBackStep {
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectBackStep {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
    pub distance: i32,
}

impl SObjectBackStep {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        write_i32_le(&mut buf, self.distance)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectBackStep as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        let distance = read_i32_le(&mut c)?;
        Ok(SObjectBackStep {
            object_id,
            location_x,
            location_y,
            direction,
            distance,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SUserDashAttack {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SUserDashAttack {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::UserDashAttack as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SUserDashAttack {
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectDashAttack {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
    pub distance: i32,
}

impl SObjectDashAttack {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        write_i32_le(&mut buf, self.distance)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectDashAttack as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        let distance = read_i32_le(&mut c)?;
        Ok(SObjectDashAttack {
            object_id,
            location_x,
            location_y,
            direction,
            distance,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SUserAttackMove {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SUserAttackMove {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::UserAttackMove as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SUserAttackMove {
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectName {
    pub object_id: u32,
    pub name: String,
}

impl SObjectName {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectName as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let name = read_string(&mut c)?;
        Ok(SObjectName { object_id, name })
    }
}

#[derive(Clone, Debug)]
pub struct SRevived;

impl SRevived {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::Revived as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SRevived payload must be empty",
            ));
        }
        Ok(SRevived)
    }
}

#[derive(Clone, Debug)]
pub struct SObjectRevived {
    pub object_id: u32,
    pub effect: bool,
}

impl SObjectRevived {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_bool(&mut buf, self.effect)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectRevived as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let effect = read_bool(&mut c)?;
        Ok(SObjectRevived { object_id, effect })
    }
}

#[derive(Clone, Debug)]
pub struct SSpellToggle {
    pub object_id: u32,
    pub spell: u8,
    pub can_use: bool,
}

impl SSpellToggle {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        buf.push(self.spell);
        write_bool(&mut buf, self.can_use)?;
        Ok(RawPacket {
            id: ServerPacketId::SpellToggle as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let spell = one[0];
        let can_use = read_bool(&mut c)?;
        Ok(SSpellToggle {
            object_id,
            spell,
            can_use,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectHealth {
    pub object_id: u32,
    pub percent: u8,
    pub expire: u8,
}

impl SObjectHealth {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        buf.push(self.percent);
        buf.push(self.expire);
        Ok(RawPacket {
            id: ServerPacketId::ObjectHealth as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let percent = one[0];
        c.read_exact(&mut one)?;
        let expire = one[0];
        Ok(SObjectHealth {
            object_id,
            percent,
            expire,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectMana {
    pub object_id: u32,
    pub percent: u8,
}

impl SObjectMana {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        buf.push(self.percent);
        Ok(RawPacket {
            id: ServerPacketId::ObjectMana as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let percent = one[0];
        Ok(SObjectMana { object_id, percent })
    }
}

#[derive(Clone, Debug)]
pub struct SAllowObserve {
    pub allow: bool,
}

impl SAllowObserve {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.allow)?;
        Ok(RawPacket {
            id: ServerPacketId::AllowObserve as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let allow = read_bool(&mut c)?;
        Ok(SAllowObserve { allow })
    }
}

#[derive(Clone, Debug)]
pub struct STimeOfDay {
    pub lights: u8,
}

impl STimeOfDay {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.lights);
        RawPacket {
            id: ServerPacketId::TimeOfDay as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "STimeOfDay payload must be exactly 1 byte",
            ));
        }
        Ok(STimeOfDay { lights: payload[0] })
    }
}

#[derive(Clone, Debug)]
pub struct SSendOutputMessage {
    pub message: String,
    pub msg_type: u8,
}

impl SSendOutputMessage {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.message)?;
        buf.push(self.msg_type);
        Ok(RawPacket {
            id: ServerPacketId::SendOutputMessage as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let message = read_string(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let msg_type = one[0];
        Ok(SSendOutputMessage { message, msg_type })
    }
}

#[derive(Clone, Debug)]
pub struct SRemoveDelayedExplosion {
    pub object_id: u32,
}

impl SRemoveDelayedExplosion {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        Ok(RawPacket {
            id: ServerPacketId::RemoveDelayedExplosion as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        Ok(SRemoveDelayedExplosion { object_id })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectSitDown {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
    pub sitting: bool,
}

impl SObjectSitDown {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        write_bool(&mut buf, self.sitting)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectSitDown as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        let sitting = read_bool(&mut c)?;
        Ok(SObjectSitDown {
            object_id,
            location_x,
            location_y,
            direction,
            sitting,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SInTrapRock {
    pub trapped: bool,
}

impl SInTrapRock {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.trapped)?;
        Ok(RawPacket {
            id: ServerPacketId::InTrapRock as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let trapped = read_bool(&mut c)?;
        Ok(SInTrapRock { trapped })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectDeco {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub image: i32,
}

impl SObjectDeco {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        write_i32_le(&mut buf, self.image)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectDeco as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let image = read_i32_le(&mut c)?;
        Ok(SObjectDeco {
            object_id,
            location_x,
            location_y,
            image,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectSneaking {
    pub object_id: u32,
    pub sneaking_active: bool,
}

impl SObjectSneaking {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_bool(&mut buf, self.sneaking_active)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectSneaking as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let sneaking_active = read_bool(&mut c)?;
        Ok(SObjectSneaking {
            object_id,
            sneaking_active,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectLevelEffects {
    pub object_id: u32,
    pub level_effects: u16,
}

impl SObjectLevelEffects {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_u16_le(&mut buf, self.level_effects)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectLevelEffects as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let level_effects = read_u16_le(&mut c)?;
        Ok(SObjectLevelEffects {
            object_id,
            level_effects,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SSetConcentration {
    pub object_id: u32,
    pub enabled: bool,
    pub interrupted: bool,
}

impl SSetConcentration {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_bool(&mut buf, self.enabled)?;
        write_bool(&mut buf, self.interrupted)?;
        Ok(RawPacket {
            id: ServerPacketId::SetConcentration as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let enabled = read_bool(&mut c)?;
        let interrupted = read_bool(&mut c)?;
        Ok(SSetConcentration {
            object_id,
            enabled,
            interrupted,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SSetElemental {
    pub object_id: u32,
    pub enabled: bool,
    pub casted: bool,
    pub value: u32,
    pub element_type: u32,
    pub exp_last: u32,
}

impl SSetElemental {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_bool(&mut buf, self.enabled)?;
        write_bool(&mut buf, self.casted)?;
        write_u32_le(&mut buf, self.value)?;
        write_u32_le(&mut buf, self.element_type)?;
        write_u32_le(&mut buf, self.exp_last)?;
        Ok(RawPacket {
            id: ServerPacketId::SetElemental as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let enabled = read_bool(&mut c)?;
        let casted = read_bool(&mut c)?;
        let value = read_u32_le(&mut c)?;
        let element_type = read_u32_le(&mut c)?;
        let exp_last = read_u32_le(&mut c)?;
        Ok(SSetElemental {
            object_id,
            enabled,
            casted,
            value,
            element_type,
            exp_last,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SMountUpdate {
    pub object_id: u32,
    pub mount_type: i16,
    pub riding_mount: bool,
}

impl SMountUpdate {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_i16_le(&mut buf, self.mount_type)?;
        write_bool(&mut buf, self.riding_mount)?;
        Ok(RawPacket {
            id: ServerPacketId::MountUpdate as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let mount_type = read_i16_le(&mut c)?;
        let riding_mount = read_bool(&mut c)?;
        Ok(SMountUpdate {
            object_id,
            mount_type,
            riding_mount,
        })
    }
}

#[derive(Clone, Debug)]
pub struct STransformUpdate {
    pub object_id: u32,
    pub transform_type: i16,
}

impl STransformUpdate {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_i16_le(&mut buf, self.transform_type)?;
        Ok(RawPacket {
            id: ServerPacketId::TransformUpdate as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let transform_type = read_i16_le(&mut c)?;
        Ok(STransformUpdate {
            object_id,
            transform_type,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SEquipSlotItem {
    pub grid: u8,
    pub unique_id: u64,
    pub to: i32,
    pub grid_to: u8,
    pub success: bool,
}

impl SEquipSlotItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.grid);
        write_u64_le(&mut buf, self.unique_id)?;
        write_i32_le(&mut buf, self.to)?;
        buf.push(self.grid_to);
        write_bool(&mut buf, self.success)?;
        Ok(RawPacket {
            id: ServerPacketId::EquipSlotItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let grid = one[0];
        let unique_id = read_u64_le(&mut c)?;
        let to = read_i32_le(&mut c)?;
        c.read_exact(&mut one)?;
        let grid_to = one[0];
        let success = read_bool(&mut c)?;
        Ok(SEquipSlotItem {
            grid,
            unique_id,
            to,
            grid_to,
            success,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SFishingUpdate {
    pub object_id: u32,
    pub fishing: bool,
    pub progress_percent: i32,
    pub chance_percent: i32,
    pub fishing_point_x: i32,
    pub fishing_point_y: i32,
    pub found_fish: bool,
}

impl SFishingUpdate {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_bool(&mut buf, self.fishing)?;
        write_i32_le(&mut buf, self.progress_percent)?;
        write_i32_le(&mut buf, self.chance_percent)?;
        write_i32_le(&mut buf, self.fishing_point_x)?;
        write_i32_le(&mut buf, self.fishing_point_y)?;
        write_bool(&mut buf, self.found_fish)?;
        Ok(RawPacket {
            id: ServerPacketId::FishingUpdate as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let fishing = read_bool(&mut c)?;
        let progress_percent = read_i32_le(&mut c)?;
        let chance_percent = read_i32_le(&mut c)?;
        let fishing_point_x = read_i32_le(&mut c)?;
        let fishing_point_y = read_i32_le(&mut c)?;
        let found_fish = read_bool(&mut c)?;
        Ok(SFishingUpdate {
            object_id,
            fishing,
            progress_percent,
            chance_percent,
            fishing_point_x,
            fishing_point_y,
            found_fish,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SSetBindingShot {
    pub object_id: u32,
    pub enabled: bool,
    pub value: i64,
}

impl SSetBindingShot {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_bool(&mut buf, self.enabled)?;
        write_i64_le(&mut buf, self.value)?;
        Ok(RawPacket {
            id: ServerPacketId::SetBindingShot as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let enabled = read_bool(&mut c)?;
        let value = read_i64_le(&mut c)?;
        Ok(SSetBindingShot {
            object_id,
            enabled,
            value,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SSwitchGroup {
    pub allow_group: bool,
}

impl SSwitchGroup {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.allow_group)?;
        Ok(RawPacket {
            id: ServerPacketId::SwitchGroup as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let allow_group = read_bool(&mut c)?;
        Ok(SSwitchGroup { allow_group })
    }
}

#[derive(Clone, Debug)]
pub struct SDeleteGroup;

impl SDeleteGroup {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::DeleteGroup as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SDeleteGroup payload must be empty",
            ));
        }
        Ok(SDeleteGroup)
    }
}

#[derive(Clone, Debug)]
pub struct SCancelReincarnation;

impl SCancelReincarnation {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::CancelReincarnation as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SCancelReincarnation payload must be empty",
            ));
        }
        Ok(SCancelReincarnation)
    }
}

#[derive(Clone, Debug)]
pub struct SRequestReincarnation;

impl SRequestReincarnation {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::RequestReincarnation as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SRequestReincarnation payload must be empty",
            ));
        }
        Ok(SRequestReincarnation)
    }
}

#[derive(Clone, Debug)]
pub struct SDeleteMember {
    pub name: String,
}

impl SDeleteMember {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ServerPacketId::DeleteMember as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        Ok(SDeleteMember { name })
    }
}

#[derive(Clone, Debug)]
pub struct SGroupInvite {
    pub name: String,
}

impl SGroupInvite {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ServerPacketId::GroupInvite as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        Ok(SGroupInvite { name })
    }
}

#[derive(Clone, Debug)]
pub struct SAddMember {
    pub name: String,
}

impl SAddMember {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ServerPacketId::AddMember as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        Ok(SAddMember { name })
    }
}

#[derive(Clone, Debug)]
pub struct SChat {
    pub message: String,
    pub chat_type: u8,
}

impl SChat {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.message)?;
        buf.push(self.chat_type);
        Ok(RawPacket {
            id: ServerPacketId::Chat as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let message = read_string(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let chat_type = one[0];
        Ok(SChat { message, chat_type })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectChat {
    pub object_id: u32,
    pub text: String,
    pub chat_type: u8,
}

impl SObjectChat {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_string(&mut buf, &self.text)?;
        buf.push(self.chat_type);
        Ok(RawPacket {
            id: ServerPacketId::ObjectChat as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let text = read_string(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let chat_type = one[0];
        Ok(SObjectChat {
            object_id,
            text,
            chat_type,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectMonster {
    pub object_id: u32,
    pub name: String,
    pub name_colour_argb: i32,
    pub location_x: i32,
    pub location_y: i32,
    pub image: u16,
    pub direction: u8,
    pub effect: u8,
    pub ai: u8,
    pub light: u8,
    pub dead: bool,
    pub skeleton: bool,
    pub poison: u16,
    pub hidden: bool,
    pub shock_time: i64,
    pub binding_shot_center: bool,
    pub extra: bool,
    pub extra_byte: u8,
    pub buffs: Vec<u8>,
}

impl SObjectMonster {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();

        write_u32_le(&mut buf, self.object_id)?;
        write_string(&mut buf, &self.name)?;
        write_i32_le(&mut buf, self.name_colour_argb)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        write_u16_le(&mut buf, self.image)?;
        buf.push(self.direction);
        buf.push(self.effect);
        buf.push(self.ai);
        buf.push(self.light);
        write_bool(&mut buf, self.dead)?;
        write_bool(&mut buf, self.skeleton)?;
        write_u16_le(&mut buf, self.poison)?;
        write_bool(&mut buf, self.hidden)?;
        write_i64_le(&mut buf, self.shock_time)?;
        write_bool(&mut buf, self.binding_shot_center)?;
        write_bool(&mut buf, self.extra)?;
        buf.push(self.extra_byte);

        let count: i32 = self
            .buffs
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many buffs"))?;
        write_i32_le(&mut buf, count)?;
        buf.extend_from_slice(&self.buffs);

        Ok(RawPacket {
            id: ServerPacketId::ObjectMonster as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);

        let object_id = read_u32_le(&mut c)?;
        let name = read_string(&mut c)?;
        let name_colour_argb = read_i32_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let image = read_u16_le(&mut c)?;

        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        c.read_exact(&mut one)?;
        let effect = one[0];
        c.read_exact(&mut one)?;
        let ai = one[0];
        c.read_exact(&mut one)?;
        let light = one[0];

        let dead = read_bool(&mut c)?;
        let skeleton = read_bool(&mut c)?;
        let poison = read_u16_le(&mut c)?;
        let hidden = read_bool(&mut c)?;
        let shock_time = read_i64_le(&mut c)?;
        let binding_shot_center = read_bool(&mut c)?;
        let extra = read_bool(&mut c)?;
        c.read_exact(&mut one)?;
        let extra_byte = one[0];

        let buff_count = read_i32_le(&mut c)?;
        if buff_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative buff count",
            ));
        }
        let mut buffs = vec![0u8; buff_count as usize];
        c.read_exact(&mut buffs)?;

        Ok(SObjectMonster {
            object_id,
            name,
            name_colour_argb,
            location_x,
            location_y,
            image,
            direction,
            effect,
            ai,
            light,
            dead,
            skeleton,
            poison,
            hidden,
            shock_time,
            binding_shot_center,
            extra,
            extra_byte,
            buffs,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectAttack {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
    pub spell: u8,
    pub level: u8,
    pub attack_type: u8,
}

impl SObjectAttack {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        buf.push(self.spell);
        buf.push(self.level);
        buf.push(self.attack_type);
        Ok(RawPacket {
            id: ServerPacketId::ObjectAttack as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        c.read_exact(&mut one)?;
        let spell = one[0];
        c.read_exact(&mut one)?;
        let level = one[0];
        c.read_exact(&mut one)?;
        let attack_type = one[0];
        Ok(SObjectAttack {
            object_id,
            location_x,
            location_y,
            direction,
            spell,
            level,
            attack_type,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SStruck {
    pub attacker_id: u32,
}

impl SStruck {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.attacker_id)?;
        Ok(RawPacket {
            id: ServerPacketId::Struck as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let attacker_id = read_u32_le(&mut c)?;
        Ok(SStruck { attacker_id })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectStruck {
    pub object_id: u32,
    pub attacker_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SObjectStruck {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_u32_le(&mut buf, self.attacker_id)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::ObjectStruck as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let attacker_id = read_u32_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SObjectStruck {
            object_id,
            attacker_id,
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SDamageIndicator {
    pub damage: i32,
    pub damage_type: u8,
    pub object_id: u32,
}

impl SDamageIndicator {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.damage)?;
        buf.push(self.damage_type);
        write_u32_le(&mut buf, self.object_id)?;
        Ok(RawPacket {
            id: ServerPacketId::DamageIndicator as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let damage = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let damage_type = one[0];
        let object_id = read_u32_le(&mut c)?;
        Ok(SDamageIndicator {
            damage,
            damage_type,
            object_id,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SHealthChanged {
    pub hp: i32,
    pub mp: i32,
}

impl SHealthChanged {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.hp)?;
        write_i32_le(&mut buf, self.mp)?;
        Ok(RawPacket {
            id: ServerPacketId::HealthChanged as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let hp = read_i32_le(&mut c)?;
        let mp = read_i32_le(&mut c)?;
        Ok(SHealthChanged { hp, mp })
    }
}

#[derive(Clone, Debug)]
pub struct SHeroHealthChanged {
    pub hp: i32,
    pub mp: i32,
}

impl SHeroHealthChanged {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.hp)?;
        write_i32_le(&mut buf, self.mp)?;
        Ok(RawPacket {
            id: ServerPacketId::HeroHealthChanged as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let hp = read_i32_le(&mut c)?;
        let mp = read_i32_le(&mut c)?;
        Ok(SHeroHealthChanged { hp, mp })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectItem {
    pub object_id: u32,
    pub name: String,
    /// ARGB colour as i32, matches System.Drawing.Color.ToArgb
    pub name_colour_argb: i32,
    pub location_x: i32,
    pub location_y: i32,
    pub image: u16,
    /// ItemGrade stored as underlying byte
    pub grade: u8,
}

impl SObjectItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_string(&mut buf, &self.name)?;
        write_i32_le(&mut buf, self.name_colour_argb)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        write_u16_le(&mut buf, self.image)?;
        buf.push(self.grade);
        Ok(RawPacket {
            id: ServerPacketId::ObjectItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let name = read_string(&mut c)?;
        let name_colour_argb = read_i32_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let image = read_u16_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let grade = one[0];
        Ok(SObjectItem {
            object_id,
            name,
            name_colour_argb,
            location_x,
            location_y,
            image,
            grade,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectGold {
    pub object_id: u32,
    pub gold: u32,
    pub location_x: i32,
    pub location_y: i32,
}

impl SObjectGold {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_u32_le(&mut buf, self.gold)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectGold as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let gold = read_u32_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        Ok(SObjectGold {
            object_id,
            gold,
            location_x,
            location_y,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SGainedItem {
    /// Raw bytes representing UserItem.Save(writer) payload.
    pub item_bytes: Vec<u8>,
}

impl SGainedItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        Ok(RawPacket {
            id: ServerPacketId::GainedItem as i16,
            payload: self.item_bytes.clone(),
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SGainedItem {
            item_bytes: payload.to_vec(),
        })
    }
}

impl SGainedItem {
    pub fn decode_user_item(&self) -> io::Result<UserItemData> {
        UserItemData::decode_from_bytes(&self.item_bytes)
    }

    pub fn from_user_item(item: &UserItemData) -> io::Result<Self> {
        let bytes = item.encode_to_bytes()?;
        Ok(SGainedItem { item_bytes: bytes })
    }
}

#[derive(Clone, Debug)]
pub struct SGainedGold {
    pub gold: u32,
}

impl SGainedGold {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.gold)?;
        Ok(RawPacket {
            id: ServerPacketId::GainedGold as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let gold = read_u32_le(&mut c)?;
        Ok(SGainedGold { gold })
    }
}

#[derive(Clone, Debug)]
pub struct SLoseGold {
    pub gold: u32,
}

impl SLoseGold {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.gold)?;
        Ok(RawPacket {
            id: ServerPacketId::LoseGold as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let gold = read_u32_le(&mut c)?;
        Ok(SLoseGold { gold })
    }
}

#[derive(Clone, Debug)]
pub struct SGainedCredit {
    pub credit: u32,
}

impl SGainedCredit {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.credit)?;
        Ok(RawPacket {
            id: ServerPacketId::GainedCredit as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let credit = read_u32_le(&mut c)?;
        Ok(SGainedCredit { credit })
    }
}

#[derive(Clone, Debug)]
pub struct SLoseCredit {
    pub credit: u32,
}

impl SLoseCredit {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.credit)?;
        Ok(RawPacket {
            id: ServerPacketId::LoseCredit as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let credit = read_u32_le(&mut c)?;
        Ok(SLoseCredit { credit })
    }
}

#[derive(Clone, Debug)]
pub struct SDeath {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SDeath {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::Death as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SDeath {
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectDied {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
    pub death_type: u8,
}

impl SObjectDied {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        buf.push(self.death_type);
        Ok(RawPacket {
            id: ServerPacketId::ObjectDied as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        c.read_exact(&mut one)?;
        let death_type = one[0];
        Ok(SObjectDied {
            object_id,
            location_x,
            location_y,
            direction,
            death_type,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SColourChanged {
    pub name_colour_argb: i32,
}

impl SColourChanged {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.name_colour_argb)?;
        Ok(RawPacket {
            id: ServerPacketId::ColourChanged as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name_colour_argb = read_i32_le(&mut c)?;
        Ok(SColourChanged { name_colour_argb })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectColourChanged {
    pub object_id: u32,
    pub name_colour_argb: i32,
}

impl SObjectColourChanged {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_i32_le(&mut buf, self.name_colour_argb)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectColourChanged as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let name_colour_argb = read_i32_le(&mut c)?;
        Ok(SObjectColourChanged {
            object_id,
            name_colour_argb,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectGuildNameChanged {
    pub object_id: u32,
    pub guild_name: String,
}

impl SObjectGuildNameChanged {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_string(&mut buf, &self.guild_name)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectGuildNameChanged as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let guild_name = read_string(&mut c)?;
        Ok(SObjectGuildNameChanged {
            object_id,
            guild_name,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SGainExperience {
    pub amount: u32,
}

impl SGainExperience {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.amount)?;
        Ok(RawPacket {
            id: ServerPacketId::GainExperience as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let amount = read_u32_le(&mut c)?;
        Ok(SGainExperience { amount })
    }
}

#[derive(Clone, Debug)]
pub struct SGainHeroExperience {
    pub amount: u32,
}

impl SGainHeroExperience {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.amount)?;
        Ok(RawPacket {
            id: ServerPacketId::GainHeroExperience as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let amount = read_u32_le(&mut c)?;
        Ok(SGainHeroExperience { amount })
    }
}

#[derive(Clone, Debug)]
pub struct SLevelChanged {
    pub level: u16,
    pub experience: i64,
    pub max_experience: i64,
}

impl SLevelChanged {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u16_le(&mut buf, self.level)?;
        write_i64_le(&mut buf, self.experience)?;
        write_i64_le(&mut buf, self.max_experience)?;
        Ok(RawPacket {
            id: ServerPacketId::LevelChanged as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let level = read_u16_le(&mut c)?;
        let experience = read_i64_le(&mut c)?;
        let max_experience = read_i64_le(&mut c)?;
        Ok(SLevelChanged {
            level,
            experience,
            max_experience,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SHeroLevelChanged {
    pub level: u16,
    pub experience: i64,
    pub max_experience: i64,
}

impl SHeroLevelChanged {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u16_le(&mut buf, self.level)?;
        write_i64_le(&mut buf, self.experience)?;
        write_i64_le(&mut buf, self.max_experience)?;
        Ok(RawPacket {
            id: ServerPacketId::HeroLevelChanged as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let level = read_u16_le(&mut c)?;
        let experience = read_i64_le(&mut c)?;
        let max_experience = read_i64_le(&mut c)?;
        Ok(SHeroLevelChanged {
            level,
            experience,
            max_experience,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectLeveled {
    pub object_id: u32,
}

impl SObjectLeveled {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectLeveled as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        Ok(SObjectLeveled { object_id })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectHide {
    pub object_id: u32,
}

impl SObjectHide {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectHide as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        Ok(SObjectHide { object_id })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectShow {
    pub object_id: u32,
}

impl SObjectShow {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectShow as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        Ok(SObjectShow { object_id })
    }
}

#[derive(Clone, Debug)]
pub struct SPoisoned {
    pub poison: u16,
}

impl SPoisoned {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u16_le(&mut buf, self.poison)?;
        Ok(RawPacket {
            id: ServerPacketId::Poisoned as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let poison = read_u16_le(&mut c)?;
        Ok(SPoisoned { poison })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectPoisoned {
    pub object_id: u32,
    pub poison: u16,
}

impl SObjectPoisoned {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_u16_le(&mut buf, self.poison)?;
        Ok(RawPacket {
            id: ServerPacketId::ObjectPoisoned as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let poison = read_u16_le(&mut c)?;
        Ok(SObjectPoisoned { object_id, poison })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectTeleportOut {
    pub object_id: u32,
    pub teleport_type: u8,
}

impl SObjectTeleportOut {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        buf.push(self.teleport_type);
        Ok(RawPacket {
            id: ServerPacketId::ObjectTeleportOut as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let teleport_type = one[0];
        Ok(SObjectTeleportOut {
            object_id,
            teleport_type,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SObjectTeleportIn {
    pub object_id: u32,
    pub teleport_type: u8,
}

impl SObjectTeleportIn {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        buf.push(self.teleport_type);
        Ok(RawPacket {
            id: ServerPacketId::ObjectTeleportIn as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let teleport_type = one[0];
        Ok(SObjectTeleportIn {
            object_id,
            teleport_type,
        })
    }
}

#[derive(Clone, Debug)]
pub struct STeleportIn;

impl STeleportIn {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::TeleportIn as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "STeleportIn payload must be empty",
            ));
        }
        Ok(STeleportIn)
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

    #[test]
    fn user_location_roundtrip() {
        let loc = SUserLocation {
            location_x: 123,
            location_y: 456,
            direction: 2,
        };

        let raw = loc.encode().expect("encode SUserLocation");
        assert_eq!(raw.id, ServerPacketId::UserLocation as i16);

        let decoded = SUserLocation::decode(&raw.payload).expect("decode SUserLocation");
        assert_eq!(decoded.location_x, loc.location_x);
        assert_eq!(decoded.location_y, loc.location_y);
        assert_eq!(decoded.direction, loc.direction);
    }

    #[test]
    fn object_player_roundtrip() {
        let op = SObjectPlayer {
            object_id: 1,
            name: "Other".to_string(),
            guild_name: "G".to_string(),
            guild_rank_name: "R".to_string(),
            name_colour_argb: -1,
            class: 1,
            gender: 0,
            level: 10,
            location_x: 50,
            location_y: 60,
            direction: 3,
            hair: 2,
            light: 5,
            weapon: 10,
            weapon_effect: 1,
            armour: 20,
            poison: 0,
            dead: false,
            hidden: false,
            effect: 0,
            wing_effect: 0,
            extra: false,
            mount_type: -1,
            riding_mount: false,
            fishing: false,
            transform_type: 0,
            element_orb_effect: 0,
            element_orb_lvl: 0,
            element_orb_max: 0,
            buffs: vec![1, 2, 3],
            level_effects: 0,
        };

        let raw = op.encode().expect("encode SObjectPlayer");
        assert_eq!(raw.id, ServerPacketId::ObjectPlayer as i16);

        let decoded = SObjectPlayer::decode(&raw.payload).expect("decode SObjectPlayer");
        assert_eq!(decoded.object_id, op.object_id);
        assert_eq!(decoded.name, op.name);
        assert_eq!(decoded.guild_name, op.guild_name);
        assert_eq!(decoded.guild_rank_name, op.guild_rank_name);
        assert_eq!(decoded.name_colour_argb, op.name_colour_argb);
        assert_eq!(decoded.class, op.class);
        assert_eq!(decoded.gender, op.gender);
        assert_eq!(decoded.level, op.level);
        assert_eq!(decoded.location_x, op.location_x);
        assert_eq!(decoded.location_y, op.location_y);
        assert_eq!(decoded.direction, op.direction);
        assert_eq!(decoded.hair, op.hair);
        assert_eq!(decoded.light, op.light);
        assert_eq!(decoded.weapon, op.weapon);
        assert_eq!(decoded.weapon_effect, op.weapon_effect);
        assert_eq!(decoded.armour, op.armour);
        assert_eq!(decoded.poison, op.poison);
        assert_eq!(decoded.dead, op.dead);
        assert_eq!(decoded.hidden, op.hidden);
        assert_eq!(decoded.effect, op.effect);
        assert_eq!(decoded.wing_effect, op.wing_effect);
        assert_eq!(decoded.extra, op.extra);
        assert_eq!(decoded.mount_type, op.mount_type);
        assert_eq!(decoded.riding_mount, op.riding_mount);
        assert_eq!(decoded.fishing, op.fishing);
        assert_eq!(decoded.transform_type, op.transform_type);
        assert_eq!(decoded.element_orb_effect, op.element_orb_effect);
        assert_eq!(decoded.element_orb_lvl, op.element_orb_lvl);
        assert_eq!(decoded.element_orb_max, op.element_orb_max);
        assert_eq!(decoded.buffs, op.buffs);
        assert_eq!(decoded.level_effects, op.level_effects);
    }

    #[test]
    fn object_hero_roundtrip() {
        let base = SObjectPlayer {
            object_id: 2,
            name: "Hero".to_string(),
            guild_name: "G2".to_string(),
            guild_rank_name: "R2".to_string(),
            name_colour_argb: -1,
            class: 2,
            gender: 1,
            level: 20,
            location_x: 10,
            location_y: 20,
            direction: 1,
            hair: 1,
            light: 3,
            weapon: 5,
            weapon_effect: 1,
            armour: 6,
            poison: 0,
            dead: false,
            hidden: false,
            effect: 0,
            wing_effect: 0,
            extra: false,
            mount_type: -1,
            riding_mount: false,
            fishing: false,
            transform_type: 0,
            element_orb_effect: 0,
            element_orb_lvl: 0,
            element_orb_max: 0,
            buffs: vec![],
            level_effects: 0,
        };

        let hero = SObjectHero {
            base,
            owner_name: "Owner".to_string(),
        };

        let raw = hero.encode().expect("encode SObjectHero");
        assert_eq!(raw.id, ServerPacketId::ObjectHero as i16);

        let decoded = SObjectHero::decode(&raw.payload).expect("decode SObjectHero");
        assert_eq!(decoded.base.name, "Hero");
        assert_eq!(decoded.owner_name, "Owner");
    }

    #[test]
    fn object_remove_roundtrip() {
        let r = SObjectRemove { object_id: 42 };
        let raw = r.encode().expect("encode SObjectRemove");
        assert_eq!(raw.id, ServerPacketId::ObjectRemove as i16);
        let decoded = SObjectRemove::decode(&raw.payload).expect("decode SObjectRemove");
        assert_eq!(decoded.object_id, r.object_id);
    }

    #[test]
    fn object_turn_walk_run_roundtrip() {
        let base = SObjectTurnWalkRun {
            object_id: 7,
            location_x: 11,
            location_y: 22,
            direction: 3,
        };

        let t = SObjectTurn(base.clone());
        let raw = t.encode().expect("encode SObjectTurn");
        assert_eq!(raw.id, ServerPacketId::ObjectTurn as i16);
        let decoded = SObjectTurn::decode(&raw.payload).expect("decode SObjectTurn");
        assert_eq!(decoded.0.object_id, base.object_id);

        let w = SObjectWalk(base.clone());
        let raw = w.encode().expect("encode SObjectWalk");
        assert_eq!(raw.id, ServerPacketId::ObjectWalk as i16);
        let decoded = SObjectWalk::decode(&raw.payload).expect("decode SObjectWalk");
        assert_eq!(decoded.0.location_y, base.location_y);

        let r = SObjectRun(base);
        let raw = r.encode().expect("encode SObjectRun");
        assert_eq!(raw.id, ServerPacketId::ObjectRun as i16);
        let decoded = SObjectRun::decode(&raw.payload).expect("decode SObjectRun");
        assert_eq!(decoded.0.direction, 3);
    }

    #[test]
    fn chat_roundtrip() {
        let c = SChat {
            message: "hello".to_string(),
            chat_type: 1,
        };
        let raw = c.encode().expect("encode SChat");
        assert_eq!(raw.id, ServerPacketId::Chat as i16);
        let decoded = SChat::decode(&raw.payload).expect("decode SChat");
        assert_eq!(decoded.message, c.message);
        assert_eq!(decoded.chat_type, c.chat_type);
    }

    #[test]
    fn object_chat_roundtrip() {
        let c = SObjectChat {
            object_id: 9,
            text: "hi".to_string(),
            chat_type: 2,
        };
        let raw = c.encode().expect("encode SObjectChat");
        assert_eq!(raw.id, ServerPacketId::ObjectChat as i16);
        let decoded = SObjectChat::decode(&raw.payload).expect("decode SObjectChat");
        assert_eq!(decoded.object_id, c.object_id);
    }

    #[test]
    fn object_item_roundtrip() {
        let p = SObjectItem {
            object_id: 100,
            name: "Sword".to_string(),
            name_colour_argb: -1,
            location_x: 10,
            location_y: 20,
            image: 5,
            grade: 2,
        };

        let raw = p.encode().expect("encode SObjectItem");
        assert_eq!(raw.id, ServerPacketId::ObjectItem as i16);

        let decoded = SObjectItem::decode(&raw.payload).expect("decode SObjectItem");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.name, p.name);
        assert_eq!(decoded.name_colour_argb, p.name_colour_argb);
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.image, p.image);
        assert_eq!(decoded.grade, p.grade);
    }

    #[test]
    fn object_gold_roundtrip() {
        let p = SObjectGold {
            object_id: 200,
            gold: 1234,
            location_x: 30,
            location_y: 40,
        };

        let raw = p.encode().expect("encode SObjectGold");
        assert_eq!(raw.id, ServerPacketId::ObjectGold as i16);

        let decoded = SObjectGold::decode(&raw.payload).expect("decode SObjectGold");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.gold, p.gold);
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
    }

    #[test]
    fn gained_item_roundtrip() {
        let p = SGainedItem {
            item_bytes: vec![1, 2, 3, 4],
        };

        let raw = p.encode().expect("encode SGainedItem");
        assert_eq!(raw.id, ServerPacketId::GainedItem as i16);
        assert_eq!(raw.payload, p.item_bytes);

        let decoded = SGainedItem::decode(&raw.payload).expect("decode SGainedItem");
        assert_eq!(decoded.item_bytes, p.item_bytes);
    }

    #[test]
    fn gained_gold_roundtrip() {
        let p = SGainedGold { gold: 999 };

        let raw = p.encode().expect("encode SGainedGold");
        assert_eq!(raw.id, ServerPacketId::GainedGold as i16);

        let decoded = SGainedGold::decode(&raw.payload).expect("decode SGainedGold");
        assert_eq!(decoded.gold, p.gold);
    }

    #[test]
    fn lose_gold_roundtrip() {
        let p = SLoseGold { gold: 500 };

        let raw = p.encode().expect("encode SLoseGold");
        assert_eq!(raw.id, ServerPacketId::LoseGold as i16);

        let decoded = SLoseGold::decode(&raw.payload).expect("decode SLoseGold");
        assert_eq!(decoded.gold, p.gold);
    }

    #[test]
    fn gained_credit_roundtrip() {
        let p = SGainedCredit { credit: 42 };

        let raw = p.encode().expect("encode SGainedCredit");
        assert_eq!(raw.id, ServerPacketId::GainedCredit as i16);

        let decoded = SGainedCredit::decode(&raw.payload).expect("decode SGainedCredit");
        assert_eq!(decoded.credit, p.credit);
    }

    #[test]
    fn lose_credit_roundtrip() {
        let p = SLoseCredit { credit: 7 };

        let raw = p.encode().expect("encode SLoseCredit");
        assert_eq!(raw.id, ServerPacketId::LoseCredit as i16);

        let decoded = SLoseCredit::decode(&raw.payload).expect("decode SLoseCredit");
        assert_eq!(decoded.credit, p.credit);
    }

    #[test]
    fn death_roundtrip() {
        let p = SDeath {
            location_x: 10,
            location_y: 20,
            direction: 3,
        };

        let raw = p.encode().expect("encode SDeath");
        assert_eq!(raw.id, ServerPacketId::Death as i16);

        let decoded = SDeath::decode(&raw.payload).expect("decode SDeath");
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.direction, p.direction);
    }

    #[test]
    fn object_died_roundtrip() {
        let p = SObjectDied {
            object_id: 99,
            location_x: 1,
            location_y: 2,
            direction: 3,
            death_type: 4,
        };

        let raw = p.encode().expect("encode SObjectDied");
        assert_eq!(raw.id, ServerPacketId::ObjectDied as i16);

        let decoded = SObjectDied::decode(&raw.payload).expect("decode SObjectDied");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.direction, p.direction);
        assert_eq!(decoded.death_type, p.death_type);
    }

    #[test]
    fn gain_experience_roundtrip() {
        let p = SGainExperience { amount: 12345 };

        let raw = p.encode().expect("encode SGainExperience");
        assert_eq!(raw.id, ServerPacketId::GainExperience as i16);

        let decoded = SGainExperience::decode(&raw.payload).expect("decode SGainExperience");
        assert_eq!(decoded.amount, p.amount);
    }

    #[test]
    fn gain_hero_experience_roundtrip() {
        let p = SGainHeroExperience { amount: 6789 };

        let raw = p.encode().expect("encode SGainHeroExperience");
        assert_eq!(raw.id, ServerPacketId::GainHeroExperience as i16);

        let decoded = SGainHeroExperience::decode(&raw.payload)
            .expect("decode SGainHeroExperience");
        assert_eq!(decoded.amount, p.amount);
    }

    #[test]
    fn level_changed_roundtrip() {
        let p = SLevelChanged {
            level: 50,
            experience: 123_456,
            max_experience: 789_000,
        };

        let raw = p.encode().expect("encode SLevelChanged");
        assert_eq!(raw.id, ServerPacketId::LevelChanged as i16);

        let decoded = SLevelChanged::decode(&raw.payload).expect("decode SLevelChanged");
        assert_eq!(decoded.level, p.level);
        assert_eq!(decoded.experience, p.experience);
        assert_eq!(decoded.max_experience, p.max_experience);
    }

    #[test]
    fn hero_level_changed_roundtrip() {
        let p = SHeroLevelChanged {
            level: 30,
            experience: 111_222,
            max_experience: 333_444,
        };

        let raw = p.encode().expect("encode SHeroLevelChanged");
        assert_eq!(raw.id, ServerPacketId::HeroLevelChanged as i16);

        let decoded = SHeroLevelChanged::decode(&raw.payload)
            .expect("decode SHeroLevelChanged");
        assert_eq!(decoded.level, p.level);
        assert_eq!(decoded.experience, p.experience);
        assert_eq!(decoded.max_experience, p.max_experience);
    }

    #[test]
    fn object_leveled_roundtrip() {
        let p = SObjectLeveled { object_id: 555 };

        let raw = p.encode().expect("encode SObjectLeveled");
        assert_eq!(raw.id, ServerPacketId::ObjectLeveled as i16);

        let decoded = SObjectLeveled::decode(&raw.payload).expect("decode SObjectLeveled");
        assert_eq!(decoded.object_id, p.object_id);
    }

    #[test]
    fn object_teleport_out_roundtrip() {
        let p = SObjectTeleportOut {
            object_id: 42,
            teleport_type: 3,
        };

        let raw = p.encode().expect("encode SObjectTeleportOut");
        assert_eq!(raw.id, ServerPacketId::ObjectTeleportOut as i16);

        let decoded = SObjectTeleportOut::decode(&raw.payload).expect("decode SObjectTeleportOut");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.teleport_type, p.teleport_type);
    }

    #[test]
    fn object_teleport_in_roundtrip() {
        let p = SObjectTeleportIn {
            object_id: 99,
            teleport_type: 1,
        };

        let raw = p.encode().expect("encode SObjectTeleportIn");
        assert_eq!(raw.id, ServerPacketId::ObjectTeleportIn as i16);

        let decoded = SObjectTeleportIn::decode(&raw.payload).expect("decode SObjectTeleportIn");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.teleport_type, p.teleport_type);
    }

    #[test]
    fn teleport_in_roundtrip() {
        let p = STeleportIn;

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::TeleportIn as i16);

        let decoded = STeleportIn::decode(&raw.payload).expect("decode STeleportIn");
        let _ = decoded;
    }

    #[test]
    fn pushed_roundtrip() {
        let p = SPushed {
            location_x: 10,
            location_y: 20,
            direction: 3,
        };

        let raw = p.encode().expect("encode SPushed");
        assert_eq!(raw.id, ServerPacketId::Pushed as i16);

        let decoded = SPushed::decode(&raw.payload).expect("decode SPushed");
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.direction, p.direction);
    }

    #[test]
    fn object_pushed_roundtrip() {
        let base = SObjectTurnWalkRun {
            object_id: 1,
            location_x: 11,
            location_y: 22,
            direction: 4,
        };
        let p = SObjectPushed(base);

        let raw = p.encode().expect("encode SObjectPushed");
        assert_eq!(raw.id, ServerPacketId::ObjectPushed as i16);

        let decoded = SObjectPushed::decode(&raw.payload).expect("decode SObjectPushed");
        assert_eq!(decoded.0.object_id, 1);
        assert_eq!(decoded.0.location_x, 11);
        assert_eq!(decoded.0.location_y, 22);
        assert_eq!(decoded.0.direction, 4);
    }

    #[test]
    fn user_dash_roundtrip() {
        let p = SUserDash {
            location_x: 5,
            location_y: 6,
            direction: 2,
        };

        let raw = p.encode().expect("encode SUserDash");
        assert_eq!(raw.id, ServerPacketId::UserDash as i16);

        let decoded = SUserDash::decode(&raw.payload).expect("decode SUserDash");
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.direction, p.direction);
    }

    #[test]
    fn object_dash_roundtrip() {
        let base = SObjectTurnWalkRun {
            object_id: 7,
            location_x: 8,
            location_y: 9,
            direction: 1,
        };
        let p = SObjectDash(base);

        let raw = p.encode().expect("encode SObjectDash");
        assert_eq!(raw.id, ServerPacketId::ObjectDash as i16);

        let decoded = SObjectDash::decode(&raw.payload).expect("decode SObjectDash");
        assert_eq!(decoded.0.object_id, 7);
        assert_eq!(decoded.0.location_x, 8);
        assert_eq!(decoded.0.location_y, 9);
        assert_eq!(decoded.0.direction, 1);
    }

    #[test]
    fn user_dash_fail_roundtrip() {
        let p = SUserDashFail {
            location_x: -1,
            location_y: -2,
            direction: 5,
        };

        let raw = p.encode().expect("encode SUserDashFail");
        assert_eq!(raw.id, ServerPacketId::UserDashFail as i16);

        let decoded = SUserDashFail::decode(&raw.payload).expect("decode SUserDashFail");
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.direction, p.direction);
    }

    #[test]
    fn object_dash_fail_roundtrip() {
        let base = SObjectTurnWalkRun {
            object_id: 3,
            location_x: 4,
            location_y: 5,
            direction: 6,
        };
        let p = SObjectDashFail(base);

        let raw = p.encode().expect("encode SObjectDashFail");
        assert_eq!(raw.id, ServerPacketId::ObjectDashFail as i16);

        let decoded = SObjectDashFail::decode(&raw.payload).expect("decode SObjectDashFail");
        assert_eq!(decoded.0.object_id, 3);
        assert_eq!(decoded.0.location_x, 4);
        assert_eq!(decoded.0.location_y, 5);
        assert_eq!(decoded.0.direction, 6);
    }

    #[test]
    fn user_back_step_roundtrip() {
        let p = SUserBackStep {
            location_x: 100,
            location_y: 200,
            direction: 1,
        };

        let raw = p.encode().expect("encode SUserBackStep");
        assert_eq!(raw.id, ServerPacketId::UserBackStep as i16);

        let decoded = SUserBackStep::decode(&raw.payload).expect("decode SUserBackStep");
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.direction, p.direction);
    }

    #[test]
    fn object_back_step_roundtrip() {
        let p = SObjectBackStep {
            object_id: 9,
            location_x: 10,
            location_y: 11,
            direction: 2,
            distance: 3,
        };

        let raw = p.encode().expect("encode SObjectBackStep");
        assert_eq!(raw.id, ServerPacketId::ObjectBackStep as i16);

        let decoded = SObjectBackStep::decode(&raw.payload).expect("decode SObjectBackStep");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.direction, p.direction);
        assert_eq!(decoded.distance, p.distance);
    }

    #[test]
    fn user_dash_attack_roundtrip() {
        let p = SUserDashAttack {
            location_x: 1,
            location_y: 2,
            direction: 3,
        };

        let raw = p.encode().expect("encode SUserDashAttack");
        assert_eq!(raw.id, ServerPacketId::UserDashAttack as i16);

        let decoded = SUserDashAttack::decode(&raw.payload).expect("decode SUserDashAttack");
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.direction, p.direction);
    }

    #[test]
    fn object_dash_attack_roundtrip() {
        let p = SObjectDashAttack {
            object_id: 7,
            location_x: 8,
            location_y: 9,
            direction: 4,
            distance: 5,
        };

        let raw = p.encode().expect("encode SObjectDashAttack");
        assert_eq!(raw.id, ServerPacketId::ObjectDashAttack as i16);

        let decoded = SObjectDashAttack::decode(&raw.payload)
            .expect("decode SObjectDashAttack");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.direction, p.direction);
        assert_eq!(decoded.distance, p.distance);
    }

    #[test]
    fn user_attack_move_roundtrip() {
        let p = SUserAttackMove {
            location_x: -10,
            location_y: -20,
            direction: 7,
        };

        let raw = p.encode().expect("encode SUserAttackMove");
        assert_eq!(raw.id, ServerPacketId::UserAttackMove as i16);

        let decoded = SUserAttackMove::decode(&raw.payload).expect("decode SUserAttackMove");
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.direction, p.direction);
    }

    #[test]
    fn object_name_roundtrip() {
        let p = SObjectName {
            object_id: 42,
            name: "Mob".to_string(),
        };

        let raw = p.encode().expect("encode SObjectName");
        assert_eq!(raw.id, ServerPacketId::ObjectName as i16);

        let decoded = SObjectName::decode(&raw.payload).expect("decode SObjectName");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.name, p.name);
    }

    #[test]
    fn revived_roundtrip() {
        let p = SRevived;

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::Revived as i16);

        let decoded = SRevived::decode(&raw.payload).expect("decode SRevived");
        let _ = decoded;
    }

    #[test]
    fn object_revived_roundtrip() {
        let p = SObjectRevived {
            object_id: 7,
            effect: true,
        };

        let raw = p.encode().expect("encode SObjectRevived");
        assert_eq!(raw.id, ServerPacketId::ObjectRevived as i16);

        let decoded = SObjectRevived::decode(&raw.payload).expect("decode SObjectRevived");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.effect, p.effect);
    }

    #[test]
    fn spell_toggle_roundtrip() {
        let p = SSpellToggle {
            object_id: 1,
            spell: 3,
            can_use: true,
        };

        let raw = p.encode().expect("encode SSpellToggle");
        assert_eq!(raw.id, ServerPacketId::SpellToggle as i16);

        let decoded = SSpellToggle::decode(&raw.payload).expect("decode SSpellToggle");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.spell, p.spell);
        assert_eq!(decoded.can_use, p.can_use);
    }

    #[test]
    fn object_health_roundtrip() {
        let p = SObjectHealth {
            object_id: 9,
            percent: 80,
            expire: 5,
        };

        let raw = p.encode().expect("encode SObjectHealth");
        assert_eq!(raw.id, ServerPacketId::ObjectHealth as i16);

        let decoded = SObjectHealth::decode(&raw.payload).expect("decode SObjectHealth");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.percent, p.percent);
        assert_eq!(decoded.expire, p.expire);
    }

    #[test]
    fn object_mana_roundtrip() {
        let p = SObjectMana {
            object_id: 10,
            percent: 60,
        };

        let raw = p.encode().expect("encode SObjectMana");
        assert_eq!(raw.id, ServerPacketId::ObjectMana as i16);

        let decoded = SObjectMana::decode(&raw.payload).expect("decode SObjectMana");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.percent, p.percent);
    }

    #[test]
    fn allow_observe_roundtrip() {
        let p = SAllowObserve { allow: true };

        let raw = p.encode().expect("encode SAllowObserve");
        assert_eq!(raw.id, ServerPacketId::AllowObserve as i16);

        let decoded = SAllowObserve::decode(&raw.payload).expect("decode SAllowObserve");
        assert_eq!(decoded.allow, p.allow);
    }

    #[test]
    fn time_of_day_roundtrip() {
        let p = STimeOfDay { lights: 3 };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::TimeOfDay as i16);
        assert_eq!(raw.payload.len(), 1);

        let decoded = STimeOfDay::decode(&raw.payload).expect("decode STimeOfDay");
        assert_eq!(decoded.lights, p.lights);
    }

    #[test]
    fn send_output_message_roundtrip() {
        let p = SSendOutputMessage {
            message: "Hello".to_string(),
            msg_type: 2,
        };

        let raw = p.encode().expect("encode SSendOutputMessage");
        assert_eq!(raw.id, ServerPacketId::SendOutputMessage as i16);

        let decoded = SSendOutputMessage::decode(&raw.payload)
            .expect("decode SSendOutputMessage");
        assert_eq!(decoded.message, p.message);
        assert_eq!(decoded.msg_type, p.msg_type);
    }

    #[test]
    fn remove_delayed_explosion_roundtrip() {
        let p = SRemoveDelayedExplosion { object_id: 123 };

        let raw = p.encode().expect("encode SRemoveDelayedExplosion");
        assert_eq!(raw.id, ServerPacketId::RemoveDelayedExplosion as i16);

        let decoded = SRemoveDelayedExplosion::decode(&raw.payload)
            .expect("decode SRemoveDelayedExplosion");
        assert_eq!(decoded.object_id, p.object_id);
    }

    #[test]
    fn object_deco_roundtrip() {
        let p = SObjectDeco {
            object_id: 7,
            location_x: 10,
            location_y: 20,
            image: 999,
        };

        let raw = p.encode().expect("encode SObjectDeco");
        assert_eq!(raw.id, ServerPacketId::ObjectDeco as i16);

        let decoded = SObjectDeco::decode(&raw.payload).expect("decode SObjectDeco");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.image, p.image);
    }

    #[test]
    fn object_sit_down_roundtrip() {
        let p = SObjectSitDown {
            object_id: 42,
            location_x: 100,
            location_y: 200,
            direction: 3,
            sitting: true,
        };

        let raw = p.encode().expect("encode SObjectSitDown");
        assert_eq!(raw.id, ServerPacketId::ObjectSitDown as i16);

        let decoded = SObjectSitDown::decode(&raw.payload).expect("decode SObjectSitDown");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.direction, p.direction);
        assert_eq!(decoded.sitting, p.sitting);
    }

    #[test]
    fn in_trap_rock_roundtrip() {
        let p = SInTrapRock { trapped: true };

        let raw = p.encode().expect("encode SInTrapRock");
        assert_eq!(raw.id, ServerPacketId::InTrapRock as i16);

        let decoded = SInTrapRock::decode(&raw.payload).expect("decode SInTrapRock");
        assert_eq!(decoded.trapped, p.trapped);
    }

    #[test]
    fn object_sneaking_roundtrip() {
        let p = SObjectSneaking {
            object_id: 5,
            sneaking_active: true,
        };

        let raw = p.encode().expect("encode SObjectSneaking");
        assert_eq!(raw.id, ServerPacketId::ObjectSneaking as i16);

        let decoded = SObjectSneaking::decode(&raw.payload)
            .expect("decode SObjectSneaking");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.sneaking_active, p.sneaking_active);
    }

    #[test]
    fn object_level_effects_roundtrip() {
        let p = SObjectLevelEffects {
            object_id: 8,
            level_effects: 0x1234,
        };

        let raw = p.encode().expect("encode SObjectLevelEffects");
        assert_eq!(raw.id, ServerPacketId::ObjectLevelEffects as i16);

        let decoded = SObjectLevelEffects::decode(&raw.payload)
            .expect("decode SObjectLevelEffects");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.level_effects, p.level_effects);
    }

    #[test]
    fn set_concentration_roundtrip() {
        let p = SSetConcentration {
            object_id: 10,
            enabled: true,
            interrupted: false,
        };

        let raw = p.encode().expect("encode SSetConcentration");
        assert_eq!(raw.id, ServerPacketId::SetConcentration as i16);

        let decoded = SSetConcentration::decode(&raw.payload)
            .expect("decode SSetConcentration");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.enabled, p.enabled);
        assert_eq!(decoded.interrupted, p.interrupted);
    }

    #[test]
    fn set_elemental_roundtrip() {
        let p = SSetElemental {
            object_id: 11,
            enabled: true,
            casted: true,
            value: 123,
            element_type: 2,
            exp_last: 999,
        };

        let raw = p.encode().expect("encode SSetElemental");
        assert_eq!(raw.id, ServerPacketId::SetElemental as i16);

        let decoded = SSetElemental::decode(&raw.payload).expect("decode SSetElemental");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.enabled, p.enabled);
        assert_eq!(decoded.casted, p.casted);
        assert_eq!(decoded.value, p.value);
        assert_eq!(decoded.element_type, p.element_type);
        assert_eq!(decoded.exp_last, p.exp_last);
    }

    #[test]
    fn mount_update_roundtrip() {
        let p = SMountUpdate {
            object_id: 12,
            mount_type: 5,
            riding_mount: true,
        };

        let raw = p.encode().expect("encode SMountUpdate");
        assert_eq!(raw.id, ServerPacketId::MountUpdate as i16);

        let decoded = SMountUpdate::decode(&raw.payload).expect("decode SMountUpdate");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.mount_type, p.mount_type);
        assert_eq!(decoded.riding_mount, p.riding_mount);
    }

    #[test]
    fn transform_update_roundtrip() {
        let p = STransformUpdate {
            object_id: 13,
            transform_type: 3,
        };

        let raw = p.encode().expect("encode STransformUpdate");
        assert_eq!(raw.id, ServerPacketId::TransformUpdate as i16);

        let decoded = STransformUpdate::decode(&raw.payload).expect("decode STransformUpdate");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.transform_type, p.transform_type);
    }

    #[test]
    fn equip_slot_item_roundtrip() {
        let p = SEquipSlotItem {
            grid: 1,
            unique_id: 0x1234_5678_9ABC_DEF0,
            to: 5,
            grid_to: 2,
            success: true,
        };

        let raw = p.encode().expect("encode SEquipSlotItem");
        assert_eq!(raw.id, ServerPacketId::EquipSlotItem as i16);

        let decoded = SEquipSlotItem::decode(&raw.payload).expect("decode SEquipSlotItem");
        assert_eq!(decoded.grid, p.grid);
        assert_eq!(decoded.unique_id, p.unique_id);
        assert_eq!(decoded.to, p.to);
        assert_eq!(decoded.grid_to, p.grid_to);
        assert_eq!(decoded.success, p.success);
    }

    #[test]
    fn fishing_update_roundtrip() {
        let p = SFishingUpdate {
            object_id: 14,
            fishing: true,
            progress_percent: 50,
            chance_percent: 75,
            fishing_point_x: 100,
            fishing_point_y: 200,
            found_fish: true,
        };

        let raw = p.encode().expect("encode SFishingUpdate");
        assert_eq!(raw.id, ServerPacketId::FishingUpdate as i16);

        let decoded = SFishingUpdate::decode(&raw.payload).expect("decode SFishingUpdate");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.fishing, p.fishing);
        assert_eq!(decoded.progress_percent, p.progress_percent);
        assert_eq!(decoded.chance_percent, p.chance_percent);
        assert_eq!(decoded.fishing_point_x, p.fishing_point_x);
        assert_eq!(decoded.fishing_point_y, p.fishing_point_y);
        assert_eq!(decoded.found_fish, p.found_fish);
    }

    #[test]
    fn set_binding_shot_roundtrip() {
        let p = SSetBindingShot {
            object_id: 9,
            enabled: true,
            value: 123456789,
        };

        let raw = p.encode().expect("encode SSetBindingShot");
        assert_eq!(raw.id, ServerPacketId::SetBindingShot as i16);

        let decoded = SSetBindingShot::decode(&raw.payload).expect("decode SSetBindingShot");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.enabled, p.enabled);
        assert_eq!(decoded.value, p.value);
    }

    #[test]
    fn switch_group_roundtrip() {
        let p = SSwitchGroup { allow_group: true };

        let raw = p.encode().expect("encode SSwitchGroup");
        assert_eq!(raw.id, ServerPacketId::SwitchGroup as i16);

        let decoded = SSwitchGroup::decode(&raw.payload).expect("decode SSwitchGroup");
        assert_eq!(decoded.allow_group, p.allow_group);
    }

    #[test]
    fn delete_group_roundtrip() {
        let p = SDeleteGroup;

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::DeleteGroup as i16);
        let _ = SDeleteGroup::decode(&raw.payload).expect("decode SDeleteGroup");
    }

    #[test]
    fn cancel_reincarnation_roundtrip() {
        let p = SCancelReincarnation;

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::CancelReincarnation as i16);
        let _ = SCancelReincarnation::decode(&raw.payload)
            .expect("decode SCancelReincarnation");
    }

    #[test]
    fn request_reincarnation_roundtrip() {
        let p = SRequestReincarnation;

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::RequestReincarnation as i16);
        let _ = SRequestReincarnation::decode(&raw.payload)
            .expect("decode SRequestReincarnation");
    }

    #[test]
    fn delete_member_roundtrip() {
        let p = SDeleteMember {
            name: "NameA".to_string(),
        };

        let raw = p.encode().expect("encode SDeleteMember");
        assert_eq!(raw.id, ServerPacketId::DeleteMember as i16);

        let decoded = SDeleteMember::decode(&raw.payload).expect("decode SDeleteMember");
        assert_eq!(decoded.name, p.name);
    }

    #[test]
    fn group_invite_roundtrip() {
        let p = SGroupInvite {
            name: "PlayerX".to_string(),
        };

        let raw = p.encode().expect("encode SGroupInvite");
        assert_eq!(raw.id, ServerPacketId::GroupInvite as i16);

        let decoded = SGroupInvite::decode(&raw.payload).expect("decode SGroupInvite");
        assert_eq!(decoded.name, p.name);
    }

    #[test]
    fn add_member_roundtrip() {
        let p = SAddMember {
            name: "PlayerY".to_string(),
        };

        let raw = p.encode().expect("encode SAddMember");
        assert_eq!(raw.id, ServerPacketId::AddMember as i16);

        let decoded = SAddMember::decode(&raw.payload).expect("decode SAddMember");
        assert_eq!(decoded.name, p.name);
    }

}
