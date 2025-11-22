use std::io::{self, Cursor, Read};

use crate::io::{read_i32_le, read_string, write_i32_le, write_string};
use crate::packet::RawPacket;

use super::ServerPacketId;

#[derive(Clone, Debug)]
pub struct SConnected;

impl SConnected {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::Connected as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SConnected payload must be empty",
            ));
        }
        Ok(SConnected)
    }
}

#[derive(Clone, Debug)]
pub struct SDisconnect {
    pub reason: u8,
}

impl SDisconnect {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.reason);
        RawPacket {
            id: ServerPacketId::Disconnect as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SDisconnect payload must be exactly 1 byte",
            ));
        }
        Ok(SDisconnect {
            reason: payload[0],
        })
    }
}

#[derive(Clone, Debug)]
pub struct SKeepAlive {
    pub time: i64,
}

impl SKeepAlive {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        crate::io::write_i64_le(&mut buf, self.time)?;
        Ok(RawPacket {
            id: ServerPacketId::KeepAlive as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SKeepAlive payload must be exactly 8 bytes",
            ));
        }
        let mut c = Cursor::new(payload);
        let time = crate::io::read_i64_le(&mut c)?;
        Ok(SKeepAlive { time })
    }
}

#[derive(Clone, Debug)]
pub struct SClientVersion {
    pub result: u8,
}

impl SClientVersion {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.result);
        RawPacket {
            id: ServerPacketId::ClientVersion as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SClientVersion payload must be exactly 1 byte",
            ));
        }
        Ok(SClientVersion {
            result: payload[0],
        })
    }
}

#[derive(Clone, Debug)]
pub struct SNewAccount {
    pub result: u8,
}

impl SNewAccount {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.result);
        RawPacket {
            id: ServerPacketId::NewAccount as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SNewAccount payload must be exactly 1 byte",
            ));
        }
        Ok(SNewAccount { result: payload[0] })
    }
}

#[derive(Clone, Debug)]
pub struct SChangePassword {
    pub result: u8,
}

impl SChangePassword {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.result);
        RawPacket {
            id: ServerPacketId::ChangePassword as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SChangePassword payload must be exactly 1 byte",
            ));
        }
        Ok(SChangePassword { result: payload[0] })
    }
}

#[derive(Clone, Debug)]
pub struct SLogin {
    pub result: u8,
}

impl SLogin {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.result);
        RawPacket {
            id: ServerPacketId::Login as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SLogin payload must be exactly 1 byte",
            ));
        }
        Ok(SLogin { result: payload[0] })
    }
}

#[derive(Clone, Debug)]
pub struct SLoginBanned {
    pub reason: String,
    pub expiry_date_binary: i64,
}

impl SLoginBanned {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.reason)?;
        crate::io::write_i64_le(&mut buf, self.expiry_date_binary)?;
        Ok(RawPacket {
            id: ServerPacketId::LoginBanned as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let reason = read_string(&mut c)?;
        let expiry_date_binary = crate::io::read_i64_le(&mut c)?;
        Ok(SLoginBanned {
            reason,
            expiry_date_binary,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SChangePasswordBanned {
    pub reason: String,
    pub expiry_date_binary: i64,
}

impl SChangePasswordBanned {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.reason)?;
        crate::io::write_i64_le(&mut buf, self.expiry_date_binary)?;
        Ok(RawPacket {
            id: ServerPacketId::ChangePasswordBanned as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let reason = read_string(&mut c)?;
        let expiry_date_binary = crate::io::read_i64_le(&mut c)?;
        Ok(SChangePasswordBanned {
            reason,
            expiry_date_binary,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SNewCharacter {
    pub result: u8,
}

impl SNewCharacter {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.result);
        RawPacket {
            id: ServerPacketId::NewCharacter as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SNewCharacter payload must be exactly 1 byte",
            ));
        }
        Ok(SNewCharacter { result: payload[0] })
    }
}

#[derive(Clone, Debug)]
pub struct SDeleteCharacter {
    pub result: u8,
}

impl SDeleteCharacter {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.result);
        RawPacket {
            id: ServerPacketId::DeleteCharacter as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SDeleteCharacter payload must be exactly 1 byte",
            ));
        }
        Ok(SDeleteCharacter { result: payload[0] })
    }
}

#[derive(Clone, Debug)]
pub struct SDeleteCharacterSuccess {
    pub character_index: i32,
}

impl SDeleteCharacterSuccess {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.character_index)?;
        Ok(RawPacket {
            id: ServerPacketId::DeleteCharacterSuccess as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let character_index = read_i32_le(&mut c)?;
        Ok(SDeleteCharacterSuccess { character_index })
    }
}

#[derive(Clone, Debug)]
pub struct SStartGame {
    pub result: u8,
    pub resolution: i32,
}

