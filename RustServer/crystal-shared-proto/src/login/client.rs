use std::io::{self, Cursor, Read};

use crate::io::{
    read_bool,
    read_i8,
    read_i16_le,
    read_i32_le,
    read_string,
    read_u32_le,
    read_u64_le,
    write_bool,
    write_i8,
    write_i16_le,
    write_i32_le,
    write_string,
    write_u32_le,
    write_u64_le,
};
use crate::packet::RawPacket;

use super::ClientPacketId;

#[derive(Clone, Debug)]
pub struct CClientVersion {
    pub version_hash: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct CChatItem {
    pub unique_id: u64,
    pub title: String,
    pub grid: u8,
}

impl CChatItem {
    pub fn encode(&self, buf: &mut Vec<u8>) -> io::Result<()> {
        write_u64_le(buf, self.unique_id)?;
        write_string(buf, &self.title)?;
        buf.push(self.grid);
        Ok(())
    }

    pub fn decode_from_cursor(c: &mut Cursor<&[u8]>) -> io::Result<Self> {
        let unique_id = read_u64_le(c)?;
        let title = read_string(c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        Ok(CChatItem {
            unique_id,
            title,
            grid: one[0],
        })
    }
}

#[derive(Clone, Debug)]
pub struct CChangeTrade {
    pub allow_trade: bool,
}

impl CChangeTrade {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.allow_trade)?;
        Ok(RawPacket {
            id: ClientPacketId::ChangeTrade as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let allow_trade = read_bool(&mut c)?;
        Ok(CChangeTrade { allow_trade })
    }
}

#[derive(Clone, Debug)]
pub struct CRangeAttack {
    pub direction: u8,
    pub x: i32,
    pub y: i32,
    pub target_id: u32,
    pub target_x: i32,
    pub target_y: i32,
}

impl CRangeAttack {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.direction);
        write_i32_le(&mut buf, self.x)?;
        write_i32_le(&mut buf, self.y)?;
        write_u32_le(&mut buf, self.target_id)?;
        write_i32_le(&mut buf, self.target_x)?;
        write_i32_le(&mut buf, self.target_y)?;
        Ok(RawPacket {
            id: ClientPacketId::RangeAttack as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let direction = one[0];
        let x = read_i32_le(&mut c)?;
        let y = read_i32_le(&mut c)?;
        let target_id = read_u32_le(&mut c)?;
        let target_x = read_i32_le(&mut c)?;
        let target_y = read_i32_le(&mut c)?;
        Ok(CRangeAttack {
            direction,
            x,
            y,
            target_id,
            target_x,
            target_y,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CSpellToggle {
    pub spell: u8,
    /// Mirrors C# SpellToggleState (sbyte): None=-1, False=0, True=1.
    pub can_use_state: i8,
}

impl CSpellToggle {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.spell);
        write_i8(&mut buf, self.can_use_state)?;
        Ok(RawPacket {
            id: ClientPacketId::SpellToggle as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let spell = one[0];
        let can_use_state = read_i8(&mut c)?;
        Ok(CSpellToggle {
            spell,
            can_use_state,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CHarvest {
    pub direction: u8,
}

impl CHarvest {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.direction);
        Ok(RawPacket {
            id: ClientPacketId::Harvest as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        Ok(CHarvest {
            direction: one[0],
        })
    }
}

#[derive(Clone, Debug)]
pub struct CFishingCast {
    pub cast_out: bool,
}

impl CFishingCast {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.cast_out)?;
        Ok(RawPacket {
            id: ClientPacketId::FishingCast as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let cast_out = read_bool(&mut c)?;
        Ok(CFishingCast { cast_out })
    }
}

#[derive(Clone, Debug)]
pub struct CFishingChangeAutocast {
    pub auto_cast: bool,
}

impl CFishingChangeAutocast {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.auto_cast)?;
        Ok(RawPacket {
            id: ClientPacketId::FishingChangeAutocast as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let auto_cast = read_bool(&mut c)?;
        Ok(CFishingChangeAutocast { auto_cast })
    }
}

#[derive(Clone, Debug)]
pub struct CAwakeningNeedMaterials {
    pub unique_id: u64,
    pub awakening_type: u8,
}

impl CAwakeningNeedMaterials {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        buf.push(self.awakening_type);
        Ok(RawPacket {
            id: ClientPacketId::AwakeningNeedMaterials as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        Ok(CAwakeningNeedMaterials {
            unique_id,
            awakening_type: one[0],
        })
    }
}

#[derive(Clone, Debug)]
pub struct CAwakeningLockedItem {
    pub unique_id: u64,
    pub locked: bool,
}

impl CAwakeningLockedItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        write_bool(&mut buf, self.locked)?;
        Ok(RawPacket {
            id: ClientPacketId::AwakeningLockedItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        let locked = read_bool(&mut c)?;
        Ok(CAwakeningLockedItem { unique_id, locked })
    }
}

#[derive(Clone, Debug)]
pub struct CAwakening {
    pub unique_id: u64,
    pub awakening_type: u8,
    pub position_idx: u32,
}

impl CAwakening {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        buf.push(self.awakening_type);
        write_u32_le(&mut buf, self.position_idx)?;
        Ok(RawPacket {
            id: ClientPacketId::Awakening as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let awakening_type = one[0];
        let position_idx = read_u32_le(&mut c)?;
        Ok(CAwakening {
            unique_id,
            awakening_type,
            position_idx,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CDisassembleItem {
    pub unique_id: u64,
}

impl CDisassembleItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        Ok(RawPacket {
            id: ClientPacketId::DisassembleItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        Ok(CDisassembleItem { unique_id })
    }
}

#[derive(Clone, Debug)]
pub struct CDowngradeAwakening {
    pub unique_id: u64,
}

impl CDowngradeAwakening {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        Ok(RawPacket {
            id: ClientPacketId::DowngradeAwakening as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        Ok(CDowngradeAwakening { unique_id })
    }
}

#[derive(Clone, Debug)]
pub struct CResetAddedItem {
    pub unique_id: u64,
}

impl CResetAddedItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        Ok(RawPacket {
            id: ClientPacketId::ResetAddedItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        Ok(CResetAddedItem { unique_id })
    }
}

#[derive(Clone, Debug)]
pub struct CRequestIntelligentCreatureUpdates {
    pub update: bool,
}

impl CRequestIntelligentCreatureUpdates {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.update)?;
        Ok(RawPacket {
            id: ClientPacketId::RequestIntelligentCreatureUpdates as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let update = read_bool(&mut c)?;
        Ok(CRequestIntelligentCreatureUpdates { update })
    }
}

#[derive(Clone, Debug)]
pub struct CUpdateIntelligentCreature {
    pub creature_bytes: Vec<u8>,
    pub summon_me: bool,
    pub unsummon_me: bool,
    pub release_me: bool,
}

impl CUpdateIntelligentCreature {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.creature_bytes);
        write_bool(&mut buf, self.summon_me)?;
        write_bool(&mut buf, self.unsummon_me)?;
        write_bool(&mut buf, self.release_me)?;
        Ok(RawPacket {
            id: ClientPacketId::UpdateIntelligentCreature as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        // ClientIntelligentCreature is a complex structure, we'll need to parse it
        // For now, we'll read it as bytes and parse later
        // The structure is: Creature (variable length), SummonMe (bool), UnSummonMe (bool), ReleaseMe (bool)
        let mut c = Cursor::new(payload);
        
        // Read until we have 3 bytes left (for the 3 booleans)
        // This is a simplified approach - in reality we'd need to parse ClientIntelligentCreature properly
        let remaining = payload.len();
        if remaining < 3 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CUpdateIntelligentCreature payload too short",
            ));
        }
        
        let creature_bytes_len = remaining - 3;
        let mut creature_bytes = vec![0u8; creature_bytes_len];
        c.read_exact(&mut creature_bytes)?;
        
        let summon_me = read_bool(&mut c)?;
        let unsummon_me = read_bool(&mut c)?;
        let release_me = read_bool(&mut c)?;
        
        Ok(CUpdateIntelligentCreature {
            creature_bytes,
            summon_me,
            unsummon_me,
            release_me,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CIntelligentCreaturePickup {
    pub mouse_mode: bool,
    pub location_x: i32,
    pub location_y: i32,
}

impl CIntelligentCreaturePickup {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.mouse_mode)?;
        write_i32_le(&mut buf, self.location_x)?;
        write_i32_le(&mut buf, self.location_y)?;
        Ok(RawPacket {
            id: ClientPacketId::IntelligentCreaturePickup as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mouse_mode = read_bool(&mut c)?;
        let location_x = read_i32_le(&mut c)?;
        let location_y = read_i32_le(&mut c)?;
        Ok(CIntelligentCreaturePickup {
            mouse_mode,
            location_x,
            location_y,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CMarriageRequest;

impl CMarriageRequest {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::MarriageRequest as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CMarriageRequest payload must be empty",
            ));
        }
        Ok(CMarriageRequest)
    }
}

#[derive(Clone, Debug)]
pub struct CMarriageReply {
    pub accept_invite: bool,
}

impl CMarriageReply {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.accept_invite)?;
        Ok(RawPacket {
            id: ClientPacketId::MarriageReply as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let accept_invite = read_bool(&mut c)?;
        Ok(CMarriageReply { accept_invite })
    }
}

#[derive(Clone, Debug)]
pub struct CChangeMarriage;

impl CChangeMarriage {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::ChangeMarriage as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CChangeMarriage payload must be empty",
            ));
        }
        Ok(CChangeMarriage)
    }
}

#[derive(Clone, Debug)]
pub struct CDivorceRequest;

impl CDivorceRequest {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::DivorceRequest as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CDivorceRequest payload must be empty",
            ));
        }
        Ok(CDivorceRequest)
    }
}

#[derive(Clone, Debug)]
pub struct CDivorceReply {
    pub accept_invite: bool,
}

impl CDivorceReply {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.accept_invite)?;
        Ok(RawPacket {
            id: ClientPacketId::DivorceReply as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let accept_invite = read_bool(&mut c)?;
        Ok(CDivorceReply { accept_invite })
    }
}

#[derive(Clone, Debug)]
pub struct CAddMentor {
    pub name: String,
}

impl CAddMentor {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ClientPacketId::AddMentor as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        Ok(CAddMentor { name })
    }
}

#[derive(Clone, Debug)]
pub struct CMentorReply {
    pub accept_invite: bool,
}

impl CMentorReply {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_bool(&mut buf, self.accept_invite)?;
        Ok(RawPacket {
            id: ClientPacketId::MentorReply as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let accept_invite = read_bool(&mut c)?;
        Ok(CMentorReply { accept_invite })
    }
}

#[derive(Clone, Debug)]
pub struct CAllowMentor;

impl CAllowMentor {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::AllowMentor as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CAllowMentor payload must be empty",
            ));
        }
        Ok(CAllowMentor)
    }
}

#[derive(Clone, Debug)]
pub struct CCancelMentor;

impl CCancelMentor {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::CancelMentor as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CCancelMentor payload must be empty",
            ));
        }
        Ok(CCancelMentor)
    }
}

#[derive(Clone, Debug)]
pub struct CGuildBuffUpdate {
    pub action: u8,
    pub id: i32,
}

impl CGuildBuffUpdate {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.action);
        write_i32_le(&mut buf, self.id)?;
        Ok(RawPacket {
            id: ClientPacketId::GuildBuffUpdate as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let action = one[0];
        let id = read_i32_le(&mut c)?;
        Ok(CGuildBuffUpdate { action, id })
    }
}

#[derive(Clone, Debug)]
pub struct CNPCConfirmInput {
    pub npc_id: u32,
    pub page_name: String,
    pub value: String,
}

impl CNPCConfirmInput {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.npc_id)?;
        write_string(&mut buf, &self.page_name)?;
        write_string(&mut buf, &self.value)?;
        Ok(RawPacket {
            id: ClientPacketId::NPCConfirmInput as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let npc_id = read_u32_le(&mut c)?;
        let page_name = read_string(&mut c)?;
        let value = read_string(&mut c)?;
        Ok(CNPCConfirmInput {
            npc_id,
            page_name,
            value,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CReportIssue {
    pub image: Vec<u8>,
    pub image_size: i32,
    pub image_chunk: i32,
    pub message: String,
}

impl CReportIssue {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.image.len() as i32)?;
        buf.extend_from_slice(&self.image);
        write_i32_le(&mut buf, self.image_size)?;
        write_i32_le(&mut buf, self.image_chunk)?;
        write_string(&mut buf, &self.message)?;
        Ok(RawPacket {
            id: ClientPacketId::ReportIssue as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let image_len = read_i32_le(&mut c)?;
        let mut image = vec![0u8; image_len as usize];
        c.read_exact(&mut image)?;
        let image_size = read_i32_le(&mut c)?;
        let image_chunk = read_i32_le(&mut c)?;
        let message = read_string(&mut c)?;
        Ok(CReportIssue {
            image,
            image_size,
            image_chunk,
            message,
        })
    }
}

#[derive(Clone, Debug)]
pub struct COpendoor {
    pub door_index: u8,
}

impl COpendoor {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.door_index);
        Ok(RawPacket {
            id: ClientPacketId::Opendoor as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        Ok(COpendoor {
            door_index: one[0],
        })
    }
}

#[derive(Clone, Debug)]
pub struct CGetRentedItems;

impl CGetRentedItems {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::GetRentedItems as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CGetRentedItems payload must be empty",
            ));
        }
        Ok(CGetRentedItems)
    }
}

#[derive(Clone, Debug)]
pub struct CItemRentalRequest;

impl CItemRentalRequest {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::ItemRentalRequest as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CItemRentalRequest payload must be empty",
            ));
        }
        Ok(CItemRentalRequest)
    }
}

