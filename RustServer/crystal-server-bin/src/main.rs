use std::{io, net::SocketAddr, sync::Arc};

use crystal_server_net::{run_server, ConnectionHandler, HandlerFactory};
use crystal_server_core::world::{self, WorldConfig};
use crystal_server_core::account::{AccountStore, CharacterSummary, FileAccountStore};
use crystal_shared_proto::login::{
    CChangePassword, CClientVersion, CDeleteCharacter, CLogin, CNewAccount, CNewCharacter,
    CStartGame, ClientPacketId, SChangePassword, SClientVersion, SConnected, SLogin,
    SLoginBanned, SNewAccount, SNewCharacter, SStartGame,
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
    account_id: Option<String>,
    characters: Vec<SelectInfo>,
    store: Arc<dyn AccountStore>,
    world_config: WorldConfig,
}

impl LoginConnection {
    fn new(store: Arc<dyn AccountStore>, world_config: WorldConfig) -> Self {
        LoginConnection {
            stage: Stage::Connected,
            account_id: None,
            characters: Vec::new(),
            store,
            world_config,
        }
    }

    fn encode_raw(raw: RawPacket) -> Vec<u8> {
        raw.encode()
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
                    } else if !self
                        .store
                        .account_exists(&msg.account_id)
                        .unwrap_or(false)
                    {
                        // 4: Account Not Exist
                        out.push(Self::encode_raw(SChangePassword { result: 4 }.encode()));
                    } else {
                        // For now we don't persist password changes; just pretend success.
                        out.push(Self::encode_raw(SChangePassword { result: 6 }.encode()));
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

                        // Attempt to load a real map (file_name "3") via crystal-server-core.
                        // This is best-effort: failures only log to stdout, stub packets still sent.
                        let map_info_core = world::map::MapInfo {
                            index: 0,
                            file_name: "3".to_string(),
                            title: "StubMap".to_string(),
                            mini_map: 0,
                            big_map: 0,
                            light: 0,
                            map_dark_light: 0,
                            music: 0,
                            weather_particles: 0,
                            safe_zones: Vec::new(),
                            respawns: Vec::new(),
                            movements: Vec::new(),
                        };

                        let map_dir = &self.world_config.map_path;
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
                            }
                            Err(e) => {
                                println!(
                                    "[core] Failed to load map '3' from {:?}: {} (falling back to stub packets)",
                                    map_dir, e
                                );
                            }
                        }

                        // Minimal MapInformation stub so client can attempt to enter a map.
                        let map = SMapInformation {
                            map_index: 0,
                            file_name: "3".to_string(),
                            title: "StubMap".to_string(),
                            mini_map: 0,
                            big_map: 0,
                            lights: 0,
                            lightning: false,
                            fire: false,
                            map_dark_light: 0,
                            music: 0,
                            weather_particles: 0,
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
                            location_x: 0,
                            location_y: 0,
                            direction: 0,
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
                            location_x: 0,
                            location_y: 0,
                            direction: 0,
                        };
                        if let Ok(raw) = loc.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        let map_changed = SMapChanged {
                            map_index: 0,
                            file_name: "3".to_string(),
                            title: "StubMap".to_string(),
                            mini_map: 0,
                            big_map: 0,
                            lights: 0,
                            location_x: 0,
                            location_y: 0,
                            direction: 0,
                            map_dark_light: 0,
                            music: 0,
                            weather: 0,
                        };
                        if let Ok(raw) = map_changed.encode() {
                            out.push(Self::encode_raw(raw));
                        }

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

    // For now, use a relative ./Map directory for .map files.
    // You can point this to your actual MapPath (e.g. from C# Settings.MapPath).
    let world_config = WorldConfig::new("./Map");

    let factory: HandlerFactory = Arc::new({
        let store = Arc::clone(&store);
        let world_config = world_config.clone();
        move || {
            Box::new(LoginConnection::new(
                Arc::clone(&store),
                world_config.clone(),
            )) as Box<dyn ConnectionHandler>
        }
    });

    println!("Rust Crystal stub server listening on {}", addr);

    run_server(addr, factory).await
}
