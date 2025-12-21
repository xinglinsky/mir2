use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crystal_shared_proto::item_types::UserItemData;

#[derive(Clone, Debug)]
pub struct RefiningRecipe {
    pub recipe_id: u32,
    pub target_item_index: i32, // Item index this recipe applies to
    pub required_materials: Vec<RefiningMaterial>,
    pub base_success_rate: u8,
    pub time_minutes: u16,
    pub max_refine_level: u8,
}

#[derive(Clone, Debug)]
pub struct RefiningMaterial {
    pub item_id: u32,
    pub count: u16,
    pub is_consumed: bool, // Whether material is consumed on refine
}

#[derive(Clone, Debug)]
pub struct RefiningSession {
    pub session_id: u64,
    pub player_id: i32,
    pub target_item: UserItemData,
    pub materials: Vec<UserItemData>,
    pub start_time: u64,
    pub completion_time: u64,
    pub success_chance: u8,
    pub recipe_id: u32,
}

impl RefiningSession {
    pub fn new(
        session_id: u64,
        player_id: i32,
        target_item: UserItemData,
        materials: Vec<UserItemData>,
        recipe: &RefiningRecipe,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            session_id,
            player_id,
            target_item,
            materials,
            start_time: now,
            completion_time: now + (recipe.time_minutes as u64 * 60),
            success_chance: recipe.base_success_rate,
            recipe_id: recipe.recipe_id,
        }
    }
    
    pub fn is_complete(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now >= self.completion_time
    }
    
    pub fn get_remaining_time(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if now >= self.completion_time {
            0
        } else {
            self.completion_time - now
        }
    }
    
    pub fn calculate_success(&self, player_luck: u8) -> bool {
        // Calculate final success chance based on various factors
        let mut final_chance = self.success_chance;
        
        // Add luck bonus
        final_chance = final_chance.saturating_add(player_luck / 2);
        
        // Reduce chance based on current refine level
        let refine_level = self.target_item.refine_added;
        let reduction = refine_level * 5; // 5% reduction per level
        final_chance = final_chance.saturating_sub(reduction);
        
        // Clamp between 5% and 95%
        final_chance = final_chance.clamp(5, 95);
        
        // Roll for success
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_range(0..100) < final_chance as i32
    }
    
    pub fn apply_refine_success(&mut self) -> UserItemData {
        let mut refined_item = self.target_item.clone();
        refined_item.refine_added += 1;
        
        refined_item
    }
    
    pub fn apply_refine_failure(&mut self) -> Option<UserItemData> {
        // Chance to destroy item on failure
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        if self.target_item.refine_added >= 7 {
            // High level items have chance to be destroyed
            if rng.gen_range(0..100) < 30 {
                return None; // Item destroyed
            }
        }
        
        // Item survives but loses all refinement
        let mut failed_item = self.target_item.clone();
        failed_item.refine_added = 0;
        
        Some(failed_item)
    }
}

#[derive(Clone, Debug)]
pub struct RefiningSystem {
    // Available refining recipes
    pub recipes: HashMap<u32, RefiningRecipe>,
    
    // Active refining sessions
    pub active_sessions: HashMap<u64, RefiningSession>,
    
    // Player deposited items (for refining)
    pub deposited_items: HashMap<i32, Vec<UserItemData>>,
    
    // Configuration
    pub max_deposit_slots: usize,
    pub max_active_sessions: usize,
    next_session_id: u64,
}

impl RefiningSystem {
    pub fn new() -> Self {
        let mut system = Self {
            recipes: HashMap::new(),
            active_sessions: HashMap::new(),
            deposited_items: HashMap::new(),
            max_deposit_slots: 10,
            max_active_sessions: 1,
            next_session_id: 1,
        };
        
        system.initialize_default_recipes();
        system
    }
    
    fn initialize_default_recipes(&mut self) {
        // Weapon refining recipes
        self.recipes.insert(1, RefiningRecipe {
            recipe_id: 1,
            target_item_index: 0, // TODO: set actual item indices for weapon recipes
            required_materials: vec![
                RefiningMaterial {
                    item_id: 5001, // Iron Ore
                    count: 5,
                    is_consumed: true,
                },
                RefiningMaterial {
                    item_id: 5002, // Magic Stone
                    count: 2,
                    is_consumed: true,
                },
            ],
            base_success_rate: 70,
            time_minutes: 5,
            max_refine_level: 10,
        });
        
        // Armor refining recipes
        self.recipes.insert(2, RefiningRecipe {
            recipe_id: 2,
            target_item_index: 0, // TODO: set actual item indices for armor recipes
            required_materials: vec![
                RefiningMaterial {
                    item_id: 5001, // Iron Ore
                    count: 3,
                    is_consumed: true,
                },
                RefiningMaterial {
                    item_id: 5003, // Protective Crystal
                    count: 1,
                    is_consumed: true,
                },
            ],
            base_success_rate: 75,
            time_minutes: 3,
            max_refine_level: 10,
        });
        
        // Accessory refining recipes
        self.recipes.insert(3, RefiningRecipe {
            recipe_id: 3,
            target_item_index: 0, // TODO: set actual item indices for accessory recipes
            required_materials: vec![
                RefiningMaterial {
                    item_id: 5004, // Essence Stone
                    count: 2,
                    is_consumed: true,
                },
            ],
            base_success_rate: 65,
            time_minutes: 10,
            max_refine_level: 10,
        });
    }
    
