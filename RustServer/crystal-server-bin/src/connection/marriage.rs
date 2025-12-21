use crystal_shared_proto::npc::{
    CMarriageRequest, CMarriageReply, CChangeMarriage,
    CDivorceRequest, CDivorceReply,
    SMarriageRequest, SMarriageReply, SChangeMarriage,
    SDivorceRequest, SDivorceReply
};
use crystal_shared_proto::user::status::SRefreshCharacter;
use crystal_server_core::marriage::{MarriageSystem, MarriageInfo};
use crate::connection::login::{LoginConnection, Stage};

// Marriage handlers for LoginConnection
impl LoginConnection {
    pub fn handle_marriage_request(&mut self, msg: CMarriageRequest, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Get target player info
        let target_name = msg.target_name.clone();
        let target_info = {
            let world = self.world.lock().unwrap();
            world.get_character_by_name(&target_name)
        };

        let target_info = match target_info {
            Some(info) => info,
            None => {
                // Send error response
                let pkt = SMarriageRequest {
                    success: false,
                    message: "Player not found".to_string(),
                    requester_name: String::new(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }
        };

        let requester_id = self.current_char_index.unwrap_or(-1);
        let target_id = target_info.index;

        // Cannot marry yourself
        if requester_id == target_id {
            let pkt = SMarriageRequest {
                success: false,
                message: "Cannot marry yourself".to_string(),
                requester_name: String::new(),
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        // Send marriage request
        let partner_id = {
            let mut world = self.world.lock().unwrap();
            let mut marriage_system = world.get_marriage_system_mut();
            
            match marriage_system.send_marriage_request(requester_id, target_id) {
                Ok(()) => {
                    // Get requester name
                    let requester_name = match self.current_character_name.as_ref() {
                        Some(name) => name.clone(),
                        None => "Unknown".to_string(),
                    };
                    
                    // Send success response to requester
                    let pkt = SMarriageRequest {
                        success: true,
                        message: "Marriage request sent".to_string(),
                        requester_name: requester_name.clone(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    
                    // Return partner_id for notification
                    Some(target_id)
                }
                Err(e) => {
                    let pkt = SMarriageRequest {
                        success: false,
                        message: e,
                        requester_name: String::new(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    None
                }
            }
        };
        
        // Send notification to target if online
        if let Some(target_id) = partner_id {
            let world = self.world.lock().unwrap();
            if let Some(target_conn) = world.get_connection_by_char_index(target_id) {
                let requester_name = match self.current_character_name.as_ref() {
                    Some(name) => name.clone(),
                    None => "Unknown".to_string(),
                };
                let notify_pkt = SMarriageRequest {
                    success: true,
                    message: format!("{} has sent you a marriage request", requester_name),
                    requester_name,
                };
                if let Ok(raw) = notify_pkt.encode() {
                    let _ = target_conn.send_raw_packet(raw);
                }
            }
        }
    }
    
    pub fn handle_marriage_reply(&mut self, msg: CMarriageReply, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let target_id = self.current_char_index.unwrap_or(-1);
        let mut marriage_system = {
            let mut world = self.world.lock().unwrap();
            world.get_marriage_system_mut()
        };

        if msg.accept_invite {
            // Accept marriage request
            match marriage_system.accept_marriage_request(target_id) {
                Ok(marriage) => {
                    // Send success response to both players
                    let pkt = SMarriageReply {
                        success: true,
                        message: "You are now married!".to_string(),
                        partner_name: String::new(), // Would need to fetch partner name
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }

                    // Notify partner
                    if let Some(partner_conn) = world.get_connection_by_char_index(marriage.partner_id) {
                        let partner_pkt = SMarriageReply {
                            success: true,
                            message: "You are now married!".to_string(),
                            partner_name: match self.current_character_name.as_ref() {
                                Some(name) => name.clone(),
                                None => "Unknown".to_string(),
                            },
                        };
                        if let Ok(raw) = partner_pkt.encode() {
                            let _ = partner_conn.send_raw_packet(raw);
                        }
                    }

                    // Update character info for both players
                    self.update_marriage_status(marriage.partner_id, true, out);
                    
                    // Update partner's status if online
                    if let Some(mut partner_conn) = world.get_connection_by_char_index_mut(marriage.partner_id) {
                        let mut partner_out = Vec::new();
                        partner_conn.update_marriage_status(target_id, true, &mut partner_out);
                        let _ = partner_conn.send_packets(partner_out);
                    }
                }
                Err(e) => {
                    let pkt = SMarriageReply {
                        success: false,
                        message: e,
                        partner_name: String::new(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        } else {
            // Decline marriage request
            match marriage_system.decline_marriage_request(target_id) {
                Ok(()) => {
                    let pkt = SMarriageReply {
                        success: true,
                        message: "Marriage request declined".to_string(),
                        partner_name: String::new(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
                Err(e) => {
                    let pkt = SMarriageReply {
                        success: false,
                        message: e,
                        partner_name: String::new(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        }
    }
    
    pub fn handle_change_marriage(&mut self, _msg: CChangeMarriage, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let player_id = self.current_char_index.unwrap_or(-1);
        let mut marriage_system = {
            let mut world = self.world.lock().unwrap();
            world.get_marriage_system_mut()
        };

        match marriage_system.toggle_recall(player_id) {
            Ok(allow_recall) => {
                let message = if allow_recall {
                    "You now allow recall from your partner".to_string()
                } else {
                    "You are now blocking recall from your partner".to_string()
                };

                let pkt = SChangeMarriage {
                    success: true,
                    message,
                    allow_recall,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            Err(e) => {
                let pkt = SChangeMarriage {
                    success: false,
                    message: e,
                    allow_recall: false,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }
    
    pub fn handle_divorce_request(&mut self, _msg: CDivorceRequest, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let player_id = self.current_char_index.unwrap_or(-1);
        let mut marriage_system = {
            let mut world = self.world.lock().unwrap();
            world.get_marriage_system_mut()
        };

        match marriage_system.send_divorce_request(player_id) {
            Ok(partner_id) => {
                // Send success response to requester
                let pkt = SDivorceRequest {
                    success: true,
                    message: "Divorce request sent".to_string(),
                    requester_name: match self.current_character_name.as_ref() {
                        Some(name) => name.clone(),
                        None => "Unknown".to_string(),
                    },
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }

                // Send notification to partner if online
                if let Some(partner_conn) = world.get_connection_by_char_index(partner_id) {
                    let notify_pkt = SDivorceRequest {
                        success: true,
                        message: format!("{} has requested a divorce", 
                            match self.current_character_name.as_ref() {
                                Some(name) => name.clone(),
                                None => "Unknown".to_string(),
                            }
                        ),
                        requester_name: match self.current_character_name.as_ref() {
                            Some(name) => name.clone(),
                            None => "Unknown".to_string(),
                        },
                    };
                    if let Ok(raw) = notify_pkt.encode() {
                        let _ = partner_conn.send_raw_packet(raw);
                    }
                }
            }
            Err(e) => {
                let pkt = SDivorceRequest {
                    success: false,
                    message: e,
                    requester_name: String::new(),
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }
    
    pub fn handle_divorce_reply(&mut self, msg: CDivorceReply, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let player_id = self.current_char_index.unwrap_or(-1);
        let mut marriage_system = {
            let mut world = self.world.lock().unwrap();
            world.get_marriage_system_mut()
        };

        if msg.accept_invite {
            // Accept divorce request
            match marriage_system.accept_divorce_request(player_id) {
                Ok((initiator_id, partner_id)) => {
                    // Send success response
                    let pkt = SDivorceReply {
                        success: true,
                        message: "You are now divorced".to_string(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }

                    // Notify ex-partner
                    let partner_id = if player_id == initiator_id { partner_id } else { initiator_id };
                    if let Some(partner_conn) = world.get_connection_by_char_index(partner_id) {
                        let partner_pkt = SDivorceReply {
                            success: true,
                            message: "You are now divorced".to_string(),
                        };
                        if let Ok(raw) = partner_pkt.encode() {
                            let _ = partner_conn.send_raw_packet(raw);
                        }
                    }

                    // Update character info for both players
                    self.update_marriage_status(0, false, out);
                    
                    // Update partner's status if online
                    if let Some(mut partner_conn) = world.get_connection_by_char_index_mut(partner_id) {
                        let mut partner_out = Vec::new();
                        partner_conn.update_marriage_status(0, false, &mut partner_out);
                        let _ = partner_conn.send_packets(partner_out);
                    }
                }
                Err(e) => {
                    let pkt = SDivorceReply {
                        success: false,
                        message: e,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        } else {
            // Decline divorce request
            match marriage_system.decline_divorce_request(player_id) {
                Ok(()) => {
                    let pkt = SDivorceReply {
                        success: true,
                        message: "Divorce request declined".to_string(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
                Err(e) => {
                    let pkt = SDivorceReply {
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

    /// Update marriage status and send refresh packet
    fn update_marriage_status(&mut self, partner_id: i32, is_married: bool, out: &mut Vec<Vec<u8>>) {
        // Update character info
        if let Some(ref account_id) = self.account_id {
            if let Some(char_idx) = self.current_char_index {
                // Update in database
                let _ = self.store.save_character_marriage(
                    account_id, 
                    char_idx, 
                    if is_married { partner_id } else { 0 }
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
}
