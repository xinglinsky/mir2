use crystal_shared_proto::npc::{
    CAwakeningNeedMaterials, CAwakeningLockedItem, CAwakening, 
    CDisassembleItem, CDowngradeAwakening, CResetAddedItem,
    SAwakeningNeedMaterials, SAwakeningLockedItem, SAwakening
};
use crystal_shared_proto::item::{SRefreshItem, UserItemData};
use crystal_shared_proto::user::status::SLoseGold;
use crystal_shared_proto::scene::SObjectEffect;
use crystal_server_core::awakening::{Awakening, AwakeType};
use crystal_server_core::awakening;
use rand::Rng;
use crate::connection::login::{LoginConnection, Stage};
use crate::world::WorldProvider;

// Awakening handlers for LoginConnection
impl LoginConnection {
    pub fn handle_awakening_need_materials(&mut self, msg: CAwakeningNeedMaterials, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let awake_type = AwakeType::from_u8(msg.awakening_type);
        if awake_type == AwakeType::None {
            return;
        }

        // Get player inventory
        let (inv, _) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        // Find the item
        for item_slot in inv.slots.iter() {
            if let Some(item) = item_slot {
                // Check if this is the awakening item (would need unique_id in real implementation)
                // For now, we'll just send default materials
                let materials = awakening::get_default_materials();
                let grade = (item.item_grade - 1) as usize;
                let awakening = Awakening::from_data(&item.awake);
                let level = awakening.get_awake_level();
                
                let material_counts = awakening::calculate_material_requirements(&materials, grade, level);
                
                // Find actual material items from world_db
                let mut material_items = [None, None];
                for info in &self.world_db.item_infos {
                    if info.item_grade == item.item_grade && info.item_type == 6 { // Awakening type
                        if info.shape == (msg.awakening_type - 1) {
                            material_items[0] = Some(info.clone());
                        } else if info.shape == 100 {
                            material_items[1] = Some(info.clone());
                        }
                    }
                }
                
                let pkt = SAwakeningNeedMaterials {
                    item_type: msg.awakening_type,
                    materials_bytes: Vec::new(), // TODO: Serialize material items
                };
                
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                break;
            }
        }
    }
    
