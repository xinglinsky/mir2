use crystal_server_core::world;
use crystal_shared_proto::io::{write_bool, write_i32_le};
use crystal_shared_proto::item_types::UserItemData;
use crystal_shared_proto::login::{
    CDepositTradeItem,
    CRetrieveTradeItem,
    CTradeConfirm,
    CTradeGold,
    CTradeReply,
    CTradeRequest,
};
use crystal_shared_proto::social::{
    STradeAccept,
    STradeCancel,
    STradeConfirm,
    STradeGold,
    STradeItem,
    STradeRequest,
};
use crystal_shared_proto::user::{SLoseGold, SUserSlotsRefresh};
use std::io;

use super::{LoginConnection, Stage};

impl LoginConnection {
    /// Handle a trade request from the client. This mirrors the high-level
    /// behaviour of C# PlayerObject.TradeRequest but in a simplified form:
    ///
    /// - Require InGame stage.
    /// - Find the player standing directly in front of this character
    ///   (one tile ahead in the current facing direction) using world
    ///   player positions.
    /// - If no such player is found, send a system message asking the user
    ///   to face the target.
    /// - If the target already has a pending trade invitation, reject with a
    ///   system message.
    /// - Otherwise, record a pending trade invite in the world state and send
    ///   an STradeRequest packet to the target session.
    pub(crate) fn handle_trade_request(
        &mut self,
        _msg: CTradeRequest,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        // Compute the tile directly in front of the player based on the
        // current facing direction. Direction encoding matches
        // world::movement::apply_step and the C# MirDirection enum.
        let (dx, dy) = match self.direction {
            0 => (0, -1),   // Up
            1 => (1, -1),   // UpRight
            2 => (1, 0),    // Right
            3 => (1, 1),    // DownRight
            4 => (0, 1),    // Down
            5 => (-1, 1),   // DownLeft
            6 => (-1, 0),   // Left
            7 => (-1, -1),  // UpLeft
            _ => (0, 0),
        };

        let target_x = self.current_x + dx;
        let target_y = self.current_y + dy;

        // Locate the player session (if any) occupying the front tile and
        // record a pending invite in the world state.
        let mut target_sid: Option<world::SessionId> = None;
        let mut inviter_name_opt: Option<String> = None;

        {
            let mut world = self.world.lock().unwrap();

            let candidates = world.players_in_view_for_map(
                self.current_map_index,
                self.current_x,
                self.current_y,
                1,
                Some(self.session_id),
            );

            for (sid, x, y, _dir) in candidates {
                if x == target_x && y == target_y {
                    target_sid = Some(sid);
                    break;
                }
            }

            if let Some(t_sid) = target_sid {
                // Respect the target player's AllowTrade setting.
                if world.player_allow_trade(t_sid) == Some(false) {
                    target_sid = None;
                } else if world.pending_trade_invite_from(t_sid).is_some() {
                    // If the target already has a pending trade invitation
                    // recorded in the world, mirror the C# behaviour and reject.
                    // Leave inviter_name_opt as None to signal this case.
                } else {
                    world.set_pending_trade_invite(t_sid, self.session_id);
                    inviter_name_opt = world.player_name(self.session_id);
                }
            }
        }

        let target_sid = match target_sid {
            Some(sid) => sid,
            None => {
                // No player in front or the target is not accepting trades.
                self.send_system_chat("对方当前不接受交易。", out);
                return;
            }
        };

        if inviter_name_opt.is_none() {
            // Target already had a pending invite from someone else.
            self.send_system_chat("该玩家已经有待处理的交易邀请，请稍后再试。", out);
            return;
        }

        let name = inviter_name_opt.unwrap_or_else(|| "玩家".to_string());
        let pkt = STradeRequest { name };
        match pkt.encode() {
            Ok(raw) => {
                let encoded = LoginConnection::encode_raw(raw);
                if target_sid == self.session_id {
                    out.push(encoded);
                } else {
                    let mut outboxes = self.outboxes.lock().unwrap();
                    outboxes.entry(target_sid).or_default().push(encoded);
                }
                tracing::debug!(
                    "TradeRequest: from session_id={} to session_id={}",
                    self.session_id,
                    target_sid,
                );
            }
            Err(e) => {
                tracing::debug!(
                    "Failed to encode STradeRequest for session_id={} err={:?}",
                    self.session_id,
                    e,
                );
            }
        }
    }

