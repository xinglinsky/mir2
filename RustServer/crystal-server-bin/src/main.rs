use std::{io, net::SocketAddr, sync::Arc};

use crystal_server_net::{run_server, ConnectionHandler, HandlerFactory};
use crystal_server_core::world::{self, WorldConfig, WorldDatabase};
use crystal_server_core::account::{AccountStore, CharacterSummary, FileAccountStore};
use crystal_shared_proto::login::{
    CChangePassword, CClientVersion, CDeleteCharacter, CLogin, CNewAccount, CNewCharacter,
    CRun, CStartGame, CTurn, CWalk, ClientPacketId, SChangePassword, SClientVersion,
    SConnected, SLogin, SLoginBanned, SNewAccount, SNewCharacter, SStartGame,
};
use crystal_shared_proto::map::{SMapChanged, SMapInformation};
use crystal_shared_proto::npc::{SObjectNpc, SNpcResponse};
use crystal_shared_proto::packet::RawPacket;
use crystal_shared_proto::scene::{
    SAddBuff,
    SColourChanged,
    SGainExperience,
    SGainedGold,
    SLevelChanged,
    SMagic,
    SMagicCast,
    SMagicLeveled,
    SNewMagic,
    SObjectColourChanged,
    SObjectGuildNameChanged,
    SObjectHide,
    SObjectLeveled,
    SObjectPoisoned,
    SObjectShow,
    SObjectTeleportIn,
    SObjectTeleportOut,
    SPauseBuff,
    SPoisoned,
    SRemoveBuff,
    SObjectHidden,
    STeleportIn,
};
use crystal_shared_proto::io::{
    write_bool,
    write_i32_le,
    write_i64_le,
    write_string,
    write_u16_le,
    write_u32_le,
};
use crystal_shared_proto::user::{SChat, SUserInformation, SUserLocation};
use crystal_shared_proto::select::{SelectInfo, SLoginSuccess, SNewCharacterSuccess};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Stage {
    Connected,
    VersionChecked,
    Select,
    InGame,
}

struct LoginConnection {
    stage: Stage,
    session_id: world::SessionId,
    account_id: Option<String>,
    characters: Vec<SelectInfo>,
    store: Arc<dyn AccountStore>,
    world_db: Arc<WorldDatabase>,
    world_config: WorldConfig,
    world: world::World<WorldDatabase>,
    current_x: i32,
    current_y: i32,
    direction: u8,
}

impl LoginConnection {
    fn new(
        store: Arc<dyn AccountStore>,
        world_db: Arc<WorldDatabase>,
        world_config: WorldConfig,
    ) -> Self {
        let world = world::World::new((*world_db).clone(), world_config.clone());
        LoginConnection {
            stage: Stage::Connected,
            session_id: 1,
            account_id: None,
            characters: Vec::new(),
            store,
            world_db,
            world_config,
            world,
            current_x: 0,
            current_y: 0,
            direction: 0,
        }
    }

    fn encode_raw(raw: RawPacket) -> Vec<u8> {
        raw.encode()
    }

    fn apply_step(&mut self, direction: u8, distance: i32) {
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
            _ => return,
        };

        let events = self.world.handle_command(cmd);
        for event in events {
            if let world::WorldEvent::UserLocation {
                x,
                y,
                direction,
                ..
            } = event
            {
                self.current_x = x;
                self.current_y = y;
                self.direction = direction;
            }
        }
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

