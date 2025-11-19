// Map-related packets (e.g. MapInformation) compatible with the existing C# Crystal implementation.

use std::io::{self, Cursor, Read};

use crate::io::{
    read_i32_le, read_string, read_u16_le, read_u32_le, write_bool, write_i16_le, write_i32_le,
    write_string, write_u16_le, write_u32_le,
};
use crate::map_types::{ClientMapInfoData, WorldMapSetupData};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct SMapInformation {
    pub map_index: i32,
    pub file_name: String,
    pub title: String,
    pub mini_map: u16,
    pub big_map: u16,
    pub lights: u8,
    pub lightning: bool,
    pub fire: bool,
    pub map_dark_light: u8,
    pub music: u16,
    pub weather_particles: u16,
}

#[derive(Clone, Debug)]
pub struct SMapChanged {
    pub map_index: i32,
    pub file_name: String,
    pub title: String,
    pub mini_map: u16,
    pub big_map: u16,
    pub lights: u8,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
    pub map_dark_light: u8,
    pub music: u16,
    pub weather: u16,
}

impl SMapChanged {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();

        write_i32_le(&mut buf, self.map_index)?;
        write_string(&mut buf, &self.file_name)?;
        write_string(&mut buf, &self.title)?;
        write_u16_le(&mut buf, self.mini_map)?;
        write_u16_le(&mut buf, self.big_map)?;
        buf.push(self.lights);
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        buf.push(self.map_dark_light);
        write_u16_le(&mut buf, self.music)?;
        write_u16_le(&mut buf, self.weather)?;

