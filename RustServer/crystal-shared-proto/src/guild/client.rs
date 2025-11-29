use std::io::{self, Cursor, Read};

use crate::io::{
    read_i32_le,
    read_string,
    read_u32_le,
    write_i32_le,
    write_string,
    write_u32_le,
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
pub struct CEditGuildNotice {
    pub notice: Vec<String>,
}

impl CEditGuildNotice {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();

        let count: i32 = self
            .notice
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many notice lines"))?;
        write_i32_le(&mut buf, count)?;
        for line in &self.notice {
            write_string(&mut buf, line)?;
        }

        Ok(RawPacket {
            id: ClientPacketId::EditGuildNotice as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let count = read_i32_le(&mut c)?;
        if count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative line count in CEditGuildNotice",
            ));
        }

        let mut notice = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let line = read_string(&mut c)?;
            notice.push(line);
        }

        Ok(CEditGuildNotice { notice })
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

#[derive(Clone, Debug)]
pub struct CRequestGuildInfo {
    pub info_type: u8,
}

impl CRequestGuildInfo {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.info_type);
        Ok(RawPacket {
            id: ClientPacketId::RequestGuildInfo as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CRequestGuildInfo payload must be exactly 1 byte",
            ));
        }
        Ok(CRequestGuildInfo {
            info_type: payload[0],
        })
    }
}

#[derive(Clone, Debug)]
pub struct CGuildStorageGoldChange {
    pub change_type: u8,
    pub amount: u32,
}

impl CGuildStorageGoldChange {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.change_type);
        write_u32_le(&mut buf, self.amount)?;
        Ok(RawPacket {
            id: ClientPacketId::GuildStorageGoldChange as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let change_type = one[0];
        let amount = read_u32_le(&mut c)?;
        Ok(CGuildStorageGoldChange {
            change_type,
            amount,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CGuildStorageItemChange {
    pub change_type: u8,
    pub from_slot: i32,
    pub to_slot: i32,
}

impl CGuildStorageItemChange {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.change_type);
        write_i32_le(&mut buf, self.from_slot)?;
        write_i32_le(&mut buf, self.to_slot)?;
        Ok(RawPacket {
            id: ClientPacketId::GuildStorageItemChange as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let change_type = one[0];
        let from_slot = read_i32_le(&mut c)?;
        let to_slot = read_i32_le(&mut c)?;
        Ok(CGuildStorageItemChange {
            change_type,
            from_slot,
            to_slot,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CGuildWarReturn {
    pub name: String,
}

impl CGuildWarReturn {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ClientPacketId::GuildWarReturn as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        Ok(CGuildWarReturn { name })
    }
}
