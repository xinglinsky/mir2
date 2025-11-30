use std::io::{self, Cursor};
use std::sync::atomic::Ordering;
use std::time::Instant;

use crystal_server_core::account::CharacterPosition;
use crystal_server_net::ConnectionHandler;
use crystal_shared_proto::guild::{
    CEditGuildMember,
    CEditGuildNotice,
    CRequestGuildInfo,
    CGuildStorageGoldChange,
    CGuildStorageItemChange,
};
use crystal_shared_proto::io::read_string;
use crystal_shared_proto::item::{CDropItem, CStoreItem, CTakeBackItem};
use crystal_shared_proto::npc::{CBuyItem, CDepositTradeItem, CRetrieveTradeItem};
use crystal_shared_proto::login::{
    CAddMember,
    CAttack,
    CCallNPC,
    CChangeAMode,
    CChangePassword,
    CClientVersion,
    CCollectParcel,
    CDeleteCharacter,
    CDeleteMail,
    CDelMember,
    CEquipItem,
    CGameshopBuy,
    CGuildInvite,
    CGuildNameReturn,
    CGroupInvite,
    CKeepAlive,
    CLockMail,
    CLogin,
    CMagic,
    CMagicKey,
    CMailCost,
    CMailLockedItem,
    CMoveItem,
    CNewAccount,
    CNewCharacter,
    CPickUp,
    CReadMail,
    CRemoveItem,
    CRequestMapInfo,
    CRun,
    CSendMail,
    CSearchMap,
    CStartGame,
    CSwitchGroup,
    CTeleportToNPC,
    CTownRevive,
    CTradeCancel,
    CTradeConfirm,
    CTradeGold,
    CTradeReply,
    CTradeRequest,
    CTurn,
    CUseItem,
    CWalk,
    ClientPacketId,
    SConnected,
};
use crystal_shared_proto::packet::RawPacket;

use super::{LoginConnection, Stage};

impl ConnectionHandler for LoginConnection {
    fn on_connect(&mut self) -> Vec<Vec<u8>> {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        vec![Self::encode_raw(SConnected.encode())]
    }

    fn handle_packet(&mut self, packet: RawPacket) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        // Temporary debug: see whether we ever receive raw Buy/Sell item
        // packets from the client.
        if packet.id == ClientPacketId::BuyItem as i16 {
            tracing::debug!(
                "[ingame] raw BuyItem packet received: id={} payload_len={}",
                packet.id,
                packet.payload.len(),
            );
        } else if packet.id == ClientPacketId::SellItem as i16 {
            tracing::debug!(
                "[ingame] raw SellItem packet received: id={} payload_len={}",
                packet.id,
                packet.payload.len(),
            );
        }

        let Some(pid) = ClientPacketId::from_i16(packet.id) else {
            // Temporary debug: surface any packet IDs that are not mapped in
            // ClientPacketId so we can see what the client is actually
            // sending for operations like NPC Buy.
            tracing::debug!(
                "[ingame] unknown client packet id={} payload_len={}",
                packet.id,
                packet.payload.len(),
            );
            return out;
        };

        self.last_active = Instant::now();

