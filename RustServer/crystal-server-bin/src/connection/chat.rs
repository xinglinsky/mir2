use crystal_shared_proto::login::SDisconnect;
use crystal_shared_proto::scene::SChat;

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

    pub(crate) fn handle_chat(&mut self, message: String, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let trimmed = message.trim();

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
    }
}
