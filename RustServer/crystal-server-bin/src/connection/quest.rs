use crystal_shared_proto::login::{
    CAbandonQuest,
    CAcceptQuest,
    CFinishQuest,
    CShareQuest,
};
use crystal_shared_proto::quest::{
    SChangeQuest,
    SCompleteQuest,
    SNewQuestInfo,
    SShareQuest,
};

use super::{LoginConnection, Stage};

impl LoginConnection {
    /// Send a SNewQuestInfo packet to the owning client using a pre-encoded
    /// ClientQuestInfo.Save payload.
    #[allow(dead_code)]
    pub(crate) fn send_new_quest_info_bytes(
        &self,
        quest_bytes: Vec<u8>,
        out: &mut Vec<Vec<u8>>,
    ) {
        let pkt = SNewQuestInfo { quest_bytes };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    /// Send a SChangeQuest packet to the owning client using a pre-encoded
    /// ClientQuestProgress.Save payload together with the quest state and
    /// tracking flag.
    pub(crate) fn send_change_quest_bytes(
        &self,
        quest_bytes: Vec<u8>,
        quest_state: u8,
        track_quest: bool,
        out: &mut Vec<Vec<u8>>,
    ) {
        let pkt = SChangeQuest {
            quest_bytes,
            quest_state,
            track_quest,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_accept_quest(
        &mut self,
        msg: CAcceptQuest,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let (progress_opt, now_ms) = {
            let mut world = self.world.lock().unwrap();
            let now_ms = world.current_time_ms();
            let progress = world.take_quest_for_player(self.session_id, msg.quest_index, now_ms);
            (progress, now_ms)
        };

        let Some(progress) = progress_opt else {
            return;
        };

        let quest_bytes = match progress.to_client_progress_bytes(now_ms) {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::debug!(
                    "handle_accept_quest: failed to encode quest progress for session_id={} quest_id={} err={:?}",
                    self.session_id,
                    msg.quest_index,
                    e,
                );
                return;
            }
        };

        const QUEST_STATE_ADD: u8 = 0;
        self.send_change_quest_bytes(quest_bytes, QUEST_STATE_ADD, true, out);
    }

    pub(crate) fn handle_finish_quest(
        &mut self,
        msg: CFinishQuest,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        // For now we ignore msg.selected_item_index because reward item
        // selection and distribution are not yet implemented on the Rust
        // server side.
        let (progress_opt, completed_ids, now_ms) = {
            let mut world = self.world.lock().unwrap();
            let now_ms = world.current_time_ms();
            let progress = world.complete_quest_for_player(self.session_id, msg.quest_index, now_ms);
            let completed = world.completed_quests_for_player(self.session_id);
            (progress, completed, now_ms)
        };

        let Some(progress) = progress_opt else {
            return;
        };

        let quest_bytes = match progress.to_client_progress_bytes(now_ms) {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::debug!(
                    "handle_finish_quest: failed to encode quest progress for session_id={} quest_id={} err={:?}",
                    self.session_id,
                    msg.quest_index,
                    e,
                );
                return;
            }
        };

        const QUEST_STATE_REMOVE: u8 = 2;
        self.send_change_quest_bytes(quest_bytes, QUEST_STATE_REMOVE, false, out);

        if !completed_ids.is_empty() {
            let pkt = SCompleteQuest {
                completed_quests: completed_ids,
            };
            if let Ok(raw) = pkt.encode() {
                out.push(LoginConnection::encode_raw(raw));
            }
        }
    }

    pub(crate) fn handle_abandon_quest(
        &mut self,
        msg: CAbandonQuest,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let (progress_opt, now_ms) = {
            let mut world = self.world.lock().unwrap();
            let now_ms = world.current_time_ms();
            let progress = world.abandon_quest_for_player(self.session_id, msg.quest_index);
            (progress, now_ms)
        };

        let Some(progress) = progress_opt else {
            return;
        };

        let quest_bytes = match progress.to_client_progress_bytes(now_ms) {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::debug!(
                    "handle_abandon_quest: failed to encode quest progress for session_id={} quest_id={} err={:?}",
                    self.session_id,
                    msg.quest_index,
                    e,
                );
                return;
            }
        };

        const QUEST_STATE_REMOVE: u8 = 2;
        self.send_change_quest_bytes(quest_bytes, QUEST_STATE_REMOVE, false, out);
    }

    pub(crate) fn handle_share_quest(
        &mut self,
        msg: CShareQuest,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        // Full quest sharing (including party lookup and distance checks) is
        // not yet implemented on the Rust server. For now, mirror a minimal
        // behaviour by sending SShareQuest to this client only so the packet
        // path is exercised.
        let pkt = SShareQuest {
            quest_index: msg.quest_index,
            sharer_name: self
                .characters
                .iter()
                .find(|c| Some(c.index) == self.current_char_index)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "".to_string()),
        };
        if let Ok(raw) = pkt.encode() {
            out.push(LoginConnection::encode_raw(raw));
        }
    }
}