    /// Get recipe by item index
    pub fn get_recipe_for_item(&self, item_index: i32) -> Option<&RefiningRecipe> {
        self.recipes
            .values()
            .find(|r| r.target_item_index == item_index)
    }
    
    /// Deposit item for refining
    pub fn deposit_item(&mut self, player_id: i32, item: UserItemData) -> Result<usize, String> {
        // Check deposit limit
        let deposited = self.deposited_items
            .get(&player_id)
            .map(|items| items.len())
            .unwrap_or(0);
        
        if deposited >= self.max_deposit_slots {
            return Err("Deposit slots full".to_string());
        }
        
        // Check if item can be refined
        let recipe = self.get_recipe_for_item(item.item_index)
            .ok_or("This item cannot be refined")?;
        
        if item.refine_added >= recipe.max_refine_level {
            return Err("Item already at max refine level".to_string());
        }
        
        // Add to deposited items
        let slot = deposited;
        self.deposited_items
            .entry(player_id)
            .or_insert_with(Vec::new)
            .push(item);
        
        Ok(slot)
    }
    
    /// Retrieve deposited item
    pub fn retrieve_item(&mut self, player_id: i32, slot: usize) -> Result<UserItemData, String> {
        let items = self.deposited_items
            .get_mut(&player_id)
            .ok_or("No deposited items")?;
        
        if slot >= items.len() {
            return Err("Invalid slot".to_string());
        }
        
        let item = items.remove(slot);
        
        // Clean up empty deposit list
        if items.is_empty() {
            self.deposited_items.remove(&player_id);
        }
        
        Ok(item)
    }
    
    /// Get deposited items for player
    pub fn get_deposited_items(&self, player_id: i32) -> &[UserItemData] {
        self.deposited_items
            .get(&player_id)
            .map(|items| items.as_slice())
            .unwrap_or(&[])
    }
    
    /// Start refining an item
    pub fn start_refining(
        &mut self,
        player_id: i32,
        item_slot: usize,
        material_slots: Vec<usize>,
        _player_luck: u8,
    ) -> Result<RefiningSession, String> {
        // Check if player has active session
        if self.active_sessions
            .values()
            .any(|s| s.player_id == player_id) {
            return Err("Already have an active refining session".to_string());
        }
        
        // Get the target item
        let target_item = self.retrieve_item(player_id, item_slot)?;
        let target_item_index = target_item.item_index;
        
        // Get recipe and clone needed data
        let recipe_id = {
            let recipe = self.get_recipe_for_item(target_item_index)
                .ok_or("No recipe for this item type")?;
            recipe.recipe_id
        };
        
        // Collect materials
        let mut materials = Vec::new();
        for &slot in &material_slots {
            let material = self.retrieve_item(player_id, slot)?;
            materials.push(material);
        }
        
        // Get recipe again for session creation (now that we have materials)
        let recipe = self.get_recipe_for_item(target_item_index)
            .ok_or("No recipe for this item type")?;
        
        // Create session
        let session = RefiningSession::new(
            self.next_session_id,
            player_id,
            target_item,
            materials,
            recipe,
        );
        
        self.next_session_id += 1;
        self.active_sessions.insert(session.session_id, session.clone());
        
        Ok(session)
    }
    
    /// Complete refining session
    pub fn complete_refining(&mut self, session_id: u64, player_luck: u8) -> Result<UserItemData, String> {
        let mut session = self.active_sessions
            .remove(&session_id)
            .ok_or("Session not found")?;
        
        if !session.is_complete() {
            // Put session back
            self.active_sessions.insert(session_id, session);
            return Err("Refining not complete yet".to_string());
        }
        
        // Calculate success
        if session.calculate_success(player_luck) {
            // Success
            let refined_item = session.apply_refine_success();
            Ok(refined_item)
        } else {
            // Failure
            session.apply_refine_failure()
                .ok_or("Item was destroyed during refining".to_string())
        }
    }
    
    /// Cancel refining session
    pub fn cancel_refining(&mut self, player_id: i32) -> Result<Vec<UserItemData>, String> {
        // Find and remove player's session
        let session_id = self.active_sessions
            .values()
            .find(|s| s.player_id == player_id)
            .map(|s| s.session_id);
        
        let session_id = match session_id {
            Some(id) => id,
            None => return Err("No active refining session".to_string()),
        };
        
        let session = self.active_sessions
            .remove(&session_id)
            .ok_or("Session not found")?;
        
        // Return all items
        let mut returned_items = vec![session.target_item];
        returned_items.extend(session.materials);
        
        Ok(returned_items)
    }
    
    /// Get active session for player
    pub fn get_active_session(&self, player_id: i32) -> Option<&RefiningSession> {
        self.active_sessions
            .values()
            .find(|s| s.player_id == player_id)
    }
    
    /// Update completed sessions
    pub fn update_sessions(&mut self) -> Vec<u64> {
        let mut completed = Vec::new();
        
        for (session_id, session) in &self.active_sessions {
            if session.is_complete() {
                completed.push(*session_id);
            }
        }
        
        completed
    }
    
    /// Check if item can be refined
    pub fn can_refine_item(&self, item: &UserItemData) -> bool {
        if let Some(recipe) = self.get_recipe_for_item(item.item_index) {
            item.refine_added < recipe.max_refine_level
        } else {
            false
        }
    }
}
