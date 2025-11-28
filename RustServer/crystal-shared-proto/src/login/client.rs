use std::io::{self, Cursor, Read};

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
use crate::packet::RawPacket;

use super::ClientPacketId;

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
pub struct CDisconnect;

impl CDisconnect {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::Disconnect as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CDisconnect payload must be empty",
            ));
        }
        Ok(CDisconnect)
    }
}

#[derive(Clone, Debug)]
pub struct CKeepAlive {
    pub time: i64,
}

impl CKeepAlive {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        crate::io::write_i64_le(&mut buf, self.time)?;
        Ok(RawPacket {
            id: ClientPacketId::KeepAlive as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CKeepAlive payload must be exactly 8 bytes",
            ));
        }
        let mut c = Cursor::new(payload);
        let time = crate::io::read_i64_le(&mut c)?;
        Ok(CKeepAlive { time })
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
pub struct CTurn {
    pub direction: u8,
}

impl CTurn {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.direction);
        RawPacket {
            id: ClientPacketId::Turn as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CTurn payload must be exactly 1 byte",
            ));
        }
        Ok(CTurn {
            direction: payload[0],
        })
    }
}

#[derive(Clone, Debug)]
pub struct CWalk {
    pub direction: u8,
}

impl CWalk {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.direction);
        RawPacket {
            id: ClientPacketId::Walk as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CWalk payload must be exactly 1 byte",
            ));
        }
        Ok(CWalk {
            direction: payload[0],
        })
    }
}

#[derive(Clone, Debug)]
pub struct CRun {
    pub direction: u8,
}

impl CRun {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.direction);
        RawPacket {
            id: ClientPacketId::Run as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CRun payload must be exactly 1 byte",
            ));
        }
        Ok(CRun {
            direction: payload[0],
        })
    }
}
#[derive(Clone, Debug)]
pub struct CAttack {
    pub direction: u8,
    pub spell: u8,
}

impl CAttack {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(2);
        buf.push(self.direction);
        buf.push(self.spell);
        RawPacket {
            id: ClientPacketId::Attack as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CAttack payload must be exactly 2 bytes",
            ));
        }
        Ok(CAttack {
            direction: payload[0],
            spell: payload[1],
        })
    }
}

#[derive(Clone, Debug)]
pub struct CTownRevive;

impl CTownRevive {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::TownRevive as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CTownRevive payload must be empty",
            ));
        }
        Ok(CTownRevive)
    }
}

#[derive(Clone, Debug)]
pub struct CPickUp;

impl CPickUp {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::PickUp as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CPickUp payload must be empty",
            ));
        }
        Ok(CPickUp)
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

#[derive(Clone, Debug)]
pub struct CRequestMapInfo {
    pub map_index: i32,
}

impl CRequestMapInfo {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.map_index)?;
        Ok(RawPacket {
            id: ClientPacketId::RequestMapInfo as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let map_index = read_i32_le(&mut c)?;
        Ok(CRequestMapInfo { map_index })
    }
}

#[derive(Clone, Debug)]
pub struct CTeleportToNPC {
    pub object_id: u32,
}

impl CTeleportToNPC {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        Ok(RawPacket {
            id: ClientPacketId::TeleportToNPC as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        Ok(CTeleportToNPC { object_id })
    }
}

#[derive(Clone, Debug)]
pub struct CSearchMap {
    pub text: String,
}

impl CSearchMap {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.text)?;
        Ok(RawPacket {
            id: ClientPacketId::SearchMap as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let text = read_string(&mut c)?;
        Ok(CSearchMap { text })
    }
}

#[derive(Clone, Debug)]
pub struct CMagic {
    pub spell: u8,
    pub direction: u8,
    pub target_id: u32,
    pub x: i32,
    pub y: i32,
}

impl CMagic {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::with_capacity(10);
        buf.push(self.spell);
        buf.push(self.direction);
        write_u32_le(&mut buf, self.target_id)?;
        write_i32_le(&mut buf, self.x)?;
        write_i32_le(&mut buf, self.y)?;
        Ok(RawPacket {
            id: ClientPacketId::Magic as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() < 10 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CMagic payload too short",
            ));
        }
        let mut c = Cursor::new(payload);
        let spell = {
            let mut b = [0u8; 1];
            c.read_exact(&mut b)?;
            b[0]
        };
        let direction = {
            let mut b = [0u8; 1];
            c.read_exact(&mut b)?;
            b[0]
        };
        let target_id = read_u32_le(&mut c)?;
        let x = read_i32_le(&mut c)?;
        let y = read_i32_le(&mut c)?;
        Ok(CMagic {
            spell,
            direction,
            target_id,
            x,
            y,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CMagicKey {
    pub spell: u8,
    pub key: u8,
    pub old_key: u8,
}

impl CMagicKey {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(3);
        buf.push(self.spell);
        buf.push(self.key);
        buf.push(self.old_key);
        RawPacket {
            id: ClientPacketId::MagicKey as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 3 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CMagicKey payload must be exactly 3 bytes",
            ));
        }
        Ok(CMagicKey {
            spell: payload[0],
            key: payload[1],
            old_key: payload[2],
        })
    }
}

#[derive(Clone, Debug)]
pub struct CSwitchGroup {
    pub allow_group: bool,
}

impl CSwitchGroup {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.allow_group)?;
        Ok(RawPacket {
            id: ClientPacketId::SwitchGroup as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let allow_group = read_bool(&mut c)?;
        Ok(CSwitchGroup { allow_group })
    }
}

#[derive(Clone, Debug)]
pub struct CAddMember {
    pub name: String,
}

impl CAddMember {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ClientPacketId::AddMember as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        Ok(CAddMember { name })
    }
}

#[derive(Clone, Debug)]
pub struct CDelMember {
    pub name: String,
}

impl CDelMember {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ClientPacketId::DellMember as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        Ok(CDelMember { name })
    }
}

#[derive(Clone, Debug)]
pub struct CGroupInvite {
    pub accept_invite: bool,
}

impl CGroupInvite {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.accept_invite)?;
        Ok(RawPacket {
            id: ClientPacketId::GroupInvite as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let accept_invite = read_bool(&mut c)?;
        Ok(CGroupInvite { accept_invite })
    }
}

#[derive(Clone, Debug)]
pub struct CTradeRequest;

impl CTradeRequest {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::TradeRequest as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CTradeRequest payload must be empty",
            ));
        }
        Ok(CTradeRequest)
    }
}

#[derive(Clone, Debug)]
pub struct CTradeReply {
    pub accept: bool,
}

impl CTradeReply {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.accept)?;
        Ok(RawPacket {
            id: ClientPacketId::TradeReply as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let accept = read_bool(&mut c)?;
        Ok(CTradeReply { accept })
    }
}

#[derive(Clone, Debug)]
pub struct CTradeGold {
    pub amount: u32,
}

impl CTradeGold {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.amount)?;
        Ok(RawPacket {
            id: ClientPacketId::TradeGold as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let amount = read_u32_le(&mut c)?;
        Ok(CTradeGold { amount })
    }
}

#[derive(Clone, Debug)]
pub struct CTradeConfirm {
    pub locked: bool,
}

impl CTradeConfirm {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.locked)?;
        Ok(RawPacket {
            id: ClientPacketId::TradeConfirm as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let locked = read_bool(&mut c)?;
        Ok(CTradeConfirm { locked })
    }
}

#[derive(Clone, Debug)]
pub struct CTradeCancel;

impl CTradeCancel {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::TradeCancel as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CTradeCancel payload must be empty",
            ));
        }
        Ok(CTradeCancel)
    }
}