#[derive(Clone, Debug)]
pub struct CItemRentalFee {
    pub amount: u32,
}

impl CItemRentalFee {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.amount)?;
        Ok(RawPacket {
            id: ClientPacketId::ItemRentalFee as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let amount = read_u32_le(&mut c)?;
        Ok(CItemRentalFee { amount })
    }
}

#[derive(Clone, Debug)]
pub struct CItemRentalPeriod {
    pub days: u32,
}

impl CItemRentalPeriod {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.days)?;
        Ok(RawPacket {
            id: ClientPacketId::ItemRentalPeriod as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let days = read_u32_le(&mut c)?;
        Ok(CItemRentalPeriod { days })
    }
}

#[derive(Clone, Debug)]
pub struct CDepositRentalItem {
    pub from: i32,
    pub to: i32,
}

impl CDepositRentalItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.from)?;
        write_i32_le(&mut buf, self.to)?;
        Ok(RawPacket {
            id: ClientPacketId::DepositRentalItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let from = read_i32_le(&mut c)?;
        let to = read_i32_le(&mut c)?;
        Ok(CDepositRentalItem { from, to })
    }
}

#[derive(Clone, Debug)]
pub struct CRetrieveRentalItem {
    pub from: i32,
    pub to: i32,
}

impl CRetrieveRentalItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.from)?;
        write_i32_le(&mut buf, self.to)?;
        Ok(RawPacket {
            id: ClientPacketId::RetrieveRentalItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let from = read_i32_le(&mut c)?;
        let to = read_i32_le(&mut c)?;
        Ok(CRetrieveRentalItem { from, to })
    }
}

