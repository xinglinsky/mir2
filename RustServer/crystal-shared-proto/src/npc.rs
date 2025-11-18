// NPC-related packets (ObjectNpc, NPCResponse) compatible with the C# Crystal implementation.

use std::io::{self, Cursor, Read};

use crate::io::{
    read_f32_le, read_i32_le, read_string, read_u16_le, read_u32_le, write_f32_le, write_i32_le,
    write_string, write_u16_le, write_u32_le,
};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct SObjectNpc {
    pub object_id: u32,
    pub name: String,
    pub name_colour_argb: i32,
    pub image: u16,
    pub colour_argb: i32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
    pub quest_ids: Vec<i32>,
}

impl SObjectNpc {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();

        write_u32_le(&mut buf, self.object_id)?;
        write_string(&mut buf, &self.name)?;
        write_i32_le(&mut buf, self.name_colour_argb)?;
        write_u16_le(&mut buf, self.image)?;
        write_i32_le(&mut buf, self.colour_argb)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);

        let count: i32 = self
            .quest_ids
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many quest ids"))?;
        write_i32_le(&mut buf, count)?;
        for id in &self.quest_ids {
            write_i32_le(&mut buf, *id)?;
        }

        Ok(RawPacket {
            id: ServerPacketId::ObjectNpc as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);

        let object_id = read_u32_le(&mut c)?;
        let name = read_string(&mut c)?;
        let name_colour_argb = read_i32_le(&mut c)?;
        let image = read_u16_le(&mut c)?;
        let colour_argb = read_i32_le(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];

        let count = read_i32_le(&mut c)?;
        if count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative quest id count",
            ));
        }
        let mut quest_ids = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let id = read_i32_le(&mut c)?;
            quest_ids.push(id);
        }

        Ok(SObjectNpc {
            object_id,
            name,
            name_colour_argb,
            image,
            colour_argb,
            location_x,
            location_y,
            direction,
            quest_ids,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SNpcResponse {
    pub page: Vec<String>,
}

impl SNpcResponse {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        let count: i32 = self
            .page
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many page lines"))?;
        write_i32_le(&mut buf, count)?;
        for line in &self.page {
            write_string(&mut buf, line)?;
        }
        Ok(RawPacket {
            id: ServerPacketId::NpcResponse as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let count = read_i32_le(&mut c)?;
        if count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative page line count",
            ));
        }
        let mut page = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let line = read_string(&mut c)?;
            page.push(line);
        }
        Ok(SNpcResponse { page })
    }
}

/// Raw NPCGoods payload as written by the C# server, including:
/// count + repeated UserItem.Save(writer) blobs + Rate + Type + HideAddedStats.
#[derive(Clone, Debug)]
pub struct SNpcGoods {
    pub goods_bytes: Vec<u8>,
}

impl SNpcGoods {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::NpcGoods as i16,
            payload: self.goods_bytes.clone(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SNpcGoods {
            goods_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SNpcSell;

impl SNpcSell {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::NpcSell as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SNpcSell payload must be empty",
            ));
        }
        Ok(SNpcSell)
    }
}

#[derive(Clone, Debug)]
pub struct SNpcRepair {
    pub rate: f32,
}

impl SNpcRepair {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_f32_le(&mut buf, self.rate)?;
        Ok(RawPacket {
            id: ServerPacketId::NpcRepair as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let rate = read_f32_le(&mut c)?;
        Ok(SNpcRepair { rate })
    }
}

#[derive(Clone, Debug)]
pub struct SNpcsRepair {
    pub rate: f32,
}

impl SNpcsRepair {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_f32_le(&mut buf, self.rate)?;
        Ok(RawPacket {
            id: ServerPacketId::NpcsRepair as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let rate = read_f32_le(&mut c)?;
        Ok(SNpcsRepair { rate })
    }
}

#[derive(Clone, Debug)]
pub struct SNpcRefine {
    pub rate: f32,
    pub refining: bool,
}

impl SNpcRefine {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_f32_le(&mut buf, self.rate)?;
        buf.push(self.refining as u8);
        Ok(RawPacket {
            id: ServerPacketId::NpcRefine as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let rate = read_f32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let refining = one[0] != 0;
        Ok(SNpcRefine { rate, refining })
    }
}

#[derive(Clone, Debug)]
pub struct SNpcCheckRefine;

impl SNpcCheckRefine {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::NpcCheckRefine as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SNpcCheckRefine payload must be empty",
            ));
        }
        Ok(SNpcCheckRefine)
    }
}

#[derive(Clone, Debug)]
pub struct SNpcCollectRefine {
    pub success: bool,
}

