use crystal_server_core::account::StoredFriend;
use crystal_shared_proto::login::{CChat, SDisconnect};
use crystal_shared_proto::hero::{SHeroInformation, SUpdateHeroSpawnState};
use crystal_shared_proto::item::SNewChatItem;
use crystal_shared_proto::scene::SChat;
use crystal_shared_proto::scene::SHeroBaseStatsInfo;
use crystal_shared_proto::user::SHeroHealthChanged;
use crystal_shared_proto::user::SAllowObserve;
use crystal_shared_proto::user::SObjectChat;
use crystal_shared_proto::io::{
    write_bool,
    write_i32_le,
    write_i64_le,
    write_u16_le,
    write_u32_le,
    write_string,
};
use crystal_server_core::world::configs::base_stats;
use crystal_server_core::world::types::Job;

use super::{LoginConnection, Stage};

impl LoginConnection {
    pub(crate) fn send_hero_bootstrap(&mut self, out: &mut Vec<Vec<u8>>) {
        let snapshot_opt = {
            let world = self.world.lock().unwrap();
            world.player_inspect_snapshot(self.session_id)
        };

        let Some((_name, _guild_name, class, gender, hair, level, _equipment, _allow_observe)) = snapshot_opt else {
            return;
        };

        let (hero_name, hero_class, hero_gender, hero_level) = if let Some(h) = &self.hero_current {
            (h.name.as_str(), h.class, h.gender, h.level)
        } else {
            ("Hero", class, gender, level)
        };

        let Some(stats) = self.current_stats.clone() else {
            return;
        };

        let max_experience = {
            let lvl = hero_level as usize;
            if lvl == 0 {
                0_i64
            } else {
                *self.exp_table.get(lvl.saturating_sub(1)).unwrap_or(&0_i64)
            }
        };

        let hero_id = super::hero_object_id(self.session_id);

        let mut core_bytes = Vec::new();
        if write_u32_le(&mut core_bytes, hero_id).is_err() {
            return;
        }
        if write_string(&mut core_bytes, hero_name).is_err() {
            return;
        }
        core_bytes.push(hero_class);
        core_bytes.push(hero_gender);
        let _ = write_u16_le(&mut core_bytes, hero_level);
        core_bytes.push(hair);

        let _ = write_i32_le(&mut core_bytes, stats.hp);
        let _ = write_i32_le(&mut core_bytes, stats.mp);

        let _ = write_i64_le(&mut core_bytes, stats.experience);
        let _ = write_i64_le(&mut core_bytes, max_experience);

        let _ = write_bool(&mut core_bytes, true);
        let _ = write_i32_le(&mut core_bytes, 46);
        for _ in 0..46 {
            let _ = write_bool(&mut core_bytes, false);
        }

        let _ = write_bool(&mut core_bytes, true);
        let _ = write_i32_le(&mut core_bytes, 14);
        for _ in 0..14 {
            let _ = write_bool(&mut core_bytes, false);
        }

        let _ = write_i32_le(&mut core_bytes, 0);

        let hero_pkt = SHeroInformation {
            core_bytes,
            auto_pot: false,
            auto_hp_percent: 0,
            auto_mp_percent: 0,
            hp_item_index: -1,
            mp_item_index: -1,
        };
        if let Ok(raw) = hero_pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        if let Some(job) = Job::from_u8(hero_class) {
            let hero_base_stats_bytes = base_stats::encode_base_stats_for_job(job);
            let hero_base_stats_pkt = SHeroBaseStatsInfo {
                stats_bytes: hero_base_stats_bytes,
            };
            out.push(Self::encode_raw(hero_base_stats_pkt.encode()));
        }

        let hero_hc_pkt = SHeroHealthChanged {
            hp: stats.hp,
            mp: stats.mp,
        };
        if let Ok(raw) = hero_hc_pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn send_system_chat(&self, text: &str, out: &mut Vec<Vec<u8>>) {
        let pkt = SChat {
            message: text.to_string(),
            chat_type: 2,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_chat(&mut self, msg: CChat, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let trimmed = msg.message.trim();
        let mut rendered = trimmed.to_string();
        let mut linked_user_items: Vec<crystal_shared_proto::item_types::UserItemData> = Vec::new();

        if trimmed.eq_ignore_ascii_case("@SUMMONHERO") {
            let next = if self.hero_spawn_state >= 2 { 1 } else { 2 };
            self.hero_spawn_state = next;

            if next >= 2 {
                self.send_hero_bootstrap(out);
            }

            let pkt = SUpdateHeroSpawnState { state: next };
            out.push(Self::encode_raw(pkt.encode()));
            return;
        }

        if trimmed.eq_ignore_ascii_case("@MANAGEHERO") {
            self.send_manage_heroes(out);
            return;
        }

        if trimmed.eq_ignore_ascii_case("@CREATEHERO") {
            self.send_hero_create_request(out);
            return;
        }

        // Mirror C# MirConnection.Chat: if the message exceeds Globals.MaxChatLength,
        // immediately disconnect the client with reason=2 (Packet Error).
        // The exact MaxChatLength is defined in the C# Globals; here we
        // conservatively treat anything over 255 characters as invalid.
        if trimmed.chars().count() > 255 {
            let pkt = SDisconnect { reason: 2 };
            let raw = pkt.encode();
            out.push(Self::encode_raw(raw));
            self.closing = true;
            return;
        }
        if !trimmed.is_empty() {
            tracing::info!(
                target = "chat",
                session_id = self.session_id,
                map_index = self.current_map_index,
                "{}",
                trimmed,
            );
        }

        if !msg.linked_items.is_empty() {
            let items_opt = {
                let world = self.world.lock().unwrap();
                world.player_items(self.session_id)
            };

            if let Some((inv, eq)) = items_opt {
                let mut cache = self.chat_item_cache.lock().unwrap();
                if cache.len() > 2048 {
                    cache.clear();
                }

                for link in &msg.linked_items {
                    // Minimal behaviour: find by unique_id across inventory/equipment
                    // regardless of the grid enum value.
                    let found = inv
                        .slots
                        .iter()
                        .chain(eq.slots.iter())
                        .find(|s| s.as_ref().map(|i| i.unique_id) == Some(link.unique_id))
                        .and_then(|s| s.as_ref());

                    if let Some(item) = found {
                        cache.insert(link.unique_id, item.clone());
                        linked_user_items.push(item.clone());
                    }

                    // Rewrite the chat text so the client can extract
                    // "Title/UniqueID" inside "<...>".
                    let needle = format!("<{}>", link.title);
                    let replacement = format!("<{}/{}>", link.title, link.unique_id);
                    if rendered.contains(&needle) {
                        rendered = rendered.replacen(&needle, &replacement, 1);
                    }
                }
            }
        }

        // Delegate all GM/admin commands to the gm_commands module.
        if self.handle_gm_chat(trimmed, out) {
            return;
        }

        // Handle NPC-triggered extended storage rental from the Storage
        // dialog. The C# client sends a chat message "@ADDSTORAGE" when the
        // user clicks the Rent/Extend button; the C# server then handles this
        // in PlayerObject.Chat under the ADDSTORAGE case. We mirror that by
        // delegating to the same logic used by the /addstorage GM command.
        if trimmed.eq_ignore_ascii_case("@ADDSTORAGE") {
            // Reuse the existing /addstorage implementation in gm_commands.
            let _ = self.handle_gm_chat("/addstorage", out);
            return;
        }

        if trimmed.eq_ignore_ascii_case("@ALLOWOBSERVE") {
            let allow_opt = {
                let mut world = self.world.lock().unwrap();
                world.toggle_player_allow_observe(self.session_id)
            };

            if let Some(allow) = allow_opt {
                let pkt = SAllowObserve { allow };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            return;
        }

        // Whisper: mirror C# behaviour for messages starting with '/name ' by
        // treating them as a private message to the named player. We enforce
        // friend Block rules from both perspectives using StoredFriend
        // persistence, and emit Chinese system messages when blocked.
        if trimmed.starts_with('/') {
            let body = &trimmed[1..];
            let mut parts = body.splitn(2, ' ');
            let target_name = match parts.next() {
                Some(n) if !n.is_empty() => n,
                _ => return,
            };

            let rest = match parts.next() {
                Some(r) if !r.is_empty() => r,
                _ => return,
            };

            // Resolve target session by current online characters.
            let target_sid_opt = {
                let world = self.world.lock().unwrap();
                world.find_session_by_name(target_name)
            };

            let target_sid = match target_sid_opt {
                Some(sid) => sid,
                None => {
                    let msg = format!("未找到玩家 {}。", target_name);
                    self.send_system_chat(&msg, out);
                    return;
                }
            };

            // Determine sender/recipient character indices for friend lookup.
            let sender_char_idx = match self.current_char_index {
                Some(i) => i,
                None => return,
            };

            let recipient_char_idx = {
                let world = self.world.lock().unwrap();
                match world.player_character_index(target_sid) {
                    Some(idx) => idx,
                    None => return,
                }
            };

            // Load friends for both players from persistent storage.
            let sender_account_id = match &self.account_id {
                Some(id) => id.clone(),
                None => return,
            };

            let recipient_account_id_opt = {
                // We don't currently store account_id per session in the
                // World, so infer it via online_accounts mapping where the
                // value is session_id and key is account_id.
                let online = self.online_accounts.lock().unwrap();
                online
                    .iter()
                    .find(|(_, &sid)| sid == target_sid)
                    .map(|(acc, _)| acc.clone())
            };

            let recipient_account_id = match recipient_account_id_opt {
                Some(a) => a,
                None => return,
            };

            let recipient_friends: Vec<StoredFriend> = match self
                .store
                .load_character_friends(&recipient_account_id, recipient_char_idx)
            {
                Ok(f) => f,
                Err(e) => {
                    tracing::debug!(
                        "Whisper: failed to load recipient friends for account_id={} idx={} err={:?}",
                        recipient_account_id,
                        recipient_char_idx,
                        e,
                    );
                    Vec::new()
                }
            };

            let sender_friends: Vec<StoredFriend> = match self
                .store
                .load_character_friends(&sender_account_id, sender_char_idx)
            {
                Ok(f) => f,
                Err(e) => {
                    tracing::debug!(
                        "Whisper: failed to load sender friends for account_id={} idx={} err={:?}",
                        sender_account_id,
                        sender_char_idx,
                        e,
                    );
                    Vec::new()
                }
            };

            let recipient_blocks_sender = recipient_friends
                .iter()
                .any(|f| f.friend_index == sender_char_idx && f.blocked);
            if recipient_blocks_sender {
                // C# text: "Player is not accepting your messages.".
                self.send_system_chat("该玩家已拒收你的私聊消息。", out);
                return;
            }

            let sender_blocks_recipient = sender_friends
                .iter()
                .any(|f| f.friend_index == recipient_char_idx && f.blocked);
            if sender_blocks_recipient {
                // C# text: "Cannot message player whilst they are on your
                // blacklist.".
                self.send_system_chat("对方在你的黑名单中，无法发送私聊。", out);
                return;
            }

            // Send feedback to sender (WhisperOut-equivalent).
            let self_pkt = SChat {
                message: format!("/{} {}", target_name, rest),
                // ChatType.WhisperOut
                chat_type: 7,
            };
            if let Ok(raw) = self_pkt.encode() {
                out.push(Self::encode_raw(raw));
            }

            // Send whisper to recipient (WhisperIn-equivalent). Prefix with
            // the sender name similarly to C# "{0}=>{1}".
            let sender_name = if let Some(idx) = self.current_char_index {
                self.characters
                    .iter()
                    .find(|c| c.index == idx)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| String::new())
            } else {
                String::new()
            };

            let recv_text = format!("{}=>{}", sender_name, rest);
            let recv_pkt = SChat {
                message: recv_text,
                // ChatType.WhisperIn
                chat_type: 6,
            };
            if let Ok(raw) = recv_pkt.encode() {
                let mut outboxes = self.outboxes.lock().unwrap();
                outboxes
                    .entry(target_sid)
                    .or_default()
                    .push(Self::encode_raw(raw));
            }

            return;
        }

        if rendered.trim().is_empty() {
            return;
        }

        // Normal chat: broadcast nearby as ObjectChat (this is what the client
        // uses to populate the chat log and show speech bubbles).
        let sender_name = {
            let world = self.world.lock().unwrap();
            world
                .player_name(self.session_id)
                .unwrap_or_else(|| "玩家".to_string())
        };

        let targets = {
            let world = self.world.lock().unwrap();
            world.sessions_in_range_for_map(
                self.current_map_index,
                self.current_x,
                self.current_y,
                Self::DATA_RANGE,
            )
        };

        // Send linked item payloads first so the client can resolve the link
        // immediately when rendering chat history.
        if !linked_user_items.is_empty() {
            for item in &linked_user_items {
                if let Ok(pkt) = SNewChatItem::from_user_item(item) {
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);
                        out.push(bytes.clone());

                        let mut outboxes = self.outboxes.lock().unwrap();
                        for &sid in &targets {
                            if sid == self.session_id {
                                continue;
                            }
                            outboxes.entry(sid).or_default().push(bytes.clone());
                        }
                    }
                }
            }
        }

        let text = format!("{}: {}", sender_name, rendered);
        let obj_pkt = SObjectChat {
            object_id: self.session_id as u32,
            text,
            // ChatType.Normal
            chat_type: 0,
        };

        if let Ok(raw) = obj_pkt.encode() {
            let bytes = Self::encode_raw(raw);

            // Echo to self so the sender sees their own message.
            out.push(bytes.clone());

            let mut outboxes = self.outboxes.lock().unwrap();
            for sid in targets {
                if sid == self.session_id {
                    continue;
                }
                outboxes.entry(sid).or_default().push(bytes.clone());
            }
        }
    }
}