                        // Signal successful start game so the client switches to GameScene.
                        let ok = SStartGame {
                            result: 4,
                            resolution: 1024,
                        };
                        if let Ok(raw) = ok.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        // Choose a MapInfo to drive loading and packets. Prefer file_name "3",
                        // otherwise fall back to the first entry or a stub if DB is empty.
                        let map_info_core = {
                            let map_infos = &self.world_db.map_infos;
                            if let Some(info) = map_infos
                                .iter()
                                .find(|m| m.file_name.eq_ignore_ascii_case("3"))
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

                                // Prefer spawning near the first SafeZone marked as StartPoint;
                                // if none, fall back to the first SafeZone; if still none,
                                // use the map centre as before.
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
                                    // No walkable cells found; fall back to the chosen centre.
                                    spawn_x = center_x;
                                    spawn_y = center_y;
                                }
                            }
                            Err(e) => {
                                println!(
                                    "[core] Failed to load map '3' from {:?}: {} (falling back to stub packets)",
                                    map_dir, e
                                );
                            }
                        }

                        // Initialize world state for this session.
                        let events = self.world.handle_command(world::WorldCommand::StartGame {
                            session_id: self.session_id,
                            map_index: map_info_core.index,
                            x: spawn_x,
                            y: spawn_y,
                            direction: 0,
                        });
                        for event in events {
                            if let world::WorldEvent::UserLocation {
                                x,
                                y,
                                direction,
                                ..
                            } = event
                            {
                                self.current_x = x;
                                self.current_y = y;
                                self.direction = direction;
                            }
                        }

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

                        // Minimal UserInformation so the client receives basic player state.
                        let user = SUserInformation {
                            object_id: 1,
                            real_id: 1,
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
                            hp: 100,
                            mp: 50,
                            experience: 0,
                            max_experience: 1,
                            level_effects: 0,
                            has_hero: false,
                            hero_behaviour: 0,
                            gold: 0,
                            credit: 0,
                            has_expanded_storage: false,
                            expanded_storage_expiry_binary: 0,
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
                                object_id: 1,
                                teleport_type: 0,
                            };
                            if let Ok(raw) = tele_out.encode() {
                                out.push(Self::encode_raw(raw));
                            }

                            let tele_in = STeleportIn;
                            out.push(Self::encode_raw(tele_in.encode()));

                            let obj_tele_in = SObjectTeleportIn {
                                object_id: 1,
                                teleport_type: 0,
                            };
                            if let Ok(raw) = obj_tele_in.encode() {
                                out.push(Self::encode_raw(raw));
                            }
                        }

                        let exp = SGainExperience { amount: 1_000 };
                        if let Ok(raw) = exp.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let level_up = SLevelChanged {
                            level: ch.level + 1,
                            experience: 0,
                            max_experience: 1,
                        };
                        if let Ok(raw) = level_up.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let leveled = SObjectLeveled { object_id: 1 };
                        if let Ok(raw) = leveled.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let gold = SGainedGold { gold: 5_000 };
                        if let Ok(raw) = gold.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let npc = SObjectNpc {
                            object_id: 100,
                            name: "Rust NPC".to_string(),
                            name_colour_argb: -1,
                            image: 0,
                            colour_argb: -1,
                            location_x: 5,
                            location_y: 5,
                            direction: 0,
                            quest_ids: Vec::new(),
                        };
                        if let Ok(raw) = npc.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let npc_resp = SNpcResponse {
                            page: vec![
                                "Welcome to the Rust NPC".to_string(),
                                "This is a stub response from the Rust server.".to_string(),
                            ],
                        };
                        if let Ok(raw) = npc_resp.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let colour_self = SColourChanged {
                            name_colour_argb: 0xFFFF_FFFFu32 as i32,
                        };
                        if let Ok(raw) = colour_self.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let colour_npc = SObjectColourChanged {
                            object_id: 100,
                            name_colour_argb: 0xFF00_FF00u32 as i32,
                        };
                        if let Ok(raw) = colour_npc.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let guild_change = SObjectGuildNameChanged {
                            object_id: 1,
                            guild_name: "RustGuild".to_string(),
                        };
                        if let Ok(raw) = guild_change.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let hide_npc = SObjectHide { object_id: 100 };
                        if let Ok(raw) = hide_npc.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let show_npc = SObjectShow { object_id: 100 };
                        if let Ok(raw) = show_npc.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let poisoned_self = SPoisoned { poison: 1 };
                        if let Ok(raw) = poisoned_self.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let poisoned_npc = SObjectPoisoned {
                            object_id: 100,
                            poison: 1,
                        };
                        if let Ok(raw) = poisoned_npc.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let new_magic_payload: io::Result<Vec<u8>> = (|| {
                            let mut buf = Vec::new();
                            write_string(&mut buf, "FireBall")?;
                            buf.push(0);
                            buf.push(10);
                            buf.push(1);
                            buf.push(0);
                            buf.push(1);
                            buf.push(2);
                            buf.push(3);
                            write_u16_le(&mut buf, 0)?;
                            write_u16_le(&mut buf, 0)?;
                            write_u16_le(&mut buf, 0)?;
                            buf.push(1);
                            buf.push(1);
                            write_u16_le(&mut buf, 0)?;
                            write_i64_le(&mut buf, 0)?;
                            buf.push(9);
                            write_i64_le(&mut buf, 0)?;
                            write_bool(&mut buf, false)?;
                            Ok(buf)
                        })();

                        if let Ok(payload) = new_magic_payload {
                            let new_magic = SNewMagic {
                                magic_bytes: payload,
                            };
                            let raw = new_magic.encode();
                            out.push(Self::encode_raw(raw));
                        }

                        // Simple Buff: a visible infinite HP buff on the main player.
                        let buff_payload: io::Result<Vec<u8>> = (|| {
                            let mut buf = Vec::new();

                            // ClientBuff.Type (BuffType) - use 0 as a generic buff type for testing.
                            buf.push(0);
                            // Visible
                            write_bool(&mut buf, true)?;
                            // ObjectID (player)
                            write_u32_le(&mut buf, 1)?;
                            // ExpireTime (ignored when Infinite=true, set 0)
                            write_i64_le(&mut buf, 0)?;
                            // Infinite
                            write_bool(&mut buf, true)?;
                            // Paused
                            write_bool(&mut buf, false)?;

                            // Stats.Save: Count=1, Stat.HP (12), Value=10.
                            write_i32_le(&mut buf, 1)?;
                            buf.push(12);
                            write_i32_le(&mut buf, 10)?;

                            // Values: zero-length for this stub.
                            write_i32_le(&mut buf, 0)?;

                            Ok(buf)
                        })();

                        if let Ok(payload) = buff_payload {
                            let add_buff = SAddBuff { buff_bytes: payload };
                            let raw = add_buff.encode();
                            out.push(Self::encode_raw(raw));
                        }

                        let pause_buff = SPauseBuff {
                            buff_type: 0,
                            object_id: 1,
                            paused: true,
                        };
                        if let Ok(raw) = pause_buff.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let remove_buff = SRemoveBuff {
                            buff_type: 0,
                            object_id: 1,
                        };
                        if let Ok(raw) = remove_buff.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let hidden_player = SObjectHidden {
                            object_id: 1,
                            hidden: true,
                        };
                        if let Ok(raw) = hidden_player.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        // Simple magic-related packets so we exercise the skill pipeline.
                        let magic_leveled = SMagicLeveled {
                            object_id: 1,
                            // Spell.FireBall = 0 in C# enum; we just use 0 as a generic spell id.
                            spell: 0,
                            level: 1,
                            experience: 100,
                        };
                        if let Ok(raw) = magic_leveled.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let magic_cast = SMagicCast { spell: 0 };
                        out.push(Self::encode_raw(magic_cast.encode()));

                        let magic = SMagic {
                            spell: 0,
                            target_id: 0,
                            target_x: 5,
                            target_y: 5,
                            cast: true,
                            level: 1,
                            secondary_target_ids: vec![0],
                        };
                        if let Ok(raw) = magic.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        // Simple welcome chat so we exercise the Chat pipeline.
                        let chat = SChat {
                            message: "Welcome to the Rust stub server".to_string(),
                            // ChatType.Normal = 0
                            chat_type: 0,
                        };
                        if let Ok(raw) = chat.encode() {
                            out.push(Self::encode_raw(raw));
                        }
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
                // Reset simple in-memory state.
                self.stage = Stage::Connected;
                self.account_id = None;
            }
            ClientPacketId::Turn => {
                if self.stage == Stage::InGame {
                    if let Ok(msg) = CTurn::decode(&packet.payload) {
                        self.apply_step(msg.direction, 0);

                        let loc = SUserLocation {
                            location_x: self.current_x,
                            location_y: self.current_y,
                            direction: self.direction,
                        };
                        if let Ok(raw) = loc.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
            }
            ClientPacketId::Walk => {
                if self.stage == Stage::InGame {
                    if let Ok(msg) = CWalk::decode(&packet.payload) {
                        self.apply_step(msg.direction, 1);

                        let loc = SUserLocation {
                            location_x: self.current_x,
                            location_y: self.current_y,
                            direction: self.direction,
                        };
                        if let Ok(raw) = loc.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
            }
            ClientPacketId::Run => {
                if self.stage == Stage::InGame {
                    if let Ok(msg) = CRun::decode(&packet.payload) {
                        self.apply_step(msg.direction, 2);

                        let loc = SUserLocation {
                            location_x: self.current_x,
                            location_y: self.current_y,
                            direction: self.direction,
                        };
                        if let Ok(raw) = loc.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
            }
            ClientPacketId::Chat => {
            }
            ClientPacketId::Disconnect | ClientPacketId::KeepAlive => {
                // For this stub, ignore these packets.
            }
        }

        out
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let addr: SocketAddr = "0.0.0.0:7000".parse().expect("invalid listen address");
    let store: Arc<dyn AccountStore> = Arc::new(
        FileAccountStore::open("./data/accounts.json")
            .expect("failed to open accounts database"),
    );

    // Load MapInfoList from the C# Server.MirDB database so we can use
    // real map metadata to drive map loading and packets. This keeps
    // compatibility with the existing C# server's database format.
    let mut world_db = WorldDatabase::new();
    match world::map::load_map_infos_from_mirdb("./Server.MirDB") {
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
    let world_db = Arc::new(world_db);

    // For now, use a relative ./Maps directory for .map files.
    // You can point this to your actual MapPath (e.g. from C# Settings.MapPath).
    let world_config = WorldConfig::new("./Maps");

    let factory: HandlerFactory = Arc::new({
        let store = Arc::clone(&store);
        let world_db = Arc::clone(&world_db);
        let world_config = world_config.clone();
        move || {
            Box::new(LoginConnection::new(
                Arc::clone(&store),
                Arc::clone(&world_db),
                world_config.clone(),
            )) as Box<dyn ConnectionHandler>
        }
    });

    println!("Rust Crystal stub server listening on {}", addr);

    run_server(addr, factory).await
}
