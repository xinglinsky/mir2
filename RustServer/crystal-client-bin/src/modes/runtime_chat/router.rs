use bevy::prelude::*;

use crystal_client_net::NetEvent;
use crystal_shared_proto::guild::SGuildBuffList;
use crystal_shared_proto::item::{SNewItemInfo, SNewRecipeInfo};
use crystal_shared_proto::login::{
    CClientVersion, CLogin, CStartGame, SClientVersion, SConnected, SKeepAlive, SLogin, SLoginBanned,
    SStartGame, SStartGameBanned, SStartGameDelay, ServerPacketId,
};
use crystal_shared_proto::mail::SReceiveMail;
use crystal_shared_proto::map::{SMapChanged, SMapInformation};
use crystal_shared_proto::npc::{SDefaultNpc, SNpcResponse, SNpcUpdate};
use crystal_shared_proto::quest::{SCompleteQuest, SNewQuestInfo};
use crystal_shared_proto::scene::{
    SAddBuff, SChat, SObjectChat, SObjectColourChanged, SObjectHealth, SObjectHero, SObjectMonster,
    SObjectNpc, SObjectPlayer, SObjectRemove, SObjectRun, SObjectTeleportIn, SObjectTurn, SObjectWalk,
    SSwitchGroup,
};
use crystal_shared_proto::select::SLoginSuccess;
use crystal_shared_proto::shop::SGameShopInfo;
use crystal_shared_proto::social::{SFriendUpdate, SLoverUpdate, SMentorUpdate};
use crystal_shared_proto::stats::SBaseStatsInfo;
use crystal_shared_proto::user::{
    SFishingUpdate, SChangeAMode, SChangePMode, SHealthChanged, SInTrapRock, STimeOfDay,
    SUserInformation, SUserLocation,
};

use super::state::{log_key_event, log_unhandled_packet, ChatState, ObjKind, ObjState};