        match pid {
            ClientPacketId::NewAccount => {
                if let Ok(msg) = CNewAccount::decode(&packet.payload) {
                    self.handle_new_account(msg, &mut out);
                }
            }
            ClientPacketId::ClientVersion => {
                if let Ok(msg) = CClientVersion::decode(&packet.payload) {
                    self.handle_client_version(msg, &mut out);
                }
            }
            ClientPacketId::Login => {
                if let Ok(msg) = CLogin::decode(&packet.payload) {
                    self.handle_login(msg, &mut out);
                }
            }
            ClientPacketId::ChangePassword => {
                if let Ok(msg) = CChangePassword::decode(&packet.payload) {
                    self.handle_change_password(msg, &mut out);
                }
            }
            ClientPacketId::NewCharacter => {
                if let Ok(msg) = CNewCharacter::decode(&packet.payload) {
                    self.handle_new_character(msg, &mut out);
                }
            }
            ClientPacketId::DeleteCharacter => {
                if let Ok(msg) = CDeleteCharacter::decode(&packet.payload) {
                    self.handle_delete_character(msg, &mut out);
                }
            }
            ClientPacketId::StartGame => {
                if let Ok(msg) = CStartGame::decode(&packet.payload) {
                    self.handle_start_game(msg, &mut out);
                }
            }
            ClientPacketId::LogOut => {
                self.handle_log_out(&mut out);
            }
            ClientPacketId::Turn => {
                if let Ok(msg) = CTurn::decode(&packet.payload) {
                    self.handle_turn(msg, &mut out);
                }
            }
            ClientPacketId::Walk => {
                if let Ok(msg) = CWalk::decode(&packet.payload) {
                    self.handle_walk(msg, &mut out);
                }
            }
            ClientPacketId::Run => {
                if let Ok(msg) = CRun::decode(&packet.payload) {
                    self.handle_run(msg, &mut out);
                }
            }
            ClientPacketId::Chat => {
                if let Ok(message) = (|| {
                    let mut c = Cursor::new(&packet.payload);
                    let text = read_string(&mut c)?;
                    Ok::<String, io::Error>(text)
                })() {
                    self.handle_chat(message, &mut out);
                }
            }
            ClientPacketId::MoveItem => {
                if let Ok(msg) = CMoveItem::decode(&packet.payload) {
                    self.handle_move_item(msg, &mut out);
                }
            }
            ClientPacketId::StoreItem => {
                if let Ok(msg) = CStoreItem::decode(&packet.payload) {
                    self.handle_store_item(msg, &mut out);
                }
            }
            ClientPacketId::TakeBackItem => {
                if let Ok(msg) = CTakeBackItem::decode(&packet.payload) {
                    self.handle_take_back_item(msg, &mut out);
                }
            }
            ClientPacketId::UseItem => {
                if let Ok(msg) = CUseItem::decode(&packet.payload) {
                    self.handle_use_item(msg, &mut out);
                }
            }
            ClientPacketId::DropItem => {
                if let Ok(msg) = CDropItem::decode(&packet.payload) {
                    self.handle_drop_item(msg, &mut out);
                }
            }
            ClientPacketId::BuyItem => {
                tracing::debug!(
                    "[ingame] dispatch BuyItem: id={} payload_len={}",
                    packet.id,
                    packet.payload.len(),
                );
                match CBuyItem::decode(&packet.payload) {
                    Ok(msg) => {
                        tracing::debug!(
                            "[ingame] decoded BuyItem: item_index={} count={} panel_type={}",
                            msg.item_index,
                            msg.count,
                            msg.panel_type,
                        );
                        self.handle_buy_item(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "[ingame] failed to decode CBuyItem: {:?}, payload_len={}",
                            e,
                            packet.payload.len(),
                        );
                    }
                }
            }
            ClientPacketId::SellItem => {
                tracing::debug!(
                    "[ingame] dispatch SellItem: id={} payload_len={}",
                    packet.id,
                    packet.payload.len(),
                );
                match crystal_shared_proto::npc::CSellItem::decode(&packet.payload) {
                    Ok(msg) => {
                        tracing::debug!(
                            "[ingame] decoded SellItem: unique_id={} count={}",
                            msg.unique_id,
                            msg.count,
                        );
                        self.handle_sell_item(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "[ingame] failed to decode CSellItem: {:?}, payload_len={}",
                            e,
                            packet.payload.len(),
                        );
                    }
                }
            }
            ClientPacketId::EquipItem => {
                if let Ok(msg) = CEquipItem::decode(&packet.payload) {
                    self.handle_equip_item(msg, &mut out);
                }
            }
            ClientPacketId::RemoveItem => {
                if let Ok(msg) = CRemoveItem::decode(&packet.payload) {
                    self.handle_remove_item(msg, &mut out);
                }
            }
            ClientPacketId::CallNPC => {
                if let Ok(msg) = CCallNPC::decode(&packet.payload) {
                    self.handle_call_npc(msg, &mut out);
                }
            }
            ClientPacketId::MagicKey => {
                if let Ok(msg) = CMagicKey::decode(&packet.payload) {
                    self.handle_magic_key(msg, &mut out);
                }
            }
            ClientPacketId::ChangeAMode => {
                if let Ok(msg) = CChangeAMode::decode(&packet.payload) {
                    self.handle_change_attack_mode(msg, &mut out);
                }
            }
            ClientPacketId::Magic => {
                if let Ok(msg) = CMagic::decode(&packet.payload) {
                    self.handle_magic(msg, &mut out);
                }
            }
            ClientPacketId::Attack => {
                if let Ok(msg) = CAttack::decode(&packet.payload) {
                    self.handle_attack(msg, &mut out);
                }
            }
            ClientPacketId::PickUp => {
                if let Ok(msg) = CPickUp::decode(&packet.payload) {
                    self.handle_pick_up(msg, &mut out);
                }
            }
            ClientPacketId::RequestMapInfo => {
                if let Ok(msg) = CRequestMapInfo::decode(&packet.payload) {
                    self.handle_request_map_info(msg, &mut out);
                }
            }
            ClientPacketId::TeleportToNPC => {
                if let Ok(msg) = CTeleportToNPC::decode(&packet.payload) {
                    self.handle_teleport_to_npc(msg, &mut out);
                }
            }
            ClientPacketId::SearchMap => {
                if let Ok(msg) = CSearchMap::decode(&packet.payload) {
                    self.handle_search_map(msg, &mut out);
                }
            }
            ClientPacketId::EditGuildMember => {
                if let Ok(msg) = CEditGuildMember::decode(&packet.payload) {
                    self.handle_edit_guild_member(msg, &mut out);
                }
            }
            ClientPacketId::EditGuildNotice => {
                if let Ok(msg) = CEditGuildNotice::decode(&packet.payload) {
                    self.handle_edit_guild_notice(msg, &mut out);
                }
            }
            ClientPacketId::TownRevive => {
                if let Ok(msg) = CTownRevive::decode(&packet.payload) {
                    self.handle_town_revive(msg, &mut out);
                }
            }
            ClientPacketId::GuildInvite => {
                if let Ok(msg) = CGuildInvite::decode(&packet.payload) {
                    self.handle_guild_invite(msg, &mut out);
                }
            }
            ClientPacketId::GuildNameReturn => {
                if let Ok(msg) = CGuildNameReturn::decode(&packet.payload) {
                    self.handle_guild_name_return(msg, &mut out);
                }
            }
            ClientPacketId::RequestGuildInfo => {
                if let Ok(msg) = CRequestGuildInfo::decode(&packet.payload) {
                    self.handle_request_guild_info(msg, &mut out);
                }
            }
            ClientPacketId::GuildStorageGoldChange => {
                match CGuildStorageGoldChange::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_guild_storage_gold_change(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CGuildStorageGoldChange from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::GuildStorageItemChange => {
                match CGuildStorageItemChange::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_guild_storage_item_change(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CGuildStorageItemChange from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::GuildWarReturn => {
                tracing::debug!("GuildWarReturn packet received but not implemented yet");
                if self.stage == Stage::InGame {
                    self.send_system_chat(
                        "行会战争相关功能尚未在 Rust 服务器上实现。",
                        &mut out,
                    );
                }
            }
            ClientPacketId::KeepAlive => {
                if let Ok(msg) = CKeepAlive::decode(&packet.payload) {
                    self.handle_keep_alive(msg, &mut out);
                }
            }
            ClientPacketId::Disconnect => {
                self.closing = true;
            }
            // Harvest protocol (stub - not implemented yet)
            ClientPacketId::Harvest => {
                tracing::debug!("Harvest packet received but not implemented yet");
                // TODO: Implement harvest logic
                if self.stage == Stage::InGame {
                    self.send_system_chat("采集系统尚未在 Rust 服务器上实现。", &mut out);
                }
            }
            // Trade protocols: request and reply are now partially handled.
            ClientPacketId::TradeRequest => {
                match CTradeRequest::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_trade_request(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CTradeRequest from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::TradeReply => {
                match CTradeReply::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_trade_reply(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CTradeReply from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::TradeGold => {
                match CTradeGold::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_trade_gold(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CTradeGold from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::TradeConfirm => {
                match CTradeConfirm::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_trade_confirm(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CTradeConfirm from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::TradeCancel => {
                match CTradeCancel::decode(&packet.payload) {
                    Ok(_) => {
                        self.handle_trade_cancel(&mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CTradeCancel from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            // Market protocols (stub - not implemented yet)
            ClientPacketId::ConsignItem => {
                tracing::debug!("ConsignItem packet received but not implemented yet");
                if self.stage == Stage::InGame {
                    self.send_system_chat(
                        "市场/拍卖行系统尚未在 Rust 服务器上实现。",
                        &mut out,
                    );
                }
            }
            ClientPacketId::MarketSearch => {
                tracing::debug!("MarketSearch packet received but not implemented yet");
                // TODO: Implement market search logic
                if self.stage == Stage::InGame {
                    self.send_system_chat(
                        "市场/拍卖行系统尚未在 Rust 服务器上实现。",
                        &mut out,
                    );
                }
            }
            ClientPacketId::MarketRefresh => {
                tracing::debug!("MarketRefresh packet received but not implemented yet");
                // TODO: Implement market refresh logic
                if self.stage == Stage::InGame {
                    self.send_system_chat(
                        "市场/拍卖行系统尚未在 Rust 服务器上实现。",
                        &mut out,
                    );
                }
            }
            ClientPacketId::MarketPage => {
                tracing::debug!("MarketPage packet received but not implemented yet");
                // TODO: Implement market page logic
                if self.stage == Stage::InGame {
                    self.send_system_chat(
                        "市场/拍卖行系统尚未在 Rust 服务器上实现。",
                        &mut out,
                    );
                }
            }
            ClientPacketId::MarketBuy => {
                tracing::debug!("MarketBuy packet received but not implemented yet");
                // TODO: Implement market buy logic
                if self.stage == Stage::InGame {
                    self.send_system_chat(
                        "市场/拍卖行系统尚未在 Rust 服务器上实现。",
                        &mut out,
                    );
                }
            }
            ClientPacketId::MarketGetBack => {
                tracing::debug!("MarketGetBack packet received but not implemented yet");
                // TODO: Implement market get back logic
                if self.stage == Stage::InGame {
                    self.send_system_chat(
                        "市场/拍卖行系统尚未在 Rust 服务器上实现。",
                        &mut out,
                    );
                }
            }
            ClientPacketId::MarketSellNow => {
                tracing::debug!("MarketSellNow packet received but not implemented yet");
                // TODO: Implement market sell now logic
                if self.stage == Stage::InGame {
                    self.send_system_chat(
                        "市场/拍卖行系统尚未在 Rust 服务器上实现。",
                        &mut out,
                    );
                }
            }
            ClientPacketId::DepositTradeItem => {
                match CDepositTradeItem::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_deposit_trade_item(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CDepositTradeItem from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            ClientPacketId::RetrieveTradeItem => {
                match CRetrieveTradeItem::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_retrieve_trade_item(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CRetrieveTradeItem from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            // Quest protocols (stub - not implemented yet)
            ClientPacketId::AcceptQuest => {
                tracing::debug!("AcceptQuest packet received but not implemented yet");
                // TODO: Implement accept quest logic
                if self.stage == Stage::InGame {
                    self.send_system_chat(
                        "任务系统尚未在 Rust 服务器上实现。",
                        &mut out,
                    );
                }
            }
            ClientPacketId::FinishQuest => {
                tracing::debug!("FinishQuest packet received but not implemented yet");
                // TODO: Implement finish quest logic
                if self.stage == Stage::InGame {
                    self.send_system_chat(
                        "任务系统尚未在 Rust 服务器上实现。",
                        &mut out,
                    );
                }
            }
            ClientPacketId::AbandonQuest => {
                tracing::debug!("AbandonQuest packet received but not implemented yet");
                // TODO: Implement abandon quest logic
                if self.stage == Stage::InGame {
                    self.send_system_chat(
                        "任务系统尚未在 Rust 服务器上实现。",
                        &mut out,
                    );
                }
            }
            ClientPacketId::ShareQuest => {
                tracing::debug!("ShareQuest packet received but not implemented yet");
                // TODO: Implement share quest logic
                if self.stage == Stage::InGame {
                    self.send_system_chat(
                        "任务系统尚未在 Rust 服务器上实现。",
                        &mut out,
                    );
                }
            }
            // Group protocols (stub - not implemented yet)
            ClientPacketId::SwitchGroup => {
                if let Ok(msg) = CSwitchGroup::decode(&packet.payload) {
                    self.handle_switch_group(msg, &mut out);
                }
            }
            ClientPacketId::AddMember => {
                if let Ok(msg) = CAddMember::decode(&packet.payload) {
                    self.handle_add_member(msg, &mut out);
                }
            }
            ClientPacketId::DellMember => {
                if let Ok(msg) = CDelMember::decode(&packet.payload) {
                    self.handle_del_member(msg, &mut out);
                }
            }
            ClientPacketId::GroupInvite => {
                if let Ok(msg) = CGroupInvite::decode(&packet.payload) {
                    self.handle_group_invite(msg, &mut out);
                }
            }
            ClientPacketId::SendMail => {
                if let Ok(msg) = CSendMail::decode(&packet.payload) {
                    self.handle_send_mail(msg, &mut out);
                }
            }
            ClientPacketId::ReadMail => {
                if let Ok(msg) = CReadMail::decode(&packet.payload) {
                    self.handle_read_mail(msg, &mut out);
                }
            }
            ClientPacketId::CollectParcel => {
                if let Ok(msg) = CCollectParcel::decode(&packet.payload) {
                    self.handle_collect_parcel(msg, &mut out);
                }
            }
            ClientPacketId::DeleteMail => {
                if let Ok(msg) = CDeleteMail::decode(&packet.payload) {
                    self.handle_delete_mail(msg, &mut out);
                }
            }
            ClientPacketId::LockMail => {
                if let Ok(msg) = CLockMail::decode(&packet.payload) {
                    self.handle_lock_mail(msg, &mut out);
                }
            }
            ClientPacketId::MailLockedItem => {
                if let Ok(msg) = CMailLockedItem::decode(&packet.payload) {
                    self.handle_mail_locked_item(msg, &mut out);
                }
            }
            ClientPacketId::MailCost => {
                if let Ok(msg) = CMailCost::decode(&packet.payload) {
                    self.handle_mail_cost(msg, &mut out);
                }
            }
            ClientPacketId::GameshopBuy => {
                match CGameshopBuy::decode(&packet.payload) {
                    Ok(msg) => {
                        self.handle_gameshop_buy(msg, &mut out);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to decode CGameshopBuy from session_id={} err={:?}",
                            self.session_id,
                            e,
                        );
                    }
                }
            }
            _ => {
                tracing::debug!(
                    "Unhandled client packet {:?} (id={} payload_len={})",
                    pid,
                    packet.id,
                    packet.payload.len(),
                );
                if self.stage == Stage::InGame {
                    self.send_system_chat(
                        "该客户端操作尚未在 Rust 服务器上实现。",
                        &mut out,
                    );
                }
            }
        }

        out
    }

    fn poll_outbound(&mut self) -> Vec<Vec<u8>> {
        let mut out = Vec::new();

        if self.stage == Stage::InGame && self.current_map_index != 0 {
            self.update_visibility(&mut out);
        }

        let mut outboxes = self.outboxes.lock().unwrap();
        if let Some(mut queued) = outboxes.remove(&self.session_id) {
            out.append(&mut queued);
        }

        if !self.closing {
            let elapsed = self.last_active.elapsed();
            if elapsed.as_millis() as u64 > self.timeout_ms {
                self.closing = true;
            }
        }

        out
    }

    fn on_disconnect(&mut self) {
        if self.stage == Stage::InGame {
            if let (Some(ref account_id), Some(char_idx)) =
                (self.account_id.as_ref(), self.current_char_index)
            {
                let magics = {
                    let world = self.world.lock().unwrap();
                    world.player_magics(self.session_id)
                };
                let _ = self
                    .store
                    .save_character_magics(account_id, char_idx, &magics);

                let (inventory, equipment) = {
                    let world = self.world.lock().unwrap();
                    world
                        .player_items(self.session_id)
                        .unwrap_or((crystal_server_core::item::Inventory::new_default(), crystal_server_core::item::Equipment::new_default()))
                };
                let _ = self
                    .store
                    .save_character_items(account_id, char_idx, &inventory, &equipment);

                let pos = CharacterPosition {
                    map_index: self.current_map_index,
                    x: self.current_x,
                    y: self.current_y,
                    direction: self.direction,
                };
                let _ = self
                    .store
                    .save_character_position(account_id, char_idx, &pos);

                if let Some(ch) = self
                    .characters
                    .iter()
                    .find(|c| c.index == char_idx)
                {
                    let _ = self
                        .store
                        .update_character_level(account_id, char_idx, ch.level);
                }
            }

            // Remove the player from the world state and occupancy tracking so
            // that disconnected characters no longer block movement.
            {
                let mut world = self.world.lock().unwrap();
                world.remove_player_from_world(self.session_id);
            }
        }

        if let Some(ref account_id) = self.account_id {
            let mut map = self.online_accounts.lock().unwrap();
            if let Some(cur) = map.get(account_id) {
                if *cur == self.session_id {
                    map.remove(account_id);
                }
            }
        }

        {
            let mut map = self.player_summaries.lock().unwrap();
            map.remove(&self.session_id);
        }

        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    fn should_close(&self) -> bool {
        self.closing
    }
}
