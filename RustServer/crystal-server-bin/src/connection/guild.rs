use crystal_shared_proto::guild::{SGuildStatus, SGuildInvite};
use crystal_shared_proto::login::{CGuildInvite, CGuildNameReturn, CEditGuildMember};
use crystal_shared_proto::scene::SObjectGuildNameChanged;
use crystal_server_core::world::world::GuildJoinError;

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

    pub(crate) fn handle_edit_guild_member(
        &mut self,
        msg: CEditGuildMember,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        // Currently only support ChangeType = 0 (add member).
        if msg.change_type != 0 {
            self.send_system_chat("Guild member operation not implemented.", out);
            return;
        }

        let target_name = msg.name.trim();
        if target_name.is_empty() {
            return;
        }

        // Determine inviter's guild name from the cached PlayerVisual.
        let inviter_guild_name = {
            let map = self.player_summaries.lock().unwrap();
            match map.get(&self.session_id) {
                Some(v) => v.guild_name.clone(),
                None => String::new(),
            }
        };

        if inviter_guild_name.is_empty() {
            self.send_system_chat("You are not in a guild.", out);
            return;
        }

        // Find target session by name.
        let target_session_id_opt = {
            let world = self.world.lock().unwrap();
            world.find_session_by_name(target_name)
        };

        let Some(target_session_id) = target_session_id_opt else {
            let msg = format!("{} is not online!", target_name);
            self.send_system_chat(&msg, out);
            return;
        };

        // Check if target already has a guild or a pending guild invite.
        let (target_guild_name, has_pending_invite) = {
            let map = self.player_summaries.lock().unwrap();
            let guild_name = map
                .get(&target_session_id)
                .map(|v| v.guild_name.clone())
                .unwrap_or_default();

            let has_pending = {
                let world = self.world.lock().unwrap();
                world.pending_guild_invite(target_session_id).is_some()
            };

            (guild_name, has_pending)
        };

        if !target_guild_name.is_empty() {
            let msg = format!("{} is already in a guild!", target_name);
            self.send_system_chat(&msg, out);
            return;
        }

        if has_pending_invite {
            let msg = format!("{} already has a guild invite pending.", target_name);
            self.send_system_chat(&msg, out);
            return;
        }

        // Record pending invite in world state.
        {
            let mut world = self.world.lock().unwrap();
            world.set_pending_guild_invite(target_session_id, &inviter_guild_name);
        }

        // Send SGuildInvite to the target session so the client shows the
        // invite dialog. The C# server sends the guild name in SGuildInvite.Name.
        let invite_pkt = SGuildInvite {
            name: inviter_guild_name.clone(),
        };
        if let Ok(raw) = invite_pkt.encode() {
            let encoded = Self::encode_raw(raw);
            if target_session_id == self.session_id {
                out.push(encoded);
            } else {
                let mut outboxes = self.outboxes.lock().unwrap();
                outboxes.entry(target_session_id).or_default().push(encoded);
            }
        }
    }

    pub(crate) fn handle_guild_invite(
        &mut self,
        msg: CGuildInvite,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }
        if !msg.accept_invite {
            let mut world = self.world.lock().unwrap();
            world.clear_pending_guild_invite(self.session_id);
            return;
        }

        let guild_name_opt = {
            let mut world = self.world.lock().unwrap();
            world.take_pending_guild_invite(self.session_id)
        };

        let Some(guild_name) = guild_name_opt else {
            self.send_system_chat("You have not been invited to a guild.", out);
            return;
        };

        // Try to add this player to the invited guild in the world state.
        let join_result = {
            let mut world = self.world.lock().unwrap();
            world.guild_add_member_by_name(&guild_name)
        };

        match join_result {
            Err(GuildJoinError::NotFound) => {
                let msg = format!("Guild {} no longer exists.", guild_name);
                self.send_system_chat(&msg, out);
                return;
            }
            Err(GuildJoinError::Full) => {
                let msg = format!("Guild {} is full.", guild_name);
                self.send_system_chat(&msg, out);
                return;
            }
            Ok(guild) => {
                // Persist updated guild snapshot.
                let _ = self.store.save_guild(&guild);

                // Update cached visual summary so nameplates show the new guild tag.
                {
                    let mut map = self.player_summaries.lock().unwrap();
                    if let Some(v) = map.get_mut(&self.session_id) {
                        v.guild_name = guild.name.clone();
                        v.guild_rank_name = "Member".to_string();
                    }
                }

                // Persist character -> guild mapping.
                if let (Some(ref account_id), Some(char_idx)) =
                    (self.account_id.as_ref(), self.current_char_index)
                {
                    let _ = self
                        .store
                        .save_character_guild(account_id, char_idx, &guild.name, 1);
                }

                // Notify self and nearby players that this object's guild name has changed.
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

                // Send updated guild status to the joining player.
                let status = SGuildStatus {
                    guild_name: guild.name.clone(),
                    guild_rank_name: "Member".to_string(),
                    level: guild.level,
                    experience: guild.experience,
                    max_experience: guild.max_experience,
                    gold: guild.gold,
                    spare_points: guild.spare_points,
                    member_count: guild.member_count,
                    max_members: guild.member_cap,
                    voting: false,
                    item_count: guild.stored_items.len() as u8,
                    buff_count: guild.buff_list.len() as u8,
                    my_options: 0,
                    my_rank_id: 1,
                };
                if let Ok(raw) = status.encode() {
                    out.push(Self::encode_raw(raw));
                }

                let ok = format!("You have joined guild {}.", guild_name);
                self.send_system_chat(&ok, out);
            }
        }

        let mut world = self.world.lock().unwrap();
        world.clear_pending_guild_invite(self.session_id);
    }
}
