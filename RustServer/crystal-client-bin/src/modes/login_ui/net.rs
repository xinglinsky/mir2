use bevy::prelude::*;

use crystal_client_net::{NetClient, NetEvent};
use crystal_shared_proto::login::{
    CClientVersion, CLogin, CStartGame, SClientVersion, SLogin, SLoginBanned, SStartGame,
    SStartGameBanned, SStartGameDelay, ServerPacketId,
};
use crystal_shared_proto::map::{SMapChanged, SMapInformation};
use crystal_shared_proto::select::SLoginSuccess;
use crystal_shared_proto::user::{SUserInformation, SUserLocation};

use super::state::{
    is_valid_account, is_valid_password, LoginUiNetState, LoginUiStage, LoginUiStartLogin, LoginUiState,
};

pub(crate) fn login_ui_begin_login(
    mut ev: EventReader<LoginUiStartLogin>,
    state: Res<LoginUiState>,
    mut net_state: ResMut<LoginUiNetState>,
) {
    if ev.read().next().is_none() {
        return;
    }

    if !is_valid_account(&state.account) || !is_valid_password(&state.password) {
        return;
    }

    net_state.pending_login = Some((state.account.clone(), state.password.clone()));

    if net_state.net.is_some() {
        return;
    }

    match NetClient::connect(&net_state.server_addr) {
        Ok(net) => {
            net_state.net = Some(net);
            net_state.connected = false;
            net_state.just_connected = false;
            net_state.sent_version = false;
            net_state.version_checked = false;
            net_state.logged_in = false;
            net_state.characters.clear();
            net_state.selected_character_index = None;
            net_state.pending_start_game = None;
            net_state.start_game_ok = false;
            net_state.map_index = None;
            net_state.map_file_name = None;
            net_state.user_name = None;
            net_state.user_location_x = None;
            net_state.user_location_y = None;
            net_state.last_error = None;
        }
        Err(e) => {
            net_state.last_error = Some(format!("connect error: {e}"));
        }
    }
}

pub(crate) fn login_ui_pump_net(mut stage: ResMut<LoginUiStage>, mut net_state: ResMut<LoginUiNetState>) {
    let mut events = Vec::new();
    if let Some(net) = net_state.net.as_mut() {
        while let Some(ev) = net.try_recv() {
            events.push(ev);
        }
    } else {
        return;
    }

    net_state.just_connected = false;

    for ev in events {
        match ev {
            NetEvent::Connected => {
                net_state.connected = true;
                net_state.just_connected = true;
            }
            NetEvent::Disconnected => {
                net_state.connected = false;
                net_state.sent_version = false;
                net_state.version_checked = false;
                net_state.logged_in = false;
                net_state.characters.clear();
                net_state.selected_character_index = None;
                net_state.pending_start_game = None;
                net_state.start_game_ok = false;
                net_state.map_index = None;
                net_state.map_file_name = None;
                net_state.user_name = None;
                net_state.user_location_x = None;
                net_state.user_location_y = None;
            }
            NetEvent::Error(e) => {
                net_state.last_error = Some(e);
            }
            NetEvent::Packet(pkt) => {
                if pkt.id == ServerPacketId::ClientVersion as i16 {
                    if let Ok(msg) = SClientVersion::decode(&pkt.payload) {
                        net_state.version_checked = msg.result == 1;
                    }
                } else if pkt.id == ServerPacketId::Login as i16 {
                    if let Ok(msg) = SLogin::decode(&pkt.payload) {
                        if msg.result != 1 {
                            net_state.last_error = Some(format!("login failed result={}", msg.result));
                        }
                    }
                } else if pkt.id == ServerPacketId::LoginBanned as i16 {
                    if let Ok(msg) = SLoginBanned::decode(&pkt.payload) {
                        net_state.last_error = Some(format!("banned: {}", msg.reason));
                    }
                } else if pkt.id == ServerPacketId::LoginSuccess as i16 {
                    if let Ok(msg) = SLoginSuccess::decode(&pkt.payload) {
                        net_state.logged_in = true;
                        net_state.characters = msg.characters;
                        net_state.selected_character_index =
                            net_state.characters.first().map(|c| c.index);
                        *stage = LoginUiStage::Select;
                    }
                } else if pkt.id == ServerPacketId::StartGame as i16 {
                    if let Ok(_msg) = SStartGame::decode(&pkt.payload) {
                        net_state.start_game_ok = true;
                        *stage = LoginUiStage::InGame;
                    }
                } else if pkt.id == ServerPacketId::StartGameDelay as i16 {
                    if let Ok(msg) = SStartGameDelay::decode(&pkt.payload) {
                        net_state.last_error = Some(format!("start game delay: {}", msg.milliseconds));
                    }
                } else if pkt.id == ServerPacketId::StartGameBanned as i16 {
                    if let Ok(msg) = SStartGameBanned::decode(&pkt.payload) {
                        net_state.last_error = Some(format!("start game banned: {}", msg.reason));
                    }
                } else if pkt.id == ServerPacketId::MapInformation as i16 {
                    if let Ok(msg) = SMapInformation::decode(&pkt.payload) {
                        net_state.map_index = Some(msg.map_index);
                        net_state.map_file_name = Some(msg.file_name);
                        *stage = LoginUiStage::InGame;
                    }
                } else if pkt.id == ServerPacketId::MapChanged as i16 {
                    if let Ok(msg) = SMapChanged::decode(&pkt.payload) {
                        net_state.map_index = Some(msg.map_index);
                        net_state.map_file_name = Some(msg.file_name);
                        *stage = LoginUiStage::InGame;
                    }
                } else if pkt.id == ServerPacketId::UserInformation as i16 {
                    if let Ok(msg) = SUserInformation::decode(&pkt.payload) {
                        net_state.user_name = Some(msg.name);
                        *stage = LoginUiStage::InGame;
                    }
                } else if pkt.id == ServerPacketId::UserLocation as i16 {
                    if let Ok(msg) = SUserLocation::decode(&pkt.payload) {
                        net_state.user_location_x = Some(msg.location_x);
                        net_state.user_location_y = Some(msg.location_y);
                        *stage = LoginUiStage::InGame;
                    }
                }
            }
        }
    }

    if net_state.just_connected && !net_state.sent_version {
        if let Some(net) = net_state.net.as_ref() {
            if let Ok(pkt) = (CClientVersion {
                version_hash: Vec::new(),
            })
            .encode()
            {
                let _ = net.send_raw(pkt);
                net_state.sent_version = true;
            }
        }
    }

    if net_state.connected && net_state.version_checked {
        if let Some((account_id, password)) = net_state.pending_login.take() {
            if let Some(net) = net_state.net.as_ref() {
                match (CLogin { account_id, password }).encode() {
                    Ok(pkt) => {
                        let _ = net.send_raw(pkt);
                    }
                    Err(e) => {
                        net_state.last_error = Some(format!("CLogin encode error: {e}"));
                    }
                }
            }
        }
    }

    if net_state.connected && net_state.logged_in {
        if let Some(character_index) = net_state.pending_start_game.take() {
            if let Some(net) = net_state.net.as_ref() {
                match (CStartGame { character_index }).encode() {
                    Ok(pkt) => {
                        let _ = net.send_raw(pkt);
                    }
                    Err(e) => {
                        net_state.last_error = Some(format!("CStartGame encode error: {e}"));
                    }
                }
            }
        }
    }
}