#[derive(Clone, Debug)]
pub struct CCancelItemRental;

impl CCancelItemRental {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::CancelItemRental as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CCancelItemRental payload must be empty",
            ));
        }
        Ok(CCancelItemRental)
    }
}

#[derive(Clone, Debug)]
pub struct CItemRentalLockFee;

impl CItemRentalLockFee {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::ItemRentalLockFee as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CItemRentalLockFee payload must be empty",
            ));
        }
        Ok(CItemRentalLockFee)
    }
}

#[derive(Clone, Debug)]
pub struct CItemRentalLockItem;

impl CItemRentalLockItem {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::ItemRentalLockItem as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CItemRentalLockItem payload must be empty",
            ));
        }
        Ok(CItemRentalLockItem)
    }
}

#[derive(Clone, Debug)]
pub struct CConfirmItemRental;

impl CConfirmItemRental {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::ConfirmItemRental as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CConfirmItemRental payload must be empty",
            ));
        }
        Ok(CConfirmItemRental)
    }
}

#[derive(Clone, Debug)]
pub struct CGuildTerritoryPage {
    pub page: i32,
}

impl CGuildTerritoryPage {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.page)?;
        Ok(RawPacket {
            id: ClientPacketId::GuildTerritoryPage as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let page = read_i32_le(&mut c)?;
        Ok(CGuildTerritoryPage { page })
    }
}

