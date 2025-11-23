use std::io::{self, Cursor, Read};

use crate::io::{
    read_i32_le,
    read_u16_le,
    read_u64_le,
    write_i32_le,
    write_u16_le,
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

#[derive(Clone, Debug)]
pub struct CUseItem {
    pub unique_id: u64,
    pub grid: u8,
}

impl CUseItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        buf.push(self.grid);
        Ok(RawPacket {
            id: ClientPacketId::UseItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let grid = one[0];
        Ok(CUseItem { unique_id, grid })
    }
}

#[derive(Clone, Debug)]
pub struct CDropItem {
    pub unique_id: u64,
    pub count: u16,
    pub hero_inventory: bool,
}

impl CDropItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        write_u16_le(&mut buf, self.count)?;
        crate::io::write_bool(&mut buf, self.hero_inventory)?;
        Ok(RawPacket {
            id: ClientPacketId::DropItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        let count = read_u16_le(&mut c)?;
        let hero_inventory = crate::io::read_bool(&mut c)?;
        Ok(CDropItem {
            unique_id,
            count,
            hero_inventory,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CBuyItem {
    pub item_index: u64,
    pub count: u16,
    pub panel_type: u8,
}

impl CBuyItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.item_index)?;
        write_u16_le(&mut buf, self.count)?;
        buf.push(self.panel_type);
        Ok(RawPacket {
            id: ClientPacketId::BuyItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let item_index = read_u64_le(&mut c)?;
        let count = read_u16_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let panel_type = one[0];
        Ok(CBuyItem {
            item_index,
            count,
            panel_type,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CSellItem {
    pub unique_id: u64,
    pub count: u16,
}

impl CSellItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        write_u16_le(&mut buf, self.count)?;
        Ok(RawPacket {
            id: ClientPacketId::SellItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        let count = read_u16_le(&mut c)?;
        Ok(CSellItem { unique_id, count })
    }
}