impl SNpcCollectRefine {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.success as u8);
        Ok(RawPacket {
            id: ServerPacketId::NpcCollectRefine as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SNpcCollectRefine payload must be exactly 1 byte",
            ));
        }
        Ok(SNpcCollectRefine {
            success: payload[0] != 0,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SNpcReplaceWedRing {
    pub rate: f32,
}

impl SNpcReplaceWedRing {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_f32_le(&mut buf, self.rate)?;
        Ok(RawPacket {
            id: ServerPacketId::NpcReplaceWedRing as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let rate = read_f32_le(&mut c)?;
        Ok(SNpcReplaceWedRing { rate })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_npc_roundtrip() {
        let p = SObjectNpc {
            object_id: 7,
            name: "NPC".to_string(),
            name_colour_argb: -1,
            image: 10,
            colour_argb: 0x00FF00,
            location_x: 100,
            location_y: 200,
            direction: 3,
            quest_ids: vec![1, 2, 3],
        };

        let raw = p.encode().expect("encode SObjectNpc");
        assert_eq!(raw.id, ServerPacketId::ObjectNpc as i16);

        let decoded = SObjectNpc::decode(&raw.payload).expect("decode SObjectNpc");
        assert_eq!(decoded.object_id, p.object_id);
        assert_eq!(decoded.name, p.name);
        assert_eq!(decoded.name_colour_argb, p.name_colour_argb);
        assert_eq!(decoded.image, p.image);
        assert_eq!(decoded.colour_argb, p.colour_argb);
        assert_eq!(decoded.location_x, p.location_x);
        assert_eq!(decoded.location_y, p.location_y);
        assert_eq!(decoded.direction, p.direction);
        assert_eq!(decoded.quest_ids, p.quest_ids);
    }

    #[test]
    fn npc_response_roundtrip() {
        let p = SNpcResponse {
            page: vec!["Line1".to_string(), "Line2".to_string()],
        };

        let raw = p.encode().expect("encode SNpcResponse");
        assert_eq!(raw.id, ServerPacketId::NpcResponse as i16);

        let decoded = SNpcResponse::decode(&raw.payload).expect("decode SNpcResponse");
        assert_eq!(decoded.page, p.page);
    }

    #[test]
    fn npc_goods_roundtrip() {
        let p = SNpcGoods {
            goods_bytes: vec![1, 2, 3, 4, 5],
        };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::NpcGoods as i16);
        assert_eq!(raw.payload, p.goods_bytes);

        let decoded = SNpcGoods::decode(&raw.payload).expect("decode SNpcGoods");
        assert_eq!(decoded.goods_bytes, p.goods_bytes);
    }

    #[test]
    fn npc_sell_roundtrip() {
        let p = SNpcSell;

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::NpcSell as i16);

        let decoded = SNpcSell::decode(&raw.payload).expect("decode SNpcSell");
        let _ = decoded;
    }

    #[test]
    fn npc_repair_roundtrip() {
        let p = SNpcRepair { rate: 1.5 };

        let raw = p.encode().expect("encode SNpcRepair");
        assert_eq!(raw.id, ServerPacketId::NpcRepair as i16);

        let decoded = SNpcRepair::decode(&raw.payload).expect("decode SNpcRepair");
        assert!((decoded.rate - p.rate).abs() < f32::EPSILON);
    }

    #[test]
    fn npcs_repair_roundtrip() {
        let p = SNpcsRepair { rate: 2.5 };

        let raw = p.encode().expect("encode SNpcsRepair");
        assert_eq!(raw.id, ServerPacketId::NpcsRepair as i16);

        let decoded = SNpcsRepair::decode(&raw.payload).expect("decode SNpcsRepair");
        assert!((decoded.rate - p.rate).abs() < f32::EPSILON);
    }

    #[test]
    fn npc_refine_roundtrip() {
        let p = SNpcRefine {
            rate: 3.5,
            refining: true,
        };

        let raw = p.encode().expect("encode SNpcRefine");
        assert_eq!(raw.id, ServerPacketId::NpcRefine as i16);

        let decoded = SNpcRefine::decode(&raw.payload).expect("decode SNpcRefine");
        assert!((decoded.rate - p.rate).abs() < f32::EPSILON);
        assert_eq!(decoded.refining, p.refining);
    }

    #[test]
    fn npc_check_refine_roundtrip() {
        let p = SNpcCheckRefine;

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::NpcCheckRefine as i16);

        let decoded = SNpcCheckRefine::decode(&raw.payload).expect("decode SNpcCheckRefine");
        let _ = decoded;
    }

    #[test]
    fn npc_collect_refine_roundtrip() {
        let p = SNpcCollectRefine { success: true };

        let raw = p.encode().expect("encode SNpcCollectRefine");
        assert_eq!(raw.id, ServerPacketId::NpcCollectRefine as i16);

        let decoded = SNpcCollectRefine::decode(&raw.payload)
            .expect("decode SNpcCollectRefine");
        assert_eq!(decoded.success, p.success);
    }

    #[test]
    fn npc_replace_wed_ring_roundtrip() {
        let p = SNpcReplaceWedRing { rate: 4.0 };

        let raw = p.encode().expect("encode SNpcReplaceWedRing");
        assert_eq!(raw.id, ServerPacketId::NpcReplaceWedRing as i16);

        let decoded = SNpcReplaceWedRing::decode(&raw.payload)
            .expect("decode SNpcReplaceWedRing");
        assert!((decoded.rate - p.rate).abs() < f32::EPSILON);
    }
}