#[derive(Clone, Debug)]
pub struct CPurchaseGuildTerritory {
    pub owner: String,
}

impl CPurchaseGuildTerritory {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.owner)?;
        Ok(RawPacket {
            id: ClientPacketId::PurchaseGuildTerritory as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let owner = read_string(&mut c)?;
        Ok(CPurchaseGuildTerritory { owner })
    }
}

#[derive(Clone, Debug)]
pub struct CConsignItem {
    pub unique_id: u64,
    pub price: u32,
    pub market_type: u8,
}

impl CConsignItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        write_u32_le(&mut buf, self.price)?;
        buf.push(self.market_type);
        Ok(RawPacket {
            id: ClientPacketId::ConsignItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        let price = read_u32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        Ok(CConsignItem {
            unique_id,
            price,
            market_type: one[0],
        })
    }
}

#[derive(Clone, Debug)]
pub struct CMarketSearch {
    pub match_str: String,
    pub item_type: u8,
    pub usermode: bool,
    pub min_shape: i16,
    pub max_shape: i16,
    pub market_type: u8,
}

impl CMarketSearch {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.match_str)?;
        buf.push(self.item_type);
        write_bool(&mut buf, self.usermode)?;
        write_i16_le(&mut buf, self.min_shape)?;
        write_i16_le(&mut buf, self.max_shape)?;
        buf.push(self.market_type);
        Ok(RawPacket {
            id: ClientPacketId::MarketSearch as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let match_str = read_string(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let item_type = one[0];
        let usermode = read_bool(&mut c)?;
        let min_shape = read_i16_le(&mut c)?;
        let max_shape = read_i16_le(&mut c)?;
        c.read_exact(&mut one)?;
        let market_type = one[0];
        Ok(CMarketSearch {
            match_str,
            item_type,
            usermode,
            min_shape,
            max_shape,
            market_type,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CMarketRefresh;

impl CMarketRefresh {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::MarketRefresh as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CMarketRefresh payload must be empty",
            ));
        }
        Ok(CMarketRefresh)
    }
}

#[derive(Clone, Debug)]
pub struct CMarketPage {
    pub page: i32,
}

impl CMarketPage {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.page)?;
        Ok(RawPacket {
            id: ClientPacketId::MarketPage as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let page = read_i32_le(&mut c)?;
        Ok(CMarketPage { page })
    }
}

#[derive(Clone, Debug)]
pub struct CMarketBuy {
    pub auction_id: u64,
    pub bid_price: u32,
}

impl CMarketBuy {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.auction_id)?;
        write_u32_le(&mut buf, self.bid_price)?;
        Ok(RawPacket {
            id: ClientPacketId::MarketBuy as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let auction_id = read_u64_le(&mut c)?;
        let bid_price = read_u32_le(&mut c)?;
        Ok(CMarketBuy {
            auction_id,
            bid_price,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CMarketSellNow {
    pub auction_id: u64,
}

impl CMarketSellNow {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.auction_id)?;
        Ok(RawPacket {
            id: ClientPacketId::MarketSellNow as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let auction_id = read_u64_le(&mut c)?;
        Ok(CMarketSellNow { auction_id })
    }
}

#[derive(Clone, Debug)]
pub struct CMarketGetBack {
    pub mode: u8,
    pub auction_id: u64,
}

impl CMarketGetBack {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.mode);
        write_u64_le(&mut buf, self.auction_id)?;
        Ok(RawPacket {
            id: ClientPacketId::MarketGetBack as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let mode = one[0];
        let auction_id = read_u64_le(&mut c)?;
        Ok(CMarketGetBack { mode, auction_id })
    }
}

#[derive(Clone, Debug)]
pub struct CChat {
    pub message: String,
    pub linked_items: Vec<CChatItem>,
}

impl CChat {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.message)?;

        let count: i32 = self
            .linked_items
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many linked items"))?;
        write_i32_le(&mut buf, count)?;
        for item in &self.linked_items {
            item.encode(&mut buf)?;
        }

        Ok(RawPacket {
            id: ClientPacketId::Chat as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let message = read_string(&mut c)?;
        let count = read_i32_le(&mut c)?;
        if count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative LinkedItems length",
            ));
        }
        if count > 128 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "LinkedItems length too large",
            ));
        }

        let mut linked_items = Vec::with_capacity(count as usize);
        for _ in 0..count {
            linked_items.push(CChatItem::decode_from_cursor(&mut c)?);
        }
        Ok(CChat { message, linked_items })
    }
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
pub struct CChangeAMode {
    pub mode: u8,
}

