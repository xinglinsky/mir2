// Map-related packets (e.g. MapInformation) compatible with the existing C# Crystal implementation.

use std::io::{self, Cursor, Read};

use crate::io::{
    read_i32_le, read_string, read_u16_le, write_i32_le, write_string, write_u16_le,
};
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
}
