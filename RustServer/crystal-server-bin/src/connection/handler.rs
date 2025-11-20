use std::io::{self, Cursor};
use std::path::Path;

use crystal_server_core::account::{CharacterPosition, CharacterStats, CharacterSummary};
use crystal_server_core::world::{self};
use crystal_server_core::world::WorldProvider;
use crystal_server_core::world::magic::{UserMagic as WorldUserMagic, encode_client_magic_bytes};
use crystal_server_net::ConnectionHandler;
use crystal_shared_proto::io::read_string;
use crystal_shared_proto::login::{
    CAttack,
    CCallNPC,
    CChangePassword,
    CClientVersion,
    CDeleteCharacter,
    CLogin,
    CNewAccount,
    CNewCharacter,
    CRun,
    CStartGame,
    CTurn,
    CWalk,
    ClientPacketId,
    SChangePassword,
    SClientVersion,
    SConnected,
    SDeleteCharacter,
    SDeleteCharacterSuccess,
    SLogin,
    SNewAccount,
    SNewCharacter,
    SStartGame,
};
use crystal_shared_proto::map::{SMapChanged, SMapInformation};
use crystal_shared_proto::packet::RawPacket;
use crystal_shared_proto::scene::{
    SNpcResponse,
    SObjectRemove,
    SObjectTeleportIn,
    SObjectTeleportOut,
    STeleportIn,
};
use crystal_shared_proto::npc::{SNpcGoods, SNpcSell};
use crystal_shared_proto::select::{
    SelectInfo,
    SLogOutFailed,
    SLogOutSuccess,
    SLoginSuccess,
    SNewCharacterSuccess,
};
use crystal_shared_proto::user::{SLoseGold, SUserInformation, SUserLocation};

