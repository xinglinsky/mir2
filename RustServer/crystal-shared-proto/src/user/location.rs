use std::io::{self, Cursor, Read};

use crate::io::{
    read_i32_le,
    write_i32_le,
};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct SUserLocation {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SUserLocation {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::UserLocation as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SUserLocation {
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SUserDash {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SUserDash {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::UserDash as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SUserDash {
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SUserDashFail {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SUserDashFail {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::UserDashFail as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SUserDashFail {
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SUserBackStep {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SUserBackStep {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::UserBackStep as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SUserBackStep {
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SUserDashAttack {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SUserDashAttack {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::UserDashAttack as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SUserDashAttack {
            location_x,
            location_y,
            direction,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SUserAttackMove {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
}

impl SUserAttackMove {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        buf.push(self.direction);
        Ok(RawPacket {
            id: ServerPacketId::UserAttackMove as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        Ok(SUserAttackMove {
            location_x,
            location_y,
            direction,
        })
    }
}
