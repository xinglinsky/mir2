use std::{
    fs,
    io,
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex, atomic::{AtomicU32, Ordering}},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use std::collections::{HashMap, HashSet};

use crystal_server_net::{run_server, ConnectionHandler, HandlerFactory};
use crystal_server_core::world::{self, WorldConfig, WorldDatabase, WorldProvider};
use crystal_server_core::world::magic::{UserMagic as WorldUserMagic, encode_client_magic_bytes};
use crystal_server_core::account::{
    AccountStore,
    CharacterStats,
    CharacterSummary,
    CharacterPosition,
};
use crystal_server_db::SqliteAccountStore;
use crystal_shared_proto::login::{
    CAttack, CCallNPC, CChangePassword, CClientVersion, CDeleteCharacter, CLogin, CNewAccount,
    CNewCharacter, CRun, CStartGame, CTurn, CWalk, ClientPacketId, SChangePassword,
    SClientVersion, SConnected, SLogin, SLoginBanned, SNewAccount, SNewCharacter, SStartGame,
};
use crystal_shared_proto::map::{SMapChanged, SMapInformation};
use crystal_shared_proto::packet::RawPacket;
use crystal_shared_proto::scene::{
    SObjectTeleportIn,
    SObjectTeleportOut,
    STeleportIn,
    SObjectMonster,
    SObjectNpc,
    SObjectRemove,
    SNpcResponse,
    SNewMagic,
    SMagicLeveled,
};
use crystal_shared_proto::user::{SObjectAttack, SObjectPlayer, SUserInformation, SUserLocation};
use crystal_shared_proto::select::{
    SelectInfo,
    SLoginSuccess,
    SLogOutSuccess,
    SLogOutFailed,
    SNewCharacterSuccess,
};
use crystal_shared_proto::io::{write_bool, read_string};

mod config;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Stage {
    Connected,
    VersionChecked,
    Select,
    InGame,
}

#[derive(Clone, Debug)]
struct PlayerVisual {
    name: String,
    guild_name: String,
    guild_rank_name: String,
    name_colour_argb: i32,
    class: u8,
    gender: u8,
    level: u16,
    hair: u8,
}

struct LoginConnection {
    stage: Stage,
    session_id: world::SessionId,
    account_id: Option<String>,
    characters: Vec<SelectInfo>,
    store: Arc<dyn AccountStore>,
    world_db: Arc<WorldDatabase>,
    world_config: WorldConfig,
    world: Arc<Mutex<world::World<WorldDatabase>>>,
    exp_table: Arc<Vec<i64>>,
    current_map_index: i32,
    current_x: i32,
    current_y: i32,
    direction: u8,
    current_char_index: Option<i32>,
    known_monsters: HashSet<u64>,
    known_npcs: HashSet<i32>,
    known_players: HashSet<world::SessionId>,
    player_summaries: Arc<Mutex<HashMap<world::SessionId, PlayerVisual>>>,
}

impl LoginConnection {
    const DATA_RANGE: i32 = 16;

    fn new(
        session_id: world::SessionId,
        store: Arc<dyn AccountStore>,
        world_db: Arc<WorldDatabase>,
        world_config: WorldConfig,
        world: Arc<Mutex<world::World<WorldDatabase>>>,
        exp_table: Arc<Vec<i64>>,
        player_summaries: Arc<Mutex<HashMap<world::SessionId, PlayerVisual>>>,
    ) -> Self {
        LoginConnection {
            stage: Stage::Connected,
            session_id,
            account_id: None,
            characters: Vec::new(),
            store,
            world_db,
            world_config,
            world,
            exp_table,
            current_map_index: 0,
            current_x: 0,
            current_y: 0,
            direction: 0,
            current_char_index: None,
            known_monsters: HashSet::new(),
            known_npcs: HashSet::new(),
            known_players: HashSet::new(),
            player_summaries,
        }
    }

    fn encode_raw(raw: RawPacket) -> Vec<u8> {
        raw.encode()
    }

    /// Send a NewMagic packet for the given learned magic, mirroring the
    /// C# SendMagicInfo(UserMagic) flow. This builds ClientMagic.Save(writer)
    /// bytes from MagicInfo + UserMagic and appends the Hero bool (false).
    fn send_new_magic(&self, magic: &WorldUserMagic, out: &mut Vec<Vec<u8>>) {
        if let Some(info) = self.world_db.get_magic_info(magic.spell) {
            if let Ok(mut bytes) = encode_client_magic_bytes(info, magic, 0) {
                if write_bool(&mut bytes, false).is_err() {
                    return;
                }
                let pkt = SNewMagic { magic_bytes: bytes };
                let raw = pkt.encode();
                out.push(Self::encode_raw(raw));
            }
        }
    }

