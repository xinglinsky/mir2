use std::io::{self, Cursor, Read};

use crate::io::{
    read_bool,
    read_i16_le,
    read_i32_le,
    read_i64_le,
    read_string,
    read_u16_le,
    read_u32_le,
    write_bool,
    write_i16_le,
    write_i32_le,
    write_i64_le,
    write_string,
    write_u16_le,
    write_u32_le,
};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

use super::flat;

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
pub struct SObjectTurnWalkRun {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SObjectTurnWalkRun {
    pub(crate) fn encode_with_id(&self, id: ServerPacketId) -> io::Result<RawPacket> {
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

    pub(crate) fn decode_from(payload: &[u8]) -> io::Result<Self> {
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

pub use flat::SObjectRemove;
pub use flat::SObjectHide;
pub use flat::SObjectShow;
pub use flat::SObjectPoisoned;
pub use flat::SObjectTeleportOut;
pub use flat::SObjectTeleportIn;
pub use flat::SObjectHealth;
pub use flat::SObjectMana;
pub use flat::SObjectDeco;
pub use flat::SObjectSneaking;
pub use flat::SObjectLevelEffects;
pub use flat::SObjectSitDown;