impl CChangeAMode {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.mode);
        RawPacket {
            id: ClientPacketId::ChangeAMode as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CChangeAMode payload must be exactly 1 byte",
            ));
        }
        Ok(CChangeAMode { mode: payload[0] })
    }
}

#[derive(Clone, Debug)]
pub struct CChangePMode {
    pub mode: u8,
}

impl CChangePMode {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.mode);
        RawPacket {
            id: ClientPacketId::ChangePMode as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CChangePMode payload must be exactly 1 byte",
            ));
        }
        Ok(CChangePMode { mode: payload[0] })
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
pub struct CInspect {
    pub object_id: u32,
    pub ranking: bool,
    pub hero: bool,
}

impl CInspect {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.object_id)?;
        write_bool(&mut buf, self.ranking)?;
        write_bool(&mut buf, self.hero)?;
        Ok(RawPacket {
            id: ClientPacketId::Inspect as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
        let ranking = read_bool(&mut c)?;
        let hero = read_bool(&mut c)?;
        Ok(CInspect {
            object_id,
            ranking,
            hero,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CObserve {
    pub name: String,
}

impl CObserve {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ClientPacketId::Observe as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        Ok(CObserve { name })
    }
}

#[derive(Clone, Debug)]
pub struct CRequestUserName {
    pub user_id: u32,
}

impl CRequestUserName {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.user_id)?;
        Ok(RawPacket {
            id: ClientPacketId::RequestUserName as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let user_id = read_u32_le(&mut c)?;
        Ok(CRequestUserName { user_id })
    }
}

#[derive(Clone, Debug)]
pub struct CRequestChatItem {
    pub chat_item_id: u64,
}

impl CRequestChatItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.chat_item_id)?;
        Ok(RawPacket {
            id: ClientPacketId::RequestChatItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let chat_item_id = read_u64_le(&mut c)?;
        Ok(CRequestChatItem { chat_item_id })
    }
}

#[derive(Clone, Debug)]
pub struct CMagic {
    pub object_id: u32,
    pub spell: u8,
    pub direction: u8,
    pub target_id: u32,
    pub x: i32,
    pub y: i32,
    pub spell_target_lock: bool,
}

impl CMagic {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::with_capacity(19);
        write_u32_le(&mut buf, self.object_id)?;
        buf.push(self.spell);
        buf.push(self.direction);
        write_u32_le(&mut buf, self.target_id)?;
        write_i32_le(&mut buf, self.x)?;
        write_i32_le(&mut buf, self.y)?;
        write_bool(&mut buf, self.spell_target_lock)?;
        Ok(RawPacket {
            id: ClientPacketId::Magic as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() < 19 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CMagic payload too short",
            ));
        }
        let mut c = Cursor::new(payload);
        let object_id = read_u32_le(&mut c)?;
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
        let spell_target_lock = read_bool(&mut c)?;
        Ok(CMagic {
            object_id,
            spell,
            direction,
            target_id,
            x,
            y,
            spell_target_lock,
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

#[derive(Clone, Debug)]
pub struct CAcceptQuest {
    pub npc_index: u32,
    pub quest_index: i32,
}

impl CAcceptQuest {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.npc_index)?;
        write_i32_le(&mut buf, self.quest_index)?;
        Ok(RawPacket {
            id: ClientPacketId::AcceptQuest as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let npc_index = read_u32_le(&mut c)?;
        let quest_index = read_i32_le(&mut c)?;
        Ok(CAcceptQuest { npc_index, quest_index })
    }
}

#[derive(Clone, Debug)]
pub struct CFinishQuest {
    pub quest_index: i32,
    pub selected_item_index: i32,
}

impl CFinishQuest {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.quest_index)?;
        write_i32_le(&mut buf, self.selected_item_index)?;
        Ok(RawPacket {
            id: ClientPacketId::FinishQuest as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let quest_index = read_i32_le(&mut c)?;
        let selected_item_index = read_i32_le(&mut c)?;
        Ok(CFinishQuest {
            quest_index,
            selected_item_index,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CAbandonQuest {
    pub quest_index: i32,
}

impl CAbandonQuest {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.quest_index)?;
        Ok(RawPacket {
            id: ClientPacketId::AbandonQuest as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let quest_index = read_i32_le(&mut c)?;
        Ok(CAbandonQuest { quest_index })
    }
}

#[derive(Clone, Debug)]
pub struct CShareQuest {
    pub quest_index: i32,
}

impl CShareQuest {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.quest_index)?;
        Ok(RawPacket {
            id: ClientPacketId::ShareQuest as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let quest_index = read_i32_le(&mut c)?;
        Ok(CShareQuest { quest_index })
    }
}

#[derive(Clone, Debug)]
pub struct CAcceptReincarnation;

impl CAcceptReincarnation {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::AcceptReincarnation as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CAcceptReincarnation payload must be empty",
            ));
        }
        Ok(CAcceptReincarnation)
    }
}

#[derive(Clone, Debug)]
pub struct CCancelReincarnation;

impl CCancelReincarnation {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::CancelReincarnation as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CCancelReincarnation payload must be empty",
            ));
        }
        Ok(CCancelReincarnation)
    }
}

