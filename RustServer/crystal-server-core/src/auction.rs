use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crystal_shared_proto::item_types::UserItemData;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketItemType {
    GameShop = 0,
    Consign = 1,
    Auction = 2,
}

impl Default for MarketItemType {
    fn default() -> Self {
        MarketItemType::GameShop
    }
}

impl From<u8> for MarketItemType {
    fn from(value: u8) -> Self {
        match value {
            0 => MarketItemType::GameShop,
            1 => MarketItemType::Consign,
            2 => MarketItemType::Auction,
            _ => MarketItemType::GameShop,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AuctionInfo {
    pub auction_id: u64,
    pub item: UserItemData,
    pub consignment_date: u64, // Unix timestamp
    pub price: u32, // Initial price for auction, fixed price for consign
    pub current_bid: u32, // Current highest bid (auction only)
    pub seller_index: i32,
    pub current_buyer_index: i32, // -1 if no bid yet
    pub expired: bool,
    pub sold: bool,
    pub item_type: MarketItemType,
}

impl AuctionInfo {
    pub fn new(
        auction_id: u64,
        seller_index: i32,
        item: UserItemData,
        price: u32,
        item_type: MarketItemType,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let current_bid = if item_type == MarketItemType::Auction {
            price
        } else {
            0
        };
        
        Self {
            auction_id,
            item,
            consignment_date: now,
            price,
            current_bid,
            seller_index,
            current_buyer_index: -1,
            expired: false,
            sold: false,
            item_type,
        }
    }
    
    pub fn place_bid(&mut self, buyer_index: i32, bid_amount: u32) -> bool {
        if self.item_type != MarketItemType::Auction {
            return false;
        }
        
        if self.expired || self.sold {
            return false;
        }
        
        // Bid must be higher than current bid
        if bid_amount <= self.current_bid {
            return false;
        }
        
        // Minimum bid increment (10% of current bid)
        let min_increment = self.current_bid / 10;
        if bid_amount < self.current_bid + min_increment {
            return false;
        }
        
        self.current_bid = bid_amount;
        self.current_buyer_index = buyer_index;
        true
    }
    
    pub fn check_expired(&mut self, auction_duration_hours: u64) -> bool {
        if self.expired {
            return true;
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let elapsed = now - self.consignment_date;
        let duration_seconds = auction_duration_hours * 3600;
        
        if elapsed >= duration_seconds {
            self.expired = true;
            
            // If auction has bids, mark as sold
            if self.item_type == MarketItemType::Auction && self.current_buyer_index != -1 {
                self.sold = true;
            }
            
            true
        } else {
            false
        }
    }
    
    pub fn get_display_price(&self) -> u32 {
        match self.item_type {
            MarketItemType::Auction => self.current_bid,
            _ => self.price,
        }
    }
    
    pub fn get_seller_label(&self, user_match: bool) -> String {
        match self.item_type {
            MarketItemType::GameShop => String::new(),
            MarketItemType::Consign => {
                if user_match {
                    if self.sold {
                        "Sold".to_string()
                    } else if self.expired {
                        "Expired".to_string()
                    } else {
                        "For Sale".to_string()
                    }
                } else {
                    // Would need to fetch seller name from player data
                    format!("Seller_{}", self.seller_index)
                }
            }
            MarketItemType::Auction => {
                if user_match {
                    if self.sold {
                        "Sold".to_string()
                    } else if self.expired {
                        "Expired".to_string()
                    } else if self.current_bid > self.price {
                        "Bid Met".to_string()
                    } else {
                        "No Bid".to_string()
                    }
                } else {
                    // Would need to fetch seller name from player data
                    format!("Seller_{}", self.seller_index)
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClientAuction {
    pub auction_id: u64,
    pub item: UserItemData,
    pub seller: String,
    pub price: u32,
    pub consignment_date: u64,
    pub item_type: MarketItemType,
}

impl From<&AuctionInfo> for ClientAuction {
    fn from(auction: &AuctionInfo) -> Self {
        Self {
            auction_id: auction.auction_id,
            item: auction.item.clone(),
            seller: auction.get_seller_label(false),
            price: auction.get_display_price(),
            consignment_date: auction.consignment_date,
            item_type: auction.item_type,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AuctionHouse {
    pub auctions: HashMap<u64, AuctionInfo>,
    pub next_auction_id: u64,
    pub auction_duration_hours: u64,
    pub consignment_fee_rate: f32, // Percentage of sale price
    pub listing_fee: u32, // Fixed fee for listing
}

impl AuctionHouse {
    pub fn new() -> Self {
        Self {
            auctions: HashMap::new(),
            next_auction_id: 1,
            auction_duration_hours: 48, // 48 hours default
            consignment_fee_rate: 0.05, // 5% fee
            listing_fee: 1000, // 1000 gold listing fee
        }
    }
    
    pub fn create_auction(
        &mut self,
        seller_index: i32,
        item: UserItemData,
        price: u32,
        item_type: MarketItemType,
    ) -> Result<u64, String> {
        // Validate item
        if item.unique_id == 0 {
            return Err("Invalid item".to_string());
        }
        
        // Validate price
        if price == 0 {
            return Err("Price must be greater than 0".to_string());
        }
        
        // Check if auction type is valid
        if item_type == MarketItemType::GameShop {
            return Err("Cannot create GameShop auctions".to_string());
        }
        
        let auction_id = self.next_auction_id;
        self.next_auction_id += 1;
        
        let auction = AuctionInfo::new(auction_id, seller_index, item, price, item_type);
        self.auctions.insert(auction_id, auction);
        
        Ok(auction_id)
    }
    
    pub fn place_bid(&mut self, auction_id: u64, buyer_index: i32, bid_amount: u32) -> Result<(), String> {
        let auction = self.auctions
            .get_mut(&auction_id)
            .ok_or("Auction not found")?;
        
        if auction.place_bid(buyer_index, bid_amount) {
            Ok(())
        } else {
            Err("Failed to place bid".to_string())
        }
    }
    
    pub fn cancel_auction(&mut self, auction_id: u64, seller_index: i32) -> Result<UserItemData, String> {
        let auction = self.auctions
            .get_mut(&auction_id)
            .ok_or("Auction not found")?;
        
        if auction.seller_index != seller_index {
            return Err("Not the seller".to_string());
        }
        
        if auction.sold {
            return Err("Item already sold".to_string());
        }
        
        if auction.current_buyer_index != -1 {
            return Err("Cannot cancel auction with bids".to_string());
        }
        
        let item = auction.item.clone();
        self.auctions.remove(&auction_id);
        Ok(item)
    }
    
    pub fn search_auctions(
        &self,
        item_type: Option<MarketItemType>,
        min_level: Option<u8>,
        max_level: Option<u8>,
        search_text: Option<String>,
        page: usize,
        page_size: usize,
    ) -> Vec<ClientAuction> {
        let mut results: Vec<ClientAuction> = Vec::new();
        
        for auction in self.auctions.values() {
            // Skip expired auctions that aren't sold
            if auction.expired && !auction.sold {
                continue;
            }
            
            // Filter by item type
            if let Some(filter_type) = item_type {
                if auction.item_type != filter_type {
                    continue;
                }
            }
            
            // Filter by level (would need item info)
            // TODO: Implement level filtering when item info is available
            
            // Filter by search text (would need item name)
            // TODO: Implement text search when item names are available
            
            results.push(ClientAuction::from(auction));
        }
        
        // Sort by consignment date (newest first)
        results.sort_by(|a, b| b.consignment_date.cmp(&a.consignment_date));
        
        // Paginate
        let start = page * page_size;
        let end = (start + page_size).min(results.len());
        
        if start < results.len() {
            results[start..end].to_vec()
        } else {
            Vec::new()
        }
    }
    
    pub fn get_user_auctions(&self, user_index: i32, page: usize, page_size: usize) -> Vec<ClientAuction> {
        let mut results: Vec<ClientAuction> = Vec::new();
        
        for auction in self.auctions.values() {
            if auction.seller_index == user_index {
                results.push(ClientAuction::from(auction));
            }
        }
        
        // Sort by consignment date (newest first)
        results.sort_by(|a, b| b.consignment_date.cmp(&a.consignment_date));
        
        // Paginate
        let start = page * page_size;
        let end = (start + page_size).min(results.len());
        
        if start < results.len() {
            results[start..end].to_vec()
        } else {
            Vec::new()
        }
    }
    
    pub fn process_expired_auctions(&mut self) -> Vec<(u64, i32, UserItemData)> {
        let mut expired_items = Vec::new();
        let mut to_remove = Vec::new();
        
        for (auction_id, auction) in &mut self.auctions {
            if auction.check_expired(self.auction_duration_hours) {
                if !auction.sold && auction.current_buyer_index == -1 {
                    // Return unsold items to seller
                    expired_items.push((auction.auction_id, auction.seller_index, auction.item.clone()));
                    to_remove.push(*auction_id);
                }
            }
        }
        
        for auction_id in to_remove {
            self.auctions.remove(&auction_id);
        }
        
        expired_items
    }
    
    pub fn get_auction(&self, auction_id: u64) -> Option<&AuctionInfo> {
        self.auctions.get(&auction_id)
    }
    
    pub fn calculate_listing_fee(&self, item_type: MarketItemType) -> u32 {
        match item_type {
            MarketItemType::GameShop => 0,
            _ => self.listing_fee,
        }
    }
    
    pub fn calculate_consignment_fee(&self, sale_price: u32) -> u32 {
        (sale_price as f32 * self.consignment_fee_rate) as u32
    }
}