    /// Handle a trade reply (accept or refuse). This mirrors the high-level
    /// C# PlayerObject.TradeReply logic for invitations only. Full trade
    /// session state and gold/item exchange will be implemented separately.
    pub(crate) fn handle_trade_reply(
        &mut self,
        msg: CTradeReply,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        // Take and clear any pending trade invite targeted at this session.
        let inviter_sid_opt = {
            let mut world = self.world.lock().unwrap();
            world.take_pending_trade_invite(self.session_id)
        };

        let Some(inviter_sid) = inviter_sid_opt else {
            if msg.accept {
                self.send_system_chat("当前没有待处理的交易邀请。", out);
            }
            return;
        };

        let (inviter_name_opt, self_name_opt) = {
            let world = self.world.lock().unwrap();
            let inviter_name = world.player_name(inviter_sid);
            let self_name = world.player_name(self.session_id);
            (inviter_name, self_name)
        };

        if !msg.accept {
            // Notify the inviter that this player refused the trade.
            if let Some(self_name) = self_name_opt {
                let mut events = Vec::new();
                events.push(world::WorldEvent::PartySystemMessage {
                    session_id: inviter_sid,
                    message: format!("玩家 {} 已拒绝与你交易。", self_name),
                });
                let _ = self.handle_world_events(events, out);
            }
            return;
        }

        let inviter_name = inviter_name_opt.unwrap_or_else(|| "玩家".to_string());
        let self_name = self_name_opt.unwrap_or_else(|| "玩家".to_string());

        let ok = {
            let mut world = self.world.lock().unwrap();
            world.set_trade_partner_pair(self.session_id, inviter_sid)
        };
        if !ok {
            self.send_system_chat("无法建立交易会话，对方可能已离线。", out);
            return;
        }

        // Send STradeAccept to this player, showing the partner's name.
        let pkt_self = STradeAccept {
            name: inviter_name.clone(),
        };
        if let Ok(raw) = pkt_self.encode() {
            out.push(LoginConnection::encode_raw(raw));
        }

        // And to the inviter, showing this player's name.
        let pkt_other = STradeAccept { name: self_name.clone() };
        if let Ok(raw) = pkt_other.encode() {
            let encoded = LoginConnection::encode_raw(raw);
            if inviter_sid == self.session_id {
                out.push(encoded);
            } else {
                let mut outboxes = self.outboxes.lock().unwrap();
                outboxes.entry(inviter_sid).or_default().push(encoded);
            }
        }

        tracing::debug!(
            "TradeReply: session_id={} accepted trade with session_id={}",
            self.session_id,
            inviter_sid,
        );
    }

    /// Handle a TradeGold packet from the client. This mirrors the C#
    /// PlayerObject.TradeGold behaviour for gold only:
    ///
    /// - Unlock both sides' TradeLocked flags.
    /// - Validate amount > 0 and that the caller has enough gold.
    /// - Deduct gold from the caller and persist CharacterStats.
    /// - Accumulate the offered gold in world-side trade state.
    /// - Send SLoseGold to the caller and STradeGold (total offered so
    ///   far) to the trade partner.
    pub(crate) fn handle_trade_gold(
        &mut self,
        msg: CTradeGold,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        if msg.amount == 0 {
            return;
        }

        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => {
                tracing::debug!(
                    "TradeGold: early-return, no current_stats for session_id={}",
                    self.session_id,
                );
                return;
            }
        };

        let amount_i64 = msg.amount as i64;
        if amount_i64 <= 0 || stats.gold < amount_i64 {
            tracing::debug!(
                "TradeGold: early-return, invalid amount or insufficient gold: amount={} gold={}",
                msg.amount,
                stats.gold,
            );
            return;
        }

