use bevy::prelude::*;

use crystal_shared_proto::login::CKeepAlive;
use std::time::{SystemTime, UNIX_EPOCH};

use super::state::ChatState;

pub(super) fn send_keep_alive(mut chat: ResMut<ChatState>, time: Res<Time>) {
    let chat = &mut *chat;

    if !chat.connected {
        return;
    }
    let Some(net) = chat.net.as_ref() else {
        return;
    };

    chat.keepalive_timer.tick(time.delta());
    if !chat.keepalive_timer.finished() {
        return;
    }

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let pkt = CKeepAlive { time: now_ms }.encode();
    match pkt {
        Ok(pkt) => {
            let _ = net.send_raw(pkt);
        }
        Err(e) => {
            chat.lines.push(format!("[net] keepalive encode error: {e}"));
        }
    }
}
