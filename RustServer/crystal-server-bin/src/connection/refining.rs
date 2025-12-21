use crystal_shared_proto::npc::{
    CDepositRefineItem, CRetrieveRefineItem, CRefineCancel, CRefineItem, CCheckRefine,
    SDepositRefineItem, SRetrieveRefineItem, SRefineCancel, SRefineItem, SCheckRefine
};
use crystal_shared_proto::item::SRefreshItem;
use crystal_server_core::refining::{RefiningSystem, RefiningSession};
use crate::connection::login::{LoginConnection, Stage};

// Refining handlers for LoginConnection
impl LoginConnection {
    pub fn handle_deposit_refine_item(&mut self, msg: CDepositRefineItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Get player inventory
        let (inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        // Find the item
        let item = if msg.from < inv.slots.len() {
            inv.slots[msg.from].clone()
        } else {
            None
        };

        let item = match item {
            Some(i) => i,
            None => {
                let pkt = SDepositRefineItem {
                    success: false,
                    message: "Item not found".to_string(),
                    from: msg.from,
                    to: msg.to,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        // Deposit item
        let player_id = self.current_char_index.unwrap_or(-1);
        let mut refining_system = {
            let mut world = self.world.lock().unwrap();
            world.get_refining_system_mut()
        };

        match refining_system.deposit_item(player_id, item) {
            Ok(slot) => {
                // Remove from inventory
                let (mut inv, eq) = {
                    let world = self.world.lock().unwrap();
                    world
                        .player_items(self.session_id)
                        .unwrap_or((
                            crystal_server_core::item::Inventory::new_default(),
                            crystal_server_core::item::Equipment::new_default(),
                        ))
                };
                
                if msg.from < inv.slots.len() {
                    inv.slots[msg.from] = None;
                }

                // Update inventory
                {
                    let mut world = self.world.lock().unwrap();
                    world.set_player_items(self.session_id, inv, eq);
                }

                let pkt = SDepositRefineItem {
                    success: true,
                    message: "Item deposited for refining".to_string(),
                    from: msg.from,
                    to: slot,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            Err(e) => {
                let pkt = SDepositRefineItem {
                    success: false,
                    message: e,
                    from: msg.from,
                    to: msg.to,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }
    
    pub fn handle_retrieve_refine_item(&mut self, msg: CRetrieveRefineItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let player_id = self.current_char_index.unwrap_or(-1);
        let mut refining_system = {
            let mut world = self.world.lock().unwrap();
            world.get_refining_system_mut()
        };

        match refining_system.retrieve_item(player_id, msg.from) {
            Ok(item) => {
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
                if msg.to < inv.slots.len() && inv.slots[msg.to].is_none() {
                    inv.slots[msg.to] = Some(item);
                    
                    // Update inventory
                    {
                        let mut world = self.world.lock().unwrap();
                        world.set_player_items(self.session_id, inv, eq);
                    }

                    let pkt = SRetrieveRefineItem {
                        success: true,
                        message: "Item retrieved".to_string(),
                        from: msg.from,
                        to: msg.to,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                } else {
                    // Inventory full
                    let pkt = SRetrieveRefineItem {
                        success: false,
                        message: "Inventory full".to_string(),
                        from: msg.from,
                        to: msg.to,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
            Err(e) => {
                let pkt = SRetrieveRefineItem {
                    success: false,
                    message: e,
                    from: msg.from,
                    to: msg.to,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }
    
    pub fn handle_refine_cancel(&mut self, _msg: CRefineCancel, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let player_id = self.current_char_index.unwrap_or(-1);
        let mut refining_system = {
            let mut world = self.world.lock().unwrap();
            world.get_refining_system_mut()
        };

        match refining_system.cancel_refining(player_id) {
            Ok(returned_items) => {
                // Return all items to inventory
                let (mut inv, eq) = {
                    let world = self.world.lock().unwrap();
                    world
                        .player_items(self.session_id)
                        .unwrap_or((
                            crystal_server_core::item::Inventory::new_default(),
                            crystal_server_core::item::Equipment::new_default(),
                        ))
                };

                let mut returned_count = 0;
                for item in returned_items {
                    // Find empty slot
                    if let Some(empty_slot) = inv.slots.iter_mut().find(|slot| slot.is_none()) {
                        *empty_slot = Some(item);
                        returned_count += 1;
                    }
                }

                // Update inventory
                {
                    let mut world = self.world.lock().unwrap();
                    world.set_player_items(self.session_id, inv, eq);
                }

                let pkt = SRefineCancel {
                    success: true,
                    message: format!("Refining cancelled. Returned {} items.", returned_count),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            Err(e) => {
                let pkt = SRefineCancel {
                    success: false,
                    message: e,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }
    
    pub fn handle_refine_item(&mut self, msg: CRefineItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let player_id = self.current_char_index.unwrap_or(-1);
        let player_luck = {
            let world = self.world.lock().unwrap();
            world.get_player_luck(player_id).unwrap_or(0)
        };

        let mut refining_system = {
            let mut world = self.world.lock().unwrap();
            world.get_refining_system_mut()
        };

        // Start refining with materials from deposit slots
        // For simplicity, we'll use the first 2 material slots
        let material_slots = vec![0, 1];
        
        match refining_system.start_refining(player_id, 0, material_slots, player_luck) {
            Ok(session) => {
                let pkt = SRefineItem {
                    success: true,
                    message: format!("Refining started. Will complete in {} minutes.", 
                        (session.completion_time - session.start_time) / 60),
                    unique_id: session.target_item.unique_id,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            Err(e) => {
                let pkt = SRefineItem {
                    success: false,
                    message: e,
                    unique_id: 0,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }
    
    pub fn handle_check_refine(&mut self, msg: CCheckRefine, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let player_id = self.current_char_index.unwrap_or(-1);
        let refining_system = {
            let world = self.world.lock().unwrap();
            world.get_refining_system()
        };

        // Get deposited items
        let deposited_items = refining_system.get_deposited_items(player_id);
        
        // Find the item to check
        let item = deposited_items
            .iter()
            .find(|i| i.unique_id == msg.unique_id);

        if let Some(item) = item {
            if item.refine_added == 0 {
                let pkt = SCheckRefine {
                    success: false,
                    message: "Item has not been refined yet".to_string(),
                    can_refine: true,
                    current_level: 0,
                    max_level: 10,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }

            // Calculate success chance for next refine
            let recipe = refining_system.get_recipe_for_item(item.item_type);
            let (can_refine, max_level) = if let Some(recipe) = recipe {
                let can_refine = item.refine_added < recipe.max_refine_level;
                (can_refine, recipe.max_refine_level)
            } else {
                (false, 0)
            };

            let pkt = SCheckRefine {
                success: true,
                message: format!("Current refine level: {}", item.refine_added),
                can_refine,
                current_level: item.refine_added,
                max_level,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
        } else {
            // Check inventory item
            let (inv, _eq) = {
                let world = self.world.lock().unwrap();
                world
                    .player_items(self.session_id)
                    .unwrap_or((
                        crystal_server_core::item::Inventory::new_default(),
                        crystal_server_core::item::Equipment::new_default(),
                    ))
            };

            let inventory_item = inv.slots
                .iter()
                .find_map(|slot| {
                    if let Some(item) = slot {
                        if item.unique_id == msg.unique_id {
                            Some(item.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });

            if let Some(item) = inventory_item {
                let can_refine = refining_system.can_refine_item(&item);
                let max_level = refining_system
                    .get_recipe_for_item(item.item_type)
                    .map(|r| r.max_refine_level)
                    .unwrap_or(0);

                let pkt = SCheckRefine {
                    success: true,
                    message: format!("Current refine level: {}", item.refine_added),
                    can_refine,
                    current_level: item.refine_added,
                    max_level,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            } else {
                let pkt = SCheckRefine {
                    success: false,
                    message: "Item not found".to_string(),
                    can_refine: false,
                    current_level: 0,
                    max_level: 0,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }

    /// Update refining system (called periodically)
    pub fn update_refining(&mut self, out: &mut Vec<Vec<u8>>) {
        let player_id = self.current_char_index.unwrap_or(-1);
        let player_luck = {
            let world = self.world.lock().unwrap();
            world.get_player_luck(player_id).unwrap_or(0)
        };

        let mut refining_system = {
            let mut world = self.world.lock().unwrap();
            world.get_refining_system_mut()
        };

        // Check for completed sessions
        let completed_sessions = refining_system.update_sessions();
        
        for session_id in completed_sessions {
            // Complete the refining
            match refining_system.complete_refining(session_id, player_luck) {
                Ok(refined_item) => {
                    // Add refined item to inventory
                    self.add_refined_item_to_inventory(refined_item, out);
                    
                    // Send notification
                    let pkt = SRefineItem {
                        success: true,
                        message: "Refining completed successfully!".to_string(),
                        unique_id: 0,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
                Err(e) => {
                    // Send failure notification
                    let pkt = SRefineItem {
                        success: false,
                        message: e,
                        unique_id: 0,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        }
    }

    /// Add refined item to inventory
    fn add_refined_item_to_inventory(&mut self, item: crystal_shared_proto::item_types::UserItemData, out: &mut Vec<Vec<u8>>) {
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

            // Send refresh
            let pkt = SRefreshItem {
                item_bytes: vec![], // Empty bytes for refresh notification
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
        }
    }
}