impl SStartGame {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.result);
        write_i32_le(&mut buf, self.resolution)?;
        Ok(RawPacket {
            id: ServerPacketId::StartGame as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut result_buf = [0u8; 1];
        c.read_exact(&mut result_buf)?;
        let resolution = read_i32_le(&mut c)?;
        Ok(SStartGame {
            result: result_buf[0],
            resolution,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SStartGameBanned {
    pub reason: String,
    pub expiry_date_binary: i64,
}

impl SStartGameBanned {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.reason)?;
        crate::io::write_i64_le(&mut buf, self.expiry_date_binary)?;
        Ok(RawPacket {
            id: ServerPacketId::StartGameBanned as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let reason = read_string(&mut c)?;
        let expiry_date_binary = crate::io::read_i64_le(&mut c)?;
        Ok(SStartGameBanned {
            reason,
            expiry_date_binary,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SStartGameDelay {
    pub milliseconds: i64,
}

impl SStartGameDelay {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        crate::io::write_i64_le(&mut buf, self.milliseconds)?;
        Ok(RawPacket {
            id: ServerPacketId::StartGameDelay as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let milliseconds = crate::io::read_i64_le(&mut c)?;
        Ok(SStartGameDelay { milliseconds })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::login::*;

    #[test]
    fn login_roundtrip() {
        let login = CLogin {
            account_id: "test_account".to_string(),
            password: "secret".to_string(),
        };

        let raw = login.encode().expect("encode CLogin");
        assert_eq!(raw.id, ClientPacketId::Login as i16);

        let decoded = CLogin::decode(&raw.payload).expect("decode CLogin");
        assert_eq!(decoded.account_id, login.account_id);
        assert_eq!(decoded.password, login.password);
    }

    #[test]
    fn start_game_roundtrip() {
        let start = CStartGame { character_index: 42 };
        let raw = start.encode().expect("encode CStartGame");
        assert_eq!(raw.id, ClientPacketId::StartGame as i16);

        let decoded = CStartGame::decode(&raw.payload).expect("decode CStartGame");
        assert_eq!(decoded.character_index, start.character_index);
    }

    #[test]
    fn new_character_roundtrip() {
        let c = CNewCharacter {
            name: "TestChar".to_string(),
            gender: 1,
            class: 2,
        };

        let raw = c.encode().expect("encode CNewCharacter");
        assert_eq!(raw.id, ClientPacketId::NewCharacter as i16);

        let decoded = CNewCharacter::decode(&raw.payload).expect("decode CNewCharacter");
        assert_eq!(decoded.name, c.name);
        assert_eq!(decoded.gender, c.gender);
        assert_eq!(decoded.class, c.class);
    }

    #[test]
    fn delete_character_roundtrip() {
        let c = CDeleteCharacter { character_index: 3 };
        let raw = c.encode().expect("encode CDeleteCharacter");
        assert_eq!(raw.id, ClientPacketId::DeleteCharacter as i16);

        let decoded = CDeleteCharacter::decode(&raw.payload).expect("decode CDeleteCharacter");
        assert_eq!(decoded.character_index, c.character_index);
    }

    #[test]
    fn attack_roundtrip() {
        let c = CAttack {
            direction: 3,
            spell: 5,
        };

        let raw = c.encode();
        assert_eq!(raw.id, ClientPacketId::Attack as i16);

        let decoded = CAttack::decode(&raw.payload).expect("decode CAttack");
        assert_eq!(decoded.direction, c.direction);
        assert_eq!(decoded.spell, c.spell);
    }

    #[test]
    fn server_login_roundtrip() {
        let resp = SLogin { result: 3 };
        let raw = resp.encode();
        assert_eq!(raw.id, ServerPacketId::Login as i16);

        let decoded = SLogin::decode(&raw.payload).expect("decode SLogin");
        assert_eq!(decoded.result, resp.result);
    }

    #[test]
    fn server_start_game_roundtrip() {
        let resp = SStartGame {
            result: 4,
            resolution: 1024,
        };
        let raw = resp.encode().expect("encode SStartGame");
        assert_eq!(raw.id, ServerPacketId::StartGame as i16);

        let decoded = SStartGame::decode(&raw.payload).expect("decode SStartGame");
        assert_eq!(decoded.result, resp.result);
        assert_eq!(decoded.resolution, resp.resolution);
    }

    #[test]
    fn server_new_character_roundtrip() {
        let resp = SNewCharacter { result: 10 };
        let raw = resp.encode();
        assert_eq!(raw.id, ServerPacketId::NewCharacter as i16);

        let decoded = SNewCharacter::decode(&raw.payload).expect("decode SNewCharacter");
        assert_eq!(decoded.result, resp.result);
    }

    #[test]
    fn server_delete_character_roundtrip() {
        let resp = SDeleteCharacter { result: 1 };
        let raw = resp.encode();
        assert_eq!(raw.id, ServerPacketId::DeleteCharacter as i16);

        let decoded = SDeleteCharacter::decode(&raw.payload).expect("decode SDeleteCharacter");
        assert_eq!(decoded.result, resp.result);
    }

    #[test]
    fn server_delete_character_success_roundtrip() {
        let resp = SDeleteCharacterSuccess { character_index: 2 };
        let raw = resp
            .encode()
            .expect("encode SDeleteCharacterSuccess");
        assert_eq!(raw.id, ServerPacketId::DeleteCharacterSuccess as i16);

        let decoded = SDeleteCharacterSuccess::decode(&raw.payload)
            .expect("decode SDeleteCharacterSuccess");
        assert_eq!(decoded.character_index, resp.character_index);
    }

    #[test]
    fn server_start_game_banned_roundtrip() {
        let resp = SStartGameBanned {
            reason: "ban".to_string(),
            expiry_date_binary: 123456,
        };
        let raw = resp
            .encode()
            .expect("encode SStartGameBanned");
        assert_eq!(raw.id, ServerPacketId::StartGameBanned as i16);

        let decoded = SStartGameBanned::decode(&raw.payload)
            .expect("decode SStartGameBanned");
        assert_eq!(decoded.reason, resp.reason);
        assert_eq!(decoded.expiry_date_binary, resp.expiry_date_binary);
    }

    #[test]
    fn server_start_game_delay_roundtrip() {
        let resp = SStartGameDelay { milliseconds: 5000 };
        let raw = resp
            .encode()
            .expect("encode SStartGameDelay");
        assert_eq!(raw.id, ServerPacketId::StartGameDelay as i16);

        let decoded = SStartGameDelay::decode(&raw.payload)
            .expect("decode SStartGameDelay");
        assert_eq!(decoded.milliseconds, resp.milliseconds);
    }
}
