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
pub struct CNewHero {
    pub name: String,
    pub gender: u8,
    pub class: u8,
}

impl CNewHero {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        buf.push(self.gender);
        buf.push(self.class);
        Ok(RawPacket {
            id: ClientPacketId::NewHero as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let gender = one[0];
        c.read_exact(&mut one)?;
        let class = one[0];
        Ok(CNewHero { name, gender, class })
    }
}

#[derive(Clone, Debug)]
pub struct CSetHeroBehaviour {
    pub behaviour: u8,
}

impl CSetHeroBehaviour {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::SetHeroBehaviour as i16,
            payload: vec![self.behaviour],
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CSetHeroBehaviour payload must be exactly 1 byte",
            ));
        }
        Ok(CSetHeroBehaviour {
            behaviour: payload[0],
        })
    }
}

#[derive(Clone, Debug)]
pub struct CChangeHero {
    pub list_index: i32,
}

impl CChangeHero {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.list_index)?;
        Ok(RawPacket {
            id: ClientPacketId::ChangeHero as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let list_index = read_i32_le(&mut c)?;
        Ok(CChangeHero { list_index })
    }
}

#[derive(Clone, Debug)]
pub struct CSetAutoPotValue {
    pub stat: u8,
    pub value: u32,
}

impl CSetAutoPotValue {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.stat);
        write_u32_le(&mut buf, self.value)?;
        Ok(RawPacket {
            id: ClientPacketId::SetAutoPotValue as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let stat = one[0];
        let value = read_u32_le(&mut c)?;
        Ok(CSetAutoPotValue { stat, value })
    }
}

#[derive(Clone, Debug)]
pub struct CSetAutoPotItem {
    pub grid: u8,
    pub item_index: i32,
}

impl CSetAutoPotItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.grid);
        write_i32_le(&mut buf, self.item_index)?;
        Ok(RawPacket {
            id: ClientPacketId::SetAutoPotItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let grid = one[0];
        let item_index = read_i32_le(&mut c)?;
        Ok(CSetAutoPotItem { grid, item_index })
    }
}

#[derive(Clone, Debug)]
pub struct CTakeBackHeroItem {
    pub from: i32,
    pub to: i32,
}

impl CTakeBackHeroItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.from)?;
        write_i32_le(&mut buf, self.to)?;
        Ok(RawPacket {
            id: ClientPacketId::TakeBackHeroItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let from = read_i32_le(&mut c)?;
        let to = read_i32_le(&mut c)?;
        Ok(CTakeBackHeroItem { from, to })
    }
}

#[derive(Clone, Debug)]
pub struct CTransferHeroItem {
    pub from: i32,
    pub to: i32,
}

impl CTransferHeroItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.from)?;
        write_i32_le(&mut buf, self.to)?;
        Ok(RawPacket {
            id: ClientPacketId::TransferHeroItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let from = read_i32_le(&mut c)?;
        let to = read_i32_le(&mut c)?;
        Ok(CTransferHeroItem { from, to })
    }
}
