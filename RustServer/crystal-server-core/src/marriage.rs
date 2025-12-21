use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarriageInfo {
    pub player_id: i32,
    pub partner_id: i32,
    pub marriage_date: u64, // Unix timestamp
    pub allow_recall: bool, // Allow partner to teleport to you
}

impl MarriageInfo {
    pub fn new(player_id: i32, partner_id: i32) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            player_id,
            partner_id,
            marriage_date: now,
            allow_recall: true,
        }
    }
    
    pub fn is_married(&self) -> bool {
        self.partner_id != 0
    }
    
    pub fn get_marriage_duration_days(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        (now - self.marriage_date) / 86400 // Convert to days
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarriageRequest {
    pub requester_id: i32,
    pub target_id: i32,
    pub request_time: u64,
    pub expires_at: u64, // Unix timestamp when request expires
}

impl MarriageRequest {
    pub fn new(requester_id: i32, target_id: i32) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Requests expire after 5 minutes
        let expires_at = now + 300;
        
        Self {
            requester_id,
            target_id,
            request_time: now,
            expires_at,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now >= self.expires_at
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DivorceRequest {
    pub initiator_id: i32,
    pub partner_id: i32,
    pub request_time: u64,
    pub expires_at: u64,
}

impl DivorceRequest {
    pub fn new(initiator_id: i32, partner_id: i32) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Requests expire after 5 minutes
        let expires_at = now + 300;
        
        Self {
            initiator_id,
            partner_id,
            request_time: now,
            expires_at,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now >= self.expires_at
    }
}

#[derive(Clone, Debug)]
pub struct MarriageSystem {
    // Map of player_id to marriage info
    marriages: HashMap<i32, MarriageInfo>,
    // Pending marriage requests
    pending_requests: HashMap<i32, MarriageRequest>,
    // Pending divorce requests
    pending_divorces: HashMap<i32, DivorceRequest>,
}

impl MarriageSystem {
    pub fn new() -> Self {
        Self {
            marriages: HashMap::new(),
            pending_requests: HashMap::new(),
            pending_divorces: HashMap::new(),
        }
    }
    
    /// Check if a player is married
    pub fn is_married(&self, player_id: i32) -> bool {
        self.marriages
            .get(&player_id)
            .map(|m| m.is_married())
            .unwrap_or(false)
    }
    
    /// Get marriage info for a player
    pub fn get_marriage_info(&self, player_id: i32) -> Option<&MarriageInfo> {
        self.marriages.get(&player_id)
    }
    
    /// Get partner ID for a player
    pub fn get_partner_id(&self, player_id: i32) -> i32 {
        self.marriages
            .get(&player_id)
            .map(|m| m.partner_id)
            .unwrap_or(0)
    }
    
    /// Send marriage request
    pub fn send_marriage_request(&mut self, requester_id: i32, target_id: i32) -> Result<(), String> {
        // Check if requester is already married
        if self.is_married(requester_id) {
            return Err("You are already married".to_string());
        }
        
        // Check if target is already married
        if self.is_married(target_id) {
            return Err("Target is already married".to_string());
        }
        
        // Check if requester already has a pending request to this target
        if self.pending_requests.contains_key(&target_id) {
            let existing = self.pending_requests.get(&target_id).unwrap();
            if existing.requester_id == requester_id {
                return Err("You already sent a marriage request".to_string());
            }
        }
        
        // Check if target already has a pending request
        if self.pending_requests.contains_key(&target_id) {
            return Err("Target has a pending marriage request".to_string());
        }
        
        // Create and store the request
        let request = MarriageRequest::new(requester_id, target_id);
        self.pending_requests.insert(target_id, request);
        
        Ok(())
    }
    
    /// Accept marriage request
    pub fn accept_marriage_request(&mut self, target_id: i32) -> Result<MarriageInfo, String> {
        let request = self.pending_requests
            .remove(&target_id)
            .ok_or("No pending marriage request".to_string())?;
        
        if request.is_expired() {
            return Err("Request has expired".to_string());
        }
        
        // Check if either player got married while request was pending
        if self.is_married(request.requester_id) {
            return Err("Requester is already married".to_string());
        }
        
        if self.is_married(target_id) {
            return Err("Target is already married".to_string());
        }
        
        // Create marriage for both players
        let marriage1 = MarriageInfo::new(request.requester_id, target_id);
        let marriage2 = MarriageInfo::new(target_id, request.requester_id);
        
        self.marriages.insert(request.requester_id, marriage1.clone());
        self.marriages.insert(target_id, marriage2);
        
        Ok(marriage1)
    }
    
    /// Decline marriage request
    pub fn decline_marriage_request(&mut self, target_id: i32) -> Result<(), String> {
        self.pending_requests
            .remove(&target_id)
            .ok_or("No pending marriage request".to_string())?;
        
        Ok(())
    }
    
    /// Send divorce request
    pub fn send_divorce_request(&mut self, initiator_id: i32) -> Result<i32, String> {
        let partner_id = self.get_partner_id(initiator_id);
        if partner_id == 0 {
            return Err("You are not married".to_string());
        }
        
        // Check if there's already a pending divorce
        if self.pending_divorces.contains_key(&partner_id) {
            return Err("Divorce already in progress".to_string());
        }
        
        // Create and store the request
        let request = DivorceRequest::new(initiator_id, partner_id);
        self.pending_divorces.insert(partner_id, request);
        
        Ok(partner_id)
    }
    
    /// Accept divorce request
    pub fn accept_divorce_request(&mut self, partner_id: i32) -> Result<(i32, i32), String> {
        let request = self.pending_divorces
            .remove(&partner_id)
            .ok_or("No pending divorce request".to_string())?;
        
        if request.is_expired() {
            return Err("Request has expired".to_string());
        }
        
        // Remove marriage from both players
        self.marriages.remove(&request.initiator_id);
        self.marriages.remove(&request.partner_id);
        
        Ok((request.initiator_id, request.partner_id))
    }
    
    /// Decline divorce request
    pub fn decline_divorce_request(&mut self, partner_id: i32) -> Result<(), String> {
        self.pending_divorces
            .remove(&partner_id)
            .ok_or("No pending divorce request".to_string())?;
        
        Ok(())
    }
    
    /// Toggle allow recall setting
    pub fn toggle_recall(&mut self, player_id: i32) -> Result<bool, String> {
        if !self.is_married(player_id) {
            return Err("You are not married".to_string());
        }
        
        if let Some(marriage) = self.marriages.get_mut(&player_id) {
            marriage.allow_recall = !marriage.allow_recall;
            Ok(marriage.allow_recall)
        } else {
            Err("Marriage info not found".to_string())
        }
    }
    
    /// Check if player can recall to partner
    pub fn can_recall_to_partner(&self, player_id: i32, partner_id: i32) -> bool {
        if let Some(marriage) = self.marriages.get(&partner_id) {
            marriage.partner_id == player_id && marriage.allow_recall
        } else {
            false
        }
    }
    
    /// Clean up expired requests
    pub fn cleanup_expired_requests(&mut self) {
        // Remove expired marriage requests
        let mut to_remove = Vec::new();
        for (target_id, request) in &self.pending_requests {
            if request.is_expired() {
                to_remove.push(*target_id);
            }
        }
        for target_id in to_remove {
            self.pending_requests.remove(&target_id);
        }
        
        // Remove expired divorce requests
        let mut to_remove = Vec::new();
        for (partner_id, request) in &self.pending_divorces {
            if request.is_expired() {
                to_remove.push(*partner_id);
            }
        }
        for partner_id in to_remove {
            self.pending_divorces.remove(&partner_id);
        }
    }
    
    /// Get pending marriage request for a player
    pub fn get_pending_request(&self, player_id: i32) -> Option<&MarriageRequest> {
        self.pending_requests.get(&player_id)
    }
    
    /// Get pending divorce request for a player
    pub fn get_pending_divorce(&self, player_id: i32) -> Option<&DivorceRequest> {
        self.pending_divorces.get(&player_id)
    }
    
    /// Force divorce (admin command)
    pub fn force_divorce(&mut self, player_id: i32) -> Result<i32, String> {
        let partner_id = self.get_partner_id(player_id);
        if partner_id == 0 {
            return Err("Player is not married".to_string());
        }
        
        // Remove marriage from both players
        self.marriages.remove(&player_id);
        self.marriages.remove(&partner_id);
        
        // Remove any pending requests
        self.pending_requests.remove(&player_id);
        self.pending_requests.remove(&partner_id);
        self.pending_divorces.remove(&player_id);
        self.pending_divorces.remove(&partner_id);
        
        Ok(partner_id)
    }
}
