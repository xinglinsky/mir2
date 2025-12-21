use crystal_shared_proto::npc::{
    CFishingCast, CFishingChangeAutocast,
};
use crystal_shared_proto::user::flat::SFishingUpdate;
use crystal_shared_proto::item::SRefreshItem;
use crystal_shared_proto::user::status::SRefreshCharacter;
use crystal_server_core::fishing::{FishingSystem, Fish};
use rand::Rng;
use crate::connection::login::{LoginConnection, Stage};

// Fishing handlers for LoginConnection
impl LoginConnection {
    pub fn handle_fishing_cast(&mut self, msg: CFishingCast, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let player_id = self.current_char_index.unwrap_or(-1);
        let player_level = {
            let world = self.world.lock().unwrap();
            world.get_player_level(player_id).unwrap_or(0)
        };

        // Get player's position
        let (map_id, x, y) = {
            let world = self.world.lock().unwrap();
            world.get_player_position(self.session_id)
                .unwrap_or((0, 0, 0))
        };

        let mut fishing_system = {
            let mut world = self.world.lock().unwrap();
            world.get_fishing_system_mut()
        };

        if msg.cast_out {
            // Cast the line
            // Check if player is at a fishing spot
            let spot_id = match fishing_system.is_at_fishing_spot(player_id, map_id, x, y) {
                Some(id) => id,
                None => {
                    let pkt = SFishingUpdate {
                        object_id: self.session_id as u32,
                        fishing: false,
                        progress_percent: 0,
                        chance_percent: 0,
                        fishing_point_x: 0,
                        fishing_point_y: 0,
                        found_fish: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    return;
                }
            };

            // Check if player has fishing rod
            let (inv, _eq) = {
                let world = self.world.lock().unwrap();
                world
                    .player_items(self.session_id)
                    .unwrap_or((
                        crystal_server_core::item::Inventory::new_default(),
                        crystal_server_core::item::Equipment::new_default(),
                    ))
            };

            let fishing_rod = self.find_fishing_rod(&inv);
            let fishing_rod = match fishing_rod {
                Some(rod) => rod,
                None => {
                    let pkt = SFishingUpdate {
                        object_id: self.session_id as u32,
                        fishing: false,
                        progress_percent: 0,
                        chance_percent: 0,
                        fishing_point_x: 0,
                        fishing_point_y: 0,
                        found_fish: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    return;
                }
            };

            // Start fishing if not already
            let state = fishing_system.get_player_state_mut(player_id);
            if !state.is_fishing {
                if let Err(e) = fishing_system.start_fishing(
                    player_id,
                    spot_id,
                    fishing_rod,
                    None, // No bait for now
                    player_level,
                ) {
                    let pkt = SFishingUpdate {
                        object_id: self.session_id as u32,
                        fishing: false,
                        progress_percent: 0,
                        chance_percent: 0,
                        fishing_point_x: 0,
                        fishing_point_y: 0,
                        found_fish: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    return;
                }
            }

            // Cast the line
            match fishing_system.cast_line(player_id) {
                Ok(()) => {
                    let pkt = SFishingUpdate {
                        object_id: self.session_id as u32,
                        fishing: true,
                        progress_percent: 0,
                        chance_percent: 100,
                        fishing_point_x: x,
                        fishing_point_y: y,
                        found_fish: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
                Err(e) => {
                    let pkt = SFishingUpdate {
                        object_id: self.session_id as u32,
                        fishing: false,
                        progress_percent: 0,
                        chance_percent: 0,
                        fishing_point_x: 0,
                        fishing_point_y: 0,
                        found_fish: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        } else {
            // Reel in the line
            match fishing_system.reel_in(player_id) {
                Ok(Some(fish)) => {
                    // Fish caught!
                    let pkt = SFishingUpdate {
                        object_id: self.session_id as u32,
                        fishing: false,
                        progress_percent: 100,
                        chance_percent: 0,
                        fishing_point_x: 0,
                        fishing_point_y: 0,
                        found_fish: true,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }

                    // Add fish to inventory
                    self.add_fish_to_inventory(fish, out);

                    // Give experience
                    self.add_experience(fish.exp_reward, out);
                }
                Ok(None) => {
                    // No fish caught
                    let pkt = SFishingUpdate {
                        object_id: self.session_id as u32,
                        fishing: false,
                        progress_percent: 100,
                        chance_percent: 0,
                        fishing_point_x: 0,
                        fishing_point_y: 0,
                        found_fish: true,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
                Err(e) => {
                    let pkt = SFishingUpdate {
                        object_id: self.session_id as u32,
                        fishing: false,
                        progress_percent: 0,
                        chance_percent: 0,
                        fishing_point_x: 0,
                        fishing_point_y: 0,
                        found_fish: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        }
    }
    
    pub fn handle_fishing_change_autocast(&mut self, msg: CFishingChangeAutocast, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let player_id = self.current_char_index.unwrap_or(-1);
        let mut fishing_system = {
            let mut world = self.world.lock().unwrap();
            world.get_fishing_system_mut()
        };

        match fishing_system.toggle_autocast(player_id) {
            Ok(autocast_enabled) => {
                let message = if autocast_enabled {
                    "Autocast enabled".to_string()
                } else {
                    "Autocast disabled".to_string()
                };

                let pkt = SFishingChangeAutocast {
                    success: true,
                    result: 0, // Use 0 as success code
                    autocast: autocast_enabled,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            Err(e) => {
                let pkt = SFishingChangeAutocast {
                    success: false,
                    result: 255, // Use 255 as error code
                    autocast: false,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }

    /// Find fishing rod in inventory
    fn find_fishing_rod(&self, inv: &crystal_server_core::item::Inventory) -> Option<crystal_shared_proto::item_types::UserItemData> {
        // Check equipment slots first
        for slot in inv.slots.iter() {
            if let Some(item) = slot {
                if self.is_fishing_rod(item) {
                    return Some(item.clone());
                }
            }
        }
        None
    }

    /// Check if item is a fishing rod
    fn is_fishing_rod(&self, item: &crystal_shared_proto::item_types::UserItemData) -> bool {
        // In a real implementation, this would check the item type
        // For now, assume item_id 3001 is fishing rod
        item.item_index == 3001
    }

    /// Add fish to player inventory
    fn add_fish_to_inventory(&mut self, fish: Fish, out: &mut Vec<Vec<u8>>) {
        // Create fish item
        let fish_item = crystal_shared_proto::item_types::UserItemData {
            unique_id: {
                let mut rng = rand::thread_rng();
                rng.gen::<u64>()
            },
            item_index: (4000 + fish.fish_type as u32) as i32, // Different IDs for different fish types
            current_dura: 100,
            max_dura: 100,
            count: 1,
            soul_bound_id: 0,
            identified: true,
            cursed: false,
            slots: Vec::new(),
            gem_count: 0,
            added_stats: crystal_shared_proto::stats::StatsMap::new(),
            awake: crystal_shared_proto::item_types::AwakeData { awake_type: 0, values: Vec::new() },
            refined_value: 0,
            refine_added: 0,
            refine_success_chance: 0,
            wedding_ring: 0,
            expire_info: None,
            rental_information: None,
            is_shop_item: false,
            sealed_info: None,
            gm_made: false,
        };

        // Add to inventory
        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        // Find empty slot
        if let Some(empty_slot) = inv.slots.iter_mut().find(|slot| slot.is_none()) {
            *empty_slot = Some(fish_item);
            
            // Update inventory
            {
                let mut world = self.world.lock().unwrap();
                world.set_player_items(self.session_id, inv, eq);
            }

            // Send refresh
            let mut buf = Vec::new();
            fish_item.encode(&mut buf).unwrap();
            let pkt = crystal_shared_proto::item::SRefreshItem {
                item_bytes: buf,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
        }
    }

    /// Add experience to player
    fn add_experience(&mut self, exp: u32, out: &mut Vec<Vec<u8>>) {
        if let Some(ref mut stats) = self.current_stats {
            stats.exp += exp as u64;
            
            // Check for level up
            let mut new_level = stats.level;
            while stats.exp >= self.get_exp_required_for_level(new_level + 1) as u64 {
                new_level += 1;
            }
            
            if new_level > stats.level {
                stats.level = new_level;
                // Send level up notification
            }
            
            // Save stats
            if let (Some(ref account_id), Some(char_idx)) = 
                (self.account_id.as_ref(), self.current_char_index) {
                let _ = self.store.save_character_stats(account_id, char_idx, stats);
            }
            
            // Send refresh
            let pkt = SRefreshCharacter {
                character_id: self.session_id,
                // ... other fields
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
        }
    }

    /// Get experience required for level
    fn get_exp_required_for_level(&self, level: u16) -> u32 {
        // Simple formula: level^2 * 1000
        (level as u32 * level as u32) * 1000
    }

    /// Get fish type name
    fn get_fish_type_name(&self, fish_type: crystal_server_core::fishing::FishType) -> &'static str {
        match fish_type {
            crystal_server_core::fishing::FishType::Common => "Common",
            crystal_server_core::fishing::FishType::Uncommon => "Uncommon",
            crystal_server_core::fishing::FishType::Rare => "Rare",
            crystal_server_core::fishing::FishType::Epic => "Epic",
            crystal_server_core::fishing::FishType::Legendary => "Legendary",
        }
    }

    /// Update fishing system (called periodically)
    pub fn update_fishing(&mut self, out: &mut Vec<Vec<u8>>) {
        let mut fishing_system = {
            let mut world = self.world.lock().unwrap();
            world.get_fishing_system_mut()
        };

        let bites = fishing_system.update_fishing();
        
        for (player_id, _) in bites {
            // Send fish bite notification to player
            if let Some(conn) = {
                let world = self.world.lock().unwrap();
                world.get_connection_by_char_index(player_id)
            } {
                let pkt = SFishingUpdate {
                    object_id: player_id as u32,
                    fishing: true,
                    progress_percent: 100,
                    chance_percent: 0,
                    fishing_point_x: 0,
                    fishing_point_y: 0,
                    found_fish: true,
                };
                if let Ok(raw) = pkt.encode() {
                    let _ = conn.send_raw_packet(raw);
                }
            }
        }
    }
}