    pub fn handle_awakening_locked_item(&mut self, msg: CAwakeningLockedItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Check if item is locked for awakening
        let (inv, _) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        let mut locked = false;
        for item_slot in inv.slots.iter() {
            if let Some(item) = item_slot {
                if item.unique_id == msg.unique_id {
                    // Check bind flags
                    if let Some(info) = WorldProvider::get_item_info(&*self.world_db, item.item_index) {
                        const BIND_DONT_UPGRADE: i16 = 0x0020;
                        // Check if item can be awakened based on item type and other properties
                        // For now, we'll assume all items can be awakened unless explicitly blocked
                        locked = (info.bind & BIND_DONT_UPGRADE) != 0;
                    }
                    break;
                }
            }
        }

        let pkt = SAwakeningLockedItem {
            unique_id: msg.unique_id,
            locked,
        };
        
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
    
    pub fn handle_awakening(&mut self, msg: CAwakening, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let awake_type = AwakeType::from_u8(msg.awakening_type);
        if awake_type == AwakeType::None {
            return;
        }

        // Check if player is dead
        let is_dead = {
            let world = self.world.lock().unwrap();
            world
                .player_current_hp_mp(self.session_id)
                .map(|(hp, _)| hp <= 0)
                .unwrap_or(true)
        };

        if is_dead {
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

        // Find the item by unique_id
        let mut item_index = None;
        for (i, item_slot) in inv.slots.iter().enumerate() {
            if let Some(item) = item_slot {
                if item.unique_id == msg.unique_id {
                    item_index = Some(i);
                    break;
                }
            }
        }

        let item_index = match item_index {
            Some(idx) => idx,
            None => {
                // Send error response
                let pkt = SAwakening {
                    result: 255, // Use 255 as error code
                    item_bytes: Vec::new(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        let mut item = inv.slots[item_index].take().unwrap();
        
        // Check if item can be awakened
        if let Some(info) = WorldProvider::get_item_info(&*self.world_db, item.item_index) {
            const BIND_DONT_UPGRADE: i16 = 0x0020;
            if (info.bind & BIND_DONT_UPGRADE) != 0 {
                // Send error response
                let pkt = SAwakening {
                    result: 255, // Use 255 as error code
                    item_bytes: Vec::new(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                inv.slots[item_index] = Some(item);
                return;
            }
        }

        let mut awakening = Awakening::from_data(&item.awake);
        
        // Check if max level
        if awakening.is_max_level() {
            let pkt = SAwakening {
                result: 254, // Use 254 as error code for max level
                item_bytes: Vec::new(),
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            inv.slots[item_index] = Some(item);
            return;
        }

        // Check gold
        let price = awakening::calculate_awakening_price(item.item_grade, awakening.get_awake_level());
        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => {
                inv.slots[item_index] = Some(item);
                return;
            }
        };

        if stats.gold < price as i64 {
            let pkt = SAwakening {
                result: 253, // Use 253 as error code for insufficient gold
                item_bytes: Vec::new(),
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            inv.slots[item_index] = Some(item);
            return;
        }

        // Check materials (simplified - would need full implementation)
        let has_materials = true; // TODO: Implement material checking

        if has_materials {
            // Deduct gold
            let mut new_stats = stats.clone();
            new_stats.gold -= price as i64;
            
            if let (Some(ref account_id), Some(char_idx)) =
                (self.account_id.as_ref(), self.current_char_index)
            {
                let _ = self
                    .store
                    .save_character_stats(account_id, char_idx, &new_stats);
            }
            
            self.current_stats = Some(new_stats);

            // Send SLoseGold
            let lose_gold = SLoseGold { gold: price as u32 };
            if let Ok(raw) = lose_gold.encode() {
                out.push(Self::encode_raw(raw));
            }

            // Perform awakening
            let (result, is_hit) = awakening.upgrade_awake(&item, awake_type);
            
            match result {
                -1 => {
                    // Condition error
                    let pkt = SAwakening {
                        result: 255, // Use 255 as error code
                        item_bytes: Vec::new(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    inv.slots[item_index] = Some(item);
                }
                0 => {
                    // Failed - item destroyed
                    // Send awakening effects
                    self.send_awakening_effect(false, &is_hit, out);
                    
                    let pkt = SAwakening {
                        result: 0,
                        item_bytes: Vec::new(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    // Item is destroyed (not put back)
                }
                1 => {
                    // Success
                    item.awake = awakening.to_data();
                    
                    // Send refresh item
                    if let Ok(refresh) = SRefreshItem::from_user_item(&item) {
                        if let Ok(raw) = refresh.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                    
                    // Send awakening effects
                    self.send_awakening_effect(true, &is_hit, out);
                    
                    let pkt = SAwakening {
                        result: 1,
                        item_bytes: item.encode_to_bytes().unwrap_or_default(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    
                    inv.slots[item_index] = Some(item);
                }
                _ => {}
            }
        } else {
            inv.slots[item_index] = Some(item);
        }

        // Update inventory
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv, eq);
        }
    }
    
    pub fn handle_disassemble_item(&mut self, msg: CDisassembleItem, out: &mut Vec<Vec<u8>>) {
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
        let mut item_index = None;
        for (i, item_slot) in inv.slots.iter().enumerate() {
            if let Some(item) = item_slot {
                if item.unique_id == msg.unique_id {
                    item_index = Some(i);
                    break;
                }
            }
        }

        let item_index = match item_index {
            Some(idx) => idx,
            None => return,
        };

        let item = inv.slots[item_index].take().unwrap();
        
        // Check if item can be disassembled
        if let Some(info) = WorldProvider::get_item_info(&*self.world_db, item.item_index) {
            if info.grade < 2 || info.item_type == 0 { // Not below Rare grade or not weapon
                inv.slots[item_index] = Some(item);
                return;
            }
        }

        // TODO: Generate disassembly drops based on AwakeningDrops
        // For now, just destroy the item
        
        // Update inventory
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv, eq);
        }
    }
    
    pub fn handle_downgrade_awakening(&mut self, msg: CDowngradeAwakening, out: &mut Vec<Vec<u8>>) {
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
        let mut item_index = None;
        for (i, item_slot) in inv.slots.iter().enumerate() {
            if let Some(item) = item_slot {
                if item.unique_id == msg.unique_id {
                    item_index = Some(i);
                    break;
                }
            }
        }

        let item_index = match item_index {
            Some(idx) => idx,
            None => return,
        };

        let mut item = inv.slots[item_index].take().unwrap();
        let mut awakening = Awakening::from_data(&item.awake);
        
        // Check gold
        let price = awakening::calculate_downgrade_price(item.item_grade);
        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => {
                inv.slots[item_index] = Some(item);
                return;
            }
        };

        if stats.gold < price as i64 {
            inv.slots[item_index] = Some(item);
            return;
        }

        // Deduct gold
        let mut new_stats = stats.clone();
        new_stats.gold -= price as i64;
        
        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let _ = self
                .store
                .save_character_stats(account_id, char_idx, &new_stats);
        }
        
        self.current_stats = Some(new_stats);

        // Send SLoseGold
        let lose_gold = SLoseGold { gold: price as u32 };
        if let Ok(raw) = lose_gold.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Downgrade awakening
        let result = awakening.remove_awake();
        
        if result == 1 {
            // Success - reduce durability by 20% chance
            let mut rng = rand::thread_rng();
            if rng.gen_range(0..20) == 0 {
                let new_max_dura = if item.max_dura >= 2000 { item.max_dura - 1000 } else { 1000 };
                item.max_dura = new_max_dura;
                if item.current_dura > new_max_dura {
                    item.current_dura = new_max_dura;
                }
            }
            
            item.awake = awakening.to_data();
            
            // Send refresh item
            if let Ok(refresh) = SRefreshItem::from_user_item(&item) {
                if let Ok(raw) = refresh.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            
            self.send_system_chat(
                &format!("Downgrade success. Level {}", awakening.get_awake_level()),
                out,
            );
        }
        
        inv.slots[item_index] = Some(item);
        
        // Update inventory
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv, eq);
        }
    }
    
    pub fn handle_reset_added_item(&mut self, msg: CResetAddedItem, out: &mut Vec<Vec<u8>>) {
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
        let mut item_index = None;
        for (i, item_slot) in inv.slots.iter().enumerate() {
            if let Some(item) = item_slot {
                if item.unique_id == msg.unique_id {
                    item_index = Some(i);
                    break;
                }
            }
        }

        let item_index = match item_index {
            Some(idx) => idx,
            None => return,
        };

        let item = inv.slots[item_index].take().unwrap();
        
        // Check gold
        const RESET_PRICE: u64 = 100000;
        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => {
                inv.slots[item_index] = Some(item);
                return;
            }
        };

        if stats.gold < RESET_PRICE as i64 {
            inv.slots[item_index] = Some(item);
            return;
        }

        // Deduct gold
        let mut new_stats = stats.clone();
        new_stats.gold -= RESET_PRICE as i64;
        
        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let _ = self
                .store
                .save_character_stats(account_id, char_idx, &new_stats);
        }
        
        self.current_stats = Some(new_stats);

        // Send SLoseGold
        let lose_gold = SLoseGold { gold: RESET_PRICE as u32 };
        if let Ok(raw) = lose_gold.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Reset item - create new item with same basic properties but no added stats
        let mut new_item = UserItemData {
            unique_id: item.unique_id,
            item_index: item.item_index,
            current_dura: item.current_dura,
            max_dura: item.max_dura,
            count: item.count,
            soul_bound_id: item.soul_bound_id,
            identified: item.identified,
            cursed: item.cursed,
            slots: item.slots.clone(),
            gem_count: item.gem_count,
            awake: item.awake.clone(), // Keep awakening
            refined_value: item.refined_value,
            refine_added: item.refine_added,
            refine_success_chance: item.refine_success_chance,
            wedding_ring: item.wedding_ring,
            expire_info: item.expire_info.clone(),
            rental_information: item.rental_information.clone(),
            is_shop_item: item.is_shop_item,
            sealed_info: item.sealed_info.clone(),
            // Reset added_stats
            added_stats: Default::default(),
        };
        
        // Send refresh item
        if let Ok(refresh) = SRefreshItem::from_user_item(&new_item) {
            if let Ok(raw) = refresh.encode() {
                out.push(Self::encode_raw(raw));
            }
        }
        
        inv.slots[item_index] = Some(new_item);
        
        // Update inventory
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv, eq);
        }
    }
    
    pub fn send_awakening_effect(&self, success: bool, is_hit: &[bool], out: &mut Vec<Vec<u8>>) {
        for (i, &hit) in is_hit.iter().enumerate() {
            let effect = if hit {
                if success { 1001 } else { 1003 } // AwakeningHit or AwakeningMiss
            } else {
                if success { 1001 } else { 1003 }
            };
            
            // Send to self
            let pkt = SObjectEffect {
                object_id: self.session_id,
                effect,
                effect_type: 0,
                delay_time: (i * 500) as u32,
                time: 1000, // 1 second duration
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            
            // Broadcast to others
            // TODO: Implement broadcast
        }
        
        // Send final effect
        let final_effect = if success { 1002 } else { 1004 }; // Success or Fail
        let pkt = SObjectEffect {
            object_id: self.session_id,
            effect: final_effect,
            effect_type: 0,
            delay_time: 2500,
            time: 1000, // 1 second duration
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
}
