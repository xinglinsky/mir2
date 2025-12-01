use std::io::{self, Cursor};

use crate::io::{read_bool, read_i32_le, read_string, write_bool, write_i32_le, write_string};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

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
pub struct SGroupMembersMap {
    pub player_name: String,
    pub player_map: String,
}

impl SGroupMembersMap {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.player_name)?;
        write_string(&mut buf, &self.player_map)?;
        Ok(RawPacket {
            id: ServerPacketId::GroupMembersMap as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let player_name = read_string(&mut c)?;
        let player_map = read_string(&mut c)?;
        Ok(SGroupMembersMap { player_name, player_map })
    }
}

#[derive(Clone, Debug)]
pub struct SSendMemberLocation {
    pub member_name: String,
    pub member_location_x: i32,
    pub member_location_y: i32,
}

impl SSendMemberLocation {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.member_name)?;
        write_i32_le(&mut buf, self.member_location_x)?;
        write_i32_le(&mut buf, self.member_location_y)?;
        Ok(RawPacket {
            id: ServerPacketId::SendMemberLocation as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let member_name = read_string(&mut c)?;
        let member_location_x = read_i32_le(&mut c)?;
        let member_location_y = read_i32_le(&mut c)?;
        Ok(SSendMemberLocation {
            member_name,
            member_location_x,
            member_location_y,
        })
    }
}