pub(super) fn pump_net_events(mut chat: ResMut<ChatState>) {
    let chat = &mut *chat;

    let mut events = Vec::new();
    if let Some(net) = chat.net.as_mut() {
        while let Some(ev) = net.try_recv() {
            events.push(ev);
        }
    } else {
        return;
    }

    let mut became_connected = false;

    for ev in events {
        match ev {
            NetEvent::Connected => {
                chat.connected = true;
                became_connected = true;
                chat.lines.push("[net] connected".to_string());
            }
            NetEvent::Disconnected => {
                chat.connected = false;
                chat.sent_version = false;
                chat.version_checked = false;
                chat.logged_in = false;
                chat.pending_start_game = None;
                chat.lines.push("[net] disconnected".to_string());
            }
            NetEvent::Error(e) => {
                chat.last_error = Some(e.clone());
                chat.lines.push(format!("[net] {e}"));
            }
            NetEvent::Packet(pkt) => {
                if pkt.id == ServerPacketId::ClientVersion as i16 {
                    if let Ok(msg) = SClientVersion::decode(&pkt.payload) {
                        chat.version_checked = msg.result == 1;
                        log_key_event(chat, format!("[login] version result={}", msg.result));
                    }
                } else if pkt.id == ServerPacketId::Login as i16 {
                    if let Ok(msg) = SLogin::decode(&pkt.payload) {
                        log_key_event(chat, format!("[login] login result={}", msg.result));
                    }
                } else if pkt.id == ServerPacketId::LoginBanned as i16 {
                    if let Ok(msg) = SLoginBanned::decode(&pkt.payload) {
                        log_key_event(chat, format!("[login] banned: {}", msg.reason));
                    }
                } else if pkt.id == ServerPacketId::LoginSuccess as i16 {
                    if let Ok(msg) = SLoginSuccess::decode(&pkt.payload) {
                        chat.logged_in = true;
                        log_key_event(
                            chat,
                            format!("[login] login success, chars={}", msg.characters.len()),
                        );
                        for ch in msg.characters {
                            log_key_event(
                                chat,
                                format!(
                                    "[char] index={} name={} level={} class={} gender={} ",
                                    ch.index, ch.name, ch.level, ch.class, ch.gender
                                ),
                            );
                        }
                    }
                } else if pkt.id == ServerPacketId::StartGame as i16 {
                    if let Ok(msg) = SStartGame::decode(&pkt.payload) {
                        log_key_event(
                            chat,
                            format!(
                                "[start_game] result={} resolution={} ",
                                msg.result, msg.resolution
                            ),
                        );
                    }
                } else if pkt.id == ServerPacketId::StartGameBanned as i16 {
                    if let Ok(msg) = SStartGameBanned::decode(&pkt.payload) {
                        log_key_event(chat, format!("[start_game] banned: {}", msg.reason));
                    }
                } else if pkt.id == ServerPacketId::StartGameDelay as i16 {
                    if let Ok(msg) = SStartGameDelay::decode(&pkt.payload) {
                        log_key_event(chat, format!("[start_game] delay ms={}", msg.milliseconds));
                    }
                } else if pkt.id == ServerPacketId::Connected as i16 {
                    let _ = SConnected::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::MapInformation as i16 {
                    if let Ok(msg) = SMapInformation::decode(&pkt.payload) {
                        chat.map_index = Some(msg.map_index);
                        chat.map_file_name = Some(msg.file_name.clone());
                        log_key_event(
                            chat,
                            format!(
                                "[map] info index={} file={} title={} ",
                                msg.map_index, msg.file_name, msg.title
                            ),
                        );
                    }
                } else if pkt.id == ServerPacketId::MapChanged as i16 {
                    if let Ok(msg) = SMapChanged::decode(&pkt.payload) {
                        chat.map_index = Some(msg.map_index);
                        chat.map_file_name = Some(msg.file_name.clone());
                        chat.user_location_x = Some(msg.location_x);
                        chat.user_location_y = Some(msg.location_y);
                        chat.user_direction = Some(msg.direction);
                        log_key_event(
                            chat,
                            format!(
                                "[map] changed index={} file={} x={} y={} dir={} ",
                                msg.map_index,
                                msg.file_name,
                                msg.location_x,
                                msg.location_y,
                                msg.direction
                            ),
                        );
                    }
                } else if pkt.id == ServerPacketId::NewItemInfo as i16 {
                    let _ = SNewItemInfo::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::NewQuestInfo as i16 {
                    let _ = SNewQuestInfo::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::NewRecipeInfo as i16 {
                    let _ = SNewRecipeInfo::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::ObjectNpc as i16 {
                    if let Ok(msg) = SObjectNpc::decode(&pkt.payload) {
                        chat.objects.insert(
                            msg.object_id,
                            ObjState {
                                kind: ObjKind::Npc,
                                name: Some(msg.name),
                                x: msg.location_x,
                                y: msg.location_y,
                                dir: msg.direction,
                            },
                        );
                    }
                } else if pkt.id == ServerPacketId::ObjectPlayer as i16 {
                    if let Ok(msg) = SObjectPlayer::decode(&pkt.payload) {
                        let is_new = !chat.objects.contains_key(&msg.object_id);
                        chat.objects.insert(
                            msg.object_id,
                            ObjState {
                                kind: ObjKind::Player,
                                name: Some(msg.name.clone()),
                                x: msg.location_x,
                                y: msg.location_y,
                                dir: msg.direction,
                            },
                        );
                        if is_new {
                            log_key_event(
                                chat,
                                format!(
                                    "[obj] player spawn id={} name={} x={} y={} dir={} ",
                                    msg.object_id,
                                    msg.name,
                                    msg.location_x,
                                    msg.location_y,
                                    msg.direction
                                ),
                            );
                        }
                    }
                } else if pkt.id == ServerPacketId::ObjectHero as i16 {
                    if let Ok(msg) = SObjectHero::decode(&pkt.payload) {
                        let b = msg.base;
                        let is_new = !chat.objects.contains_key(&b.object_id);
                        chat.objects.insert(
                            b.object_id,
                            ObjState {
                                kind: ObjKind::Hero,
                                name: Some(b.name.clone()),
                                x: b.location_x,
                                y: b.location_y,
                                dir: b.direction,
                            },
                        );
                        if is_new {
                            log_key_event(
                                chat,
                                format!(
                                    "[obj] hero spawn id={} name={} owner={} x={} y={} dir={} ",
                                    b.object_id,
                                    b.name,
                                    msg.owner_name,
                                    b.location_x,
                                    b.location_y,
                                    b.direction
                                ),
                            );
                        }
                    }
                } else if pkt.id == ServerPacketId::ObjectRemove as i16 {
                    if let Ok(msg) = SObjectRemove::decode(&pkt.payload) {
                        if let Some(old) = chat.objects.remove(&msg.object_id) {
                            match old.kind {
                                ObjKind::Player | ObjKind::Hero => {
                                    log_key_event(chat, format!("[obj] remove id={}", msg.object_id));
                                }
                                _ => {}
                            }
                        }
                    }
                } else if pkt.id == ServerPacketId::ObjectRun as i16 {
                    if let Ok(msg) = SObjectRun::decode(&pkt.payload) {
                        let b = msg.0;
                        chat.objects
                            .entry(b.object_id)
                            .and_modify(|s| {
                                s.x = b.location_x;
                                s.y = b.location_y;
                                s.dir = b.direction;
                            })
                            .or_insert(ObjState {
                                kind: ObjKind::Unknown,
                                name: None,
                                x: b.location_x,
                                y: b.location_y,
                                dir: b.direction,
                            });
                    }
                } else if pkt.id == ServerPacketId::CompleteQuest as i16 {
                    let _ = SCompleteQuest::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::ReceiveMail as i16 {
                    let _ = SReceiveMail::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::FriendUpdate as i16 {
                    let _ = SFriendUpdate::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::LoverUpdate as i16 {
                    let _ = SLoverUpdate::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::MentorUpdate as i16 {
                    let _ = SMentorUpdate::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::BaseStatsInfo as i16 {
                    let _ = SBaseStatsInfo::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::TimeOfDay as i16 {
                    let _ = STimeOfDay::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::ChangeAMode as i16 {
                    let _ = SChangeAMode::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::ChangePMode as i16 {
                    let _ = SChangePMode::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::SwitchGroup as i16 {
                    let _ = SSwitchGroup::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::DefaultNPC as i16 {
                    let _ = SDefaultNpc::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::GuildBuffList as i16 {
                    let _ = SGuildBuffList::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::ObjectMonster as i16 {
                    if let Ok(msg) = SObjectMonster::decode(&pkt.payload) {
                        chat.objects.entry(msg.object_id).or_insert(ObjState {
                            kind: ObjKind::Monster,
                            name: Some(msg.name),
                            x: msg.location_x,
                            y: msg.location_y,
                            dir: msg.direction,
                        });
                    }
                } else if pkt.id == ServerPacketId::AddBuff as i16 {
                    let _ = SAddBuff::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::NPCUpdate as i16 {
                    let _ = SNpcUpdate::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::InTrapRock as i16 {
                    let _ = SInTrapRock::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::ObjectColourChanged as i16 {
                    let _ = SObjectColourChanged::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::UserInformation as i16 {
                    if let Ok(msg) = SUserInformation::decode(&pkt.payload) {
                        chat.user_object_id = Some(msg.object_id);
                        chat.user_name = Some(msg.name.clone());
                        chat.user_location_x = Some(msg.location_x);
                        chat.user_location_y = Some(msg.location_y);
                        chat.user_direction = Some(msg.direction);
                        chat.user_hp = Some(msg.hp);
                        chat.user_mp = Some(msg.mp);
                        log_key_event(
                            chat,
                            format!(
                                "[user] info id={} name={} class={} level={} hp={} mp={} ",
                                msg.object_id, msg.name, msg.class, msg.level, msg.hp, msg.mp
                            ),
                        );
                    } else {
                        chat.lines.push("[user] info decode failed".to_string());
                    }
                } else if pkt.id == ServerPacketId::UserLocation as i16 {
                    if let Ok(msg) = SUserLocation::decode(&pkt.payload) {
                        chat.user_location_x = Some(msg.location_x);
                        chat.user_location_y = Some(msg.location_y);
                        chat.user_direction = Some(msg.direction);
                        log_key_event(
                            chat,
                            format!("[user] loc x={} y={} dir={} ", msg.location_x, msg.location_y, msg.direction),
                        );
                    }
                } else if pkt.id == ServerPacketId::HealthChanged as i16 {
                    if let Ok(msg) = SHealthChanged::decode(&pkt.payload) {
                        chat.user_hp = Some(msg.hp);
                        chat.user_mp = Some(msg.mp);
                        log_key_event(chat, format!("[user] hpmp hp={} mp={} ", msg.hp, msg.mp));
                    }
                } else if pkt.id == ServerPacketId::ObjectTeleportIn as i16 {
                    if let Ok(msg) = SObjectTeleportIn::decode(&pkt.payload) {
                        log_key_event(
                            chat,
                            format!("[obj] teleport_in id={} type={} ", msg.object_id, msg.teleport_type),
                        );
                    }
                } else if pkt.id == ServerPacketId::GameShopInfo as i16 {
                    if let Ok(msg) = SGameShopInfo::decode(&pkt.payload) {
                        log_key_event(chat, format!("[shop] info bytes={}", msg.info_bytes.len()));
                    }
                } else if pkt.id == ServerPacketId::KeepAlive as i16 {
                    let _ = SKeepAlive::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::ObjectSpell as i16 {
                    if let Ok(msg) = crystal_shared_proto::magic::SObjectSpell::decode(&pkt.payload) {
                        log_key_event(
                            chat,
                            format!(
                                "[obj] spell id={} x={} y={} spell={} dir={} param={} ",
                                msg.object_id,
                                msg.location_x,
                                msg.location_y,
                                msg.spell,
                                msg.direction,
                                msg.param
                            ),
                        );
                    }
                } else if pkt.id == ServerPacketId::ObjectTurn as i16 {
                    if let Ok(msg) = SObjectTurn::decode(&pkt.payload) {
                        let b = msg.0;
                        log_key_event(
                            chat,
                            format!("[obj] turn id={} x={} y={} dir={} ", b.object_id, b.location_x, b.location_y, b.direction),
                        );
                    }
                } else if pkt.id == ServerPacketId::ObjectWalk as i16 {
                    if let Ok(msg) = SObjectWalk::decode(&pkt.payload) {
                        let b = msg.0;
                        log_key_event(
                            chat,
                            format!("[obj] walk id={} x={} y={} dir={} ", b.object_id, b.location_x, b.location_y, b.direction),
                        );
                    }
                } else if pkt.id == ServerPacketId::ObjectHealth as i16 {
                    if let Ok(msg) = SObjectHealth::decode(&pkt.payload) {
                        log_key_event(
                            chat,
                            format!("[obj] health id={} percent={} expire={} ", msg.object_id, msg.percent, msg.expire),
                        );
                    }
                } else if pkt.id == ServerPacketId::NpcResponse as i16 {
                    let _ = SNpcResponse::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::FishingUpdate as i16 {
                    let _ = SFishingUpdate::decode(&pkt.payload);
                } else if pkt.id == ServerPacketId::Chat as i16 {
                    if let Ok(msg) = SChat::decode(&pkt.payload) {
                        chat.lines.push(msg.message);
                    }
                } else if pkt.id == ServerPacketId::ObjectChat as i16 {
                    if let Ok(msg) = SObjectChat::decode(&pkt.payload) {
                        chat.lines.push(format!("{}: {}", msg.object_id, msg.text));
                    }
                } else {
                    log_unhandled_packet(chat, pkt.id, pkt.payload.len());
                }
            }
        }
    }

    if became_connected && !chat.sent_version {
        if let Some(net) = chat.net.as_ref() {
            let pkt = CClientVersion {
                version_hash: Vec::new(),
            }
            .encode();
            match pkt {
                Ok(pkt) => {
                    let _ = net.send_raw(pkt);
                    chat.sent_version = true;
                }
                Err(e) => {
                    chat.lines.push(format!("[net] client_version encode error: {e}"));
                }
            }
        }
    }

    if chat.connected && chat.version_checked {
        if let Some((account_id, password)) = chat.pending_login.take() {
            if let Some(net) = chat.net.as_ref() {
                let pkt = CLogin { account_id, password }.encode();
                match pkt {
                    Ok(pkt) => {
                        let _ = net.send_raw(pkt);
                    }
                    Err(e) => {
                        chat.lines.push(format!("[login] encode error: {e}"));
                    }
                }
            }
        }
    }

    if chat.connected && chat.logged_in {
        if let Some(character_index) = chat.pending_start_game.take() {
            if let Some(net) = chat.net.as_ref() {
                let pkt = CStartGame { character_index }.encode();
                match pkt {
                    Ok(pkt) => {
                        let _ = net.send_raw(pkt);
                        chat.lines
                            .push(format!("[start_game] requested index={}", character_index));
                    }
                    Err(e) => {
                        chat.lines.push(format!("[start_game] encode error: {e}"));
                    }
                }
            }
        }
    }
}
