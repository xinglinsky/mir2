use crystal_server_core::account::StoredFriend;
use crystal_shared_proto::login::{CChat, SDisconnect};
use crystal_shared_proto::scene::SChat;
use crystal_shared_proto::user::SAllowObserve;

use super::{LoginConnection, Stage};

impl LoginConnection {
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
                    let found = match link.grid {
                        1 => inv
                            .slots
                            .iter()
                            .find(|s| s.as_ref().map(|i| i.unique_id) == Some(link.unique_id))
                            .and_then(|s| s.as_ref()),
                        2 => eq
                            .slots
                            .iter()
                            .find(|s| s.as_ref().map(|i| i.unique_id) == Some(link.unique_id))
                            .and_then(|s| s.as_ref()),
                        _ => None,
                    };

                    if let Some(item) = found {
                        cache.insert(link.unique_id, item.clone());
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
    }
}
