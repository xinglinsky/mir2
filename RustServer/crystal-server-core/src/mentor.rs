use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MentorInfo {
    pub mentor_id: i32,
    pub mentee_id: i32,
    pub mentorship_date: u64, // Unix timestamp
    pub mentor_exp: u64, // Experience gained by mentor
    pub is_mentor: bool, // True if this record is from mentor's perspective
}

impl MentorInfo {
    pub fn new(mentor_id: i32, mentee_id: i32, is_mentor: bool) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            mentor_id,
            mentee_id,
            mentorship_date: now,
            mentor_exp: 0,
            is_mentor,
        }
    }
    
    pub fn get_mentorship_duration_days(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        (now - self.mentorship_date) / 86400 // Convert to days
    }
    
    /// Check if mentee can graduate (level requirement met)
    pub fn can_graduate(&self, mentee_level: u16, mentor_level: u16) -> bool {
        // Mentee must be at least level 40
        if mentee_level < 40 {
            return false;
        }
        
        // Mentee must be within 10 levels of mentor
        if mentee_level > mentor_level {
            return false;
        }
        
        // Must have been mentor for at least 7 days
        if self.get_mentorship_duration_days() < 7 {
            return false;
        }
        
        true
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MentorRequest {
    pub mentor_id: i32,
    pub mentee_id: i32,
    pub request_time: u64,
    pub expires_at: u64,
}

impl MentorRequest {
    pub fn new(mentor_id: i32, mentee_id: i32) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Requests expire after 5 minutes
        let expires_at = now + 300;
        
        Self {
            mentor_id,
            mentee_id,
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
pub struct MentorSystem {
    // Map of player_id to their mentor/mentee relationship
    relationships: HashMap<i32, MentorInfo>,
    // Pending mentor requests (mentee_id -> request)
    pending_requests: HashMap<i32, MentorRequest>,
    // Configuration
    max_mentees: usize,
    exp_bonus_rate: f32, // Percentage bonus for mentor
    mentee_exp_bonus: f32, // Percentage bonus for mentee
    graduation_bonus_exp: u64, // Bonus exp for graduation
}

impl MentorSystem {
    pub fn new() -> Self {
        Self {
            relationships: HashMap::new(),
            pending_requests: HashMap::new(),
            max_mentees: 3, // Maximum mentees per mentor
            exp_bonus_rate: 0.1, // 10% bonus for mentor
            mentee_exp_bonus: 0.05, // 5% bonus for mentee
            graduation_bonus_exp: 100000, // 100k exp bonus
        }
    }
    
    /// Check if a player is a mentor
    pub fn is_mentor(&self, player_id: i32) -> bool {
        self.relationships
            .get(&player_id)
            .map(|r| r.is_mentor)
            .unwrap_or(false)
    }
    
    /// Check if a player is a mentee
    pub fn is_mentee(&self, player_id: i32) -> bool {
        self.relationships
            .get(&player_id)
            .map(|r| !r.is_mentor)
            .unwrap_or(false)
    }
    
    /// Get mentor info for a player
    pub fn get_mentor_info(&self, player_id: i32) -> Option<&MentorInfo> {
        self.relationships.get(&player_id)
    }
    
    /// Get mentor ID for a mentee
    pub fn get_mentor_id(&self, mentee_id: i32) -> i32 {
        self.relationships
            .get(&mentee_id)
            .filter(|r| !r.is_mentor)
            .map(|r| r.mentor_id)
            .unwrap_or(0)
    }
    
    /// Get all mentees for a mentor
    pub fn get_mentees(&self, mentor_id: i32) -> Vec<i32> {
        self.relationships
            .values()
            .filter(|r| r.is_mentor && r.mentor_id == mentor_id)
            .map(|r| r.mentee_id)
            .collect()
    }
    
    /// Send mentor request (from mentee to mentor)
    pub fn send_mentor_request(&mut self, mentor_id: i32, mentee_id: i32, 
                               mentee_level: u16, mentor_level: u16) -> Result<(), String> {
        // Check level requirements
        if mentee_level >= mentor_level {
            return Err("Mentor must be higher level than mentee".to_string());
        }
        
        if mentee_level < 15 {
            return Err("Mentee must be at least level 15".to_string());
        }
        
        // Check if mentee already has a mentor
        if self.is_mentee(mentee_id) {
            return Err("You already have a mentor".to_string());
        }
        
        // Check if mentor has reached max mentees
        let current_mentees = self.get_mentees(mentor_id).len();
        if current_mentees >= self.max_mentees {
            return Err("Mentor has reached maximum mentees".to_string());
        }
        
        // Check if mentee already has a pending request
        if self.pending_requests.contains_key(&mentee_id) {
            return Err("You already have a pending mentor request".to_string());
        }
        
        // Create and store the request
        let request = MentorRequest::new(mentor_id, mentee_id);
        self.pending_requests.insert(mentee_id, request);
        
        Ok(())
    }
    
    /// Accept mentor request
    pub fn accept_mentor_request(&mut self, mentee_id: i32) -> Result<MentorInfo, String> {
        let request = self.pending_requests
            .remove(&mentee_id)
            .ok_or("No pending mentor request".to_string())?;
        
        if request.is_expired() {
            return Err("Request has expired".to_string());
        }
        
        // Check if mentor still has capacity
        let current_mentees = self.get_mentees(request.mentor_id).len();
        if current_mentees >= self.max_mentees {
            return Err("Mentor has reached maximum mentees".to_string());
        }
        
        // Create mentorship for both players
        let mentor_record = MentorInfo::new(request.mentor_id, request.mentee_id, true);
        let mentee_record = MentorInfo::new(request.mentor_id, request.mentee_id, false);
        
        self.relationships.insert(request.mentor_id, mentor_record.clone());
        self.relationships.insert(request.mentee_id, mentee_record);
        
        Ok(mentor_record)
    }
    
    /// Decline mentor request
    pub fn decline_mentor_request(&mut self, mentee_id: i32) -> Result<(), String> {
        self.pending_requests
            .remove(&mentee_id)
            .ok_or("No pending mentor request".to_string())?;
        
        Ok(())
    }
    
    /// Cancel mentorship (by mentor or mentee)
    pub fn cancel_mentorship(&mut self, player_id: i32) -> Result<i32, String> {
        let relationship = self.relationships
            .get(&player_id)
            .ok_or("No mentorship found".to_string())?
            .clone();
        
        let partner_id = if relationship.is_mentor {
            relationship.mentee_id
        } else {
            relationship.mentor_id
        };
        
        // Remove relationship from both players
        self.relationships.remove(&player_id);
        self.relationships.remove(&partner_id);
        
        // Remove any pending requests
        self.pending_requests.remove(&player_id);
        self.pending_requests.remove(&partner_id);
        
        Ok(partner_id)
    }
    
    /// Graduate mentee
    pub fn graduate_mentee(&mut self, mentee_id: i32, mentee_level: u16, mentor_level: u16) -> Result<u64, String> {
        let relationship = self.relationships
            .get(&mentee_id)
            .filter(|r| !r.is_mentor)
            .ok_or("No mentorship found".to_string())?
            .clone();
        
        if !relationship.can_graduate(mentee_level, mentor_level) {
            return Err("Graduation requirements not met".to_string());
        }
        
        let mentor_id = relationship.mentor_id;
        
        // Remove relationship
        self.relationships.remove(&mentee_id);
        self.relationships.remove(&mentor_id);
        
        // Return graduation bonus
        Ok(self.graduation_bonus_exp)
    }
    
    /// Add mentor experience when mentee gains exp
    pub fn add_mentor_exp(&mut self, mentee_id: i32, exp_gained: u64) -> Option<u64> {
        let mentor_id = self.get_mentor_id(mentee_id);
        if mentor_id == 0 {
            return None;
        }
        
        let mentor_exp = (exp_gained as f32 * self.exp_bonus_rate) as u64;
        
        if let Some(relationship) = self.relationships.get_mut(&mentor_id) {
            relationship.mentor_exp += mentor_exp;
            Some(mentor_exp)
        } else {
            None
        }
    }
    
    /// Calculate experience bonus for mentee
    pub fn calculate_mentee_bonus(&self, base_exp: u64) -> u64 {
        (base_exp as f32 * self.mentee_exp_bonus) as u64
    }
    
    /// Check if two players can form mentor relationship
    pub fn can_mentor(&self, mentor_id: i32, mentee_id: i32, 
                      mentor_level: u16, mentee_level: u16) -> Result<(), String> {
        // Check levels
        if mentee_level >= mentor_level {
            return Err("Mentor must be higher level than mentee".to_string());
        }
        
        if mentee_level < 15 {
            return Err("Mentee must be at least level 15".to_string());
        }
        
        // Check if already in relationship
        if self.is_mentor(mentor_id) && self.is_mentee(mentee_id) {
            let existing_mentor = self.get_mentor_id(mentee_id);
            if existing_mentor == mentor_id {
                return Err("Already in mentor relationship".to_string());
            }
        }
        
        if self.is_mentee(mentor_id) {
            return Err("Mentor cannot be a mentee".to_string());
        }
        
        if self.is_mentor(mentee_id) {
            return Err("Mentee cannot be a mentor".to_string());
        }
        
        // Check mentor capacity
        let current_mentees = self.get_mentees(mentor_id).len();
        if current_mentees >= self.max_mentees {
            return Err("Mentor has reached maximum mentees".to_string());
        }
        
        Ok(())
    }
    
    /// Clean up expired requests
    pub fn cleanup_expired_requests(&mut self) {
        let mut to_remove = Vec::new();
        for (mentee_id, request) in &self.pending_requests {
            if request.is_expired() {
                to_remove.push(*mentee_id);
            }
        }
        for mentee_id in to_remove {
            self.pending_requests.remove(&mentee_id);
        }
    }
    
    /// Get pending mentor request for a player
    pub fn get_pending_request(&self, mentee_id: i32) -> Option<&MentorRequest> {
        self.pending_requests.get(&mentee_id)
    }
    
    /// Get mentor statistics
    pub fn get_mentor_stats(&self, mentor_id: i32) -> Option<(usize, u64)> {
        if !self.is_mentor(mentor_id) {
            return None;
        }
        
        let mentee_count = self.get_mentees(mentor_id).len();
        let total_exp = self.relationships
            .values()
            .filter(|r| r.is_mentor && r.mentor_id == mentor_id)
            .map(|r| r.mentor_exp)
            .sum();
        
        Some((mentee_count, total_exp))
    }
}