        // Unlock trade for both sides and accumulate the offered gold on the
        // world-side PlayerState, mirroring C# TradeUnlock + TradeGoldAmount.
        let (partner_id, new_total) = {
            let mut world = self.world.lock().unwrap();

            let partner_id = match world.trade_partner_for(self.session_id) {
                Some(p) => p,
                None => {
                    tracing::debug!(
                        "TradeGold: early-return, no trade_partner for session_id={}",
                        self.session_id,
                    );
                    return;
                }
            };

            world.trade_unlock(self.session_id);

            let total = match world.add_trade_gold(self.session_id, msg.amount) {
                Some(t) => t,
                None => {
                    tracing::debug!(
                        "TradeGold: early-return, failed to add_trade_gold for session_id={}",
                        self.session_id,
                    );
                    return;
                }
            };

            (partner_id, total)
        };

        // Deduct gold from the caller and persist CharacterStats.
        let mut new_stats = stats.clone();
        new_stats.gold = new_stats.gold.saturating_sub(amount_i64);

        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let _ = self
                .store
                .save_character_stats(account_id, char_idx, &new_stats);
        }

        self.current_stats = Some(new_stats.clone());

        // Notify the caller about the gold deduction.
        let lose = SLoseGold { gold: msg.amount };
        if let Ok(raw) = lose.encode() {
            out.push(LoginConnection::encode_raw(raw));
        }

        // Notify the trade partner about the updated total offered gold.
        let pkt = STradeGold { amount: new_total };
        if let Ok(raw) = pkt.encode() {
            let encoded = LoginConnection::encode_raw(raw);
            if partner_id == self.session_id {
                out.push(encoded);
            } else {
                let mut outboxes = self.outboxes.lock().unwrap();
                outboxes.entry(partner_id).or_default().push(encoded);
            }
        }

        tracing::debug!(
            "TradeGold: session_id={} offered amount={} total_offered={} to partner_id={}",
            self.session_id,
            msg.amount,
            new_total,
            partner_id,
        );
    }

    fn in_range(ax: i32, ay: i32, bx: i32, by: i32, range: i32) -> bool {
        (ax - bx).abs() <= range && (ay - by).abs() <= range
    }

    fn direction_from_point(src_x: i32, src_y: i32, dst_x: i32, dst_y: i32) -> u8 {
        if src_x < dst_x {
            if src_y < dst_y {
                3
            } else if src_y > dst_y {
                1
            } else {
                2
            }
        } else if src_x > dst_x {
            if src_y < dst_y {
                5
            } else if src_y > dst_y {
                7
            } else {
                6
            }
        } else if src_y < dst_y {
            4
        } else {
            0
        }
    }

    fn facing_each_other(
        a_dir: u8,
        a_x: i32,
        a_y: i32,
        b_dir: u8,
        b_x: i32,
        b_y: i32,
    ) -> bool {
        let dir_a_to_b = Self::direction_from_point(a_x, a_y, b_x, b_y);
        let dir_b_to_a = Self::direction_from_point(b_x, b_y, a_x, a_y);
        a_dir == dir_a_to_b && b_dir == dir_b_to_a
    }

    pub(crate) fn handle_trade_confirm(
        &mut self,
        msg: CTradeConfirm,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        if !msg.locked {
            let mut world = self.world.lock().unwrap();
            world.set_trade_locked(self.session_id, false);
            return;
        }

        let partner_id_opt = {
            let world = self.world.lock().unwrap();
            world.trade_partner_for(self.session_id)
        };

        let partner_id = match partner_id_opt {
            Some(p) => p,
            None => {
                self.handle_trade_cancel(out);
                return;
            }
        };

        let cancel_due_to_position = {
            let world = self.world.lock().unwrap();
            let (self_pos, partner_pos) = (world.player_position(self.session_id), world.player_position(partner_id));

            match (self_pos, partner_pos) {
                (Some((self_map, sx, sy, sdir)), Some((other_map, ox, oy, odir))) => {
                    if self_map != other_map {
                        true
                    } else if !Self::in_range(sx, sy, ox, oy, Self::DATA_RANGE) {
                        true
                    } else if !Self::facing_each_other(sdir, sx, sy, odir, ox, oy) {
                        true
                    } else {
                        false
                    }
                }
                _ => true,
            }
        };

        if cancel_due_to_position {
            self.handle_trade_cancel(out);
            return;
        }

        {
            let mut world = self.world.lock().unwrap();
            world.set_trade_locked(self.session_id, true);
        }

        let (self_locked, partner_locked, self_gold, partner_gold) = {
            let world = self.world.lock().unwrap();
            let self_locked = world.is_trade_locked(self.session_id).unwrap_or(false);
            let partner_locked = world.is_trade_locked(partner_id).unwrap_or(false);
            let self_gold = world.trade_gold_for(self.session_id).unwrap_or(0);
            let partner_gold = world.trade_gold_for(partner_id).unwrap_or(0);
            (self_locked, partner_locked, self_gold, partner_gold)
        };

        if !self_locked {
            return;
        }

        if self_locked && !partner_locked {
            let self_name = {
                let world = self.world.lock().unwrap();
                world.player_name(self.session_id)
            }
            .unwrap_or_else(|| "玩家".to_string());

            let mut events = Vec::new();
            events.push(world::WorldEvent::PartySystemMessage {
                session_id: partner_id,
                message: format!("玩家 {} 正在等待你确认交易。", self_name),
            });
            let _ = self.handle_world_events(events, out);
            return;
        }

        if !partner_locked {
            return;
        }

        // Both sides have their TradeLocked flags set. Before finalising the
        // trade, mirror the C# CanGainItems semantics and ensure that each
        // side has enough inventory capacity to accept the other's offered
        // trade items.
        let (self_can_gain_items, partner_can_gain_items) = {
            let world = self.world.lock().unwrap();
            let self_can_gain =
                world.can_gain_items_from_trade(self.session_id, partner_id);
            let partner_can_gain =
                world.can_gain_items_from_trade(partner_id, self.session_id);
            (self_can_gain, partner_can_gain)
        };

        if !self_can_gain_items || !partner_can_gain_items {
            {
                let mut world = self.world.lock().unwrap();
                world.trade_unlock(self.session_id);
            }

            let mut events = Vec::new();

            if !partner_can_gain_items {
                // Our partner cannot accept all of our items.
                events.push(world::WorldEvent::PartySystemMessage {
                    session_id: self.session_id,
                    message: "Trading partner cannot accept all items.".to_string(),
                });
                events.push(world::WorldEvent::PartySystemMessage {
                    session_id: partner_id,
                    message: "Unable to accept all items.".to_string(),
                });
            }

            if !self_can_gain_items {
                // We cannot accept all of our partner's items.
                events.push(world::WorldEvent::PartySystemMessage {
                    session_id: partner_id,
                    message: "Trading partner cannot accept all items.".to_string(),
                });
                events.push(world::WorldEvent::PartySystemMessage {
                    session_id: self.session_id,
                    message: "Unable to accept all items.".to_string(),
                });
            }

            if !events.is_empty() {
                let _ = self.handle_world_events(events, out);
            }

            let pkt_self = STradeCancel { unlock: true };
            if let Ok(raw) = pkt_self.encode() {
                out.push(LoginConnection::encode_raw(raw));
            }

            let pkt_other = STradeCancel { unlock: true };
            if let Ok(raw) = pkt_other.encode() {
                let encoded = LoginConnection::encode_raw(raw);
                if partner_id == self.session_id {
                    out.push(encoded);
                } else {
                    let mut outboxes = self.outboxes.lock().unwrap();
                    outboxes.entry(partner_id).or_default().push(encoded);
                }
            }

            tracing::debug!(
                "TradeConfirm: cancelled due to insufficient inventory capacity (self_can_gain_items={} partner_can_gain_items={}) for session_id={} partner_id={}",
                self_can_gain_items,
                partner_can_gain_items,
                self.session_id,
                partner_id,
            );

            return;
        }

        // Gold capacity checks for both participants, approximating the C#
        // CanGainGold(uint) semantics. We ensure that:
        // - this player can accept partner_gold
        // - the partner can accept self_gold
        // before applying any transfers. On failure we unlock the trade,
        // notify both sides, and send S.TradeCancel(unlock=true) so they can
        // adjust their offers.
        let mut self_can_gain_gold = true;
        let mut partner_can_gain_gold = true;

        if partner_gold > 0 {
            self_can_gain_gold = match self.current_stats.clone() {
                Some(stats) => {
                    let current_u64 = if stats.gold <= 0 {
                        0u64
                    } else {
                        stats.gold as u64
                    };
                    let new_total_u64 = current_u64.saturating_add(partner_gold as u64);
                    new_total_u64 <= u32::MAX as u64
                }
                None => false,
            };
        }

        if self_gold > 0 {
            // Resolve the partner's account_id from the shared online_accounts
            // map and their character_index from the world state so that we
            // can load up-to-date CharacterStats for the gold cap check.
            let partner_account_id_opt: Option<String> = {
                let online_map = self.online_accounts.lock().unwrap();
                online_map
                    .iter()
                    .find_map(|(acc, sid)| if *sid == partner_id {
                        Some(acc.clone())
                    } else {
                        None
                    })
            };

            let partner_char_idx_opt = {
                let world = self.world.lock().unwrap();
                world.player_character_index(partner_id)
            };

            if let (Some(acc_id), Some(char_idx)) = (partner_account_id_opt, partner_char_idx_opt)
            {
                let partner_stats_opt = self
                    .store
                    .load_character_stats(&acc_id, char_idx)
                    .ok()
                    .flatten();

                if let Some(stats) = partner_stats_opt {
                    let current_u64 = if stats.gold <= 0 {
                        0u64
                    } else {
                        stats.gold as u64
                    };
                    let new_total_u64 = current_u64.saturating_add(self_gold as u64);
                    if new_total_u64 > u32::MAX as u64 {
                        partner_can_gain_gold = false;
                    }
                } else {
                    partner_can_gain_gold = false;
                }
            } else {
                partner_can_gain_gold = false;
            }
        }

        if !self_can_gain_gold || !partner_can_gain_gold {
            {
                let mut world = self.world.lock().unwrap();
                world.trade_unlock(self.session_id);
            }

            let mut events = Vec::new();

            if !partner_can_gain_gold {
                // Our partner cannot accept our gold offer.
                events.push(world::WorldEvent::PartySystemMessage {
                    session_id: self.session_id,
                    message: "Trading partner cannot accept any more gold.".to_string(),
                });
                events.push(world::WorldEvent::PartySystemMessage {
                    session_id: partner_id,
                    message: "Unable to accept any more gold.".to_string(),
                });
            }

            if !self_can_gain_gold {
                // We cannot accept the partner's gold offer.
                events.push(world::WorldEvent::PartySystemMessage {
                    session_id: partner_id,
                    message: "Trading partner cannot accept any more gold.".to_string(),
                });
                events.push(world::WorldEvent::PartySystemMessage {
                    session_id: self.session_id,
                    message: "Unable to accept any more gold.".to_string(),
                });
            }

            if !events.is_empty() {
                let _ = self.handle_world_events(events, out);
            }

            let pkt_self = STradeCancel { unlock: true };
            if let Ok(raw) = pkt_self.encode() {
                out.push(LoginConnection::encode_raw(raw));
            }

            let pkt_other = STradeCancel { unlock: true };
            if let Ok(raw) = pkt_other.encode() {
                let encoded = LoginConnection::encode_raw(raw);
                if partner_id == self.session_id {
                    out.push(encoded);
                } else {
                    let mut outboxes = self.outboxes.lock().unwrap();
                    outboxes.entry(partner_id).or_default().push(encoded);
                }
            }

            tracing::debug!(
                "TradeConfirm: cancelled due to gold cap (self_can_gain_gold={} partner_can_gain_gold={}) for session_id={} partner_id={} self_gold={} partner_gold={}",
                self_can_gain_gold,
                partner_can_gain_gold,
                self.session_id,
                partner_id,
                self_gold,
                partner_gold,
            );

            return;
        }

        // At this point both sides are locked and have enough capacity to
        // accept the other's items and gold. Apply the gold and item transfers
        // and then clear the trade session.
        let mut events = Vec::new();
        if partner_gold > 0 {
            events.push(world::WorldEvent::PlayerGainedGold {
                session_id: self.session_id,
                amount: partner_gold,
            });
        }
        if self_gold > 0 {
            events.push(world::WorldEvent::PlayerGainedGold {
                session_id: partner_id,
                amount: self_gold,
            });
        }

        {
            let mut world = self.world.lock().unwrap();
            world.apply_trade_items_for_pair(self.session_id, partner_id, &mut events);
            world.clear_trade_session(self.session_id);
        }

        events.push(world::WorldEvent::PartySystemMessage {
            session_id: self.session_id,
            message: "交易成功。".to_string(),
        });
        events.push(world::WorldEvent::PartySystemMessage {
            session_id: partner_id,
            message: "交易成功。".to_string(),
        });

        if !events.is_empty() {
            let _ = self.handle_world_events(events, out);
        }

        let pkt_self = STradeConfirm;
        let raw_self = pkt_self.encode();
        out.push(LoginConnection::encode_raw(raw_self));

        let pkt_other = STradeConfirm;
        let raw_other = pkt_other.encode();
        let encoded = LoginConnection::encode_raw(raw_other);
        if partner_id == self.session_id {
            out.push(encoded);
        } else {
            let mut outboxes = self.outboxes.lock().unwrap();
            outboxes.entry(partner_id).or_default().push(encoded);
        }

        tracing::debug!(
            "TradeConfirm: session_id={} and partner_id={} finalized trade: self_gold={} partner_gold={}",
            self.session_id,
            partner_id,
            self_gold,
            partner_gold,
        );
    }

    pub(crate) fn handle_trade_cancel(&mut self, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let partner_id_opt = {
            let world = self.world.lock().unwrap();
            world.trade_partner_for(self.session_id)
        };

        let partner_id = match partner_id_opt {
            Some(p) => p,
            None => {
                return;
            }
        };

        let (self_gold, partner_gold) = {
            let world = self.world.lock().unwrap();
            let self_gold = world.trade_gold_for(self.session_id).unwrap_or(0);
            let partner_gold = world.trade_gold_for(partner_id).unwrap_or(0);
            (self_gold, partner_gold)
        };

        let mut events = Vec::new();

        {
            let mut world = self.world.lock().unwrap();

            if self_gold > 0 {
                events.push(world::WorldEvent::PlayerGainedGold {
                    session_id: self.session_id,
                    amount: self_gold,
                });
            }
            if partner_gold > 0 {
                events.push(world::WorldEvent::PlayerGainedGold {
                    session_id: partner_id,
                    amount: partner_gold,
                });
            }

            // Roll back any items that were placed into the trade grid for
            // both participants. Items are moved back into their own
            // inventories where possible and otherwise dropped on the ground
            // near the owner, matching the intent of the C# TradeCancel logic
            // which ensures items are not silently lost.
            world.rollback_trade_items_for_player(self.session_id, &mut events);
            world.rollback_trade_items_for_player(partner_id, &mut events);

            world.clear_trade_session(self.session_id);
        }

        if !events.is_empty() {
            let _ = self.handle_world_events(events, out);
        }

        let pkt_self = STradeCancel { unlock: false };
        if let Ok(raw) = pkt_self.encode() {
            out.push(LoginConnection::encode_raw(raw));
        }

        let pkt_other = STradeCancel { unlock: false };
        if let Ok(raw) = pkt_other.encode() {
            let encoded = LoginConnection::encode_raw(raw);
            if partner_id == self.session_id {
                out.push(encoded);
            } else {
                let mut outboxes = self.outboxes.lock().unwrap();
                outboxes.entry(partner_id).or_default().push(encoded);
            }
        }

        tracing::debug!(
            "TradeCancel: session_id={} cancelled trade with session_id={} (refund_self={} refund_partner={})",
            self.session_id,
            partner_id,
            self_gold,
            partner_gold,
        );
    }

    fn encode_trade_items_bytes(slots: &[Option<UserItemData>]) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, slots.len() as i32)?;
        for slot in slots {
            match slot {
                None => {
                    write_bool(&mut buf, false)?;
                }
                Some(item) => {
                    write_bool(&mut buf, true)?;
                    let bytes = item.encode_to_bytes()?;
                    buf.extend_from_slice(&bytes);
                }
            }
        }
        Ok(buf)
    }

    pub(crate) fn handle_deposit_trade_item(
        &mut self,
        msg: CDepositTradeItem,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let (success, inv, eq, trade_slots, partner_id) = {
            let mut world = self.world.lock().unwrap();

            let ok = world.deposit_trade_item_for_player(self.session_id, msg.from, msg.to);

            let (inv, eq) = world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ));

            let trade_slots = world.player_trade_items(self.session_id);
            let partner = world.trade_partner_for(self.session_id);

            if ok {
                world.trade_unlock(self.session_id);
            }

            (ok, inv, eq, trade_slots, partner)
        };

        if !success {
            return;
        }

        let refresh = SUserSlotsRefresh {
            inventory: inv.slots,
            equipment: eq.slots,
        };
        if let Ok(raw) = refresh.encode() {
            out.push(LoginConnection::encode_raw(raw));
        }

        if let Some(partner_id) = partner_id {
            if let Ok(items_bytes) = Self::encode_trade_items_bytes(&trade_slots) {
                let pkt = STradeItem { items_bytes };
                let raw = pkt.encode();
                let encoded = LoginConnection::encode_raw(raw);
                if partner_id == self.session_id {
                    out.push(encoded);
                } else {
                    let mut outboxes = self.outboxes.lock().unwrap();
                    outboxes.entry(partner_id).or_default().push(encoded);
                }
            }
        }

        tracing::debug!(
            "DepositTradeItem: session_id={} from={} to={} success={}",
            self.session_id,
            msg.from,
            msg.to,
            success,
        );
    }

    pub(crate) fn handle_retrieve_trade_item(
        &mut self,
        msg: CRetrieveTradeItem,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let (success, inv, eq, trade_slots, partner_id) = {
            let mut world = self.world.lock().unwrap();

            let ok = world.retrieve_trade_item_for_player(self.session_id, msg.from, msg.to);

            let (inv, eq) = world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ));

            let trade_slots = world.player_trade_items(self.session_id);
            let partner = world.trade_partner_for(self.session_id);

            if ok {
                world.trade_unlock(self.session_id);
            }

            (ok, inv, eq, trade_slots, partner)
        };

        if !success {
            return;
        }

        let refresh = SUserSlotsRefresh {
            inventory: inv.slots,
            equipment: eq.slots,
        };
        if let Ok(raw) = refresh.encode() {
            out.push(LoginConnection::encode_raw(raw));
        }

        if let Some(partner_id) = partner_id {
            if let Ok(items_bytes) = Self::encode_trade_items_bytes(&trade_slots) {
                let pkt = STradeItem { items_bytes };
                let raw = pkt.encode();
                let encoded = LoginConnection::encode_raw(raw);
                if partner_id == self.session_id {
                    out.push(encoded);
                } else {
                    let mut outboxes = self.outboxes.lock().unwrap();
                    outboxes.entry(partner_id).or_default().push(encoded);
                }
            }
        }

        tracing::debug!(
            "RetrieveTradeItem: session_id={} from={} to={} success={}",
            self.session_id,
            msg.from,
            msg.to,
            success,
        );
    }
}

