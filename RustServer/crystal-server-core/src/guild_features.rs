use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuildBuff {
    pub buff_id: u32,
    pub buff_type: GuildBuffType,
    pub level: u8,
    pub active: bool,
    pub start_time: u64, // Unix timestamp when activated
    pub duration: u64, // Duration in seconds
    pub cost: u32, // Gold cost to activate
    pub required_guild_level: u8,
}

impl GuildBuff {
    pub fn new(buff_id: u32, buff_type: GuildBuffType, level: u8, cost: u32, required_guild_level: u8) -> Self {
        Self {
            buff_id,
            buff_type,
            level,
            active: false,
            start_time: 0,
            duration: 0,
            cost,
            required_guild_level,
        }
    }
    
    pub fn activate(&mut self, duration_hours: u8) {
        self.active = true;
        self.start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.duration = duration_hours as u64 * 3600;
    }
    
    pub fn deactivate(&mut self) {
        self.active = false;
        self.start_time = 0;
        self.duration = 0;
    }
    
    pub fn is_expired(&self) -> bool {
        if !self.active {
            return false;
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now >= (self.start_time + self.duration)
    }
    
    pub fn get_remaining_time(&self) -> u64 {
        if !self.active {
            return 0;
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let end_time = self.start_time + self.duration;
        if now >= end_time {
            0
        } else {
            end_time - now
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuildBuffType {
    Attack = 0,
    Defence = 1,
    Experience = 2,
    DropRate = 3,
    HP = 4,
    MP = 5,
}

impl From<u8> for GuildBuffType {
    fn from(value: u8) -> Self {
        match value {
            0 => GuildBuffType::Attack,
            1 => GuildBuffType::Defence,
            2 => GuildBuffType::Experience,
            3 => GuildBuffType::DropRate,
            4 => GuildBuffType::HP,
            5 => GuildBuffType::MP,
            _ => GuildBuffType::Attack,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuildTerritory {
    pub territory_id: i32,
    pub guild_id: i32,
    pub name: String,
    pub rent_time: u64, // Unix timestamp when rent expires
    pub purchase_time: u64, // Unix timestamp when purchased
    pub daily_cost: u32,
    pub benefits: Vec<TerritoryBenefit>,
}

impl GuildTerritory {
    pub fn new(territory_id: i32, guild_id: i32, name: String, daily_cost: u32) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            territory_id,
            guild_id,
            name,
            rent_time: now + 86400, // 1 day initial rent
            purchase_time: now,
            daily_cost,
            benefits: vec![
                TerritoryBenefit::ExperienceBonus(5),
                TerritoryBenefit::DropRateBonus(3),
                TerritoryBenefit::TeleportAccess,
            ],
        }
    }
    
    pub fn is_rent_active(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now < self.rent_time
    }
    
    pub fn extend_rent(&mut self, days: u32) {
        self.rent_time += days as u64 * 86400;
    }
    
    pub fn get_daily_cost(&self) -> u32 {
        self.daily_cost
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TerritoryBenefit {
    ExperienceBonus(u8), // Percentage
    DropRateBonus(u8), // Percentage
    TeleportAccess,
    SpecialNPC,
    WarehouseExpansion,
}

#[derive(Clone, Debug)]
pub struct GuildFeatures {
    // Guild buffs
    pub available_buffs: Vec<GuildBuff>,
    pub active_buffs: HashMap<u32, GuildBuff>,
    
    // Guild territories
    pub territories: HashMap<i32, GuildTerritory>,
    
    // Guild war return points
    pub war_return_points: HashMap<i32, WarReturnPoint>,
    
    // Configuration
    max_active_buffs: usize,
    max_territories: usize,
}

impl GuildFeatures {
    pub fn new() -> Self {
        let mut features = Self {
            available_buffs: Vec::new(),
            active_buffs: HashMap::new(),
            territories: HashMap::new(),
            war_return_points: HashMap::new(),
            max_active_buffs: 3,
            max_territories: 5,
        };
        
        // Initialize default guild buffs
        features.initialize_default_buffs();
        features
    }
    
    fn initialize_default_buffs(&mut self) {
        self.available_buffs.push(GuildBuff::new(
            1, 
            GuildBuffType::Attack, 
            1, 
            100000, 
            1
        ));
        
        self.available_buffs.push(GuildBuff::new(
            2, 
            GuildBuffType::Defence, 
            1, 
            100000, 
            1
        ));
        
        self.available_buffs.push(GuildBuff::new(
            3, 
            GuildBuffType::Experience, 
            1, 
            150000, 
            2
        ));
        
        self.available_buffs.push(GuildBuff::new(
            4, 
            GuildBuffType::DropRate, 
            1, 
            200000, 
            3
        ));
        
        self.available_buffs.push(GuildBuff::new(
            5, 
            GuildBuffType::HP, 
            1, 
            120000, 
            2
        ));
        
        self.available_buffs.push(GuildBuff::new(
            6, 
            GuildBuffType::MP, 
            1, 
            120000, 
            2
        ));
    }
    
    /// Update guild buff
    pub fn update_buff(&mut self, buff_id: u32, guild_level: u8, guild_gold: u64) -> Result<(), String> {
        // Find the buff
        let buff = self.available_buffs
            .iter()
            .find(|b| b.buff_id == buff_id)
            .ok_or("Buff not found")?;
        
        // Check guild level requirement
        if guild_level < buff.required_guild_level {
            return Err("Guild level too low".to_string());
        }
        
        // Check if already active
        if self.active_buffs.contains_key(&buff_id) {
            return Err("Buff already active".to_string());
        }
        
        // Check max active buffs
        if self.active_buffs.len() >= self.max_active_buffs {
            return Err("Maximum active buffs reached".to_string());
        }
        
        // Check gold
        if guild_gold < buff.cost as u64 {
            return Err("Insufficient guild gold".to_string());
        }
        
        // Activate the buff
        let mut new_buff = buff.clone();
        new_buff.activate(24); // 24 hours default
        
        self.active_buffs.insert(buff_id, new_buff);
        
        Ok(())
    }
    
    /// Get all active buffs for a guild
    pub fn get_active_buffs(&self) -> Vec<&GuildBuff> {
        self.active_buffs
            .values()
            .filter(|b| b.active && !b.is_expired())
            .collect()
    }
    
    /// Clean up expired buffs
    pub fn cleanup_expired_buffs(&mut self) -> Vec<u32> {
        let mut expired = Vec::new();
        
        for (buff_id, buff) in &self.active_buffs {
            if buff.is_expired() {
                expired.push(*buff_id);
            }
        }
        
        for buff_id in expired.iter() {
            self.active_buffs.remove(buff_id);
        }
        
        expired
    }
    
    /// Purchase guild territory
    pub fn purchase_territory(&mut self, guild_id: i32, territory_id: i32, name: String, cost: u32) -> Result<(), String> {
        // Check max territories
        if self.territories.len() >= self.max_territories {
            return Err("Maximum territories reached".to_string());
        }
        
        // Check if territory already owned
        if self.territories.contains_key(&territory_id) {
            return Err("Territory already owned".to_string());
        }
        
        let territory = GuildTerritory::new(territory_id, guild_id, name, cost);
        self.territories.insert(territory_id, territory);
        
        Ok(())
    }
    
    /// Get territories for a guild
    pub fn get_guild_territories(&self, guild_id: i32) -> Vec<&GuildTerritory> {
        self.territories
            .values()
            .filter(|t| t.guild_id == guild_id && t.is_rent_active())
            .collect()
    }
    
    /// Set war return point for a guild
    pub fn set_war_return_point(&mut self, guild_id: i32, map_id: u32, x: u16, y: u16) {
        let point = WarReturnPoint {
            guild_id,
            map_id,
            x,
            y,
            set_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        self.war_return_points.insert(guild_id, point);
    }
    
    /// Get war return point for a guild
    pub fn get_war_return_point(&self, guild_id: i32) -> Option<&WarReturnPoint> {
        self.war_return_points.get(&guild_id)
    }
    
    /// Remove war return point
    pub fn remove_war_return_point(&mut self, guild_id: i32) {
        self.war_return_points.remove(&guild_id);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WarReturnPoint {
    pub guild_id: i32,
    pub map_id: u32,
    pub x: u16,
    pub y: u16,
    pub set_time: u64,
}
