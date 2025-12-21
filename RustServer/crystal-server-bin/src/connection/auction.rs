use crystal_shared_proto::npc::{
    CAuctionSearch, CAuctionBid, CAuctionCreate, CAuctionCancel,
    SAuctionSearch, SAuctionBid, SAuctionCreate, SAuctionCancel
};
use crystal_shared_proto::item::SRefreshItem;
use crystal_shared_proto::user::status::SLoseGold;
use crystal_server_core::auction::{AuctionHouse, MarketItemType, ClientAuction};
use crate::connection::login::{LoginConnection, Stage};

// Auction handlers for LoginConnection
impl LoginConnection {
    pub fn handle_auction_search(&mut self, msg: CAuctionSearch, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let auction_house = {
            let world = self.world.lock().unwrap();
            world.get_auction_house().clone()
        };

        let item_type = if msg.item_type == 255 {
            None
        } else {
            Some(MarketItemType::from(msg.item_type))
        };

        let results = auction_house.search_auctions(
            item_type,
            Some(msg.min_level),
            Some(msg.max_level),
            None, // TODO: Implement search text
            msg.page as usize,
            20, // Page size
        );

        let pkt = SAuctionSearch {
            page: msg.page,
            auctions: results.into_iter().map(|a| a.into()).collect(),
        };
        
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
    
    pub fn handle_auction_bid(&mut self, msg: CAuctionBid, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Get current gold
        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => return,
        };

        if stats.gold < msg.bid_amount as i64 {
            // Send error response
            let pkt = SAuctionBid {
                success: false,
                message: "Not enough gold".to_string(),
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        let mut auction_house = {
            let mut world = self.world.lock().unwrap();
            world.get_auction_house_mut()
        };

        let user_index = self.current_char_index.unwrap_or(-1);
        
        match auction_house.place_bid(msg.auction_id, user_index, msg.bid_amount) {
            Ok(()) => {
                // Deduct gold from bidder
                let mut new_stats = stats.clone();
                new_stats.gold -= msg.bid_amount as i64;
                
                if let (Some(ref account_id), Some(char_idx)) =
                    (self.account_id.as_ref(), self.current_char_index)
                {
                    let _ = self
                        .store
                        .save_character_stats(account_id, char_idx, &new_stats);
                }
                
                self.current_stats = Some(new_stats);

                // Send SLoseGold
                let lose_gold = SLoseGold { gold: msg.bid_amount };
                if let Ok(raw) = lose_gold.encode() {
                    out.push(Self::encode_raw(raw));
                }

                // Send success response
                let pkt = SAuctionBid {
                    success: true,
                    message: "Bid placed successfully".to_string(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            Err(e) => {
                let pkt = SAuctionBid {
                    success: false,
                    message: e,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }
    
    pub fn handle_auction_create(&mut self, msg: CAuctionCreate, out: &mut Vec<Vec<u8>>) {
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
                let pkt = SAuctionCreate {
                    success: false,
                    message: "Item not found".to_string(),
                    auction_id: 0,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        let item = inv.slots[item_index].take().unwrap();
        
        // Check gold for listing fee
        let auction_house = {
            let world = self.world.lock().unwrap();
            world.get_auction_house().clone()
        };
        
        let listing_fee = auction_house.calculate_listing_fee(MarketItemType::from(msg.item_type));
        
        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => {
                inv.slots[item_index] = Some(item);
                return;
            }
        };

        if stats.gold < listing_fee as i64 {
            let pkt = SAuctionCreate {
                success: false,
                message: "Not enough gold for listing fee".to_string(),
                auction_id: 0,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            inv.slots[item_index] = Some(item);
            return;
        }

        // Create auction
        let mut auction_house = {
            let mut world = self.world.lock().unwrap();
            world.get_auction_house_mut()
        };

        let user_index = self.current_char_index.unwrap_or(-1);
        
        match auction_house.create_auction(
            user_index,
            item.clone(),
            msg.price,
            MarketItemType::from(msg.item_type),
        ) {
            Ok(auction_id) => {
                // Deduct listing fee
                let mut new_stats = stats.clone();
                new_stats.gold -= listing_fee as i64;
                
                if let (Some(ref account_id), Some(char_idx)) =
                    (self.account_id.as_ref(), self.current_char_index)
                {
                    let _ = self
                        .store
                        .save_character_stats(account_id, char_idx, &new_stats);
                }
                
                self.current_stats = Some(new_stats);

                // Send SLoseGold
                let lose_gold = SLoseGold { gold: listing_fee };
                if let Ok(raw) = lose_gold.encode() {
                    out.push(Self::encode_raw(raw));
                }

                // Send success response
                let pkt = SAuctionCreate {
                    success: true,
                    message: "Auction created successfully".to_string(),
                    auction_id,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            Err(e) => {
                inv.slots[item_index] = Some(item);
                
                let pkt = SAuctionCreate {
                    success: false,
                    message: e,
                    auction_id: 0,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }

        // Update inventory (item removed if auction created)
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv, eq);
        }
    }
    
    pub fn handle_auction_cancel(&mut self, msg: CAuctionCancel, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let mut auction_house = {
            let mut world = self.world.lock().unwrap();
            world.get_auction_house_mut()
        };

        let user_index = self.current_char_index.unwrap_or(-1);
        
        match auction_house.cancel_auction(msg.auction_id, user_index) {
            Ok(item) => {
                // Return item to player inventory
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

                    // Send success response
                    let pkt = SAuctionCancel {
                        success: true,
                        message: "Auction cancelled successfully".to_string(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                } else {
                    // Inventory full
                    let pkt = SAuctionCancel {
                        success: false,
                        message: "Inventory full".to_string(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
            Err(e) => {
                let pkt = SAuctionCancel {
                    success: false,
                    message: e,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }
}
