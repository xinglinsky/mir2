use crystal_shared_proto::npc::{
    CNPCConfirmInput, CReportIssue, COpendoor,
    SNPCConfirmInput, SReportIssue, SOpendoor
};
use crate::connection::login::{LoginConnection, Stage};

// Miscellaneous handlers for LoginConnection
impl LoginConnection {
    pub fn handle_npc_confirm_input(&mut self, msg: CNPCConfirmInput, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Store the input value in player data
        let player_id = self.current_char_index.unwrap_or(-1);
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_npc_input(player_id, msg.value.clone());
        }

        // Call the NPC with the confirmed input
        let npc_id = msg.npc_id;
        let page_name = msg.page_name.clone();

        // Check if it's the default NPC
        let is_default_npc = {
            let world = self.world.lock().unwrap();
            npc_id == world.get_default_npc_id()
        };

        if is_default_npc {
            // Call default NPC
            self.call_default_npc(&page_name, out);
        } else {
            // Call specific NPC
            self.call_npc(npc_id, &page_name, out);
        }

        // Send confirmation response
        let pkt = SNPCConfirmInput {
            success: true,
            message: "Input confirmed".to_string(),
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
    
    pub fn handle_report_issue(&mut self, msg: CReportIssue, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Log the issue report
        let player_name = match self.current_character_name.as_ref() {
            Some(name) => name.clone(),
            None => "Unknown".to_string(),
        };

        let report_info = format!(
            "Issue Report from {}: {}\nMap: {}\nPosition: ({}, {})",
            player_name,
            msg.message,
            msg.map_id,
            msg.x,
            msg.y
        );

        // Log to server
        log::info!("ReportIssue: {}", report_info);

        // In a real implementation, this would:
        // 1. Save to database
        // 2. Send notification to GMs
        // 3. Handle image attachments if any

        // Send confirmation
        let pkt = SReportIssue {
            success: true,
            message: "Issue reported successfully. Thank you for your feedback!".to_string(),
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
    
    pub fn handle_open_door(&mut self, msg: COpendoor, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let player_id = self.current_char_index.unwrap_or(-1);
        let (map_id, x, y) = {
            let world = self.world.lock().unwrap();
            world.get_player_position(self.session_id)
                .unwrap_or((0, 0, 0))
        };

        // Try to open the door
        let result = {
            let mut world = self.world.lock().unwrap();
            world.open_door(player_id, map_id, msg.door_index, x, y)
        };

        match result {
            Ok(()) => {
                let pkt = SOpendoor {
                    success: true,
                    message: "Door opened".to_string(),
                    door_index: msg.door_index,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            Err(e) => {
                let pkt = SOpendoor {
                    success: false,
                    message: e,
                    door_index: msg.door_index,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }

    /// Call default NPC
    fn call_default_npc(&self, page_name: &str, out: &mut Vec<Vec<u8>>) {
        let player_id = self.current_char_index.unwrap_or(-1);
        
        let result = {
            let mut world = self.world.lock().unwrap();
            world.call_default_npc(player_id, page_name.to_string())
        };

        if let Err(e) = result {
            log::error!("Failed to call default NPC: {}", e);
        }
    }

    /// Call specific NPC
    fn call_npc(&self, npc_id: u32, page_name: &str, out: &mut Vec<Vec<u8>>) {
        let player_id = self.current_char_index.unwrap_or(-1);
        
        let result = {
            let mut world = self.world.lock().unwrap();
            world.call_npc(player_id, npc_id, page_name.to_string())
        };

        if let Err(e) = result {
            log::error!("Failed to call NPC {}: {}", npc_id, e);
        }
    }
}
