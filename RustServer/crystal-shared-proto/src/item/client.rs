use std::io::{self, Cursor, Read};

use crate::io::{
    read_i32_le,
    read_u16_le,
    read_u32_le,
    read_u64_le,
    write_i32_le,
    write_u16_le,
    write_u32_le,
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
pub struct CStoreItem {
    pub from: i32,
    pub to: i32,
}

impl CStoreItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.from)?;
        write_i32_le(&mut buf, self.to)?;
        Ok(RawPacket {
            id: ClientPacketId::StoreItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let from = read_i32_le(&mut c)?;
        let to = read_i32_le(&mut c)?;
        Ok(CStoreItem { from, to })
    }
}

#[derive(Clone, Debug)]
pub struct CTakeBackItem {
    pub from: i32,
    pub to: i32,
}

impl CTakeBackItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.from)?;
        write_i32_le(&mut buf, self.to)?;
        Ok(RawPacket {
            id: ClientPacketId::TakeBackItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let from = read_i32_le(&mut c)?;
        let to = read_i32_le(&mut c)?;
        Ok(CTakeBackItem { from, to })
    }
}


#[derive(Clone, Debug)]
pub struct CRemoveSlotItem {
    pub grid: u8,
    pub grid_to: u8,
    pub unique_id: u64,
    pub to: i32,
    pub from_unique_id: u64,
}

impl CRemoveSlotItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.grid);
        buf.push(self.grid_to);
        write_u64_le(&mut buf, self.unique_id)?;
        write_i32_le(&mut buf, self.to)?;
        write_u64_le(&mut buf, self.from_unique_id)?;
        Ok(RawPacket {
            id: ClientPacketId::RemoveSlotItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let grid = one[0];
        c.read_exact(&mut one)?;
        let grid_to = one[0];
        let unique_id = read_u64_le(&mut c)?;
        let to = read_i32_le(&mut c)?;
        let from_unique_id = read_u64_le(&mut c)?;
        Ok(CRemoveSlotItem {
            grid,
            grid_to,
            unique_id,
            to,
            from_unique_id,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CSplitItem {
    pub grid: u8,
    pub unique_id: u64,
    pub count: u16,
}

impl CSplitItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.grid);
        write_u64_le(&mut buf, self.unique_id)?;
        write_u16_le(&mut buf, self.count)?;
        Ok(RawPacket {
            id: ClientPacketId::SplitItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let grid = one[0];
        let unique_id = read_u64_le(&mut c)?;
        let count = read_u16_le(&mut c)?;
        Ok(CSplitItem {
            grid,
            unique_id,
            count,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CMergeItem {
    pub grid_from: u8,
    pub grid_to: u8,
    pub id_from: u64,
    pub id_to: u64,
}

impl CMergeItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.grid_from);
        buf.push(self.grid_to);
        write_u64_le(&mut buf, self.id_from)?;
        write_u64_le(&mut buf, self.id_to)?;
        Ok(RawPacket {
            id: ClientPacketId::MergeItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let grid_from = one[0];
        c.read_exact(&mut one)?;
        let grid_to = one[0];
        let id_from = read_u64_le(&mut c)?;
        let id_to = read_u64_le(&mut c)?;
        Ok(CMergeItem {
            grid_from,
            grid_to,
            id_from,
            id_to,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CDropGold {
    pub amount: u32,
}

impl CDropGold {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.amount)?;
        Ok(RawPacket {
            id: ClientPacketId::DropGold as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let amount = read_u32_le(&mut c)?;
        Ok(CDropGold { amount })
    }
}

#[derive(Clone, Debug)]
pub struct CBuyItemBack {
    pub unique_id: u64,
    pub count: u16,
}

impl CBuyItemBack {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        write_u16_le(&mut buf, self.count)?;
        Ok(RawPacket {
            id: ClientPacketId::BuyItemBack as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        let count = read_u16_le(&mut c)?;
        Ok(CBuyItemBack { unique_id, count })
    }
}

#[derive(Clone, Debug)]
pub struct CEquipSlotItem {
    pub grid: u8,
    pub unique_id: u64,
    pub to: i32,
    pub grid_to: u8,
    pub to_unique_id: u64,
}

impl CEquipSlotItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.grid);
        write_u64_le(&mut buf, self.unique_id)?;
        write_i32_le(&mut buf, self.to)?;
        buf.push(self.grid_to);
        write_u64_le(&mut buf, self.to_unique_id)?;
        Ok(RawPacket {
            id: ClientPacketId::EquipSlotItem as i16,
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
        c.read_exact(&mut one)?;
        let grid_to = one[0];
        let to_unique_id = read_u64_le(&mut c)?;
        Ok(CEquipSlotItem {
            grid,
            unique_id,
            to,
            grid_to,
            to_unique_id,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CCombineItem {
    pub grid: u8,
    pub id_from: u64,
    pub id_to: u64,
}

impl CCombineItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.grid);
        write_u64_le(&mut buf, self.id_from)?;
        write_u64_le(&mut buf, self.id_to)?;
        Ok(RawPacket {
            id: ClientPacketId::CombineItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let grid = one[0];
        let id_from = read_u64_le(&mut c)?;
        let id_to = read_u64_le(&mut c)?;
        Ok(CCombineItem {
            grid,
            id_from,
            id_to,
        })
    }
}
