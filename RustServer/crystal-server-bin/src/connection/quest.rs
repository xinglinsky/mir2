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

        // Mirror C# PlayerObject.FinishQuest behavior:
        // 1. Check if quest is completed (without removing it)
        // 2. Check if player can gain all reward items (BEFORE completing quest)
        // 3. If space is insufficient, send error message and return (quest not completed)
        // 4. If space is sufficient, complete quest, remove quest items, give rewards

        let (progress_opt, completed_ids, now_ms, rewards_given, events) = {
            let mut world = self.world.lock().unwrap();
            let now_ms = world.current_time_ms();
            
            // Step 1: Check if quest is completed (without removing it yet)
            let _progress_clone = match world.check_quest_completion(self.session_id, msg.quest_index, now_ms) {
                Some((true, p)) => p,
                _ => {
                    return;
                },
            };

            // Step 2: Check if player can gain all reward items BEFORE completing the quest
            // (mirroring C# CanGainItems check before quest completion)
            let (can_gain, reward_items) = world.check_quest_rewards_space(
                self.session_id,
                msg.quest_index,
                Some(msg.selected_item_index),
            );
            
            if !can_gain {
                // Send error message and return (don't complete quest)
                let mut events = Vec::new();
                events.push(crate::world::WorldEvent::PartySystemMessage {
                    session_id: self.session_id,
                    message: "Cannot hand in quest whilst bag is full.".to_string(),
                });
                drop(world); // Release lock before calling self method
                let _ = self.handle_world_events(events, out);
                return;
            }

            // Step 3: Space is sufficient, proceed with quest completion
            let mut events = Vec::new();
            let progress = world.complete_quest_for_player(self.session_id, msg.quest_index, now_ms);
            
            // Step 4: Give quest rewards (we've already verified space is available)
            let rewards_given = if progress.is_some() {
                world.give_quest_rewards(self.session_id, msg.quest_index, Some(msg.selected_item_index), reward_items, &mut events)
            } else {
                false
            };
            
            let completed = world.completed_quests_for_player(self.session_id);
            (progress, completed, now_ms, rewards_given, events)
        };

        // Send events (including error messages if any)
        let _ = self.handle_world_events(events, out);

        // If rewards weren't given (e.g., inventory full), don't send quest completion packets
        if !rewards_given {
            return;
        }

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

        // Mirror C# PlayerObject.ShareQuest behavior:
        // 1. Check if player is in a party
        // 2. Find party members on same map within range (14 tiles / Globals.DataRange)
        // 3. Check if quest can be shared (quest type restrictions)
        // 4. Check if each member can accept the quest (requirements)
        // 5. Send SShareQuest to eligible members
        let events = {
            let mut world = self.world.lock().unwrap();
            let mut events = Vec::new();
            world.share_quest_with_party(self.session_id, msg.quest_index, &mut events);
            events
        };

        // Handle WorldEvents (including QuestShared events and system messages)
        let _ = self.handle_world_events(events, out);
    }
}

