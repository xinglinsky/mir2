use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crystal_shared_proto::item_types::UserItemData;

#[derive(Clone, Debug)]
pub struct RentalItem {
    pub item: UserItemData,
    pub owner_id: i32,
    pub rental_price: u32,
    pub rental_period_days: u32,
    pub deposit_time: u64, // Unix timestamp when deposited
    pub expiry_time: u64, // Unix timestamp when rental expires
    pub is_locked: bool, // Whether the rental is locked for confirmation
    pub rental_fee: u32, // Fee paid for rental
}

impl RentalItem {
    pub fn new(
        item: UserItemData,
        owner_id: i32,
        rental_price: u32,
        rental_period_days: u32,
        rental_fee: u32,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let expiry_time = now + (rental_period_days as u64 * 86400);
        
        Self {
            item,
            owner_id,
            rental_price,
            rental_period_days,
            deposit_time: now,
            expiry_time,
            is_locked: false,
            rental_fee,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now >= self.expiry_time
    }
    
    pub fn get_remaining_days(&self) -> u32 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if now >= self.expiry_time {
            0
        } else {
            ((self.expiry_time - now) / 86400) as u32
        }
    }
    
    pub fn extend_rental(&mut self, additional_days: u32, extension_fee: u32) {
        self.expiry_time += additional_days as u64 * 86400;
        self.rental_period_days += additional_days;
        self.rental_fee += extension_fee;
    }
}

#[derive(Clone, Debug)]
pub struct ItemRentalSystem {
    // Map of rental_id to rental item
    rentals: HashMap<u64, RentalItem>,
    // Map of player_id to their rental items
    player_rentals: HashMap<i32, Vec<u64>>,
    // Map of player_id to their deposited items
    deposited_items: HashMap<i32, Vec<u64>>,
    next_rental_id: u64,
    // Configuration
    max_rentals_per_player: usize,
    min_rental_period_days: u32,
    max_rental_period_days: u32,
    deposit_fee_rate: f32, // Percentage of item value as deposit
}

impl ItemRentalSystem {
    pub fn new() -> Self {
        Self {
            rentals: HashMap::new(),
            player_rentals: HashMap::new(),
            deposited_items: HashMap::new(),
            next_rental_id: 1,
            max_rentals_per_player: 10,
            min_rental_period_days: 1,
            max_rental_period_days: 30,
            deposit_fee_rate: 0.1, // 10% of item value
        }
    }
    
