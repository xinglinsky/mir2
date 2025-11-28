use std::io::{self, Cursor, Read};

use crate::io::{
    read_string,
    write_string,
};
use crate::login::ClientPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct CGuildInvite {
    pub accept_invite: bool,
}

impl CGuildInvite {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::with_capacity(1);
        buf.push(if self.accept_invite { 1 } else { 0 });
        Ok(RawPacket {
            id: ClientPacketId::GuildInvite as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CGuildInvite payload must be exactly 1 byte",
            ));
        }
        Ok(CGuildInvite {
            accept_invite: payload[0] != 0,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CEditGuildMember {
    pub change_type: u8,
    pub rank_index: u8,
    pub name: String,
    pub rank_name: String,
}

impl CEditGuildMember {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.change_type);
        buf.push(self.rank_index);
        write_string(&mut buf, &self.name)?;
        write_string(&mut buf, &self.rank_name)?;
        Ok(RawPacket {
            id: ClientPacketId::EditGuildMember as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let change_type = one[0];
        c.read_exact(&mut one)?;
        let rank_index = one[0];
        let name = read_string(&mut c)?;
        let rank_name = read_string(&mut c)?;
        Ok(CEditGuildMember {
            change_type,
            rank_index,
            name,
            rank_name,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CGuildNameReturn {
    pub name: String,
}

impl CGuildNameReturn {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ClientPacketId::GuildNameReturn as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        Ok(CGuildNameReturn { name })
    }
}
