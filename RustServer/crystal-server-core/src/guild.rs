use std::collections::HashMap;

use crate::world::Job;
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GuildId(pub i32);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuildMember {
    pub id: i32,
    pub name: String,
    pub level: u16,
    pub class: Job,
    pub last_login_ticks: i64,
    pub online: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuildRank {
    pub index: u8,
    pub name: String,
    pub options: u8,
    pub members: Vec<GuildMember>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuildBuff {
    pub id: i32,
    pub active: bool,
    pub active_time_remaining: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuildStorageItem {
    pub item_id: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuildInfo {
    pub id: GuildId,
    pub name: String,
    pub level: u8,
    pub spare_points: u8,
    pub experience: i64,
    pub gold: u32,
    pub votes: i32,
    pub member_count: i32,
    pub max_experience: i64,
    pub member_cap: i32,
    pub flag_image: u16,
    pub flag_colour_argb: i32,
    pub ranks: Vec<GuildRank>,
    pub stored_items: Vec<GuildStorageItem>,
    pub buff_list: Vec<GuildBuff>,
    pub notice: Vec<String>,
    pub gt_rent_ticks: i64,
    pub gt_begin_ticks: i64,
    pub gt_index: i32,
    pub gt_key: i32,
    pub gt_price: i32,
}

impl GuildInfo {
    pub fn has_gt(&self, now_ticks: i64) -> bool {
        self.gt_rent_ticks > now_ticks
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct GuildManager {
    next_id: i32,
    guilds: HashMap<GuildId, GuildInfo>,
}

impl GuildManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_guilds(guilds: Vec<GuildInfo>) -> Self {
        let mut map = HashMap::new();
        let mut max_id = 0;

        for g in guilds {
            max_id = max_id.max(g.id.0);
            map.insert(g.id, g);
        }

        GuildManager {
            next_id: max_id,
            guilds: map,
        }
    }

    pub fn next_id_hint(&self) -> i32 {
        self.next_id
    }

    pub fn guilds(&self) -> impl Iterator<Item = &GuildInfo> {
        self.guilds.values()
    }

    pub fn guilds_mut(&mut self) -> impl Iterator<Item = &mut GuildInfo> {
        self.guilds.values_mut()
    }

    pub fn create_guild(&mut self, name: String) -> &mut GuildInfo {
        self.next_id += 1;
        let id = GuildId(self.next_id);
        let info = GuildInfo {
            id,
            name,
            level: 0,
            spare_points: 0,
            experience: 0,
            gold: 0,
            votes: 0,
            member_count: 0,
            max_experience: 0,
            member_cap: 0,
            flag_image: 1000,
            flag_colour_argb: 0x00ff_ffff,
            ranks: Vec::new(),
            stored_items: Vec::new(),
            buff_list: Vec::new(),
            notice: Vec::new(),
            gt_rent_ticks: 0,
            gt_begin_ticks: 0,
            gt_index: -1,
            gt_key: 0,
            gt_price: 0,
        };
        self.guilds.insert(id, info);
        self.guilds.get_mut(&id).unwrap()
    }

    pub fn get_guild(&self, id: GuildId) -> Option<&GuildInfo> {
        self.guilds.get(&id)
    }

    pub fn get_guild_mut(&mut self, id: GuildId) -> Option<&mut GuildInfo> {
        self.guilds.get_mut(&id)
    }

    pub fn remove_guild(&mut self, id: GuildId) -> Option<GuildInfo> {
        self.guilds.remove(&id)
    }
}