    /// Get all rental items for a player
    pub fn get_player_rentals(&self, player_id: i32) -> Vec<&RentalItem> {
        self.player_rentals
            .get(&player_id)
            .map(|rental_ids| {
                rental_ids
                    .iter()
                    .filter_map(|&id| self.rentals.get(&id))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get all deposited items for a player
    pub fn get_deposited_items(&self, player_id: i32) -> Vec<&RentalItem> {
        self.deposited_items
            .get(&player_id)
            .map(|item_ids| {
                item_ids
                    .iter()
                    .filter_map(|&id| self.rentals.get(&id))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Calculate rental cost
    pub fn calculate_rental_cost(&self, item_value: u32, days: u32, rental_fee: u32) -> u32 {
        // Base cost is rental fee per day
        let base_cost = rental_fee * days;
        
        // Adjust based on item value (higher value items cost more)
        let value_adjustment = (item_value as f32 * 0.01) as u32 * days;
        
        base_cost + value_adjustment
    }
    
    /// Calculate deposit amount
    pub fn calculate_deposit(&self, item_value: u32) -> u32 {
        (item_value as f32 * self.deposit_fee_rate) as u32
    }
    
    /// Deposit item for rental
    pub fn deposit_item(
        &mut self,
        item: UserItemData,
        owner_id: i32,
        rental_price: u32,
        rental_period_days: u32,
        rental_fee: u32,
    ) -> Result<u64, String> {
        // Validate rental period
        if rental_period_days < self.min_rental_period_days 
            || rental_period_days > self.max_rental_period_days {
            return Err("Invalid rental period".to_string());
        }
        
        // Check max deposits
        let deposit_count = self.deposited_items
            .get(&owner_id)
            .map(|items| items.len())
            .unwrap_or(0);
        
        if deposit_count >= self.max_rentals_per_player {
            return Err("Maximum deposited items reached".to_string());
        }
        
        // Create rental
        let rental_id = self.next_rental_id;
        self.next_rental_id += 1;
        
        let rental = RentalItem::new(
            item,
            owner_id,
            rental_price,
            rental_period_days,
            rental_fee,
        );
        
        self.rentals.insert(rental_id, rental);
        
        // Add to deposited items
        self.deposited_items
            .entry(owner_id)
            .or_insert_with(Vec::new)
            .push(rental_id);
        
        Ok(rental_id)
    }
    
    /// Rent an item
    pub fn rent_item(
        &mut self,
        rental_id: u64,
        renter_id: i32,
    ) -> Result<UserItemData, String> {
        let rental = self.rentals
            .get_mut(&rental_id)
            .ok_or("Rental not found".to_string())?;
        
        // Check if rental is available
        if rental.is_locked {
            return Err("Item is currently locked".to_string());
        }
        
        // Check if expired
        if rental.is_expired() {
            return Err("Rental has expired".to_string());
        }
        
        // Lock the rental
        rental.is_locked = true;
        
        // Move from deposited to rentals
        let owner_id = rental.owner_id;
        if let Some(items) = self.deposited_items.get_mut(&owner_id) {
            items.retain(|&id| id != rental_id);
        }
        
        self.player_rentals
            .entry(renter_id)
            .or_insert_with(Vec::new)
            .push(rental_id);
        
        Ok(rental.item.clone())
    }
    
    /// Retrieve deposited item
    pub fn retrieve_deposited_item(
        &mut self,
        rental_id: u64,
        owner_id: i32,
    ) -> Result<UserItemData, String> {
        let rental = self.rentals
            .get(&rental_id)
            .ok_or("Rental not found".to_string())?;
        
        // Check ownership
        if rental.owner_id != owner_id {
            return Err("Not the owner".to_string());
        }
        
        // Check if currently rented
        if rental.is_locked {
            return Err("Item is currently rented".to_string());
        }
        
        let item = rental.item.clone();
        
        // Remove from system
        self.rentals.remove(&rental_id);
        if let Some(items) = self.deposited_items.get_mut(&owner_id) {
            items.retain(|&id| id != rental_id);
        }
        
        Ok(item)
    }
    
    /// Return rented item
    pub fn return_rented_item(
        &mut self,
        rental_id: u64,
        renter_id: i32,
    ) -> Result<(), String> {
        let rental = self.rentals
            .get(&rental_id)
            .ok_or("Rental not found".to_string())?;
        
        // Check if rented by this player
        if !self.player_rentals
            .get(&renter_id)
            .map(|items| items.contains(&rental_id))
            .unwrap_or(false) {
            return Err("Not renting this item".to_string());
        }
        
        let owner_id = rental.owner_id;
        
        // Move back to deposited items
        if let Some(items) = self.player_rentals.get_mut(&renter_id) {
            items.retain(|&id| id != rental_id);
        }
        
        self.deposited_items
            .entry(owner_id)
            .or_insert_with(Vec::new)
            .push(rental_id);
        
        // Unlock the rental
        if let Some(rental) = self.rentals.get_mut(&rental_id) {
            rental.is_locked = false;
        }
        
        Ok(())
    }
    
    /// Cancel rental (owner cancels before it's rented)
    pub fn cancel_rental(
        &mut self,
        rental_id: u64,
        owner_id: i32,
    ) -> Result<UserItemData, String> {
        let rental = self.rentals
            .get(&rental_id)
            .ok_or("Rental not found".to_string())?;
        
        // Check ownership
        if rental.owner_id != owner_id {
            return Err("Not the owner".to_string());
        }
        
        // Cannot cancel if already rented
        if rental.is_locked {
            return Err("Cannot cancel active rental".to_string());
        }
        
        let item = rental.item.clone();
        
        // Remove from system
        self.rentals.remove(&rental_id);
        if let Some(items) = self.deposited_items.get_mut(&owner_id) {
            items.retain(|&id| id != rental_id);
        }
        
        Ok(item)
    }
    
    /// Extend rental period
    pub fn extend_rental(
        &mut self,
        rental_id: u32,
        additional_days: u32,
        extension_fee: u32,
        renter_id: i32,
    ) -> Result<(), String> {
        let rental = self.rentals
            .get_mut(&(rental_id as u64))
            .ok_or("Rental not found".to_string())?;
        
        // Check if rented by this player
        if !self.player_rentals
            .get(&renter_id)
            .map(|items| items.contains(&(rental_id as u64)))
            .unwrap_or(false) {
            return Err("Not renting this item".to_string());
        }
        
        // Check extension limits
        let new_period = rental.rental_period_days + additional_days;
        if new_period > self.max_rental_period_days {
            return Err("Rental period exceeds maximum".to_string());
        }
        
        rental.extend_rental(additional_days, extension_fee);
        
        Ok(())
    }
    
    /// Clean up expired rentals
    pub fn cleanup_expired_rentals(&mut self) -> Vec<(u64, i32, UserItemData)> {
        let mut expired = Vec::new();
        let mut to_remove = Vec::new();
        
        for (&rental_id, rental) in &self.rentals {
            if rental.is_expired() && rental.is_locked {
                // Return expired items to owner
                expired.push((rental_id, rental.owner_id, rental.item.clone()));
                to_remove.push(rental_id);
            }
        }
        
        for rental_id in to_remove {
            self.remove_rental(rental_id);
        }
        
        expired
    }
    
    /// Remove rental from system
    fn remove_rental(&mut self, rental_id: u64) {
        if let Some(rental) = self.rentals.remove(&rental_id) {
            // Remove from player rentals
            if let Some(items) = self.player_rentals.get_mut(&rental.owner_id) {
                items.retain(|&id| id != rental_id);
            }
            
            // Remove from deposited items
            if let Some(items) = self.deposited_items.get_mut(&rental.owner_id) {
                items.retain(|&id| id != rental_id);
            }
        }
    }
    
    /// Get rental by ID
    pub fn get_rental(&self, rental_id: u64) -> Option<&RentalItem> {
        self.rentals.get(&rental_id)
    }
    
    /// Check if player can rent more items
    pub fn can_rent_more(&self, player_id: i32) -> bool {
        let rental_count = self.player_rentals
            .get(&player_id)
            .map(|items| items.len())
            .unwrap_or(0);
        
        rental_count < self.max_rentals_per_player
    }
}