use super::{LoginConnection, Stage};

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
                        out.push(Self::encode_raw(SNewAccount { result: 1 }.encode()));
                    } else if msg.password.is_empty() {
                        out.push(Self::encode_raw(SNewAccount { result: 2 }.encode()));
                    } else if self
                        .store
                        .account_exists(&msg.account_id)
                        .unwrap_or(false)
                    {
                        out.push(Self::encode_raw(SNewAccount { result: 7 }.encode()));
                    } else {
                        let _ = self.store.create_account(&msg.account_id, &msg.password);
                        out.push(Self::encode_raw(SNewAccount { result: 8 }.encode()));
                    }
                }
            }
            ClientPacketId::ClientVersion => {
                if let Ok(_msg) = CClientVersion::decode(&packet.payload) {
                    out.push(Self::encode_raw(SClientVersion { result: 1 }.encode()));
                    self.stage = Stage::VersionChecked;
                }
            }
            ClientPacketId::Login => {
                if let Ok(msg) = CLogin::decode(&packet.payload) {
                    if msg.account_id.is_empty() {
                        out.push(Self::encode_raw(SLogin { result: 1 }.encode()));
                        return out;
                    }
                    if msg.password.is_empty() {
                        out.push(Self::encode_raw(SLogin { result: 2 }.encode()));
                        return out;
                    }

                    match self.store.verify_password(&msg.account_id, &msg.password) {
                        Ok(true) => {
                            self.account_id = Some(msg.account_id.clone());
                            self.stage = Stage::Select;

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
                        Ok(false) | Err(_) => {
                            out.push(Self::encode_raw(SLogin { result: 4 }.encode()));
                        }
                    }
                }
            }
            ClientPacketId::ChangePassword => {
                if let Ok(msg) = CChangePassword::decode(&packet.payload) {
                    if msg.new_password.is_empty() {
                        out.push(Self::encode_raw(SChangePassword { result: 3 }.encode()));
                    } else {
                        let exists = self
                            .store
                            .account_exists(&msg.account_id)
                            .unwrap_or(false);

                        if !exists {
                            out.push(Self::encode_raw(SChangePassword { result: 4 }.encode()));
                        } else {
                            match self
                                .store
                                .verify_password(&msg.account_id, &msg.current_password)
                            {
                                Ok(true) => {
                                    let _ = self
                                        .store
                                        .set_password(&msg.account_id, &msg.new_password);
                                    out.push(Self::encode_raw(SChangePassword { result: 6 }.encode()));
                                }
                                Ok(false) | Err(_) => {
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
                        match self.store.create_character(acc_id, msg.name, msg.class, msg.gender) {
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

                            let succ = SDeleteCharacterSuccess {
                                character_index: msg.character_index,
                            };
                            if let Ok(raw) = succ.encode() {
                                out.push(Self::encode_raw(raw));
                            }
                        } else {
                            let err = SDeleteCharacter { result: 1 };
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

                        {
                            let mut map = self.player_summaries.lock().unwrap();
                            map.insert(
                                self.session_id,
                                super::PlayerVisual {
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

                        let ok = SStartGame { result: 4, resolution: 1024 };
                        if let Ok(raw) = ok.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let stored_pos = if let Some(ref account_id) = self.account_id {
                            self
                                .store
                                .load_character_position(account_id, ch.index)
                                .unwrap_or(None)
                        } else {
                            None
                        };

                        let stats_opt = if let Some(ref account_id) = self.account_id {
                            self
                                .store
                                .load_character_stats(account_id, ch.index)
                                .unwrap_or(None)
                        } else {
                            None
                        };

                        let bind_pos = if let Some(ref account_id) = self.account_id {
                            self
                                .store
                                .load_character_bind(account_id, ch.index)
                                .unwrap_or(None)
                        } else {
                            None
                        };

                        let map_info_core = {
                            let map_infos = &self.world_db.map_infos;
                            if let Some(pos) = &stored_pos {
                                if let Some(info) = map_infos.iter().find(|m| m.index == pos.map_index) {
                                    info.clone()
                                } else if let Some(info) = map_infos
                                    .iter()
                                    .find(|m| m.safe_zones.iter().any(|z| z.start_point))
                                {
                                    info.clone()
                                } else if let Some(info) = map_infos.first() {
                                    info.clone()
                                } else {
                                    stub_map_info()
                                }
                            } else if let Some(pos) = &bind_pos {
                                if let Some(info) = map_infos.iter().find(|m| m.index == pos.map_index) {
                                    info.clone()
                                } else if let Some(info) = map_infos
                                    .iter()
                                    .find(|m| m.safe_zones.iter().any(|z| z.start_point))
                                {
                                    info.clone()
                                } else if let Some(info) = map_infos.first() {
                                    info.clone()
                                } else {
                                    stub_map_info()
                                }
                            } else if let Some(info) = map_infos
                                .iter()
                                .find(|m| m.safe_zones.iter().any(|z| z.start_point))
                            {
                                info.clone()
                            } else if let Some(info) = map_infos.first() {
                                info.clone()
                            } else {
                                stub_map_info()
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

                        let mut user_magics: Vec<WorldUserMagic> = if let Some(ref account_id) = self.account_id {
                            self
                                .store
                                .load_character_magics(account_id, ch.index)
                                .unwrap_or_else(|_| Vec::new())
                        } else {
                            Vec::new()
                        };

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
                                level: ch.level,
                                magics: user_magics,
                            })
                        };
                        self.handle_world_events(events, &mut out);

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

                        let stats = stats_opt.unwrap_or(CharacterStats {
                            hp: 100,
                            mp: 50,
                            experience: 0,
                            gold: 0,
                            credit: 0,
                        });

                        self.current_stats = Some(stats.clone());

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
                            result: 2,
                            resolution: 0,
                        };
                        if let Ok(raw) = err.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
            }
            ClientPacketId::LogOut => {
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

                    {
                        let mut map = self.player_summaries.lock().unwrap();
                        map.remove(&self.session_id);
                    }
                } else {
                    let resp = SLogOutFailed;
                    out.push(Self::encode_raw(resp.encode()));
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
                            self.known_monsters.clear();
                            self.known_npcs.clear();
                        }

                        self.update_visibility(&mut out);
                    }
                }
            }
            ClientPacketId::Chat => {
                if self.stage == Stage::InGame {
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
                                let key_upper = key.as_str();

                                // Shop-related keys: mirror C# NPCScript.Call behaviour for
                                // Buy/BuySell/Sell/Craft entries by sending an NPCGoods
                                // packet (and optionally an NPCSell packet) instead of a
                                // normal NPC dialog page.
                                let is_buy_panel = matches!(
                                    key_upper,
                                    "@BUY" | "@BUYNEW" | "@BUYBACK" | "@BUYUSED" | "@PEARLBUY" | "@BUYSELL" | "@BUYSELLNEW"
                                );
                                let wants_sell_panel = matches!(key_upper, "@BUYSELL" | "@BUYSELLNEW");
                                let is_sell_only = key_upper == "@SELL";

                                if is_buy_panel || is_sell_only {
                                    // For now we use an empty goods list but a byte layout
                                    // that exactly matches C# ServerPackets.NPCGoods.
                                    // PanelType.Buy = 0, PanelType.Sell = 3, PanelType.Craft = 2.
                                    let panel_type: u8 = 0; // Buy panel for now.
                                    let rate: f32 = 1.0; // Placeholder for PriceRate(player).
                                    let goods: Vec<_> = Vec::new();

                                    if let Ok(bytes) =
                                        Self::build_npc_goods_bytes(&goods, rate, panel_type, false)
                                    {
                                        let pkt = SNpcGoods { goods_bytes: bytes };
                                        let raw = pkt.encode();
                                        out.push(Self::encode_raw(raw));
                                    }

                                    if wants_sell_panel || is_sell_only {
                                        let sell = SNpcSell;
                                        let raw = sell.encode();
                                        out.push(Self::encode_raw(raw));
                                    }

                                    return out;
                                }

                                let root = Path::new("./deploy/Envir/NPCs");
                                let mut maybe_page: Option<Vec<String>> = None;

                                if root.exists() {
                                    if let Some(script_path) = Self::find_npc_script_path(root, &npc.file_name) {
                                        if let Ok((pages, moves)) = Self::load_npc_script_from_file(&script_path) {
                                            let paid = Self::extract_paid_teleport_info(&script_path, &key);

                                            if let Some((map_name, tx, ty)) = moves.get(&key) {
                                                let mut dest_index: Option<i32> = None;

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
                                                    if let Some((price, fail_label)) = paid {
                                                        if let Some(mut stats) = self.current_stats.clone() {
                                                            if stats.gold < price {
                                                                if let Some(fail_key) = fail_label {
                                                                    maybe_page = pages.get(&fail_key).cloned();
                                                                }
                                                            } else {
                                                                stats.gold = stats.gold.saturating_sub(price);
                                                                if let (Some(ref account_id), Some(char_idx)) =
                                                                    (self.account_id.as_ref(), self.current_char_index)
                                                                {
                                                                    let _ = self
                                                                        .store
                                                                        .save_character_stats(account_id, char_idx, &stats);
                                                                }
                                                                self.current_stats = Some(stats.clone());

                                                                let lose = SLoseGold { gold: price as u32 };
                                                                if let Ok(raw) = lose.encode() {
                                                                    out.push(Self::encode_raw(raw));
                                                                }

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
                                                    } else {
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
            ClientPacketId::Disconnect | ClientPacketId::KeepAlive => {}
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

        {
            let mut map = self.player_summaries.lock().unwrap();
            map.remove(&self.session_id);
        }
    }
}

fn stub_map_info() -> world::map::MapInfo {
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
