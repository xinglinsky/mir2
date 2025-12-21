use crystal_shared_proto::npc::{
    CGuildBuffUpdate, CGuildTerritoryPage, CPurchaseGuildTerritory, CGuildWarReturn,
    SGuildBuffUpdate, SGuildTerritoryPage, SPurchaseGuildTerritory, SGuildWarReturn
};
use crystal_shared_proto::user::status::SLoseGold;
use crystal_server_core::guild_features::{GuildFeatures, GuildBuff, GuildTerritory, WarReturnPoint};
use crate::connection::login::{LoginConnection, Stage};

// Guild Features handlers for LoginConnection
impl LoginConnection {
    pub fn handle_guild_buff_update(&mut self, msg: CGuildBuffUpdate, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Get player's guild info
        let guild_info = {
            let world = self.world.lock().unwrap();
            world.get_player_guild(self.session_id)
        };

        let guild_info = match guild_info {
            Some(info) => info,
            None => {
                let pkt = SGuildBuffUpdate {
                    success: false,
                    message: "Not in a guild".to_string(),
                    buffs: Vec::new(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        // Get guild features
        let mut guild_features = {
            let mut world = self.world.lock().unwrap();
            world.get_guild_features_mut()
        };

        // Update the buff
        match guild_features.update_buff(msg.buff_id, guild_info.level, guild_info.gold) {
            Ok(()) => {
                // Deduct gold from guild
                let buff_cost = guild_features.available_buffs
                    .iter()
                    .find(|b| b.buff_id == msg.buff_id)
                    .map(|b| b.cost)
                    .unwrap_or(0);

                let mut updated_guild = guild_info.clone();
                updated_guild.gold = updated_guild.gold.saturating_sub(buff_cost as u64);

                // Update guild gold in database
                if let Some(ref account_id) = self.account_id {
                    let _ = self.store.save_guild_gold(account_id, updated_guild.index, updated_guild.gold);
                }

                // Get updated active buffs
                let active_buffs = guild_features.get_active_buffs();
                let mut buff_data = Vec::new();
                for buff in active_buffs {
                    buff_data.push(SGuildBuffData {
                        buff_id: buff.buff_id,
                        buff_type: buff.buff_type as u8,
                        level: buff.level,
                        remaining_time: buff.get_remaining_time(),
                    });
                }

                let pkt = SGuildBuffUpdate {
                    success: true,
                    message: "Guild buff activated".to_string(),
                    buffs: buff_data,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            Err(e) => {
                let pkt = SGuildBuffUpdate {
                    success: false,
                    message: e,
                    buffs: Vec::new(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }
    
    pub fn handle_guild_territory_page(&mut self, _msg: CGuildTerritoryPage, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Get player's guild info
        let guild_info = {
            let world = self.world.lock().unwrap();
            world.get_player_guild(self.session_id)
        };

        let guild_info = match guild_info {
            Some(info) => info,
            None => {
                let pkt = SGuildTerritoryPage {
                    success: false,
                    owned_territories: Vec::new(),
                    available_territories: Vec::new(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        // Get guild features
        let guild_features = {
            let world = self.world.lock().unwrap();
            world.get_guild_features()
        };

        // Get owned territories
        let owned_territories = guild_features.get_guild_territories(guild_info.index);
        let mut owned_data = Vec::new();
        for territory in owned_territories {
            owned_data.push(STerritoryData {
                territory_id: territory.territory_id,
                name: territory.name.clone(),
                rent_expiry: territory.rent_time,
                daily_cost: territory.daily_cost,
            });
        }

        // Get available territories (simplified - would normally check which are available)
        let available_data = vec![
            STerritoryData {
                territory_id: 101,
                name: "Bichon Province".to_string(),
                rent_expiry: 0,
                daily_cost: 50000,
            },
            STerritoryData {
                territory_id: 102,
                name: "Tao Village".to_string(),
                rent_expiry: 0,
                daily_cost: 75000,
            },
            STerritoryData {
                territory_id: 103,
                name: "Sabuk Wall".to_string(),
                rent_expiry: 0,
                daily_cost: 100000,
            },
        ];

        let pkt = SGuildTerritoryPage {
            success: true,
            owned_territories: owned_data,
            available_territories: available_data,
        };
        
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
    
    pub fn handle_purchase_guild_territory(&mut self, msg: CPurchaseGuildTerritory, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Get player's guild info
        let guild_info = {
            let world = self.world.lock().unwrap();
            world.get_player_guild(self.session_id)
        };

        let guild_info = match guild_info {
            Some(info) => info,
            None => {
                let pkt = SPurchaseGuildTerritory {
                    success: false,
                    message: "Not in a guild".to_string(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        // Check if player has permission (guild leader)
        if guild_info.leader_id != self.current_char_index.unwrap_or(-1) {
            let pkt = SPurchaseGuildTerritory {
                success: false,
                message: "Only guild leader can purchase territories".to_string(),
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Get territory cost
        let territory_cost = match msg.territory_id {
            101 => 50000,
            102 => 75000,
            103 => 100000,
            _ => {
                let pkt = SPurchaseGuildTerritory {
                    success: false,
                    message: "Invalid territory".to_string(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        // Check guild gold
        if guild_info.gold < territory_cost as u64 {
            let pkt = SPurchaseGuildTerritory {
                success: false,
                message: "Insufficient guild gold".to_string(),
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Purchase territory
        let mut guild_features = {
            let mut world = self.world.lock().unwrap();
            world.get_guild_features_mut()
        };

        let territory_name = format!("Territory {}", msg.territory_id);
        match guild_features.purchase_territory(guild_info.index, msg.territory_id, territory_name, territory_cost) {
            Ok(()) => {
                // Deduct gold from guild
                let mut updated_guild = guild_info.clone();
                updated_guild.gold = updated_guild.gold.saturating_sub(territory_cost as u64);

                // Update guild gold in database
                if let Some(ref account_id) = self.account_id {
                    let _ = self.store.save_guild_gold(account_id, updated_guild.index, updated_guild.gold);
                }

                // Send guild gold loss packet
                let lose_gold = SLoseGold { gold: territory_cost };
                if let Ok(raw) = lose_gold.encode() {
                    out.push(Self::encode_raw(raw));
                }

                let pkt = SPurchaseGuildTerritory {
                    success: true,
                    message: "Territory purchased successfully".to_string(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            Err(e) => {
                let pkt = SPurchaseGuildTerritory {
                    success: false,
                    message: e,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }
    
    pub fn handle_guild_war_return(&mut self, msg: CGuildWarReturn, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Get player's guild info
        let guild_info = {
            let world = self.world.lock().unwrap();
            world.get_player_guild(self.session_id)
        };

        let guild_info = match guild_info {
            Some(info) => info,
            None => {
                let pkt = SGuildWarReturn {
                    success: false,
                    message: "Not in a guild".to_string(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        // Check if in guild war map
        let current_map = {
            let world = self.world.lock().unwrap();
            world.get_player_map_id(self.session_id)
        };

        // Only allow return in specific war maps
        let is_war_map = match current_map {
            Some(map_id) => map_id >= 500 && map_id <= 599, // Example war map range
            None => false,
        };

        if !is_war_map {
            let pkt = SGuildWarReturn {
                success: false,
                message: "Not in a guild war map".to_string(),
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Get guild features
        let mut guild_features = {
            let mut world = self.world.lock().unwrap();
            world.get_guild_features_mut()
        };

        // Set or update war return point
        guild_features.set_war_return_point(guild_info.index, msg.map_id, msg.x, msg.y);

        let pkt = SGuildWarReturn {
            success: true,
            message: "Guild war return point set".to_string(),
        };
        
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    /// Get active guild buffs for a player
    pub fn get_guild_buffs(&self) -> Vec<GuildBuff> {
        let guild_info = {
            let world = self.world.lock().unwrap();
            world.get_player_guild(self.session_id)
        };

        if let Some(guild) = guild_info {
            let guild_features = {
                let world = self.world.lock().unwrap();
                world.get_guild_features()
            };
            
            guild_features.get_active_buffs().into_iter().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

// Helper structs for packet data
#[derive(Clone, Debug)]
struct SGuildBuffData {
    buff_id: u32,
    buff_type: u8,
    level: u8,
    remaining_time: u64,
}

#[derive(Clone, Debug)]
struct STerritoryData {
    territory_id: i32,
    name: String,
    rent_expiry: u64,
    daily_cost: u32,
}
