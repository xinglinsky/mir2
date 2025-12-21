use crystal_shared_proto::npc::{
    CAddMentor, CMentorReply, CAllowMentor, CCancelMentor,
    SMentorRequest, SMentorReply, SAllowMentor, SCancelMentor
};
use crystal_shared_proto::user::status::SRefreshCharacter;
use crystal_server_core::mentor::{MentorSystem, MentorInfo};
use crate::connection::login::{LoginConnection, Stage};

// Mentor handlers for LoginConnection
impl LoginConnection {
    pub fn handle_add_mentor(&mut self, msg: CAddMentor, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Get target player info
        let target_name = msg.mentor_name.clone();
        let target_info = {
            let world = self.world.lock().unwrap();
            world.get_character_by_name(&target_name)
        };

        let target_info = match target_info {
            Some(info) => info,
            None => {
                // Send error response
                let pkt = SMentorRequest {
                    success: false,
                    message: "Mentor not found".to_string(),
                    mentee_name: String::new(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        let mentee_id = self.current_char_index.unwrap_or(-1);
        let mentor_id = target_info.index;

        // Cannot mentor yourself
        if mentee_id == mentor_id {
            let pkt = SMentorRequest {
                success: false,
                message: "Cannot mentor yourself".to_string(),
                mentee_name: String::new(),
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Get player levels
        let (mentee_level, mentor_level) = {
            let world = self.world.lock().unwrap();
            let mentee_lvl = world.get_player_level(mentee_id).unwrap_or(0);
            let mentor_lvl = world.get_player_level(mentor_id).unwrap_or(0);
            (mentee_lvl, mentor_lvl)
        };

        // Send mentor request
        let result = {
            let mut world = self.world.lock().unwrap();
            let mut mentor_system = world.get_mentor_system_mut();
            
            mentor_system.send_mentor_request(mentor_id, mentee_id, mentee_level, mentor_level)
        };

        match result {
            Ok(()) => {
                // Get mentee name
                let mentee_name = match self.current_character_name.as_ref() {
                    Some(name) => name.clone(),
                    None => "Unknown".to_string(),
                };

                // Send success response to mentee
                let pkt = SMentorRequest {
                    success: true,
                    message: "Mentor request sent".to_string(),
                    mentee_name: mentee_name.clone(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }

                // Send notification to mentor if online
                let world = self.world.lock().unwrap();
                if let Some(mentor_conn) = world.get_connection_by_char_index(mentor_id) {
                    let notify_pkt = SMentorRequest {
                        success: true,
                        message: format!("{} has requested you as their mentor", mentee_name),
                        mentee_name,
                    };
                    if let Ok(raw) = notify_pkt.encode() {
                        let _ = mentor_conn.send_raw_packet(raw);
                    }
                }
            }
            Err(e) => {
                let pkt = SMentorRequest {
                    success: false,
                    message: e,
                    mentee_name: String::new(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }
    
    pub fn handle_mentor_reply(&mut self, msg: CMentorReply, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let mentor_id = self.current_char_index.unwrap_or(-1);
        let mut mentor_system = {
            let mut world = self.world.lock().unwrap();
            world.get_mentor_system_mut()
        };

        if msg.accept_invite {
            // Accept mentor request
            match mentor_system.accept_mentor_request(mentor_id) {
                Ok(mentorship) => {
                    // Send success response to mentor
                    let pkt = SMentorReply {
                        success: true,
                        message: "You are now a mentor!".to_string(),
                        mentee_name: String::new(), // Would need to fetch mentee name
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }

                    // Notify mentee
                    let world = self.world.lock().unwrap();
                    if let Some(mentee_conn) = world.get_connection_by_char_index(mentorship.mentee_id) {
                        let mentee_pkt = SMentorReply {
                            success: true,
                            message: "You now have a mentor!".to_string(),
                            mentee_name: match self.current_character_name.as_ref() {
                                Some(name) => name.clone(),
                                None => "Unknown".to_string(),
                            },
                        };
                        if let Ok(raw) = mentee_pkt.encode() {
                            let _ = mentee_conn.send_raw_packet(raw);
                        }
                    }

                    // Update character info for both players
                    self.update_mentor_status(mentorship.mentee_id, true, out);
                    
                    // Update mentee's status if online
                    if let Some(mut mentee_conn) = world.get_connection_by_char_index_mut(mentorship.mentee_id) {
                        let mut mentee_out = Vec::new();
                        mentee_conn.update_mentor_status(mentor_id, true, &mut mentee_out);
                        let _ = mentee_conn.send_packets(mentee_out);
                    }
                }
                Err(e) => {
                    let pkt = SMentorReply {
                        success: false,
                        message: e,
                        mentee_name: String::new(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        } else {
            // Decline mentor request
            match mentor_system.decline_mentor_request(mentor_id) {
                Ok(()) => {
                    let pkt = SMentorReply {
                        success: true,
                        message: "Mentor request declined".to_string(),
                        mentee_name: String::new(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
                Err(e) => {
                    let pkt = SMentorReply {
                        success: false,
                        message: e,
                        mentee_name: String::new(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        }
    }
    
    pub fn handle_allow_mentor(&mut self, _msg: CAllowMentor, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let player_id = self.current_char_index.unwrap_or(-1);
        let mentor_system = {
            let world = self.world.lock().unwrap();
            world.get_mentor_system()
        };

        // Check if player is a mentor
        if let Some((mentee_count, total_exp)) = mentor_system.get_mentor_stats(player_id) {
            let message = format!(
                "You have {} mentee(s) and earned {} mentor experience",
                mentee_count, total_exp
            );

            let pkt = SAllowMentor {
                success: true,
                message,
                mentee_count: mentee_count as u8,
                mentor_exp: total_exp,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
        } else {
            let pkt = SAllowMentor {
                success: false,
                message: "You are not a mentor".to_string(),
                mentee_count: 0,
                mentor_exp: 0,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
        }
    }
    
    pub fn handle_cancel_mentor(&mut self, _msg: CCancelMentor, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let player_id = self.current_char_index.unwrap_or(-1);
        let mut mentor_system = {
            let mut world = self.world.lock().unwrap();
            world.get_mentor_system_mut()
        };

        match mentor_system.cancel_mentorship(player_id) {
            Ok(partner_id) => {
                // Send success response
                let pkt = SCancelMentor {
                    success: true,
                    message: "Mentorship cancelled".to_string(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }

                // Notify partner
                let world = self.world.lock().unwrap();
                if let Some(partner_conn) = world.get_connection_by_char_index(partner_id) {
                    let partner_pkt = SCancelMentor {
                        success: true,
                        message: "Your mentorship has been cancelled".to_string(),
                    };
                    if let Ok(raw) = partner_pkt.encode() {
                        let _ = partner_conn.send_raw_packet(raw);
                    }
                }

                // Update character info for both players
                self.update_mentor_status(0, false, out);
                
                // Update partner's status if online
                if let Some(mut partner_conn) = world.get_connection_by_char_index_mut(partner_id) {
                    let mut partner_out = Vec::new();
                    partner_conn.update_mentor_status(0, false, &mut partner_out);
                    let _ = partner_conn.send_packets(partner_out);
                }
            }
            Err(e) => {
                let pkt = SCancelMentor {
                    success: false,
                    message: e,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }

    /// Update mentor status and send refresh packet
    fn update_mentor_status(&mut self, partner_id: i32, has_mentor: bool, out: &mut Vec<Vec<u8>>) {
        // Update character info
        if let Some(ref account_id) = self.account_id {
            if let Some(char_idx) = self.current_char_index {
                // Update in database
                let _ = self.store.save_character_mentor(
                    account_id, 
                    char_idx, 
                    if has_mentor { partner_id } else { 0 },
                    has_mentor // is_mentor flag
                );
            }
        }

        // Send refresh packet to update client
        let pkt = SRefreshCharacter {
            character_id: self.session_id,
            // Would include other fields as needed
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    /// Add mentor experience when player gains exp
    pub fn add_mentor_exp(&mut self, exp_gained: u64) -> Option<u64> {
        let player_id = self.current_char_index.unwrap_or(-1);
        
        let mut mentor_system = {
            let mut world = self.world.lock().unwrap();
            world.get_mentor_system_mut()
        };

        mentor_system.add_mentor_exp(player_id, exp_gained)
    }

    /// Calculate experience bonus if player has mentor
    pub fn calculate_exp_bonus(&self, base_exp: u64) -> u64 {
        let player_id = self.current_char_index.unwrap_or(-1);
        
        let mentor_system = {
            let world = self.world.lock().unwrap();
            world.get_mentor_system()
        };

        if mentor_system.is_mentee(player_id) {
            mentor_system.calculate_mentee_bonus(base_exp)
        } else {
            0
        }
    }
}
