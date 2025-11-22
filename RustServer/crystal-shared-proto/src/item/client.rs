use std::io::{self, Cursor, Read};

use crate::io::{
    read_i32_le,
    read_u64_le,
    write_i32_le,
    write_u64_le,
};
use crate::login::ClientPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct CMoveItem {
    pub grid: u8,
    pub from: i32,
    pub to: i32,
}

impl CMoveItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.grid);
        write_i32_le(&mut buf, self.from)?;
        write_i32_le(&mut buf, self.to)?;
        Ok(RawPacket {
            id: ClientPacketId::MoveItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let grid = one[0];
        let from = read_i32_le(&mut c)?;
        let to = read_i32_le(&mut c)?;
        Ok(CMoveItem { grid, from, to })
    }
}

#[derive(Clone, Debug)]
pub struct CEquipItem {
    pub grid: u8,
    pub unique_id: u64,
    pub to: i32,
}

impl CEquipItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.grid);
        write_u64_le(&mut buf, self.unique_id)?;
        write_i32_le(&mut buf, self.to)?;
        Ok(RawPacket {
            id: ClientPacketId::EquipItem as i16,
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
        Ok(CEquipItem {
            grid,
            unique_id,
            to,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CRemoveItem {
    pub grid: u8,
    pub unique_id: u64,
    pub to: i32,
}

impl CRemoveItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.grid);
        write_u64_le(&mut buf, self.unique_id)?;
        write_i32_le(&mut buf, self.to)?;
        Ok(RawPacket {
            id: ClientPacketId::RemoveItem as i16,
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
        Ok(CRemoveItem {
            grid,
            unique_id,
            to,
        })
    }
}
