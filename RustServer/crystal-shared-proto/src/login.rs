// Login-related protocol messages compatible with the existing C# Crystal implementation.
// This module focuses on the minimal set needed for client/server login and start-game/character management flow.

use std::io::{self, Cursor, Read};

use crate::io::{read_i32_le, read_string, write_i32_le, write_string};
use crate::packet::RawPacket;

#[repr(i16)]
#[derive(Copy, Clone, Debug)]
pub enum ClientPacketId {
    ClientVersion = 0,
    Disconnect = 1,
    KeepAlive = 2,
    NewAccount = 3,
    ChangePassword = 4,
    Login = 5,
    NewCharacter = 6,
    DeleteCharacter = 7,
    StartGame = 8,
    LogOut = 9,
}

impl ClientPacketId {
    pub fn from_i16(id: i16) -> Option<Self> {
        match id {
            0 => Some(ClientPacketId::ClientVersion),
            1 => Some(ClientPacketId::Disconnect),
            2 => Some(ClientPacketId::KeepAlive),
            3 => Some(ClientPacketId::NewAccount),
            4 => Some(ClientPacketId::ChangePassword),
            5 => Some(ClientPacketId::Login),
            6 => Some(ClientPacketId::NewCharacter),
            7 => Some(ClientPacketId::DeleteCharacter),
            8 => Some(ClientPacketId::StartGame),
            9 => Some(ClientPacketId::LogOut),
            _ => None,
        }
    }
}

#[repr(i16)]
#[derive(Copy, Clone, Debug)]
pub enum ServerPacketId {
    Connected = 0,
    ClientVersion = 1,
    Disconnect = 2,
    KeepAlive = 3,
    NewAccount = 4,
    ChangePassword = 5,
    ChangePasswordBanned = 6,
    Login = 7,
    LoginBanned = 8,
    LoginSuccess = 9,
    NewCharacter = 10,
    NewCharacterSuccess = 11,
    DeleteCharacter = 12,
    DeleteCharacterSuccess = 13,
    StartGame = 14,
    StartGameBanned = 15,
    StartGameDelay = 16,
    MapInformation = 17,
    NewMapInfo = 18,
    WorldMapSetup = 19,
    SearchMapResult = 20,
    UserInformation = 21,
    UserSlotsRefresh = 22,
    UserLocation = 23,
    ObjectPlayer = 24,
    ObjectHero = 25,
    ObjectRemove = 26,
    ObjectTurn = 27,
    ObjectWalk = 28,
    ObjectRun = 29,
    Chat = 30,
    ObjectChat = 31,
}

// ===== Client -> Server =====

#[derive(Clone, Debug)]
pub struct CClientVersion {
    pub version_hash: Vec<u8>,
}

impl CClientVersion {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        let len: i32 = self
            .version_hash
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "VersionHash too long"))?;
        write_i32_le(&mut buf, len)?;
        buf.extend_from_slice(&self.version_hash);
        Ok(RawPacket {
            id: ClientPacketId::ClientVersion as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let len = read_i32_le(&mut c)?;
        if len < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative VersionHash length",
            ));
        }
        let len = len as usize;
        let mut buf = vec![0u8; len];
        c.read_exact(&mut buf)?;
        Ok(CClientVersion { version_hash: buf })
    }
}

#[derive(Clone, Debug)]
pub struct CNewAccount {
    pub account_id: String,
    pub password: String,
    pub birth_date_binary: i64,
    pub user_name: String,
    pub secret_question: String,
    pub secret_answer: String,
    pub email_address: String,
}

impl CNewAccount {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.account_id)?;
        write_string(&mut buf, &self.password)?;
        crate::io::write_i64_le(&mut buf, self.birth_date_binary)?;
        write_string(&mut buf, &self.user_name)?;
        write_string(&mut buf, &self.secret_question)?;
        write_string(&mut buf, &self.secret_answer)?;
        write_string(&mut buf, &self.email_address)?;
        Ok(RawPacket {
            id: ClientPacketId::NewAccount as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let account_id = read_string(&mut c)?;
        let password = read_string(&mut c)?;
        let birth_date_binary = crate::io::read_i64_le(&mut c)?;
        let user_name = read_string(&mut c)?;
        let secret_question = read_string(&mut c)?;
        let secret_answer = read_string(&mut c)?;
        let email_address = read_string(&mut c)?;
        Ok(CNewAccount {
            account_id,
            password,
            birth_date_binary,
            user_name,
            secret_question,
            secret_answer,
            email_address,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CChangePassword {
    pub account_id: String,
    pub current_password: String,
    pub new_password: String,
}

impl CChangePassword {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.account_id)?;
        write_string(&mut buf, &self.current_password)?;
        write_string(&mut buf, &self.new_password)?;
        Ok(RawPacket {
            id: ClientPacketId::ChangePassword as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let account_id = read_string(&mut c)?;
        let current_password = read_string(&mut c)?;
        let new_password = read_string(&mut c)?;
        Ok(CChangePassword {
            account_id,
            current_password,
            new_password,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CLogin {
    pub account_id: String,
    pub password: String,
}

impl CLogin {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.account_id)?;
        write_string(&mut buf, &self.password)?;
        Ok(RawPacket {
            id: ClientPacketId::Login as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let account_id = read_string(&mut c)?;
        let password = read_string(&mut c)?;
        Ok(CLogin { account_id, password })
    }
}

#[derive(Clone, Debug)]
pub struct CNewCharacter {
    pub name: String,
    pub gender: u8,
    pub class: u8,
}

impl CNewCharacter {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        buf.push(self.gender);
        buf.push(self.class);
        Ok(RawPacket {
            id: ClientPacketId::NewCharacter as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        let mut b = [0u8; 1];
        c.read_exact(&mut b)?;
        let gender = b[0];
        c.read_exact(&mut b)?;
        let class = b[0];
        Ok(CNewCharacter { name, gender, class })
    }
}

#[derive(Clone, Debug)]
pub struct CDeleteCharacter {
    pub character_index: i32,
}

impl CDeleteCharacter {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.character_index)?;
        Ok(RawPacket {
            id: ClientPacketId::DeleteCharacter as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let character_index = read_i32_le(&mut c)?;
        Ok(CDeleteCharacter { character_index })
    }
}

#[derive(Clone, Debug)]
pub struct CLogOut;

impl CLogOut {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::LogOut as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CLogOut payload must be empty",
            ));
        }
        Ok(CLogOut)
    }
}

#[derive(Clone, Debug)]
pub struct CStartGame {
    pub character_index: i32,
}

impl CStartGame {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.character_index)?;
        Ok(RawPacket {
            id: ClientPacketId::StartGame as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let character_index = read_i32_le(&mut c)?;
        Ok(CStartGame { character_index })
    }
}

// ===== Server -> Client =====
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
