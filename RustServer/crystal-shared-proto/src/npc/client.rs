use std::io::{self, Cursor, Read};

use crate::io::{
    read_i32_le,
    read_u16_le,
    read_u32_le,
    read_u64_le,
    read_string,
    write_i32_le,
    write_u16_le,
    write_u32_le,
    write_u64_le,
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

#[derive(Clone, Debug)]
pub struct CCraftItem {
    pub unique_id: u64,
    pub count: u16,
    pub slots: Vec<i32>,
}

impl CCraftItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        write_u16_le(&mut buf, self.count)?;

        let len: i32 = self
            .slots
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many craft slots"))?;
        write_i32_le(&mut buf, len)?;
        for slot in &self.slots {
            write_i32_le(&mut buf, *slot)?;
        }

        Ok(RawPacket {
            id: ClientPacketId::CraftItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        let count = read_u16_le(&mut c)?;
        let slots_len = read_i32_le(&mut c)?;
        if slots_len < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative craft slots length",
            ));
        }
        let mut slots = Vec::with_capacity(slots_len as usize);
        for _ in 0..slots_len {
            let slot = read_i32_le(&mut c)?;
            slots.push(slot);
        }
        Ok(CCraftItem {
            unique_id,
            count,
            slots,
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

#[derive(Clone, Debug)]
pub struct CDepositRefineItem {
    pub from: i32,
    pub to: i32,
}

impl CDepositRefineItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.from)?;
        write_i32_le(&mut buf, self.to)?;
        Ok(RawPacket {
            id: ClientPacketId::DepositRefineItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let from = read_i32_le(&mut c)?;
        let to = read_i32_le(&mut c)?;
        Ok(CDepositRefineItem { from, to })
    }
}

#[derive(Clone, Debug)]
pub struct CRetrieveRefineItem {
    pub from: i32,
    pub to: i32,
}

impl CRetrieveRefineItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.from)?;
        write_i32_le(&mut buf, self.to)?;
        Ok(RawPacket {
            id: ClientPacketId::RetrieveRefineItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let from = read_i32_le(&mut c)?;
        let to = read_i32_le(&mut c)?;
        Ok(CRetrieveRefineItem { from, to })
    }
}

#[derive(Clone, Debug)]
pub struct CRefineCancel;

impl CRefineCancel {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::RefineCancel as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CRefineCancel payload must be empty",
            ));
        }
        Ok(CRefineCancel)
    }
}

#[derive(Clone, Debug)]
pub struct CRefineItem {
    pub unique_id: u64,
}

impl CRefineItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        Ok(RawPacket {
            id: ClientPacketId::RefineItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        Ok(CRefineItem { unique_id })
    }
}

#[derive(Clone, Debug)]
pub struct CCheckRefine {
    pub unique_id: u64,
}

impl CCheckRefine {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        Ok(RawPacket {
            id: ClientPacketId::CheckRefine as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        Ok(CCheckRefine { unique_id })
    }
}

#[derive(Clone, Debug)]
pub struct CReplaceWedRing {
    pub unique_id: u64,
}

impl CReplaceWedRing {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        Ok(RawPacket {
            id: ClientPacketId::ReplaceWedRing as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        Ok(CReplaceWedRing { unique_id })
    }
}

#[derive(Clone, Debug)]
pub struct CDepositTradeItem {
    pub from: i32,
    pub to: i32,
}

impl CDepositTradeItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.from)?;
        write_i32_le(&mut buf, self.to)?;
        Ok(RawPacket {
            id: ClientPacketId::DepositTradeItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let from = read_i32_le(&mut c)?;
        let to = read_i32_le(&mut c)?;
        Ok(CDepositTradeItem { from, to })
    }
}

#[derive(Clone, Debug)]
pub struct CRetrieveTradeItem {
    pub from: i32,
    pub to: i32,
}

impl CRetrieveTradeItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.from)?;
        write_i32_le(&mut buf, self.to)?;
        Ok(RawPacket {
            id: ClientPacketId::RetrieveTradeItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let from = read_i32_le(&mut c)?;
        let to = read_i32_le(&mut c)?;
        Ok(CRetrieveTradeItem { from, to })
    }
}

#[derive(Clone, Debug)]
pub struct CRepairItem {
    pub unique_id: u64,
}

impl CRepairItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        Ok(RawPacket {
            id: ClientPacketId::RepairItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        Ok(CRepairItem { unique_id })
    }
}

#[derive(Clone, Debug)]
pub struct CSRepairItem {
    pub unique_id: u64,
}

impl CSRepairItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        Ok(RawPacket {
            id: ClientPacketId::SRepairItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        Ok(CSRepairItem { unique_id })
    }
}