    /// Send a MagicLeveled packet to notify the client that a magic's level
    /// and experience have changed, mirroring C# HumanObject.MagicLeveled.
    fn send_magic_leveled(
        &self,
        spell: u8,
        level: u8,
        experience: u16,
        out: &mut Vec<Vec<u8>>,
    ) {
        let pkt = SMagicLeveled {
            object_id: self.session_id, // TODO: wire real object ID when object system is in place.
            spell,
            level,
            experience,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    fn send_monsters_for_map(&self, map_index: i32, out: &mut Vec<Vec<u8>>) {
        let monsters = {
            let world = self.world.lock().unwrap();
            world.monsters_for_map(map_index)
        };

        for monster in monsters {
            if let Some(info) = self.world_db.get_monster_info(monster.monster_index) {
                let packet = SObjectMonster {
                    object_id: monster.id as u32,
                    name: info.name.clone(),
                    name_colour_argb: -1,
                    location_x: monster.x,
                    location_y: monster.y,
                    image: info.image,
                    direction: monster.direction,
                    effect: info.effect,
                    ai: info.ai,
                    light: info.light,
                    dead: false,
                    skeleton: false,
                    poison: 0,
                    hidden: false,
                    shock_time: 0,
                    binding_shot_center: false,
                    extra: false,
                    extra_byte: 0,
                    buffs: Vec::new(),
                };
                if let Ok(raw) = packet.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }

    fn send_npcs_for_map(&self, map_index: i32, out: &mut Vec<Vec<u8>>) {
        for npc in self
            .world_db
            .npc_infos
            .iter()
            .filter(|n| n.map_index == map_index)
        {
            let packet = SObjectNpc {
                object_id: npc.index as u32,
                name: npc.name.clone(),
                name_colour_argb: -1,
                image: npc.image,
                colour_argb: -1,
                location_x: npc.location_x,
                location_y: npc.location_y,
                direction: 0,
                quest_ids: npc.collect_quest_indexes.clone(),
            };
            if let Ok(raw) = packet.encode() {
                out.push(Self::encode_raw(raw));
            }
        }
    }

    fn update_visibility(&mut self, out: &mut Vec<Vec<u8>>) {
        if self.current_map_index == 0 {
            return;
        }

        let range = Self::DATA_RANGE;
        let map_index = self.current_map_index;

        // Monsters in view.
        let monsters_in_view = {
            let world = self.world.lock().unwrap();
            world
                .monsters_for_map(map_index)
                .into_iter()
                .filter(|m| {
                    (m.x - self.current_x).abs() <= range
                        && (m.y - self.current_y).abs() <= range
                })
                .collect::<Vec<_>>()
        };

        let mut visible_monster_ids: HashSet<u64> = HashSet::new();

        for monster in &monsters_in_view {
            visible_monster_ids.insert(monster.id);

            if !self.known_monsters.contains(&monster.id) {
                if let Some(info) = self.world_db.get_monster_info(monster.monster_index) {
                    let packet = SObjectMonster {
                        object_id: monster.id as u32,
                        name: info.name.clone(),
                        name_colour_argb: -1,
                        location_x: monster.x,
                        location_y: monster.y,
                        image: info.image,
                        direction: monster.direction,
                        effect: info.effect,
                        ai: info.ai,
                        light: info.light,
                        dead: false,
                        skeleton: false,
                        poison: 0,
                        hidden: false,
                        shock_time: 0,
                        binding_shot_center: false,
                        extra: false,
                        extra_byte: 0,
                        buffs: Vec::new(),
                    };
                    if let Ok(raw) = packet.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        }

        // Monsters leaving view.
        let removed_monsters: Vec<u64> = self
            .known_monsters
            .iter()
            .filter(|id| !visible_monster_ids.contains(id))
            .cloned()
            .collect();

        for id in removed_monsters {
            let pkt = SObjectRemove { object_id: id as u32 };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
        }

        self.known_monsters = visible_monster_ids;

        // NPCs in view.
        let mut visible_npc_ids: HashSet<i32> = HashSet::new();

        for npc in self
            .world_db
            .npc_infos
            .iter()
            .filter(|n| n.map_index == map_index)
        {
            if (npc.location_x - self.current_x).abs() <= range
                && (npc.location_y - self.current_y).abs() <= range
            {
                visible_npc_ids.insert(npc.index);

                if !self.known_npcs.contains(&npc.index) {
                    let packet = SObjectNpc {
                        object_id: npc.index as u32,
                        name: npc.name.clone(),
                        name_colour_argb: -1,
                        image: npc.image,
                        colour_argb: -1,
                        location_x: npc.location_x,
                        location_y: npc.location_y,
                        direction: 0,
                        quest_ids: npc.collect_quest_indexes.clone(),
                    };
                    if let Ok(raw) = packet.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        }

        // NPCs leaving view.
        let removed_npcs: Vec<i32> = self
            .known_npcs
            .iter()
            .filter(|id| !visible_npc_ids.contains(id))
            .cloned()
            .collect();

        for id in removed_npcs {
            let pkt = SObjectRemove { object_id: id as u32 };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
        }

        self.known_npcs = visible_npc_ids;

        // Players in view.
        let players_in_view: Vec<(world::SessionId, i32, i32, u8)> = {
            let world = self.world.lock().unwrap();
            world
                .players
                .iter()
                .filter_map(|(&sid, p)| {
                    if sid == self.session_id {
                        return None;
                    }
                    if p.map_index != map_index {
                        return None;
                    }
                    if (p.x - self.current_x).abs() > range
                        || (p.y - self.current_y).abs() > range
                    {
                        return None;
                    }
                    Some((sid, p.x, p.y, p.direction))
                })
                .collect()
        };

        let mut visible_player_ids: HashSet<world::SessionId> = HashSet::new();

        for (sid, x, y, direction) in players_in_view {
            visible_player_ids.insert(sid);

            if !self.known_players.contains(&sid) {
                let snapshot = {
                    let map = self.player_summaries.lock().unwrap();
                    map.get(&sid).cloned()
                };

                if let Some(snap) = snapshot {
                    let pkt = SObjectPlayer {
                        object_id: sid,
                        name: snap.name,
                        guild_name: snap.guild_name,
                        guild_rank_name: snap.guild_rank_name,
                        name_colour_argb: snap.name_colour_argb,
                        class: snap.class,
                        gender: snap.gender,
                        level: snap.level,
                        location_x: x,
                        location_y: y,
                        direction,
                        hair: snap.hair,
                        light: 0,
                        weapon: 0,
                        weapon_effect: 0,
                        armour: 0,
                        poison: 0,
                        dead: false,
                        hidden: false,
                        effect: 0,
                        wing_effect: 0,
                        extra: false,
                        mount_type: 0,
                        riding_mount: false,
                        fishing: false,
                        transform_type: 0,
                        element_orb_effect: 0,
                        element_orb_lvl: 0,
                        element_orb_max: 0,
                        buffs: Vec::new(),
                        level_effects: 0,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        }

        // Players leaving view.
        let removed_players: Vec<world::SessionId> = self
            .known_players
            .iter()
            .filter(|sid| !visible_player_ids.contains(sid))
            .cloned()
            .collect();

        for sid in removed_players {
            let pkt = SObjectRemove { object_id: sid };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
        }

        self.known_players = visible_player_ids;
    }

    fn normalize_npc_key(key: &str) -> String {
        let mut s = key.trim().to_ascii_uppercase();
        if s.starts_with('[') && s.ends_with(']') && s.len() >= 3 {
            s = s[1..s.len() - 1].to_string();
        }
        if !s.starts_with('@') {
            s = format!("@{}", s);
        }
        if s == "@MAIN" {
            "@MAIN-1".to_string()
        } else {
            s
        }
    }

    fn find_npc_script_path(root: &Path, file_name: &str) -> Option<std::path::PathBuf> {
        let target = format!("{}.txt", file_name).to_lowercase();
        let mut stack = vec![root.to_path_buf()];

        while let Some(dir) = stack.pop() {
            let read_dir = match fs::read_dir(&dir) {
                Ok(r) => r,
                Err(_) => continue,
            };

            for entry_res in read_dir {
                let entry = match entry_res {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if name.to_lowercase() == target {
                        return Some(path);
                    }
                }
            }
        }

        None
    }

    fn load_npc_script_from_file(
        path: &Path,
    ) -> io::Result<(
        HashMap<String, Vec<String>>,
        HashMap<String, (String, i32, i32)>,
    )> {
        let text = fs::read_to_string(path)?;
        let lines: Vec<&str> = text.lines().collect();
        let mut pages: HashMap<String, Vec<String>> = HashMap::new();
        let mut moves: HashMap<String, (String, i32, i32)> = HashMap::new();

        let mut current_label: Option<String> = None;
        let mut i: usize = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            if trimmed.starts_with("[@") {
                if let Some(end) = trimmed.find(']') {
                    let label_inner = &trimmed[1..end];
                    let key = label_inner.to_ascii_uppercase();
                    current_label = Some(key);
                } else {
                    current_label = None;
                }
                i += 1;
                continue;
            }

            if let Some(label) = &current_label {
                if trimmed.eq_ignore_ascii_case("#SAY") {
                    let mut page_lines: Vec<String> = Vec::new();
                    i += 1;
                    while i < lines.len() {
                        let l = lines[i];
                        let t = l.trim_start();
                        if t.starts_with("[@") || (t.starts_with('#') && !t.eq_ignore_ascii_case("#SAY")) {
                            break;
                        }
                        page_lines.push(l.to_string());
                        i += 1;
                    }
                    pages.insert(label.clone(), page_lines);
                    continue;
                } else if trimmed.eq_ignore_ascii_case("#ACT") {
                    i += 1;
                    while i < lines.len() {
                        let l = lines[i];
                        let t = l.trim();
                        if t.is_empty() {
                            i += 1;
                            continue;
                        }
                        if t.starts_with("[@") || t.starts_with('#') {
                            i -= 1;
                            break;
                        }

                        let parts: Vec<&str> = t.split_whitespace().collect();
                        if !parts.is_empty() && parts[0].eq_ignore_ascii_case("MOVE") {
                            if parts.len() >= 2 {
                                let map_name = parts[1].to_string();
                                let x = parts
                                    .get(2)
                                    .and_then(|s| s.parse::<i32>().ok())
                                    .unwrap_or(0);
                                let y = parts
                                    .get(3)
                                    .and_then(|s| s.parse::<i32>().ok())
                                    .unwrap_or(0);
                                moves.insert(label.clone(), (map_name, x, y));
                            }
                            break;
                        }

                        i += 1;
                    }
                    // We've either consumed until MOVE or hit a new section; continue
                    continue;
                }
            }

            i += 1;
        }

        Ok((pages, moves))
    }

    fn apply_step(&mut self, direction: u8, distance: i32, out: &mut Vec<Vec<u8>>) -> bool {
        let cmd = match distance {
            0 => world::WorldCommand::Turn {
                session_id: self.session_id,
                direction,
            },
            1 => world::WorldCommand::Walk {
                session_id: self.session_id,
                direction,
            },
            2 => world::WorldCommand::Run {
                session_id: self.session_id,
                direction,
            },
            _ => return false,
        };

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(cmd)
        };
        self.handle_world_events(events, out)
    }

    fn handle_world_events(
        &mut self,
        events: Vec<world::WorldEvent>,
        out: &mut Vec<Vec<u8>>,
    ) -> bool {
        let mut map_changed = false;

        for event in events {
            match event {
                world::WorldEvent::UserLocation {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                } => {
                    if session_id != self.session_id {
                        continue;
                    }

                    self.current_map_index = map_index;
                    self.current_x = x;
                    self.current_y = y;
                    self.direction = direction;

                    let loc = SUserLocation {
                        location_x: x,
                        location_y: y,
                        direction,
                    };
                    if let Ok(raw) = loc.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
                world::WorldEvent::MapChanged {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                } => {
                    if session_id != self.session_id {
                        continue;
                    }

                    self.current_map_index = map_index;
                    self.current_x = x;
                    self.current_y = y;
                    self.direction = direction;
                    map_changed = true;

                    if let Some(info) = self.world_db.get_map_info(map_index) {
                        let pkt = SMapChanged {
                            map_index: info.index,
                            file_name: info.file_name.clone(),
                            title: info.title.clone(),
                            mini_map: info.mini_map,
                            big_map: info.big_map,
                            lights: info.light,
                            location_x: x,
                            location_y: y,
                            direction,
                            map_dark_light: info.map_dark_light,
                            music: info.music,
                            weather: 0,
                        };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
                world::WorldEvent::ObjectAttack {
                    session_id,
                    map_index: _,
                    x,
                    y,
                    direction,
                    spell,
                    level,
                    attack_type,
                } => {
                    if session_id != self.session_id {
                        continue;
                    }

                    let attack = SObjectAttack {
                        object_id: self.session_id,
                        location_x: x,
                        location_y: y,
                        direction,
                        spell,
                        level,
                        attack_type,
                    };
                    if let Ok(raw) = attack.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        }

        map_changed
    }
}

impl ConnectionHandler for LoginConnection {
    fn on_connect(&mut self) -> Vec<Vec<u8>> {
        vec![Self::encode_raw(SConnected.encode())]
    }

    fn handle_packet(&mut self, packet: RawPacket) -> Vec<Vec<u8>> {
        let mut out = Vec::new();

        let Some(pid) = ClientPacketId::from_i16(packet.id) else {
            return out;
        };

        match pid {
            ClientPacketId::NewAccount => {
                if let Ok(msg) = CNewAccount::decode(&packet.payload) {
                    if msg.account_id.is_empty() {
                        // 1: Bad AccountID
                        out.push(Self::encode_raw(SNewAccount { result: 1 }.encode()));
                    } else if msg.password.is_empty() {
                        // 2: Bad Password
                        out.push(Self::encode_raw(SNewAccount { result: 2 }.encode()));
                    } else if self
                        .store
                        .account_exists(&msg.account_id)
                        .unwrap_or(false)
                    {
                        // 7: Account already exists
                        out.push(Self::encode_raw(SNewAccount { result: 7 }.encode()));
                    } else {
                        let _ = self.store.create_account(&msg.account_id, &msg.password);

                        // 8: Success (regardless of race condition errors)
                        out.push(Self::encode_raw(SNewAccount { result: 8 }.encode()));
                    }
                }
            }
            ClientPacketId::ClientVersion => {
                if let Ok(_msg) = CClientVersion::decode(&packet.payload) {
                    // In the original C# server, Connected is sent on accept.
                    // Here we send Connected + ClientVersion(OK) together
                    // when the client sends ClientVersion.
                    out.push(Self::encode_raw(SClientVersion { result: 1 }.encode()));
                    self.stage = Stage::VersionChecked;
                }
            }
            ClientPacketId::Login => {
                if let Ok(msg) = CLogin::decode(&packet.payload) {
                    if msg.account_id.is_empty() {
                        // 1: Bad AccountID
                        out.push(Self::encode_raw(SLogin { result: 1 }.encode()));
                        return out;
                    }
                    if msg.password.is_empty() {
                        // 2: Bad Password
                        out.push(Self::encode_raw(SLogin { result: 2 }.encode()));
                        return out;
                    }

                    match self.store.verify_password(&msg.account_id, &msg.password) {
                        Ok(true) => {
                            self.account_id = Some(msg.account_id.clone());
                            self.stage = Stage::Select;

                            // Load characters for this account from the store.
                            let chars: Vec<SelectInfo> = self
                                .store
                                .list_characters(&msg.account_id)
                                .unwrap_or_default()
                                .into_iter()
                                .map(|c: CharacterSummary| SelectInfo {
                                    index: c.index,
                                    name: c.name,
                                    level: c.level,
                                    class: c.class,
                                    gender: c.gender,
                                    last_access_binary: c.last_access_binary,
                                })
                                .collect();
                            self.characters = chars.clone();

                            let resp = SLoginSuccess { characters: chars };
                            if let Ok(raw) = resp.encode() {
                                out.push(Self::encode_raw(raw));
                            }
                        }
                        Ok(false) => {
                            // Wrong password or account missing; keep it simple:
                            out.push(Self::encode_raw(SLogin { result: 4 }.encode()));
                        }
                        Err(_) => {
                            // Treat store errors as login failure.
                            out.push(Self::encode_raw(SLogin { result: 4 }.encode()));
                        }
                    }
                }
            }
            ClientPacketId::ChangePassword => {
                if let Ok(msg) = CChangePassword::decode(&packet.payload) {
                    if msg.new_password.is_empty() {
                        // 3: Bad New Password
                        out.push(Self::encode_raw(SChangePassword { result: 3 }.encode()));
                    } else {
                        // First ensure the account exists.
                        let exists = self
                            .store
                            .account_exists(&msg.account_id)
                            .unwrap_or(false);

                        if !exists {
                            // 4: Account Not Exist
                            out.push(Self::encode_raw(SChangePassword { result: 4 }.encode()));
                        } else {
                            // Then verify the current password.
                            match self
                                .store
                                .verify_password(&msg.account_id, &msg.current_password)
                            {
                                Ok(true) => {
                                    // Update the stored password hash.
                                    let _ = self
                                        .store
                                        .set_password(&msg.account_id, &msg.new_password);

                                    // 6: Success
                                    out.push(Self::encode_raw(SChangePassword { result: 6 }.encode()));
                                }
                                Ok(false) | Err(_) => {
                                    // 5: Wrong Password (or treat store errors as failure)
                                    out.push(Self::encode_raw(SChangePassword { result: 5 }.encode()));
                                }
                            }
                        }
                    }
                }
            }
            ClientPacketId::NewCharacter => {
                if let Ok(msg) = CNewCharacter::decode(&packet.payload) {
                    if let Some(acc_id) = &self.account_id {
                        match self.store.create_character(
                            acc_id,
                            msg.name,
                            msg.class,
                            msg.gender,
                        ) {
                            Ok(ch) => {
                                let info = SelectInfo {
                                    index: ch.index,
                                    name: ch.name,
                                    level: ch.level,
                                    class: ch.class,
                                    gender: ch.gender,
                                    last_access_binary: ch.last_access_binary,
                                };

                                self.characters.push(info.clone());

                                // Result code 10 in the C# comments indicates success.
                                out.push(Self::encode_raw(SNewCharacter { result: 10 }.encode()));

                                let succ = SNewCharacterSuccess { char_info: info };
                                if let Ok(raw) = succ.encode() {
                                    out.push(Self::encode_raw(raw));
                                }
                            }
                            Err(_) => {
                                out.push(Self::encode_raw(SNewCharacter { result: 0 }.encode()));
                            }
                        }
                    }
                }
            }
            ClientPacketId::DeleteCharacter => {
                if let Ok(msg) = CDeleteCharacter::decode(&packet.payload) {
                    if let Some(acc_id) = &self.account_id {
                        let ok = self
                            .store
                            .delete_character(acc_id, msg.character_index)
                            .unwrap_or(false);
                        if ok {
                            if let Some(pos) = self
                                .characters
                                .iter()
                                .position(|c| c.index == msg.character_index)
                            {
                                self.characters.remove(pos);
                            }

                            // Success uses DeleteCharacterSuccess only.
                            let succ = crystal_shared_proto::login::SDeleteCharacterSuccess {
                                character_index: msg.character_index,
                            };
                            if let Ok(raw) = succ.encode() {
                                out.push(Self::encode_raw(raw));
                            }
                        } else {
                            let err = crystal_shared_proto::login::SDeleteCharacter { result: 1 };
                            out.push(Self::encode_raw(err.encode()));
                        }
                    }
                }
            }
            ClientPacketId::StartGame => {
                if let Ok(msg) = CStartGame::decode(&packet.payload) {
                    if let Some(ch) = self
                        .characters
                        .iter()
                        .find(|c| c.index == msg.character_index)
                        .cloned()
                    {
                        self.stage = Stage::InGame;
                        self.current_char_index = Some(ch.index);

                        // Record a basic appearance snapshot for this session so that
                        // other connections can render this player via SObjectPlayer.
                        {
                            let mut map = self.player_summaries.lock().unwrap();
                            map.insert(
                                self.session_id,
                                PlayerVisual {
                                    name: ch.name.clone(),
                                    guild_name: String::new(),
                                    guild_rank_name: String::new(),
                                    name_colour_argb: -1,
                                    class: ch.class,
                                    gender: ch.gender,
                                    level: ch.level,
                                    hair: 0,
                                },
                            );
                        }

                        // Signal successful start game so the client switches to GameScene.
                        let ok = SStartGame {
                            result: 4,
                            resolution: 1024,
                        };
                        if let Ok(raw) = ok.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        // Load last known position for this character, if any.
                        let stored_pos = if let Some(ref account_id) = self.account_id {
                            self
                                .store
                                .load_character_position(account_id, ch.index)
                                .unwrap_or(None)
                        } else {
                            None
                        };

                        // Load basic stats (HP/MP/experience/gold/credit) for this character.
                        let stats_opt = if let Some(ref account_id) = self.account_id {
                            self
                                .store
                                .load_character_stats(account_id, ch.index)
                                .unwrap_or(None)
                        } else {
                            None
                        };

                        // Load bind point (BindMapIndex / BindLocation equivalent), if any.
                        let bind_pos = if let Some(ref account_id) = self.account_id {
                            self
                                .store
                                .load_character_bind(account_id, ch.index)
                                .unwrap_or(None)
                        } else {
                            None
                        };

                        // Choose a MapInfo to drive loading and packets. Priority:
                        // 1) stored_pos.map_index if valid; 2) bind_pos.map_index if valid;
                        // 3) first map with a SafeZone marked StartPoint; 4) first entry;
                        // 5) stub if DB is empty.
                        let map_info_core = {
                            let map_infos = &self.world_db.map_infos;
                            if let Some(pos) = &stored_pos {
                                if let Some(info) = map_infos.iter().find(|m| m.index == pos.map_index)
                                {
                                    info.clone()
                                } else if let Some(info) = map_infos
                                    .iter()
                                    .find(|m| m.safe_zones.iter().any(|z| z.start_point))
                                {
                                    info.clone()
                                } else if let Some(info) = map_infos.first() {
                                    info.clone()
                                } else {
                                    world::map::MapInfo {
                                        index: 0,
                                        file_name: "3".to_string(),
                                        title: "StubMap".to_string(),
                                        mini_map: 0,
                                        big_map: 0,
                                        light: 0,
                                        map_dark_light: 0,
                                        music: 0,
                                        weather_particles: 0,
                                        no_teleport: false,
                                        no_reconnect: false,
                                        no_random: false,
                                        no_escape: false,
                                        no_recall: false,
                                        no_drug: false,
                                        no_position: false,
                                        no_throw_item: false,
                                        no_drop_player: false,
                                        no_drop_monster: false,
                                        no_names: false,
                                        no_mount: false,
                                        need_bridle: false,
                                        no_fight: false,
                                        fight: false,
                                        fire: false,
                                        fire_damage: 0,
                                        lightning: false,
                                        lightning_damage: 0,
                                        no_town_teleport: false,
                                        no_reincarnation: false,
                                        no_reconnect_map: String::new(),
                                        mine_zones: Vec::new(),
                                        mine_index: 0,
                                        gt: false,
                                        gt_index: 0,
                                        safe_zones: Vec::new(),
                                        respawns: Vec::new(),
                                        movements: Vec::new(),
                                    }
                                }
                            } else if let Some(pos) = &bind_pos {
                                if let Some(info) = map_infos.iter().find(|m| m.index == pos.map_index)
                                {
                                    info.clone()
                                } else if let Some(info) = map_infos
                                    .iter()
                                    .find(|m| m.safe_zones.iter().any(|z| z.start_point))
                                {
                                    info.clone()
                                } else if let Some(info) = map_infos.first() {
                                    info.clone()
                                } else {
                                    world::map::MapInfo {
                                        index: 0,
                                        file_name: "3".to_string(),
                                        title: "StubMap".to_string(),
                                        mini_map: 0,
                                        big_map: 0,
                                        light: 0,
                                        map_dark_light: 0,
                                        music: 0,
                                        weather_particles: 0,
                                        no_teleport: false,
                                        no_reconnect: false,
                                        no_random: false,
                                        no_escape: false,
                                        no_recall: false,
                                        no_drug: false,
                                        no_position: false,
                                        no_throw_item: false,
                                        no_drop_player: false,
                                        no_drop_monster: false,
                                        no_names: false,
                                        no_mount: false,
                                        need_bridle: false,
                                        no_fight: false,
                                        fight: false,
                                        fire: false,
                                        fire_damage: 0,
                                        lightning: false,
                                        lightning_damage: 0,
                                        no_town_teleport: false,
                                        no_reincarnation: false,
                                        no_reconnect_map: String::new(),
                                        mine_zones: Vec::new(),
                                        mine_index: 0,
                                        gt: false,
                                        gt_index: 0,
                                        safe_zones: Vec::new(),
                                        respawns: Vec::new(),
                                        movements: Vec::new(),
                                    }
                                }
                            } else if let Some(info) = map_infos
                                .iter()
                                .find(|m| m.safe_zones.iter().any(|z| z.start_point))
                            {
                                info.clone()
                            } else if let Some(info) = map_infos.first() {
                                info.clone()
                            } else {
                                world::map::MapInfo {
                                    index: 0,
                                    file_name: "3".to_string(),
                                    title: "StubMap".to_string(),
                                    mini_map: 0,
                                    big_map: 0,
                                    light: 0,
                                    map_dark_light: 0,
                                    music: 0,
                                    weather_particles: 0,
                                    no_teleport: false,
                                    no_reconnect: false,
                                    no_random: false,
                                    no_escape: false,
                                    no_recall: false,
                                    no_drug: false,
                                    no_position: false,
                                    no_throw_item: false,
                                    no_drop_player: false,
                                    no_drop_monster: false,
                                    no_names: false,
                                    no_mount: false,
                                    need_bridle: false,
                                    no_fight: false,
                                    fight: false,
                                    fire: false,
                                    fire_damage: 0,
                                    lightning: false,
                                    lightning_damage: 0,
                                    no_town_teleport: false,
                                    no_reincarnation: false,
                                    no_reconnect_map: String::new(),
                                    mine_zones: Vec::new(),
                                    mine_index: 0,
                                    gt: false,
                                    gt_index: 0,
                                    safe_zones: Vec::new(),
                                    respawns: Vec::new(),
                                    movements: Vec::new(),
                                }
                            }
                        };

                        let map_dir = &self.world_config.map_path;
                        let mut spawn_x: i32 = 0;
                        let mut spawn_y: i32 = 0;

                        match world::map::load_map_from_file(map_info_core.clone(), map_dir) {
                            Ok(loaded_map) => {
                                println!(
                                    "[core] Loaded map '{}' ({}x{}, walkable cells: {}) from {:?}",
                                    loaded_map.info.file_name,
                                    loaded_map.width,
                                    loaded_map.height,
                                    loaded_map.walkable_cells.len(),
                                    map_dir,
                                );

                                // Default spawn based on SafeZone.StartPoint / first SafeZone /
                                // map centre plus nearest walkable cell.
                                let (center_x, center_y) = map_info_core
                                    .safe_zones
                                    .iter()
                                    .find(|z| z.start_point)
                                    .or_else(|| map_info_core.safe_zones.first())
                                    .map(|z| (z.location_x, z.location_y))
                                    .unwrap_or_else(|| {
                                        (
                                            loaded_map.width as i32 / 2,
                                            loaded_map.height as i32 / 2,
                                        )
                                    });

                                if let Some(&(wx, wy)) = loaded_map
                                    .walkable_cells
                                    .iter()
                                    .min_by_key(|(x, y)| {
                                        let dx = *x as i32 - center_x;
                                        let dy = *y as i32 - center_y;
                                        dx.abs() + dy.abs()
                                    })
                                {
                                    spawn_x = wx as i32;
                                    spawn_y = wy as i32;
                                } else {
                                    spawn_x = center_x;
                                    spawn_y = center_y;
                                }

                                // If we have a stored position on this map, use it directly;
                                // otherwise, if we have a bind point on this map, use that.
                                if let Some(pos) = &stored_pos {
                                    if pos.map_index == map_info_core.index {
                                        spawn_x = pos.x;
                                        spawn_y = pos.y;
                                    }
                                } else if let Some(pos) = &bind_pos {
                                    if pos.map_index == map_info_core.index {
                                        spawn_x = pos.x;
                                        spawn_y = pos.y;
                                    }
                                }

                                // If this is the very first login (no stored position and no
                                // bind yet), initialize a default bind at the chosen spawn
                                // location so future escape/respawn logic can use it.
                                if stored_pos.is_none() && bind_pos.is_none() {
                                    if let Some(ref account_id) = self.account_id {
                                        let bind = CharacterPosition {
                                            map_index: map_info_core.index,
                                            x: spawn_x,
                                            y: spawn_y,
                                            direction: 0,
                                        };
                                        let _ = self
                                            .store
                                            .save_character_bind(account_id, ch.index, &bind);
                                    }
                                }
                            }
                            Err(e) => {
                                println!(
                                    "[core] Failed to load map '3' from {:?}: {} (falling back to stub packets)",
                                    map_dir, e
                                );
                            }
                        }

                        // Build initial magic list for this character from persistent storage.
                        let mut user_magics = if let Some(ref account_id) = self.account_id {
                            self
                                .store
                                .load_character_magics(account_id, ch.index)
                                .unwrap_or_else(|_| Vec::new())
                        } else {
                            Vec::new()
                        };

                        // Convert stored UserMagic entries into ClientMagic.Save(writer) bytes
                        // for the UserInformation.Magics list.
                        let mut magic_bytes = Vec::new();
                        for um in &user_magics {
                            if let Some(mi) = self.world_db.get_magic_info(um.spell) {
                                if let Ok(bytes) = encode_client_magic_bytes(mi, um, 0) {
                                    magic_bytes.push(bytes);
                                }
                            }
                        }

                        if let Some(ref account_id) = self.account_id {
                            let _ = self
                                .store
                                .save_character_magics(account_id, ch.index, &user_magics);
                        }

                        // Initialize world state for this session.
                        let initial_direction = stored_pos
                            .as_ref()
                            .map(|p| p.direction)
                            .unwrap_or(0);
                        let events = {
                            let mut world = self.world.lock().unwrap();
                            world.handle_command(world::WorldCommand::StartGame {
                                session_id: self.session_id,
                                map_index: map_info_core.index,
                                x: spawn_x,
                                y: spawn_y,
                                direction: initial_direction,
                                magics: user_magics,
                            })
                        };
                        self.handle_world_events(events, &mut out);

                        // MapInformation packet using the chosen MapInfo. Lightning/Fire flags are
                        // still stubbed for now until the full set of MapInfo flags is mirrored.
                        let map = SMapInformation {
                            map_index: map_info_core.index,
                            file_name: map_info_core.file_name.clone(),
                            title: map_info_core.title.clone(),
                            mini_map: map_info_core.mini_map,
                            big_map: map_info_core.big_map,
                            lights: map_info_core.light,
                            lightning: map_info_core.lightning,
                            fire: map_info_core.fire,
                            map_dark_light: map_info_core.map_dark_light,
                            music: map_info_core.music,
                            weather_particles: map_info_core.weather_particles,
                        };
                        if let Ok(raw) = map.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        // Compute MaxExperience using the same ExperienceList semantics as
                        // the C# Settings.ExperienceList table.
                        let max_experience = {
                            let lvl = ch.level as usize;
                            if lvl == 0 {
                                0_i64
                            } else {
                                *self
                                    .exp_table
                                    .get(lvl.saturating_sub(1))
                                    .unwrap_or(&0_i64)
                            }
                        };

                        // Fall back to simple defaults if we somehow do not have a stats row
                        // yet (e.g. for migrated databases).
                        let stats = stats_opt.unwrap_or(CharacterStats {
                            hp: 100,
                            mp: 50,
                            experience: 0,
                            gold: 0,
                            credit: 0,
                        });

                        // UserInformation so the client receives basic player state and the
                        // initial list of learned magics.
                        let user = SUserInformation {
                            object_id: self.session_id,
                            real_id: self.session_id,
                            name: ch.name,
                            guild_name: String::new(),
                            guild_rank: String::new(),
                            name_colour_argb: -1,
                            class: ch.class,
                            gender: ch.gender,
                            level: ch.level,
                            location_x: self.current_x,
                            location_y: self.current_y,
                            direction: self.direction,
                            hair: 0,
                            hp: stats.hp,
                            mp: stats.mp,
                            experience: stats.experience,
                            max_experience,
                            level_effects: 0,
                            has_hero: false,
                            hero_behaviour: 0,
                            gold: stats.gold as u32,
                            credit: stats.credit as u32,
                            has_expanded_storage: false,
                            expanded_storage_expiry_binary: 0,
                            magics: magic_bytes,
                            summoned_creature_type: 0,
                            creature_summoned: false,
                            allow_observe: false,
                            observer: false,
                        };
                        if let Ok(raw) = user.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        // Initial UserLocation so client processes a location update.
                        let loc = SUserLocation {
                            location_x: self.current_x,
                            location_y: self.current_y,
                            direction: self.direction,
                        };
                        if let Ok(raw) = loc.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let map_changed = SMapChanged {
                            map_index: map_info_core.index,
                            file_name: map_info_core.file_name.clone(),
                            title: map_info_core.title.clone(),
                            mini_map: map_info_core.mini_map,
                            big_map: map_info_core.big_map,
                            lights: map_info_core.light,
                            location_x: spawn_x,
                            location_y: spawn_y,
                            direction: 0,
                            map_dark_light: map_info_core.map_dark_light,
                            music: map_info_core.music,
                            weather: 0,
                        };
                        if let Ok(raw) = map_changed.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        if !map_info_core.no_teleport {
                            let tele_out = SObjectTeleportOut {
                                object_id: self.session_id,
                                teleport_type: 0,
                            };
                            if let Ok(raw) = tele_out.encode() {
                                out.push(Self::encode_raw(raw));
                            }

                            let tele_in = STeleportIn;
                            out.push(Self::encode_raw(tele_in.encode()));

                            let obj_tele_in = SObjectTeleportIn {
                                object_id: self.session_id,
                                teleport_type: 0,
                            };
                            if let Ok(raw) = obj_tele_in.encode() {
                                out.push(Self::encode_raw(raw));
                            }
                        }
                        self.current_map_index = map_info_core.index;
                        self.known_monsters.clear();
                        self.known_npcs.clear();
                        self.update_visibility(&mut out);
                    } else {
                        let err = SStartGame {
                            result: 2, // Character not found
                            resolution: 0,
                        };
                        if let Ok(raw) = err.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
            }
            ClientPacketId::LogOut => {
                // On logout, persist the latest learned magics and position for this character.
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

                // After a successful logout, keep the account logged in but
                // return to the Select stage and send LogOutSuccess with the
                // latest character list, mirroring the C# server.
                if let Some(ref acc_id) = self.account_id {
                    let chars: Vec<SelectInfo> = self
                        .store
                        .list_characters(acc_id)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|c: CharacterSummary| SelectInfo {
                            index: c.index,
                            name: c.name,
                            level: c.level,
                            class: c.class,
                            gender: c.gender,
                            last_access_binary: c.last_access_binary,
                        })
                        .collect();
                    self.characters = chars.clone();

                    let resp = SLogOutSuccess { characters: chars };
                    if let Ok(raw) = resp.encode() {
                        out.push(Self::encode_raw(raw));
                    }

                    self.stage = Stage::Select;
                    self.current_char_index = None;

                    // Mark this session as no longer visible to other players.
                    {
                        let mut map = self.player_summaries.lock().unwrap();
                        map.remove(&self.session_id);
                    }
                } else {
                    // If we somehow do not have an account associated with
                    // this connection, signal failure so the client can
                    // re-enable its UI.
                    let resp = SLogOutFailed;
                    let raw = resp.encode();
                    out.push(Self::encode_raw(raw));
                }
            }
            ClientPacketId::Turn => {
                if self.stage == Stage::InGame {
                    if let Ok(msg) = CTurn::decode(&packet.payload) {
                        let _ = self.apply_step(msg.direction, 0, &mut out);
                    }
                }
            }
            ClientPacketId::Walk => {
                if self.stage == Stage::InGame {
                    if let Ok(msg) = CWalk::decode(&packet.payload) {
                        let map_changed = self.apply_step(msg.direction, 1, &mut out);

                        if map_changed {
                            if let Some(info) = self.world_db.get_map_info(self.current_map_index) {
                            }
                            self.known_monsters.clear();
                            self.known_npcs.clear();
                        }

                        self.update_visibility(&mut out);
                    }
                }
            }
            ClientPacketId::Run => {
                if self.stage == Stage::InGame {
                    if let Ok(msg) = CRun::decode(&packet.payload) {
                        let map_changed = self.apply_step(msg.direction, 2, &mut out);

                        if map_changed {
                            if let Some(info) = self.world_db.get_map_info(self.current_map_index) {
                            }
                        }
                        if map_changed {
                            self.known_monsters.clear();
                            self.known_npcs.clear();
                        }

                        self.update_visibility(&mut out);
                    }
                }
            }
            ClientPacketId::Chat => {
                if self.stage == Stage::InGame {
                    use std::io::Cursor;

                    if let Ok(message) = (|| {
                        let mut c = Cursor::new(&packet.payload);
                        let text = read_string(&mut c)?;
                        Ok::<String, io::Error>(text)
                    })() {
                        if message.trim().eq_ignore_ascii_case("/kill") {
                            let killed_id = {
                                let mut world = self.world.lock().unwrap();
                                world.kill_nearest_monster(
                                    self.current_map_index,
                                    self.current_x,
                                    self.current_y,
                                )
                            };

                            if let Some(id) = killed_id {
                                self.known_monsters.remove(&id);
                                let pkt = SObjectRemove {
                                    object_id: id as u32,
                                };
                                if let Ok(raw) = pkt.encode() {
                                    out.push(Self::encode_raw(raw));
                                }
                            }
                        }
                    }
                }
            }
            ClientPacketId::CallNPC => {
                if self.stage == Stage::InGame {
                    if let Ok(msg) = CCallNPC::decode(&packet.payload) {
                        if msg.key.len() > 64 {
                            return out;
                        }

                        if let Some(npc) = self
                            .world_db
                            .npc_infos
                            .iter()
                            .find(|n| n.index as u32 == msg.object_id && n.map_index == self.current_map_index)
                        {
                            let dx = npc.location_x - self.current_x;
                            let dy = npc.location_y - self.current_y;

                            if dx.abs() <= Self::DATA_RANGE && dy.abs() <= Self::DATA_RANGE {
                                let key = Self::normalize_npc_key(&msg.key);
                                let root = Path::new("./deploy/Envir/NPCs");
                                let mut maybe_page: Option<Vec<String>> = None;

                                if root.exists() {
                                    if let Some(script_path) = Self::find_npc_script_path(root, &npc.file_name) {
                                        if let Ok((pages, moves)) = Self::load_npc_script_from_file(&script_path) {
                                            if let Some((map_name, tx, ty)) = moves.get(&key) {
                                                let mut dest_index: Option<i32> = None;

                                                // First try to interpret map_name as a numeric map index.
                                                if let Ok(idx) = map_name.parse::<i32>() {
                                                    if self
                                                        .world_db
                                                        .map_infos
                                                        .iter()
                                                        .any(|m| m.index == idx)
                                                    {
                                                        dest_index = Some(idx);
                                                    }
                                                }

                                                // Fallback: treat map_name as a map file_name.
                                                if dest_index.is_none() {
                                                    if let Some(info) = self
                                                        .world_db
                                                        .map_infos
                                                        .iter()
                                                        .find(|m| m.file_name.eq_ignore_ascii_case(map_name))
                                                    {
                                                        dest_index = Some(info.index);
                                                    }
                                                }

                                                if let Some(map_index) = dest_index {
                                                    let x = *tx;
                                                    let y = *ty;

                                                    let events = {
                                                        let mut world = self.world.lock().unwrap();
                                                        world.handle_command(world::WorldCommand::Teleport {
                                                            session_id: self.session_id,
                                                            map_index,
                                                            x,
                                                            y,
                                                        })
                                                    };

                                                    let map_changed = self.handle_world_events(events, &mut out);
                                                    if map_changed {
                                                        self.known_monsters.clear();
                                                        self.known_npcs.clear();
                                                        self.update_visibility(&mut out);
                                                    }

                                                    return out;
                                                }
                                            }

                                            maybe_page = pages.get(&key).cloned();
                                        }
                                    }
                                }

                                let page = maybe_page.unwrap_or_else(|| vec![npc.name.clone()]);
                                let resp = SNpcResponse { page };
                                if let Ok(raw) = resp.encode() {
                                    out.push(Self::encode_raw(raw));
                                }
                            }
                        }
                    }
                }
            }
            ClientPacketId::Attack => {
                if self.stage == Stage::InGame {
                    if let Ok(msg) = CAttack::decode(&packet.payload) {
                        let events = {
                            let mut world = self.world.lock().unwrap();
                            world.handle_command(world::WorldCommand::Attack {
                                session_id: self.session_id,
                                direction: msg.direction,
                                spell: msg.spell,
                            })
                        };

                        let _ = self.handle_world_events(events, &mut out);

                        self.update_visibility(&mut out);
                    }
                }
            }
            ClientPacketId::Disconnect | ClientPacketId::KeepAlive => {
                // For this stub, ignore these packets.
            }
        }

        out
    }

    fn on_disconnect(&mut self) {
        // Best-effort persist on TCP disconnect: mirror the LogOut
        // persistence path but without sending any packets.
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

        // Ensure this session is no longer considered visible by other
        // connections once the TCP stream is closed.
        {
            let mut map = self.player_summaries.lock().unwrap();
            map.remove(&self.session_id);
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let cfg = config::load_server_config("server.toml")
        .expect("failed to load server configuration");

    let filter = tracing_subscriber::EnvFilter::new(
        cfg.log_filter
            .as_deref()
            .unwrap_or("info"),
    );

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    let addr: SocketAddr = cfg.listen_addr;
    let store: Arc<dyn AccountStore> = Arc::new(
        SqliteAccountStore::open(&cfg.accounts_db_path)
            .expect("failed to open accounts sqlite database"),
    );

    // Load MapInfoList from the C# Server.MirDB database so we can use
    // real map metadata to drive map loading and packets. This keeps
    // compatibility with the existing C# server's database format.
    let mut world_db = WorldDatabase::new();
    match world::map::load_map_infos_from_mirdb(&cfg.server_mirdb_path) {
        Ok(maps) => {
            println!(
                "[core] Loaded {} MapInfo entries from Server.MirDB",
                maps.len()
            );
            world_db.map_infos = maps;
        }
        Err(e) => {
            println!(
                "[core] Failed to load Server.MirDB (MapInfoList): {} (continuing with empty DB)",
                e
            );
        }
    }

    // Load MonsterInfoList from the same Server.MirDB so combat logic can
    // access real monster definitions. For now this is only stored in
    // WorldDatabase and not yet wired into spawn logic.
    match world::map::load_monster_infos_from_mirdb(&cfg.server_mirdb_path) {
        Ok(monsters) => {
            println!(
                "[core] Loaded {} MonsterInfo entries from Server.MirDB",
                monsters.len()
            );
            world_db.monster_infos = monsters;
        }
        Err(e) => {
            println!(
                "[core] Failed to load Server.MirDB (MonsterInfoList): {} (continuing without Monster DB)",
                e
            );
        }
    }

    // Load NPCInfoList from the same Server.MirDB so world logic can access
    // real NPC definitions. For now this is only stored in WorldDatabase and
    // not yet wired into scene packets.
    match world::map::load_npc_infos_from_mirdb(&cfg.server_mirdb_path) {
        Ok(npcs) => {
            println!(
                "[core] Loaded {} NPCInfo entries from Server.MirDB",
                npcs.len()
            );
            world_db.npc_infos = npcs;
        }
        Err(e) => {
            println!(
                "[core] Failed to load Server.MirDB (NPCInfoList): {} (continuing without NPC DB)",
                e
            );
        }
    }

    // Load MagicInfoList so that the combat/skill system can access real spell
    // definitions (costs, ranges, power, etc.) from the MirDB.
    match world::map::load_magic_infos_from_mirdb(&cfg.server_mirdb_path) {
        Ok(magics) => {
            println!(
                "[core] Loaded {} MagicInfo entries from Server.MirDB",
                magics.len()
            );
            world_db.magic_infos = magics;
        }
        Err(e) => {
            println!(
                "[core] Failed to load Server.MirDB (MagicInfoList): {} (continuing without Magic DB)",
                e
            );
        }
    }
    let world_db = Arc::new(world_db);

    // Map directory comes from configuration (maps_path), mirroring C# Settings.MapPath.
    let world_config = WorldConfig::new(&cfg.maps_path);

    // Global world instance shared by all connections, mirroring the single-world
    // design of the original C# server. For now this is used synchronously; later
    // we can introduce a dedicated world tick loop and message queues.
    let world = world::World::new((*world_db).clone(), world_config.clone());
    let world = Arc::new(Mutex::new(world));

    // Simple world tick thread driving the World::update loop. For now this
    // only advances time for future respawn/AI logic and does not emit any
    // network events. The tick interval roughly mirrors the C# Envir.Process
    // cadence (50–100ms range).
    {
        let world = Arc::clone(&world);
        thread::spawn(move || {
            let tick = Duration::from_millis(50);
            loop {
                let now_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
                {
                    let mut w = world.lock().unwrap();
                    let _events = w.update(now_ms);
                    // TODO: handle world events (monster movement, buffs, etc.).
                }
                thread::sleep(tick);
            }
        });
    }

    // Load experience table (ExpList.ini) so we can compute MaxExperience in
    // the same way as the C# Settings.ExperienceList. We look for the file in
    // ./Configs relative to the current working directory.
    let exp_table: Arc<Vec<i64>> = {
        let mut table = Vec::new();
        let path = "./Configs/ExpList.ini";
        if let Ok(text) = fs::read_to_string(path) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('[') {
                    continue;
                }
                if let Some((_, value)) = line.split_once('=') {
                    if let Ok(v) = value.trim().parse::<i64>() {
                        table.push(v);
                    }
                }
            }
        }
        Arc::new(table)
    };

    let player_summaries: Arc<Mutex<HashMap<world::SessionId, PlayerVisual>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let next_session_id = Arc::new(AtomicU32::new(1));

    let factory: HandlerFactory = Arc::new({
        let store = Arc::clone(&store);
        let world_db = Arc::clone(&world_db);
        let world_config = world_config.clone();
        let world = Arc::clone(&world);
        let exp_table = Arc::clone(&exp_table);
        let player_summaries = Arc::clone(&player_summaries);
        let next_session_id = Arc::clone(&next_session_id);
        move || {
            let session_id = next_session_id.fetch_add(1, Ordering::Relaxed);
            Box::new(LoginConnection::new(
                session_id,
                Arc::clone(&store),
                Arc::clone(&world_db),
                world_config.clone(),
                Arc::clone(&world),
                Arc::clone(&exp_table),
                Arc::clone(&player_summaries),
            )) as Box<dyn ConnectionHandler>
        }
    });

    tracing::info!("Rust Crystal stub server listening on {}", addr);

    run_server(addr, factory).await
}
