use bevy::prelude::*;
use crystal_client_net::NetClient;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Component)]
pub(super) struct FpsText;

#[derive(Component)]
pub(super) struct ChatLogText;

#[derive(Component)]
pub(super) struct ChatInputText;

#[derive(Component)]
pub(super) struct ChatStatusText;

#[derive(Clone, Debug)]
pub(super) enum ObjKind {
    Player,
    Hero,
    Monster,
    Npc,
    Unknown,
}

#[derive(Clone, Debug)]
pub(super) struct ObjState {
    pub(super) kind: ObjKind,
    pub(super) name: Option<String>,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) dir: u8,
}

#[derive(Resource)]
pub(super) struct ChatState {
    pub(super) input: String,
    pub(super) lines: Vec<String>,
    pub(super) connected: bool,
    pub(super) net: Option<NetClient>,
    pub(super) last_error: Option<String>,
    pub(super) sent_version: bool,
    pub(super) version_checked: bool,
    pub(super) logged_in: bool,
    pub(super) pending_login: Option<(String, String)>,
    pub(super) pending_start_game: Option<i32>,
    pub(super) keepalive_timer: Timer,
    pub(super) map_index: Option<i32>,
    pub(super) map_file_name: Option<String>,
    pub(super) user_object_id: Option<u32>,
    pub(super) user_name: Option<String>,
    pub(super) user_location_x: Option<i32>,
    pub(super) user_location_y: Option<i32>,
    pub(super) user_direction: Option<u8>,
    pub(super) user_hp: Option<i32>,
    pub(super) user_mp: Option<i32>,
    pub(super) unhandled_log_path: String,
    pub(super) unhandled_log_notified: bool,
    pub(super) unhandled_log_failed: bool,
    pub(super) unhandled_counts: HashMap<i16, u32>,
    pub(super) key_log_path: String,
    pub(super) key_log_notified: bool,
    pub(super) key_log_failed: bool,
    pub(super) objects: HashMap<u32, ObjState>,
}

impl Default for ChatState {
    fn default() -> Self {
        ChatState {
            input: String::new(),
            lines: Vec::new(),
            connected: false,
            net: None,
            last_error: None,
            sent_version: false,
            version_checked: false,
            logged_in: false,
            pending_login: None,
            pending_start_game: None,
            keepalive_timer: Timer::from_seconds(5.0, TimerMode::Repeating),
            map_index: None,
            map_file_name: None,
            user_object_id: None,
            user_name: None,
            user_location_x: None,
            user_location_y: None,
            user_direction: None,
            user_hp: None,
            user_mp: None,
            unhandled_log_path: "logs/unhandled_packets.log".to_string(),
            unhandled_log_notified: false,
            unhandled_log_failed: false,
            unhandled_counts: HashMap::new(),
            key_log_path: "logs/key_events.log".to_string(),
            key_log_notified: false,
            key_log_failed: false,
            objects: HashMap::new(),
        }
    }
}

pub(super) fn log_key_event(chat: &mut ChatState, line: String) {
    if !chat.key_log_notified {
        chat.lines
            .push(format!("[log] key events -> {}", chat.key_log_path));
        chat.key_log_notified = true;
    }

    if chat.key_log_failed {
        return;
    }

    if let Some(parent) = std::path::Path::new(&chat.key_log_path).parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            chat.lines.push(format!("[log] key log init error: {e}"));
            chat.key_log_failed = true;
            return;
        }
    }

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let mut f = match OpenOptions::new().create(true).append(true).open(&chat.key_log_path) {
        Ok(f) => f,
        Err(e) => {
            chat.lines.push(format!("[log] key log open error: {e}"));
            chat.key_log_failed = true;
            return;
        }
    };

    if let Err(e) = writeln!(f, "{}\t{}", now_ms, line) {
        chat.lines.push(format!("[log] key log write error: {e}"));
        chat.key_log_failed = true;
    }
}

pub(super) fn log_unhandled_packet(chat: &mut ChatState, id: i16, payload_len: usize) {
    if !chat.unhandled_log_notified {
        chat.lines
            .push(format!("[net] unhandled packets -> {}", chat.unhandled_log_path));
        chat.unhandled_log_notified = true;
    }

    let count = chat.unhandled_counts.entry(id).or_insert(0);
    *count = count.saturating_add(1);
    if *count == 1 {
        chat.lines
            .push(format!("[net] unhandled server packet id={} (logged)", id));
    }

    if chat.unhandled_log_failed {
        return;
    }

    if let Some(parent) = std::path::Path::new(&chat.unhandled_log_path).parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            chat.lines.push(format!("[net] log init error: {e}"));
            chat.unhandled_log_failed = true;
            return;
        }
    }

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let mut f = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&chat.unhandled_log_path)
    {
        Ok(f) => f,
        Err(e) => {
            chat.lines.push(format!("[net] log open error: {e}"));
            chat.unhandled_log_failed = true;
            return;
        }
    };

    if let Err(e) = writeln!(f, "{}\tid={}\tlen={}", now_ms, id, payload_len) {
        chat.lines.push(format!("[net] log write error: {e}"));
        chat.unhandled_log_failed = true;
    }
}
