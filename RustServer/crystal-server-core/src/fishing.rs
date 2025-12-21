use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crystal_shared_proto::item_types::UserItemData;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FishingSpot {
    pub spot_id: u32,
    pub map_id: u32,
    pub x: u16,
    pub y: u16,
    pub required_level: u16,
    pub fish_types: Vec<FishType>,
    pub rarity_weights: Vec<u32>, // Corresponds to fish_types
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FishType {
    Common = 0,
    Uncommon = 1,
    Rare = 2,
    Epic = 3,
    Legendary = 4,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fish {
    pub fish_type: FishType,
    pub size: u16, // Size in cm
    pub value: u32,
    pub exp_reward: u32,
}

impl Fish {
    pub fn new(fish_type: FishType, size: u16) -> Self {
        let (value, exp) = match fish_type {
            FishType::Common => (100 + size as u32 * 2, 50 + size as u32),
            FishType::Uncommon => (300 + size as u32 * 5, 150 + size as u32 * 2),
            FishType::Rare => (800 + size as u32 * 10, 400 + size as u32 * 3),
            FishType::Epic => (2000 + size as u32 * 20, 1000 + size as u32 * 5),
            FishType::Legendary => (5000 + size as u32 * 50, 2500 + size as u32 * 10),
        };
        
        Self {
            fish_type,
            size,
            value,
            exp_reward: exp,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FishingState {
    pub player_id: i32,
    pub is_fishing: bool,
    pub cast_out: bool,
    pub autocast: bool,
    pub spot_id: Option<u32>,
    pub cast_time: u64, // Unix timestamp when cast
    pub fish_bite_time: Option<u64>, // When fish will bite
    pub current_fish: Option<Fish>,
    pub fishing_rod: Option<UserItemData>,
    pub fishing_bait: Option<UserItemData>,
}

impl FishingState {
    pub fn new(player_id: i32) -> Self {
        Self {
            player_id,
            is_fishing: false,
            cast_out: false,
            autocast: false,
            spot_id: None,
            cast_time: 0,
            fish_bite_time: None,
            current_fish: None,
            fishing_rod: None,
            fishing_bait: None,
        }
    }
    
    pub fn start_fishing(&mut self, spot_id: u32, rod: UserItemData, bait: Option<UserItemData>) {
        self.is_fishing = true;
        self.spot_id = Some(spot_id);
        self.fishing_rod = Some(rod);
        self.fishing_bait = bait;
        self.cast_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
    
    pub fn cast_line(&mut self) {
        if !self.is_fishing {
            return;
        }
        
        self.cast_out = true;
        
        // Fish will bite after 3-10 seconds
        let bite_delay = 3 + (rand::random::<u8>() % 8);
        self.fish_bite_time = Some(self.cast_time + bite_delay as u64);
    }
    
    pub fn reel_in(&mut self) -> Option<Fish> {
        if !self.cast_out {
            return None;
        }
        
        self.cast_out = false;
        self.fish_bite_time = None;
        
        // Check if fish is on the hook
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if let Some(bite_time) = self.fish_bite_time {
            if now >= bite_time && now <= (bite_time + 3) {
                // Fish caught!
                if let Some(fish) = self.current_fish.take() {
                    return Some(fish);
                }
            }
        }
        
        None
    }
    
    pub fn stop_fishing(&mut self) {
        self.is_fishing = false;
        self.cast_out = false;
        self.spot_id = None;
        self.cast_time = 0;
        self.fish_bite_time = None;
        self.current_fish = None;
        self.fishing_rod = None;
        self.fishing_bait = None;
    }
    
    pub fn toggle_autocast(&mut self) -> bool {
        self.autocast = !self.autocast;
        self.autocast
    }
    
    pub fn check_fish_bite(&mut self) -> bool {
        if !self.cast_out || self.fish_bite_time.is_none() {
            return false;
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if let Some(bite_time) = self.fish_bite_time {
            if now >= bite_time {
                // Generate a fish
                self.current_fish = Some(self.generate_random_fish());
                true
            } else {
                false
            }
        } else {
            false
        }
    }
    
    fn generate_random_fish(&self) -> Fish {
        // Random fish type based on rarity
        let rarity_roll = rand::random::<u32>() % 100;
        let fish_type = match rarity_roll {
            0..=59 => FishType::Common,      // 60%
            60..=84 => FishType::Uncommon,   // 25%
            85..=94 => FishType::Rare,       // 10%
            95..=98 => FishType::Epic,       // 4%
            _ => FishType::Legendary,        // 1%
        };
        
        // Random size (10-100cm)
        let size = 10 + rand::random::<u16>() % 91;
        
        Fish::new(fish_type, size)
    }
}

#[derive(Clone, Debug)]
pub struct FishingSystem {
    // Map of fishing spots
    pub fishing_spots: HashMap<u32, FishingSpot>,
    // Player fishing states
    pub player_states: HashMap<i32, FishingState>,
    // Configuration
    max_fishing_spots: usize,
    fishing_rod_item_id: u32,
    fishing_bait_item_id: u32,
}

impl FishingSystem {
    pub fn new() -> Self {
        let mut system = Self {
            fishing_spots: HashMap::new(),
            player_states: HashMap::new(),
            max_fishing_spots: 100,
            fishing_rod_item_id: 3001, // Example fishing rod ID
            fishing_bait_item_id: 3002, // Example bait ID
        };
        
        system.initialize_default_spots();
        system
    }
    
    fn initialize_default_spots(&mut self) {
        // Add some default fishing spots
        self.fishing_spots.insert(1, FishingSpot {
            spot_id: 1,
            map_id: 0, // Bichon
            x: 100,
            y: 200,
            required_level: 1,
            fish_types: vec![FishType::Common, FishType::Uncommon],
            rarity_weights: vec![70, 30],
        });
        
        self.fishing_spots.insert(2, FishingSpot {
            spot_id: 2,
            map_id: 1, // Woomyon Woods
            x: 150,
            y: 300,
            required_level: 20,
            fish_types: vec![FishType::Common, FishType::Uncommon, FishType::Rare],
            rarity_weights: vec![50, 35, 15],
        });
        
        self.fishing_spots.insert(3, FishingSpot {
            spot_id: 3,
            map_id: 2, // Serpent Valley
            x: 200,
            y: 400,
            required_level: 40,
            fish_types: vec![FishType::Uncommon, FishType::Rare, FishType::Epic],
            rarity_weights: vec![30, 50, 20],
        });
    }
    
    /// Get fishing state for a player
    pub fn get_player_state(&self, player_id: i32) -> Option<&FishingState> {
        self.player_states.get(&player_id)
    }
    
    /// Get mutable fishing state for a player
    pub fn get_player_state_mut(&mut self, player_id: i32) -> &mut FishingState {
        self.player_states
            .entry(player_id)
            .or_insert_with(|| FishingState::new(player_id))
    }
    
    /// Start fishing at a spot
    pub fn start_fishing(
        &mut self,
        player_id: i32,
        spot_id: u32,
        rod: UserItemData,
        bait: Option<UserItemData>,
        player_level: u16,
    ) -> Result<(), String> {
        // Check if spot exists
        let spot = self.fishing_spots
            .get(&spot_id)
            .ok_or("Invalid fishing spot")?;
        
        // Check level requirement
        if player_level < spot.required_level {
            return Err("Level too low for this fishing spot".to_string());
        }
        
        // Check if already fishing
        if let Some(state) = self.player_states.get(&player_id) {
            if state.is_fishing {
                return Err("Already fishing".to_string());
            }
        }
        
        // Start fishing
        let state = self.get_player_state_mut(player_id);
        state.start_fishing(spot_id, rod, bait);
        
        Ok(())
    }
    
    /// Cast the fishing line
    pub fn cast_line(&mut self, player_id: i32) -> Result<(), String> {
        let state = self.player_states
            .get_mut(&player_id)
            .ok_or("Not fishing")?;
        
        if state.cast_out {
            return Err("Line already cast".to_string());
        }
        
        state.cast_line();
        Ok(())
    }
    
    /// Reel in the line
    pub fn reel_in(&mut self, player_id: i32) -> Result<Option<Fish>, String> {
        let state = self.player_states
            .get_mut(&player_id)
            .ok_or("Not fishing")?;
        
        if !state.cast_out {
            return Err("Line not cast".to_string());
        }
        
        let fish = state.reel_in();
        
        // If autocast is enabled, cast again
        if state.autocast && fish.is_none() {
            state.cast_line();
        }
        
        Ok(fish)
    }
    
    /// Stop fishing
    pub fn stop_fishing(&mut self, player_id: i32) {
        if let Some(state) = self.player_states.get_mut(&player_id) {
            state.stop_fishing();
        }
    }
    
    /// Toggle autocast
    pub fn toggle_autocast(&mut self, player_id: i32) -> Result<bool, String> {
        let state = self.player_states
            .get_mut(&player_id)
            .ok_or("Not fishing")?;
        
        Ok(state.toggle_autocast())
    }
    
    /// Check for fish bites
    pub fn update_fishing(&mut self) -> Vec<(i32, bool)> {
        let mut bites = Vec::new();
        
        for (player_id, state) in self.player_states.iter_mut() {
            if state.check_fish_bite() {
                bites.push((*player_id, true));
            }
        }
        
        bites
    }
    
    /// Get fishing spot info
    pub fn get_fishing_spot(&self, spot_id: u32) -> Option<&FishingSpot> {
        self.fishing_spots.get(&spot_id)
    }
    
    /// Get all fishing spots
    pub fn get_all_spots(&self) -> Vec<&FishingSpot> {
        self.fishing_spots.values().collect()
    }
    
    /// Check if player is at a fishing spot
    pub fn is_at_fishing_spot(&self, player_id: i32, map_id: u32, x: u16, y: u16) -> Option<u32> {
        for spot in self.fishing_spots.values() {
            if spot.map_id == map_id {
                // Check if player is within range (5 tiles)
                let dx = if spot.x > x { spot.x - x } else { x - spot.x };
                let dy = if spot.y > y { spot.y - y } else { y - spot.y };
                
                if dx <= 5 && dy <= 5 {
                    return Some(spot.spot_id);
                }
            }
        }
        None
    }
}
