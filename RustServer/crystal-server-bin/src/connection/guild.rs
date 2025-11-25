use crystal_shared_proto::guild::SGuildStatus;
use crystal_shared_proto::login::{CGuildInvite, CGuildNameReturn};
use crystal_shared_proto::scene::SObjectGuildNameChanged;

use super::{LoginConnection, Stage};

impl LoginConnection {
    fn guild_member_exists_for_session(&self) -> bool {
        let map = self.player_summaries.lock().unwrap();
        if let Some(v) = map.get(&self.session_id) {
            !v.guild_name.is_empty()
        } else {
            false
        }
    }

    pub(crate) fn handle_create_guild_command(&mut self, name_raw: &str, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let name = name_raw.trim();
        let len = name.chars().count();
        if len < 3 || len > 20 {
            self.send_system_chat("Guild name must be between 3 and 20 characters.", out);
            return;
        }

        if name.contains('\\') {
            self.send_system_chat("Guild name contains invalid characters.", out);
            return;
        }

        if self.guild_member_exists_for_session() {
            self.send_system_chat("You are already part of a guild.", out);
            return;
        }

        let guild_info = {
            let mut world = self.world.lock().unwrap();
            world.create_guild(name)
        };

        let Some(guild) = guild_info else {
            let msg = format!("Guild {} already exists.", name);
            self.send_system_chat(&msg, out);
            return;
        };

        let _ = self.store.save_guild(&guild);

        {
            let mut map = self.player_summaries.lock().unwrap();
            if let Some(v) = map.get_mut(&self.session_id) {
                v.guild_name = guild.name.clone();
                v.guild_rank_name = "Leader".to_string();
            }
        }

        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let _ = self
                .store
                .save_character_guild(account_id, char_idx, &guild.name, 0);
        }

        // Notify self and nearby players that this object's guild name has
        // changed, so clients like B immediately see A's new guild tag
        // without requiring a visibility refresh.
        let guild_name_changed = SObjectGuildNameChanged {
            object_id: self.session_id,
            guild_name: guild.name.clone(),
        };
        if let Ok(pkt) = guild_name_changed.encode() {
            let raw = Self::encode_raw(pkt);
            // Send to self
            out.push(raw.clone());
            // Broadcast to other players in view
            self.enqueue_for_viewers(
                self.current_map_index,
                self.current_x,
                self.current_y,
                raw,
            );
        }

        let status = SGuildStatus {
            guild_name: guild.name.clone(),
            guild_rank_name: "Leader".to_string(),
            level: guild.level,
            experience: guild.experience,
            max_experience: guild.max_experience,
            gold: guild.gold,
            spare_points: guild.spare_points,
            member_count: 1,
            max_members: guild.member_cap,
            voting: false,
            item_count: guild.stored_items.len() as u8,
            buff_count: guild.buff_list.len() as u8,
            my_options: 0xff,
            my_rank_id: 0,
        };
        if let Ok(raw) = status.encode() {
            out.push(Self::encode_raw(raw));
        }

        let ok = format!("Successfully created guild {}", name);
        self.send_system_chat(&ok, out);
    }

    pub(crate) fn handle_guild_name_return(
        &mut self,
        msg: CGuildNameReturn,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let name = msg.name.trim();
        if name.is_empty() {
            return;
        }

        self.handle_create_guild_command(name, out);
    }

    pub(crate) fn handle_guild_invite(
        &mut self,
        msg: CGuildInvite,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        if self.pending_guild_invite.is_none() {
            self.send_system_chat("You have not been invited to a guild.", out);
            return;
        }

        if !msg.accept_invite {
            self.pending_guild_invite = None;
            return;
        }

        self.send_system_chat(
            "Guild invite accept/decline handling is not implemented yet.",
            out,
        );
        self.pending_guild_invite = None;
    }
}
