use std::io::{self, Cursor};
use std::sync::atomic::Ordering;
use std::time::Instant;

use crystal_server_core::account::CharacterPosition;
use crystal_server_net::ConnectionHandler;
use crystal_shared_proto::io::read_string;
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
    CTurn,
    CWalk,
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

        let Some(pid) = ClientPacketId::from_i16(packet.id) else {
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
