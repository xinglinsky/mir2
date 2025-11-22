use std::io::{self, Cursor, Read};

use crate::io::{read_bool, read_string, write_bool, write_string};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

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
