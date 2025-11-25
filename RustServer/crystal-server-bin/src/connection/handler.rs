use std::io::{self, Cursor};
use std::sync::atomic::Ordering;
use std::time::Instant;

use crystal_server_core::account::CharacterPosition;
use crystal_server_net::ConnectionHandler;
use crystal_shared_proto::io::read_string;
use crystal_shared_proto::item::{CBuyItem, CDropItem};
use crystal_shared_proto::login::{
    CAttack,
    CCallNPC,
    CChangePassword,
    CEquipItem,
    CKeepAlive,
    CClientVersion,
    CDeleteCharacter,
    CLogin,
    CMoveItem,
    CNewAccount,
    CNewCharacter,
    CRemoveItem,
    CRun,
    CStartGame,
    CTownRevive,
    CPickUp,
    CTurn,
    CUseItem,
    CWalk,
    CRequestMapInfo,
    CTeleportToNPC,
    CSearchMap,
    CGuildInvite,
    CGuildNameReturn,
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
            println!(
                "[ingame] raw BuyItem packet received: id={} payload_len={}",
                packet.id,
                packet.payload.len(),
            );
        } else if packet.id == ClientPacketId::SellItem as i16 {
            println!(
                "[ingame] raw SellItem packet received: id={} payload_len={}",
                packet.id,
                packet.payload.len(),
            );
        }

        let Some(pid) = ClientPacketId::from_i16(packet.id) else {
            // Temporary debug: surface any packet IDs that are not mapped in
            // ClientPacketId so we can see what the client is actually
            // sending for operations like NPC Buy.
            println!(
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
                println!(
                    "[ingame] dispatch BuyItem: id={} payload_len={}",
                    packet.id,
                    packet.payload.len(),
                );
                match CBuyItem::decode(&packet.payload) {
                    Ok(msg) => {
                        println!(
                            "[ingame] decoded BuyItem: item_index={} count={} panel_type={}",
                            msg.item_index,
                            msg.count,
                            msg.panel_type,
                        );
                        self.handle_buy_item(msg, &mut out);
                    }
                    Err(e) => {
                        println!(
                            "[ingame] failed to decode CBuyItem: {:?}, payload_len={}",
                            e,
                            packet.payload.len(),
                        );
                    }
                }
            }
            ClientPacketId::SellItem => {
                println!(
                    "[ingame] dispatch SellItem: id={} payload_len={}",
                    packet.id,
                    packet.payload.len(),
                );
                match crystal_shared_proto::item::CSellItem::decode(&packet.payload) {
                    Ok(msg) => {
                        println!(
                            "[ingame] decoded SellItem: unique_id={} count={}",
                            msg.unique_id,
                            msg.count,
                        );
                        self.handle_sell_item(msg, &mut out);
                    }
                    Err(e) => {
                        println!(
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
            }
            // Trade protocols (stub - not implemented yet)
            ClientPacketId::TradeRequest => {
                tracing::debug!("TradeRequest packet received but not implemented yet");
                // TODO: Implement trade request logic
            }
            ClientPacketId::TradeReply => {
                tracing::debug!("TradeReply packet received but not implemented yet");
                // TODO: Implement trade reply logic
            }
            ClientPacketId::TradeGold => {
                tracing::debug!("TradeGold packet received but not implemented yet");
                // TODO: Implement trade gold logic
            }
            ClientPacketId::TradeConfirm => {
                tracing::debug!("TradeConfirm packet received but not implemented yet");
                // TODO: Implement trade confirm logic
            }
            ClientPacketId::TradeCancel => {
                tracing::debug!("TradeCancel packet received but not implemented yet");
                // TODO: Implement trade cancel logic
            }
            // Market protocols (stub - not implemented yet)
            ClientPacketId::ConsignItem => {
                tracing::debug!("ConsignItem packet received but not implemented yet");
            }
            ClientPacketId::MarketSearch => {
                tracing::debug!("MarketSearch packet received but not implemented yet");
                // TODO: Implement market search logic
            }
            ClientPacketId::MarketRefresh => {
                tracing::debug!("MarketRefresh packet received but not implemented yet");
                // TODO: Implement market refresh logic
            }
            ClientPacketId::MarketPage => {
                tracing::debug!("MarketPage packet received but not implemented yet");
                // TODO: Implement market page logic
            }
            ClientPacketId::MarketBuy => {
                tracing::debug!("MarketBuy packet received but not implemented yet");
                // TODO: Implement market buy logic
            }
            ClientPacketId::MarketGetBack => {
                tracing::debug!("MarketGetBack packet received but not implemented yet");
                // TODO: Implement market get back logic
            }
            ClientPacketId::MarketSellNow => {
                tracing::debug!("MarketSellNow packet received but not implemented yet");
                // TODO: Implement market sell now logic
            }
            // Quest protocols (stub - not implemented yet)
            ClientPacketId::AcceptQuest => {
                tracing::debug!("AcceptQuest packet received but not implemented yet");
                // TODO: Implement accept quest logic
            }
            ClientPacketId::FinishQuest => {
                tracing::debug!("FinishQuest packet received but not implemented yet");
                // TODO: Implement finish quest logic
            }
            ClientPacketId::AbandonQuest => {
                tracing::debug!("AbandonQuest packet received but not implemented yet");
                // TODO: Implement abandon quest logic
            }
            ClientPacketId::ShareQuest => {
                tracing::debug!("ShareQuest packet received but not implemented yet");
                // TODO: Implement share quest logic
            }
            // Group protocols (stub - not implemented yet)
            ClientPacketId::SwitchGroup => {
                tracing::debug!("SwitchGroup packet received but not implemented yet");
                // TODO: Implement switch group logic
            }
            ClientPacketId::AddMember => {
                tracing::debug!("AddMember packet received but not implemented yet");
                // TODO: Implement add member logic
            }
            ClientPacketId::DellMember => {
                tracing::debug!("DellMember packet received but not implemented yet");
                // TODO: Implement delete member logic
            }
            ClientPacketId::GroupInvite => {
                tracing::debug!("GroupInvite packet received but not implemented yet");
                // TODO: Implement group invite logic
            }
        }

        out
    }

    fn poll_outbound(&mut self) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
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

                let pos = CharacterPosition {
                    map_index: self.current_map_index,
                    x: self.current_x,
                    y: self.current_y,
                    direction: self.direction,
                };
                let _ = self
                    .store
                    .save_character_position(account_id, char_idx, &pos);
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
