use std::{collections::HashMap, io, net::SocketAddr, sync::{Arc, Mutex}};

use crystal_server_net::{run_server, ConnectionHandler, HandlerFactory};
use crystal_shared_proto::login::{
    CChangePassword, CClientVersion, CDeleteCharacter, CLogin, CNewAccount, CNewCharacter,
    CStartGame, ClientPacketId, SChangePassword, SClientVersion, SConnected, SLogin,
    SLoginBanned, SNewAccount, SNewCharacter, SStartGame,
};
use crystal_shared_proto::map::{SMapChanged, SMapInformation};
use crystal_shared_proto::npc::{SObjectNpc, SNpcResponse};
use crystal_shared_proto::packet::RawPacket;
use crystal_shared_proto::scene::{
    SGainExperience,
    SGainedGold,
    SLevelChanged,
    SObjectLeveled,
    SObjectTeleportIn,
    SObjectTeleportOut,
    STeleportIn,
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

#[derive(Clone, Debug)]
struct AccountStub {
    password: String,
}

type SharedAccounts = Arc<Mutex<HashMap<String, AccountStub>>>;

struct LoginConnection {
    stage: Stage,
    account_id: Option<String>,
    characters: Vec<SelectInfo>,
    next_char_index: i32,
    accounts: SharedAccounts,
}

impl LoginConnection {
    fn new(accounts: SharedAccounts) -> Self {
        LoginConnection {
            stage: Stage::Connected,
            account_id: None,
            characters: Vec::new(),
            next_char_index: 0,
            accounts,
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
                    let mut accounts = self.accounts.lock().expect("accounts mutex poisoned");

                    if accounts.contains_key(&msg.account_id) {
                        out.push(Self::encode_raw(SNewAccount { result: 7 }.encode()));
                    } else if msg.account_id.is_empty() {
                        // 1: Bad AccountID
                        out.push(Self::encode_raw(SNewAccount { result: 1 }.encode()));
                    } else if msg.password.is_empty() {
                        // 2: Bad Password
                        out.push(Self::encode_raw(SNewAccount { result: 2 }.encode()));
                    } else {
                        accounts.insert(
                            msg.account_id.clone(),
                            AccountStub {
                                password: msg.password.clone(),
                            },
                        );

                        // 8: Success
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

                    let accounts = self.accounts.lock().expect("accounts mutex poisoned");
                    if let Some(acc) = accounts.get(&msg.account_id) {
                        if acc.password != msg.password {
                            // 4: Wrong Password
                            out.push(Self::encode_raw(SLogin { result: 4 }.encode()));
                            return out;
                        }

                        self.account_id = Some(msg.account_id);
                        self.stage = Stage::Select;

                        let resp = SLoginSuccess {
                            characters: self.characters.clone(),
                        };
                        if let Ok(raw) = resp.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    } else {
                        // 3: Account Not Exist
                        out.push(Self::encode_raw(SLogin { result: 3 }.encode()));
                    }
                }
            }
            ClientPacketId::ChangePassword => {
                if let Ok(msg) = CChangePassword::decode(&packet.payload) {
                    let mut accounts = self.accounts.lock().expect("accounts mutex poisoned");

                    if let Some(acc) = accounts.get_mut(&msg.account_id) {
                        if acc.password != msg.current_password {
                            // 5: Wrong Password
                            out.push(Self::encode_raw(SChangePassword { result: 5 }.encode()));
                        } else if msg.new_password.is_empty() {
                            // 3: Bad New Password
                            out.push(Self::encode_raw(SChangePassword { result: 3 }.encode()));
                        } else {
                            acc.password = msg.new_password.clone();
                            // 6: Success
                            out.push(Self::encode_raw(SChangePassword { result: 6 }.encode()));
                        }
                    } else {
                        // 4: Account Not Exist
                        out.push(Self::encode_raw(SChangePassword { result: 4 }.encode()));
                    }
                }
            }
            ClientPacketId::NewCharacter => {
                if let Ok(msg) = CNewCharacter::decode(&packet.payload) {
                    // Create a new in-memory character.
                    let idx = self.next_char_index;
                    self.next_char_index += 1;

                    let info = SelectInfo {
                        index: idx,
                        name: msg.name,
                        level: 1,
                        class: msg.class,
                        gender: msg.gender,
                        last_access_binary: 0,
                    };

                    self.characters.push(info.clone());

                    // Result code 10 in the C# comments indicates success.
                    out.push(Self::encode_raw(SNewCharacter { result: 10 }.encode()));

                    let succ = SNewCharacterSuccess { char_info: info };
                    if let Ok(raw) = succ.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
            ClientPacketId::DeleteCharacter => {
                if let Ok(msg) = CDeleteCharacter::decode(&packet.payload) {
                    if let Some(pos) = self
                        .characters
                        .iter()
                        .position(|c| c.index == msg.character_index)
                    {
                        self.characters.remove(pos);

                        // In C#, DeleteCharacter errors are sent via SDeleteCharacter,
                        // while success uses DeleteCharacterSuccess only.
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

                        // Minimal MapInformation stub so client can attempt to enter a map.
                        let map = SMapInformation {
                            map_index: 0,
                            // Use an existing map file used by the original server (see Settings.PKTownMapName = "3").
                            // Title can be arbitrary for the stub.
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
    let accounts: SharedAccounts = Arc::new(Mutex::new(HashMap::new()));

    let factory: HandlerFactory = Arc::new({
        let accounts = Arc::clone(&accounts);
        move || Box::new(LoginConnection::new(Arc::clone(&accounts))) as Box<dyn ConnectionHandler>
    });

    println!("Rust Crystal stub server listening on {}", addr);

    run_server(addr, factory).await
}
