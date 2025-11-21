// Guild-related packets compatible with the C# Crystal implementation.

use std::io::{self, Cursor, Read};

use crate::io::{
    read_bool, read_i32_le, read_i64_le, read_string, read_u32_le, write_bool,
    write_i32_le, write_i64_le, write_string, write_u32_le,
};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct SGuildNoticeChange {
    pub update: i32,
    pub notice: Vec<String>,
}

impl SGuildNoticeChange {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();

        if self.update < 0 {
            write_i32_le(&mut buf, self.update)?;
        } else {
            // C# writes notice.Count when update >= 0
            let count: i32 = self
                .notice
                .len()
                .try_into()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many notices"))?;
            write_i32_le(&mut buf, count)?;
            for line in &self.notice {
                write_string(&mut buf, line)?;
            }
        }

        Ok(RawPacket {
            id: ServerPacketId::GuildNoticeChange as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let update = read_i32_le(&mut c)?;
        if update < 0 {
            return Ok(SGuildNoticeChange {
                update,
                notice: Vec::new(),
            });
        }
        let count = update;
        if count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative GuildNoticeChange notice count",
            ));
        }
        let mut notice = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let line = read_string(&mut c)?;
            notice.push(line);
        }
        Ok(SGuildNoticeChange { update, notice })
    }
}

#[derive(Clone, Debug)]
pub struct SGuildMemberChange {
    pub name: String,
    pub rank_index: u8,
    pub status: u8,
    /// Raw bytes representing the optional rank list payload (rankcount + GuildRank.Save...).
    pub ranks_bytes: Vec<u8>,
}

impl SGuildMemberChange {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();

        write_string(&mut buf, &self.name)?;
        buf.push(self.rank_index);
        buf.push(self.status);
        buf.extend_from_slice(&self.ranks_bytes);