#[derive(Clone, Debug)]
pub struct CGameshopBuy {
    pub g_index: i32,
    pub quantity: u8,
    pub p_type: i32,
}

impl CGameshopBuy {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.g_index)?;
        buf.push(self.quantity);
        write_i32_le(&mut buf, self.p_type)?;
        Ok(RawPacket {
            id: ClientPacketId::GameshopBuy as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let g_index = read_i32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let quantity = one[0];
        let p_type = read_i32_le(&mut c)?;
        Ok(CGameshopBuy {
            g_index,
            quantity,
            p_type,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CSendMail {
    pub name: String,
    pub message: String,
    pub gold: u32,
    pub items_idx: [u64; 5],
    pub stamped: bool,
}

impl CSendMail {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        write_string(&mut buf, &self.message)?;
        write_u32_le(&mut buf, self.gold)?;
        for id in &self.items_idx {
            write_u64_le(&mut buf, *id)?;
        }
        write_bool(&mut buf, self.stamped)?;
        Ok(RawPacket {
            id: ClientPacketId::SendMail as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        let message = read_string(&mut c)?;
        let gold = read_u32_le(&mut c)?;
        let mut items_idx = [0u64; 5];
        for i in 0..5 {
            items_idx[i] = read_u64_le(&mut c)?;
        }
        let stamped = read_bool(&mut c)?;
        Ok(CSendMail {
            name,
            message,
            gold,
            items_idx,
            stamped,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CReadMail {
    pub mail_id: u64,
}

impl CReadMail {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.mail_id)?;
        Ok(RawPacket {
            id: ClientPacketId::ReadMail as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mail_id = read_u64_le(&mut c)?;
        Ok(CReadMail { mail_id })
    }
}

#[derive(Clone, Debug)]
pub struct CDeleteMail {
    pub mail_id: u64,
}

impl CDeleteMail {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.mail_id)?;
        Ok(RawPacket {
            id: ClientPacketId::DeleteMail as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mail_id = read_u64_le(&mut c)?;
        Ok(CDeleteMail { mail_id })
    }
}

#[derive(Clone, Debug)]
pub struct CLockMail {
    pub mail_id: u64,
    pub lock: bool,
}

impl CLockMail {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.mail_id)?;
        write_bool(&mut buf, self.lock)?;
        Ok(RawPacket {
            id: ClientPacketId::LockMail as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mail_id = read_u64_le(&mut c)?;
        let lock = read_bool(&mut c)?;
        Ok(CLockMail { mail_id, lock })
    }
}

#[derive(Clone, Debug)]
pub struct CMailCost {
    pub gold: u32,
    pub items_idx: [u64; 5],
    pub stamped: bool,
}

impl CMailCost {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.gold)?;
        for id in &self.items_idx {
            write_u64_le(&mut buf, *id)?;
        }
        write_bool(&mut buf, self.stamped)?;
        Ok(RawPacket {
            id: ClientPacketId::MailCost as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let gold = read_u32_le(&mut c)?;
        let mut items_idx = [0u64; 5];
        for i in 0..5 {
            items_idx[i] = read_u64_le(&mut c)?;
        }
        let stamped = read_bool(&mut c)?;
        Ok(CMailCost {
            gold,
            items_idx,
            stamped,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CMailLockedItem {
    pub unique_id: u64,
    pub locked: bool,
}

impl CMailLockedItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        write_bool(&mut buf, self.locked)?;
        Ok(RawPacket {
            id: ClientPacketId::MailLockedItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        let locked = read_bool(&mut c)?;
        Ok(CMailLockedItem { unique_id, locked })
    }
}

#[derive(Clone, Debug)]
pub struct CCollectParcel {
    pub mail_id: u64,
}

impl CCollectParcel {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.mail_id)?;
        Ok(RawPacket {
            id: ClientPacketId::CollectParcel as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mail_id = read_u64_le(&mut c)?;
        Ok(CCollectParcel { mail_id })
    }
}

#[derive(Clone, Debug)]
pub struct CAddFriend {
    pub name: String,
    pub blocked: bool,
}

impl CAddFriend {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        write_bool(&mut buf, self.blocked)?;
        Ok(RawPacket {
            id: ClientPacketId::AddFriend as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        let blocked = read_bool(&mut c)?;
        Ok(CAddFriend { name, blocked })
    }
}

#[derive(Clone, Debug)]
pub struct CRemoveFriend {
    pub character_index: i32,
}

impl CRemoveFriend {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.character_index)?;
        Ok(RawPacket {
            id: ClientPacketId::RemoveFriend as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let character_index = read_i32_le(&mut c)?;
        Ok(CRemoveFriend { character_index })
    }
}

#[derive(Clone, Debug)]
pub struct CRefreshFriends;

impl CRefreshFriends {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ClientPacketId::RefreshFriends as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CRefreshFriends payload must be empty",
            ));
        }
        Ok(CRefreshFriends)
    }
}

#[derive(Clone, Debug)]
pub struct CAddMemo {
    pub character_index: i32,
    pub memo: String,
}

impl CAddMemo {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.character_index)?;
        write_string(&mut buf, &self.memo)?;
        Ok(RawPacket {
            id: ClientPacketId::AddMemo as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let character_index = read_i32_le(&mut c)?;
        let memo = read_string(&mut c)?;
        Ok(CAddMemo {
            character_index,
            memo,
        })
    }
}