        Ok(RawPacket {
            id: ServerPacketId::MapChanged as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);

        let map_index = read_i32_le(&mut c)?;
        let file_name = read_string(&mut c)?;
        let title = read_string(&mut c)?;
        let mini_map = read_u16_le(&mut c)?;
        let big_map = read_u16_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let lights = one[0];
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        c.read_exact(&mut one)?;
        let direction = one[0];
        c.read_exact(&mut one)?;
        let map_dark_light = one[0];
        let music = read_u16_le(&mut c)?;
        let weather = read_u16_le(&mut c)?;

        Ok(SMapChanged {
            map_index,
            file_name,
            title,
            mini_map,
            big_map,
            lights,
            location_x,
            location_y,
            direction,
            map_dark_light,
            music,
            weather,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SMapEffect {
    pub location_x: i32,
    pub location_y: i32,
    pub effect: u8,
    pub value: u8,
}

impl SMapEffect {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();

        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.effect);
        buf.push(self.value);

        Ok(RawPacket {
            id: ServerPacketId::MapEffect as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);

        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let effect = one[0];
        c.read_exact(&mut one)?;
        let value = one[0];

        Ok(SMapEffect {
            location_x,
            location_y,
            effect,
            value,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SNewMapInfo {
    pub map_index: i32,
    /// Raw bytes representing ClientMapInfo.Save(writer) payload.
    pub info_bytes: Vec<u8>,
}

impl SNewMapInfo {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.map_index)?;
        buf.extend_from_slice(&self.info_bytes);
        Ok(RawPacket {
            id: ServerPacketId::NewMapInfo as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let map_index = read_i32_le(&mut c)?;
        let pos = c.position() as usize;
        let info_bytes = payload[pos..].to_vec();

        Ok(SNewMapInfo {
            map_index,
            info_bytes,
        })
    }
}

impl SNewMapInfo {
    pub fn decode_map_info(&self) -> io::Result<ClientMapInfoData> {
        ClientMapInfoData::decode_from_bytes(&self.info_bytes)
    }

    pub fn from_map_info(map_index: i32, info: &ClientMapInfoData) -> io::Result<Self> {
        let bytes = info.encode_to_bytes()?;
        Ok(SNewMapInfo {
            map_index,
            info_bytes: bytes,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SWorldMapSetupInfo {
    /// Raw bytes representing WorldMapSetup.Save(writer) payload.
    pub setup_bytes: Vec<u8>,
    pub teleport_to_npc_cost: i32,
}

impl SWorldMapSetupInfo {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.setup_bytes);
        write_i32_le(&mut buf, self.teleport_to_npc_cost)?;

        Ok(RawPacket {
            id: ServerPacketId::WorldMapSetup as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() < 4 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SWorldMapSetupInfo payload too short",
            ));
        }

        let split = payload.len() - 4;
        let (setup_bytes, tail) = payload.split_at(split);
        let mut c = Cursor::new(tail);
        let teleport_to_npc_cost = read_i32_le(&mut c)?;

        Ok(SWorldMapSetupInfo {
            setup_bytes: setup_bytes.to_vec(),
            teleport_to_npc_cost,
        })
    }
}

impl SWorldMapSetupInfo {
    pub fn decode_world_map_setup(&self) -> io::Result<WorldMapSetupData> {
        WorldMapSetupData::decode_from_bytes(&self.setup_bytes)
    }

    pub fn from_world_map_setup(setup: &WorldMapSetupData, teleport_to_npc_cost: i32) -> io::Result<Self> {
        let bytes = setup.encode_to_bytes()?;
        Ok(SWorldMapSetupInfo {
            setup_bytes: bytes,
            teleport_to_npc_cost,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SSearchMapResult {
    pub map_index: i32,
    pub npc_index: u32,
}

impl SSearchMapResult {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.map_index)?;
        write_u32_le(&mut buf, self.npc_index)?;

        Ok(RawPacket {
            id: ServerPacketId::SearchMapResult as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SSearchMapResult payload must be exactly 8 bytes",
            ));
        }

        let mut c = Cursor::new(payload);
        let map_index = read_i32_le(&mut c)?;
        let npc_index = read_u32_le(&mut c)?;

        Ok(SSearchMapResult { map_index, npc_index })
    }
}

impl SMapInformation {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();

        write_i32_le(&mut buf, self.map_index)?;
        write_string(&mut buf, &self.file_name)?;
        write_string(&mut buf, &self.title)?;
        write_u16_le(&mut buf, self.mini_map)?;
        write_u16_le(&mut buf, self.big_map)?;

        // Lights as byte
        buf.push(self.lights);

        // Lightning/Fire packed into a single byte as in C# implementation.
        let mut bools: u8 = 0;
        if self.lightning {
            bools |= 0x01;
        }
        if self.fire {
            bools |= 0x02;
        }
        buf.push(bools);

        // MapDarkLight
        buf.push(self.map_dark_light);

        // Music and WeatherParticles
        write_u16_le(&mut buf, self.music)?;
        write_u16_le(&mut buf, self.weather_particles)?;

        Ok(RawPacket {
            id: ServerPacketId::MapInformation as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);

        let map_index = read_i32_le(&mut c)?;
        let file_name = read_string(&mut c)?;
        let title = read_string(&mut c)?;
        let mini_map = read_u16_le(&mut c)?;
        let big_map = read_u16_le(&mut c)?;

        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let lights = one[0];

        c.read_exact(&mut one)?;
        let bools = one[0];
        let lightning = (bools & 0x01) == 0x01;
        let fire = (bools & 0x02) == 0x02;

        c.read_exact(&mut one)?;
        let map_dark_light = one[0];

        let music = read_u16_le(&mut c)?;
        let weather_particles = read_u16_le(&mut c)?;

        Ok(SMapInformation {
            map_index,
            file_name,
            title,
            mini_map,
            big_map,
            lights,
            lightning,
            fire,
            map_dark_light,
            music,
            weather_particles,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_types::{
        ClientMapInfoData,
        ClientMovementInfoData,
        ClientNpcInfoData,
        WorldMapIconData,
        WorldMapSetupData,
    };

    #[test]
    fn map_information_roundtrip() {
        let packet = SMapInformation {
            map_index: 1,
            file_name: "0".to_string(),
            title: "Map".to_string(),
            mini_map: 10,
            big_map: 20,
            lights: 3,
            lightning: true,
            fire: false,
            map_dark_light: 5,
            music: 100,
            weather_particles: 2,
        };

        let raw = packet.encode().expect("encode SMapInformation");
        assert_eq!(raw.id, ServerPacketId::MapInformation as i16);

        let decoded = SMapInformation::decode(&raw.payload).expect("decode SMapInformation");
        assert_eq!(decoded.map_index, packet.map_index);
        assert_eq!(decoded.file_name, packet.file_name);
        assert_eq!(decoded.title, packet.title);
        assert_eq!(decoded.mini_map, packet.mini_map);
        assert_eq!(decoded.big_map, packet.big_map);
        assert_eq!(decoded.lights, packet.lights);
        assert_eq!(decoded.lightning, packet.lightning);
        assert_eq!(decoded.fire, packet.fire);
        assert_eq!(decoded.map_dark_light, packet.map_dark_light);
        assert_eq!(decoded.music, packet.music);
        assert_eq!(decoded.weather_particles, packet.weather_particles);
    }

    #[test]
    fn map_changed_roundtrip() {
        let packet = SMapChanged {
            map_index: 2,
            file_name: "3".to_string(),
            title: "Another".to_string(),
            mini_map: 11,
            big_map: 22,
            lights: 2,
            location_x: 50,
            location_y: 60,
            direction: 1,
            map_dark_light: 7,
            music: 200,
            weather: 5,
        };

        let raw = packet.encode().expect("encode SMapChanged");
        assert_eq!(raw.id, ServerPacketId::MapChanged as i16);

        let decoded = SMapChanged::decode(&raw.payload).expect("decode SMapChanged");
        assert_eq!(decoded.map_index, packet.map_index);
        assert_eq!(decoded.file_name, packet.file_name);
        assert_eq!(decoded.title, packet.title);
        assert_eq!(decoded.mini_map, packet.mini_map);
        assert_eq!(decoded.big_map, packet.big_map);
        assert_eq!(decoded.lights, packet.lights);
        assert_eq!(decoded.location_x, packet.location_x);
        assert_eq!(decoded.location_y, packet.location_y);
        assert_eq!(decoded.direction, packet.direction);
        assert_eq!(decoded.map_dark_light, packet.map_dark_light);
        assert_eq!(decoded.music, packet.music);
        assert_eq!(decoded.weather, packet.weather);
    }

    #[test]
    fn map_effect_roundtrip() {
        let packet = SMapEffect {
            location_x: 10,
            location_y: 20,
            effect: 3,
            value: 7,
        };

        let raw = packet.encode().expect("encode SMapEffect");
        assert_eq!(raw.id, ServerPacketId::MapEffect as i16);

        let decoded = SMapEffect::decode(&raw.payload).expect("decode SMapEffect");
        assert_eq!(decoded.location_x, packet.location_x);
        assert_eq!(decoded.location_y, packet.location_y);
        assert_eq!(decoded.effect, packet.effect);
        assert_eq!(decoded.value, packet.value);
    }

    #[test]
    fn new_map_info_roundtrip() {
        let packet = SNewMapInfo {
            map_index: 5,
            info_bytes: vec![1, 2, 3, 4, 5],
        };

        let raw = packet.encode().expect("encode SNewMapInfo");
        assert_eq!(raw.id, ServerPacketId::NewMapInfo as i16);

        let decoded = SNewMapInfo::decode(&raw.payload).expect("decode SNewMapInfo");
        assert_eq!(decoded.map_index, packet.map_index);
        assert_eq!(decoded.info_bytes, packet.info_bytes);
    }

    #[test]
    fn new_map_info_structured_roundtrip() {
        let info = ClientMapInfoData {
            width: 100,
            height: 200,
            big_map: 1,
            title: "Test Map".to_string(),
            movements: vec![ClientMovementInfoData {
                destination: 1,
                title: "Gate".to_string(),
                location_x: 10,
                location_y: 20,
                icon: 0,
            }],
            npcs: vec![ClientNpcInfoData {
                object_id: 42,
                name: "NPC".to_string(),
                location_x: 5,
                location_y: 6,
                icon: 1,
                can_teleport_to: true,
            }],
        };

        let pkt = SNewMapInfo::from_map_info(3, &info).expect("from_map_info");
        let raw = pkt.encode().expect("encode SNewMapInfo");
        assert_eq!(raw.id, ServerPacketId::NewMapInfo as i16);

        let decoded_pkt = SNewMapInfo::decode(&raw.payload).expect("decode SNewMapInfo");
        assert_eq!(decoded_pkt.map_index, 3);
        let decoded_info = decoded_pkt
            .decode_map_info()
            .expect("decode_map_info");
        assert_eq!(decoded_info, info);
    }

    #[test]
    fn world_map_setup_info_roundtrip() {
        let packet = SWorldMapSetupInfo {
            setup_bytes: vec![9, 8, 7],
            teleport_to_npc_cost: 1234,
        };

        let raw = packet.encode().expect("encode SWorldMapSetupInfo");
        assert_eq!(raw.id, ServerPacketId::WorldMapSetup as i16);

        let decoded =
            SWorldMapSetupInfo::decode(&raw.payload).expect("decode SWorldMapSetupInfo");
        assert_eq!(decoded.setup_bytes, packet.setup_bytes);
        assert_eq!(decoded.teleport_to_npc_cost, packet.teleport_to_npc_cost);
    }

    #[test]
    fn world_map_setup_structured_roundtrip() {
        let setup = WorldMapSetupData {
            enabled: true,
            icons: vec![WorldMapIconData {
                image_index: 5,
                title: "Town".to_string(),
                map_index: 1,
            }],
        };

        let pkt = SWorldMapSetupInfo::from_world_map_setup(&setup, 500)
            .expect("from_world_map_setup");
        let raw = pkt.encode().expect("encode SWorldMapSetupInfo");

        let decoded_pkt =
            SWorldMapSetupInfo::decode(&raw.payload).expect("decode SWorldMapSetupInfo");
        assert_eq!(decoded_pkt.teleport_to_npc_cost, 500);
        let decoded_setup = decoded_pkt
            .decode_world_map_setup()
            .expect("decode_world_map_setup");
        assert_eq!(decoded_setup, setup);
    }

    #[test]
    fn search_map_result_roundtrip() {
        let packet = SSearchMapResult {
            map_index: 42,
            npc_index: 123456,
        };

        let raw = packet.encode().expect("encode SSearchMapResult");
        assert_eq!(raw.id, ServerPacketId::SearchMapResult as i16);

        let decoded = SSearchMapResult::decode(&raw.payload).expect("decode SSearchMapResult");
        assert_eq!(decoded.map_index, packet.map_index);
        assert_eq!(decoded.npc_index, packet.npc_index);
    }
}
