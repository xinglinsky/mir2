use std::io::{self, Cursor};

use crate::io::{
    read_u32_le,
    read_string,
    write_u32_le,
    write_string,
};
use crate::login::ClientPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct CCallNPC {
    pub object_id: u32,
    pub key: String,
}

impl CCallNPC {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_string(&mut buf, &self.key)?;
        Ok(RawPacket {
            id: ClientPacketId::CallNPC as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let key = read_string(&mut c)?;
        Ok(CCallNPC { object_id, key })
    }
}
