use std::io::{self, Read, Write};

use crate::io::{
    read_bool,
    read_i32_le,
    read_string,
    read_u32_le,
    write_bool,
    write_i32_le,
    write_string,
    write_u32_le,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientMovementInfoData {
    pub destination: i32,
    pub title: String,
    pub location_x: i32,
    pub location_y: i32,
    pub icon: i32,
}

impl ClientMovementInfoData {
    pub fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write_i32_le(w, self.destination)?;
        write_string(w, &self.title)?;
        write_i32_le(w, self.location_x)?;
        write_i32_le(w, self.location_y)?;
        write_i32_le(w, self.icon)
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
        let destination = read_i32_le(r)?;
        let title = read_string(r)?;
        let location_x = read_i32_le(r)?;
        let location_y = read_i32_le(r)?;
        let icon = read_i32_le(r)?;
        Ok(ClientMovementInfoData {
            destination,
            title,
            location_x,
            location_y,
            icon,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientNpcInfoData {
    pub object_id: u32,
    pub name: String,
    pub location_x: i32,
    pub location_y: i32,
    pub icon: i32,
    pub can_teleport_to: bool,
}

impl ClientNpcInfoData {
    pub fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write_u32_le(w, self.object_id)?;
        write_string(w, &self.name)?;
        write_i32_le(w, self.location_x)?;
        write_i32_le(w, self.location_y)?;
        write_i32_le(w, self.icon)?;
        write_bool(w, self.can_teleport_to)
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
        let object_id = read_u32_le(r)?;
        let name = read_string(r)?;
        let location_x = read_i32_le(r)?;
        let location_y = read_i32_le(r)?;
        let icon = read_i32_le(r)?;
        let can_teleport_to = read_bool(r)?;
        Ok(ClientNpcInfoData {
            object_id,
            name,
            location_x,
            location_y,
            icon,
            can_teleport_to,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientMapInfoData {
    pub width: i32,
    pub height: i32,
    pub big_map: i32,
    pub title: String,
    pub movements: Vec<ClientMovementInfoData>,
    pub npcs: Vec<ClientNpcInfoData>,
}

impl ClientMapInfoData {
    pub fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write_string(w, &self.title)?;
        write_i32_le(w, self.width)?;
        write_i32_le(w, self.height)?;
        write_i32_le(w, self.big_map)?;

        let mov_count: i32 = self
            .movements
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many movements"))?;
        write_i32_le(w, mov_count)?;
        for m in &self.movements {
            m.encode(w)?;
        }

        let npc_count: i32 = self
            .npcs
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many NPCs"))?;
        write_i32_le(w, npc_count)?;
        for n in &self.npcs {
            n.encode(w)?;
        }

        Ok(())
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
        let title = read_string(r)?;
        let width = read_i32_le(r)?;
        let height = read_i32_le(r)?;
        let big_map = read_i32_le(r)?;

        let mov_count = read_i32_le(r)?;
        if mov_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative movement count",
            ));
        }
        let mut movements = Vec::with_capacity(mov_count as usize);
        for _ in 0..mov_count {
            movements.push(ClientMovementInfoData::decode(r)?);
        }

        let npc_count = read_i32_le(r)?;
        if npc_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative NPC count",
            ));
        }
        let mut npcs = Vec::with_capacity(npc_count as usize);
        for _ in 0..npc_count {
            npcs.push(ClientNpcInfoData::decode(r)?);
        }

        Ok(ClientMapInfoData {
            width,
            height,
            big_map,
            title,
            movements,
            npcs,
        })
    }

    pub fn encode_to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.encode(&mut buf)?;
        Ok(buf)
    }

    pub fn decode_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let mut c = std::io::Cursor::new(bytes);
        Self::decode(&mut c)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldMapIconData {
    pub image_index: i32,
    pub title: String,
    pub map_index: i32,
}

impl WorldMapIconData {
    pub fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write_i32_le(w, self.image_index)?;
        write_string(w, &self.title)?;
        write_i32_le(w, self.map_index)
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
        let image_index = read_i32_le(r)?;
        let title = read_string(r)?;
        let map_index = read_i32_le(r)?;
        Ok(WorldMapIconData {
            image_index,
            title,
            map_index,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldMapSetupData {
    pub enabled: bool,
    pub icons: Vec<WorldMapIconData>,
}

impl WorldMapSetupData {
    pub fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write_bool(w, self.enabled)?;
        let count: i32 = self
            .icons
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many icons"))?;
        write_i32_le(w, count)?;
        for icon in &self.icons {
            icon.encode(w)?;
        }
        Ok(())
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
        let enabled = read_bool(r)?;
        let count = read_i32_le(r)?;
        if count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative world map icon count",
            ));
        }
        let mut icons = Vec::with_capacity(count as usize);
        for _ in 0..count {
            icons.push(WorldMapIconData::decode(r)?);
        }
        Ok(WorldMapSetupData { enabled, icons })
    }

    pub fn encode_to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.encode(&mut buf)?;
        Ok(buf)
    }

    pub fn decode_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let mut c = std::io::Cursor::new(bytes);
        Self::decode(&mut c)
    }
}
