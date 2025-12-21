use crystal_shared_proto::npc::{
    CGetRentedItems, CItemRentalRequest, CItemRentalFee, CItemRentalPeriod,
    CDepositRentalItem, CRetrieveRentalItem, CCancelItemRental,
    CItemRentalLockFee, CItemRentalLockItem, CConfirmItemRental,
    SGetRentedItems, SItemRentalRequest, SItemRentalFee, SItemRentalPeriod,
    SDepositRentalItem, SRetrieveRentalItem, SCancelItemRental
};
use crystal_shared_proto::item::SRefreshItem;
use crystal_shared_proto::user::status::SLoseGold;
use crystal_server_core::item_rental::{ItemRentalSystem, RentalItem};
use rand::Rng;
use crate::connection::login::{LoginConnection, Stage};

// Item Rental handlers for LoginConnection
impl LoginConnection {
    pub fn handle_get_rented_items(&mut self, _msg: CGetRentedItems, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let player_id = self.current_char_index.unwrap_or(-1);
        let rental_system = {
            let world = self.world.lock().unwrap();
            world.get_item_rental_system()
        };

        // Get rented items
        let rented_items = rental_system.get_player_rentals(player_id);
        let mut rented_item_data = Vec::new();
        for rental in rented_items {
            rented_item_data.push(SRentalItemData {
                rental_id: rental.rental_id,
                item: rental.item.clone(),
                remaining_days: rental.get_remaining_days(),
                rental_price: rental.rental_price,
                is_expired: rental.is_expired(),
            });
        }

        // Get deposited items
        let deposited_items = rental_system.get_deposited_items(player_id);
        let mut deposited_item_data = Vec::new();
        for rental in deposited_items {
            deposited_item_data.push(SRentalItemData {
                rental_id: rental.rental_id,
                item: rental.item.clone(),
                remaining_days: rental.rental_period_days,
                rental_price: rental.rental_price,
                is_expired: false,
            });
        }

        let pkt = SGetRentedItems {
            rented_items: rented_item_data,
            deposited_items: deposited_item_data,
        };
        
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
    
    pub fn handle_item_rental_request(&mut self, _msg: CItemRentalRequest, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let player_id = self.current_char_index.unwrap_or(-1);
        let rental_system = {
            let world = self.world.lock().unwrap();
            world.get_item_rental_system()
        };

        // Check if player can rent more items
        if !rental_system.can_rent_more(player_id) {
            let pkt = SItemRentalRequest {
                success: false,
                message: "Maximum rental limit reached".to_string(),
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Send available rental items
        // In a real implementation, this would show items available for rent
        let pkt = SItemRentalRequest {
            success: true,
            message: "Rental market available".to_string(),
        };
        
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
    
    pub fn handle_item_rental_fee(&mut self, msg: CItemRentalFee, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Store rental fee setting for player
        // This would be used when depositing items
        let pkt = SItemRentalFee {
            success: true,
            fee: msg.amount,
        };
        
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
    
    pub fn handle_item_rental_period(&mut self, msg: CItemRentalPeriod, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Store rental period setting for player
        // This would be used when depositing items
        let pkt = SItemRentalPeriod {
            success: true,
            days: msg.days,
        };
        
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
    
    pub fn handle_deposit_rental_item(&mut self, msg: CDepositRentalItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Get player inventory
        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        // Find the item
        let mut item = None;
        for slot in inv.slots.iter() {
            if let Some(item_data) = slot {
                if item_data.unique_id == msg.unique_id {
                    item = Some(item_data.clone());
                    break;
                }
            }
        }

        let item = match item {
            Some(i) => i,
            None => {
                let pkt = SDepositRentalItem {
                    success: false,
                    message: "Item not found".to_string(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        // Calculate deposit and rental fees
        let rental_system = {
            let world = self.world.lock().unwrap();
            world.get_item_rental_system()
        };

        let item_value = self.get_item_value(&item);
        let deposit = rental_system.calculate_deposit(item_value);
        let rental_cost = rental_system.calculate_rental_cost(
            item_value,
            msg.period_days,
            msg.rental_fee,
        );

        // Check if player has enough gold for deposit
        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => {
                let pkt = SDepositRentalItem {
                    success: false,
                    message: "Failed to get player stats".to_string(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        if stats.gold < deposit as i64 {
            let pkt = SDepositRentalItem {
                success: false,
                message: "Not enough gold for deposit".to_string(),
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Deposit the item
        let player_id = self.current_char_index.unwrap_or(-1);
        let mut rental_system = {
            let mut world = self.world.lock().unwrap();
            world.get_item_rental_system_mut()
        };

        match rental_system.deposit_item(
            item.clone(),
            player_id,
            msg.rental_price,
            msg.period_days,
            msg.rental_fee,
        ) {
            Ok(_rental_id) => {
                // Deduct deposit
                let mut new_stats = stats.clone();
                new_stats.gold -= deposit as i64;
                
                if let (Some(ref account_id), Some(char_idx)) =
                    (self.account_id.as_ref(), self.current_char_index)
                {
                    let _ = self
                        .store
                        .save_character_stats(account_id, char_idx, &new_stats);
                }
                
                self.current_stats = Some(new_stats);

                // Send SLoseGold
                let lose_gold = SLoseGold { gold: deposit };
                if let Ok(raw) = lose_gold.encode() {
                    out.push(Self::encode_raw(raw));
                }

                // Remove item from inventory
                for slot in inv.slots.iter_mut() {
                    if let Some(item_data) = slot {
                        if item_data.unique_id == msg.unique_id {
                            *slot = None;
                            break;
                        }
                    }
                }

                // Update inventory
                {
                    let mut world = self.world.lock().unwrap();
                    world.set_player_items(self.session_id, inv, eq);
                }

                let pkt = SDepositRentalItem {
                    success: true,
                    message: "Item deposited for rental".to_string(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            Err(e) => {
                let pkt = SDepositRentalItem {
                    success: false,
                    message: e,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }
    
    pub fn handle_retrieve_rental_item(&mut self, msg: CRetrieveRentalItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let player_id = self.current_char_index.unwrap_or(-1);
        let mut rental_system = {
            let mut world = self.world.lock().unwrap();
            world.get_item_rental_system_mut()
        };

        match rental_system.retrieve_deposited_item(msg.rental_id, player_id) {
            Ok(item) => {
                // Add item to inventory
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
                    *empty_slot = Some(item);
                    
                    // Update inventory
                    {
                        let mut world = self.world.lock().unwrap();
                        world.set_player_items(self.session_id, inv, eq);
                    }

                    let pkt = SRetrieveRentalItem {
                        success: true,
                        message: "Item retrieved".to_string(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                } else {
                    // Inventory full
                    let pkt = SRetrieveRentalItem {
                        success: false,
                        message: "Inventory full".to_string(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
            Err(e) => {
                let pkt = SRetrieveRentalItem {
                    success: false,
                    message: e,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }
    
    pub fn handle_cancel_item_rental(&mut self, _msg: CCancelItemRental, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Cancel current rental operation
        // This would reset any pending rental transaction
        let pkt = SCancelItemRental {
            success: true,
            message: "Rental cancelled".to_string(),
        };
        
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
    
    pub fn handle_item_rental_lock_fee(&mut self, _msg: CItemRentalLockFee, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Lock in the rental fee
        // This is part of the rental confirmation process
        let pkt = SItemRentalFee {
            success: true,
            fee: 0, // Would return the locked fee
        };
        
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
    
    pub fn handle_item_rental_lock_item(&mut self, _msg: CItemRentalLockItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Lock the selected item for rental
        // This is part of the rental confirmation process
        let pkt = SItemRentalRequest {
            success: true,
            message: "Item locked for rental".to_string(),
        };
        
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
    
    pub fn handle_confirm_item_rental(&mut self, _msg: CConfirmItemRental, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Confirm and finalize the rental
        // This would complete the rental transaction
        let pkt = SItemRentalRequest {
            success: true,
            message: "Rental confirmed".to_string(),
        };
        
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    /// Helper function to get item value
    fn get_item_value(&self, item: &crystal_shared_proto::item_types::UserItemData) -> u32 {
        // In a real implementation, this would look up the item's value from the database
        // For now, return a default value based on item grade
        // Use item_index to determine value ranges
        match item.item_index {
            0..=1000 => 1000,    // Common
            1001..=2000 => 5000,    // Uncommon
            2001..=3000 => 20000,   // Rare
            3001..=4000 => 50000,   // Epic
            4001..=5000 => 100000,  // Legendary
            _ => 1000,
        }
    }
}

// Helper struct for rental item data
#[derive(Clone, Debug)]
struct SRentalItemData {
    rental_id: u64,
    item: crystal_shared_proto::item_types::UserItemData,
    remaining_days: u32,
    rental_price: u32,
    is_expired: bool,
}