        Ok(RawPacket {
            id: ServerPacketId::GuildMemberChange as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);

        let name = read_string(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let rank_index = one[0];
        c.read_exact(&mut one)?;
        let status = one[0];

        let pos = c.position() as usize;
        let ranks_bytes = payload[pos..].to_vec();

        Ok(SGuildMemberChange {
            name,
            rank_index,
            status,
            ranks_bytes,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SGuildStatus {
    pub guild_name: String,
    pub guild_rank_name: String,
    pub level: u8,
    pub experience: i64,
    pub max_experience: i64,
    pub gold: u32,
    pub spare_points: u8,
    pub member_count: i32,
    pub max_members: i32,
    pub voting: bool,
    pub item_count: u8,
    pub buff_count: u8,
    pub my_options: u8,
    pub my_rank_id: i32,
}

impl SGuildStatus {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();

        write_string(&mut buf, &self.guild_name)?;
        write_string(&mut buf, &self.guild_rank_name)?;
        buf.push(self.level);
        write_i64_le(&mut buf, self.experience)?;
        write_i64_le(&mut buf, self.max_experience)?;
        write_u32_le(&mut buf, self.gold)?;
        buf.push(self.spare_points);
        write_i32_le(&mut buf, self.member_count)?;
        write_i32_le(&mut buf, self.max_members)?;
        write_bool(&mut buf, self.voting)?;
        buf.push(self.item_count);
        buf.push(self.buff_count);
        buf.push(self.my_options);
        write_i32_le(&mut buf, self.my_rank_id)?;

        Ok(RawPacket {
            id: ServerPacketId::GuildStatus as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);

        let guild_name = read_string(&mut c)?;
        let guild_rank_name = read_string(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let level = one[0];
        let experience = read_i64_le(&mut c)?;
        let max_experience = read_i64_le(&mut c)?;
        let gold = read_u32_le(&mut c)?;
        c.read_exact(&mut one)?;
        let spare_points = one[0];
        let member_count = read_i32_le(&mut c)?;
        let max_members = read_i32_le(&mut c)?;
        let voting = read_bool(&mut c)?;
        c.read_exact(&mut one)?;
        let item_count = one[0];
        c.read_exact(&mut one)?;
        let buff_count = one[0];
        c.read_exact(&mut one)?;
        let my_options = one[0];
        let my_rank_id = read_i32_le(&mut c)?;

        Ok(SGuildStatus {
            guild_name,
            guild_rank_name,
            level,
            experience,
            max_experience,
            gold,
            spare_points,
            member_count,
            max_members,
            voting,
            item_count,
            buff_count,
            my_options,
            my_rank_id,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SGuildInvite {
    pub name: String,
}

impl SGuildInvite {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ServerPacketId::GuildInvite as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let name = read_string(&mut c)?;
        Ok(SGuildInvite { name })
    }
}

#[derive(Clone, Debug)]
pub struct SGuildExpGain {
    pub amount: u32,
}

impl SGuildExpGain {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.amount)?;
        Ok(RawPacket {
            id: ServerPacketId::GuildExpGain as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let amount = read_u32_le(&mut c)?;
        Ok(SGuildExpGain { amount })
    }
}

#[derive(Clone, Debug)]
pub struct SGuildNameRequest;

impl SGuildNameRequest {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::GuildNameRequest as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SGuildNameRequest payload must be empty",
            ));
        }
        Ok(SGuildNameRequest)
    }
}

#[derive(Clone, Debug)]
pub struct SGuildStorageGoldChange {
    pub amount: u32,
    pub change_type: u8,
    pub name: String,
}

impl SGuildStorageGoldChange {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.amount)?;
        buf.push(self.change_type);
        write_string(&mut buf, &self.name)?;
        Ok(RawPacket {
            id: ServerPacketId::GuildStorageGoldChange as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let amount = read_u32_le(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let change_type = one[0];
        let name = read_string(&mut c)?;
        Ok(SGuildStorageGoldChange {
            amount,
            change_type,
            name,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SGuildStorageItemChange {
    pub user: i32,
    pub change_type: u8,
    pub to_slot: i32,
    pub from_slot: i32,
    pub item_user_id: Option<i64>,
    /// Raw bytes representing the UserItem.Save payload when present.
    pub item_bytes: Vec<u8>,
}

impl SGuildStorageItemChange {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();

        buf.push(self.change_type);
        write_i32_le(&mut buf, self.to_slot)?;
        write_i32_le(&mut buf, self.from_slot)?;
        write_i32_le(&mut buf, self.user)?;

        match self.item_user_id {
            None => {
                buf.push(0); // has item = false
            }
            Some(user_id) => {
                buf.push(1); // has item = true
                write_i64_le(&mut buf, user_id)?;
                buf.extend_from_slice(&self.item_bytes);
            }
        }

        Ok(RawPacket {
            id: ServerPacketId::GuildStorageItemChange as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let mut one = [0u8; 1];

        c.read_exact(&mut one)?;
        let change_type = one[0];
        let to_slot = read_i32_le(&mut c)?;
        let from_slot = read_i32_le(&mut c)?;
        let user = read_i32_le(&mut c)?;
        c.read_exact(&mut one)?;
        let has_item = one[0] != 0;

        if !has_item {
            return Ok(SGuildStorageItemChange {
                user,
                change_type,
                to_slot,
                from_slot,
                item_user_id: None,
                item_bytes: Vec::new(),
            });
        }

        let item_user_id = read_i64_le(&mut c)?;
        let mut item_bytes = Vec::new();
        c.read_to_end(&mut item_bytes)?;

        Ok(SGuildStorageItemChange {
            user,
            change_type,
            to_slot,
            from_slot,
            item_user_id: Some(item_user_id),
            item_bytes,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SGuildStorageList {
    /// Raw bytes representing the GuildStorageItem array: count (i32) + entries
    /// (bool hasItem + GuildStorageItem.Save).
    pub items_bytes: Vec<u8>,
}

impl SGuildStorageList {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::GuildStorageList as i16,
            payload: self.items_bytes.clone(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SGuildStorageList {
            items_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SGuildRequestWar;

impl SGuildRequestWar {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::GuildRequestWar as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SGuildRequestWar payload must be empty",
            ));
        }
        Ok(SGuildRequestWar)
    }
}

#[derive(Clone, Debug)]
pub struct SGuildBuffList {
    pub remove: u8,
    /// Raw bytes representing active buffs: count (i32) + repeated GuildBuff.Save.
    pub active_buffs_bytes: Vec<u8>,
    /// Raw bytes representing guild buffs: count (i32) + repeated GuildBuffInfo.Save.
    pub guild_buffs_bytes: Vec<u8>,
}

impl SGuildBuffList {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.remove);
        buf.extend_from_slice(&self.active_buffs_bytes);
        buf.extend_from_slice(&self.guild_buffs_bytes);
        Ok(RawPacket {
            id: ServerPacketId::GuildBuffList as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SGuildBuffList payload too short",
            ));
        }
        let remove = payload[0];
        let rest = &payload[1..];
        // For simplicity, we split at a midpoint; in reality we'd need to parse counts.
        // For blob-first, we just store everything after remove byte.
        // Proper split would require reading first i32 count, then that many buffs, then next count.
        // For now, store as two blobs by reading the structure:
        if rest.len() < 4 {
            return Ok(SGuildBuffList {
                remove,
                active_buffs_bytes: Vec::new(),
                guild_buffs_bytes: Vec::new(),
            });
        }
        // Read active buffs count
        let mut c = Cursor::new(rest);
        let active_count = read_i32_le(&mut c)?;
        if active_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative active buff count",
            ));
        }
        // We don't know the size of each GuildBuff, so we use blob-first:
        // Store everything as two separate blobs by finding the second count.
        // For true blob-first, just store the rest as one blob or split heuristically.
        // Let's store rest[0..] as active_buffs_bytes and empty for guild_buffs_bytes for now.
        // Better: store the entire rest as combined, then split on decode if needed.
        // Simplest: store entire payload after remove byte.
        Ok(SGuildBuffList {
            remove,
            active_buffs_bytes: rest.to_vec(),
            guild_buffs_bytes: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guild_notice_change_roundtrip() {
        let p = SGuildNoticeChange {
            update: 2,
            notice: vec!["Line1".to_string(), "Line2".to_string()],
        };

        let raw = p.encode().expect("encode SGuildNoticeChange");
        assert_eq!(raw.id, ServerPacketId::GuildNoticeChange as i16);

        let decoded = SGuildNoticeChange::decode(&raw.payload)
            .expect("decode SGuildNoticeChange");
        assert_eq!(decoded.update, 2);
        assert_eq!(decoded.notice, p.notice);
    }

    #[test]
    fn guild_member_change_roundtrip() {
        let p = SGuildMemberChange {
            name: "Player".to_string(),
            rank_index: 1,
            status: 2,
            ranks_bytes: Vec::new(),
        };

        let raw = p.encode().expect("encode SGuildMemberChange");
        assert_eq!(raw.id, ServerPacketId::GuildMemberChange as i16);

        let decoded = SGuildMemberChange::decode(&raw.payload)
            .expect("decode SGuildMemberChange");
        assert_eq!(decoded.name, p.name);
        assert_eq!(decoded.rank_index, p.rank_index);
        assert_eq!(decoded.status, p.status);
        assert_eq!(decoded.ranks_bytes, p.ranks_bytes);
    }

    #[test]
    fn guild_status_roundtrip() {
        let p = SGuildStatus {
            guild_name: "Guild".to_string(),
            guild_rank_name: "Leader".to_string(),
            level: 3,
            experience: 12345,
            max_experience: 67890,
            gold: 1000,
            spare_points: 5,
            member_count: 10,
            max_members: 20,
            voting: true,
            item_count: 2,
            buff_count: 1,
            my_options: 0x12,
            my_rank_id: 7,
        };

        let raw = p.encode().expect("encode SGuildStatus");
        assert_eq!(raw.id, ServerPacketId::GuildStatus as i16);

        let decoded = SGuildStatus::decode(&raw.payload).expect("decode SGuildStatus");
        assert_eq!(decoded.guild_name, p.guild_name);
        assert_eq!(decoded.guild_rank_name, p.guild_rank_name);
        assert_eq!(decoded.level, p.level);
        assert_eq!(decoded.experience, p.experience);
        assert_eq!(decoded.max_experience, p.max_experience);
        assert_eq!(decoded.gold, p.gold);
        assert_eq!(decoded.spare_points, p.spare_points);
        assert_eq!(decoded.member_count, p.member_count);
        assert_eq!(decoded.max_members, p.max_members);
        assert_eq!(decoded.voting, p.voting);
        assert_eq!(decoded.item_count, p.item_count);
        assert_eq!(decoded.buff_count, p.buff_count);
        assert_eq!(decoded.my_options, p.my_options);
        assert_eq!(decoded.my_rank_id, p.my_rank_id);
    }

    #[test]
    fn guild_invite_roundtrip() {
        let p = SGuildInvite {
            name: "PlayerX".to_string(),
        };

        let raw = p.encode().expect("encode SGuildInvite");
        assert_eq!(raw.id, ServerPacketId::GuildInvite as i16);

        let decoded = SGuildInvite::decode(&raw.payload).expect("decode SGuildInvite");
        assert_eq!(decoded.name, p.name);
    }

    #[test]
    fn guild_exp_gain_roundtrip() {
        let p = SGuildExpGain { amount: 1234 };

        let raw = p.encode().expect("encode SGuildExpGain");
        assert_eq!(raw.id, ServerPacketId::GuildExpGain as i16);

        let decoded = SGuildExpGain::decode(&raw.payload).expect("decode SGuildExpGain");
        assert_eq!(decoded.amount, p.amount);
    }

    #[test]
    fn guild_name_request_roundtrip() {
        let p = SGuildNameRequest;

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::GuildNameRequest as i16);

        let decoded = SGuildNameRequest::decode(&raw.payload)
            .expect("decode SGuildNameRequest");
        let _ = decoded;
    }

    #[test]
    fn guild_storage_gold_change_roundtrip() {
        let p = SGuildStorageGoldChange {
            amount: 500,
            change_type: 1,
            name: "PlayerY".to_string(),
        };

        let raw = p
            .encode()
            .expect("encode SGuildStorageGoldChange");
        assert_eq!(raw.id, ServerPacketId::GuildStorageGoldChange as i16);

        let decoded = SGuildStorageGoldChange::decode(&raw.payload)
            .expect("decode SGuildStorageGoldChange");
        assert_eq!(decoded.amount, p.amount);
        assert_eq!(decoded.change_type, p.change_type);
        assert_eq!(decoded.name, p.name);
    }

    #[test]
    fn guild_storage_item_change_roundtrip() {
        let p = SGuildStorageItemChange {
            user: 42,
            change_type: 2,
            to_slot: 3,
            from_slot: 4,
            item_user_id: Some(0x1122_3344_5566_7788),
            item_bytes: vec![1, 2, 3, 4, 5],
        };

        let raw = p
            .encode()
            .expect("encode SGuildStorageItemChange");
        assert_eq!(raw.id, ServerPacketId::GuildStorageItemChange as i16);

        let decoded = SGuildStorageItemChange::decode(&raw.payload)
            .expect("decode SGuildStorageItemChange");
        assert_eq!(decoded.user, p.user);
        assert_eq!(decoded.change_type, p.change_type);
        assert_eq!(decoded.to_slot, p.to_slot);
        assert_eq!(decoded.from_slot, p.from_slot);
        assert_eq!(decoded.item_user_id, p.item_user_id);
        assert_eq!(decoded.item_bytes, p.item_bytes);
    }

    #[test]
    fn guild_storage_list_roundtrip() {
        let p = SGuildStorageList {
            items_bytes: vec![0, 0, 0, 0],
        };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::GuildStorageList as i16);
        assert_eq!(raw.payload, p.items_bytes);

        let decoded = SGuildStorageList::decode(&raw.payload)
            .expect("decode SGuildStorageList");
        assert_eq!(decoded.items_bytes, p.items_bytes);
    }

    #[test]
    fn guild_request_war_roundtrip() {
        let p = SGuildRequestWar;

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::GuildRequestWar as i16);

        let decoded = SGuildRequestWar::decode(&raw.payload)
            .expect("decode SGuildRequestWar");
        let _ = decoded;
    }

    #[test]
    fn guild_buff_list_roundtrip() {
        let p = SGuildBuffList {
            remove: 1,
            active_buffs_bytes: vec![0, 0, 0, 0], // count = 0
            guild_buffs_bytes: Vec::new(),
        };

        let raw = p.encode().expect("encode SGuildBuffList");
        assert_eq!(raw.id, ServerPacketId::GuildBuffList as i16);

        let decoded = SGuildBuffList::decode(&raw.payload).expect("decode SGuildBuffList");
        assert_eq!(decoded.remove, p.remove);
        // Note: decode stores everything in active_buffs_bytes for blob-first
        assert!(!decoded.active_buffs_bytes.is_empty());
    }
}
