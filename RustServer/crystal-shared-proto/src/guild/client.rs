use std::io::{self, Cursor};

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
