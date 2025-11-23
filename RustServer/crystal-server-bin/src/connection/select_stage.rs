use crystal_server_core::account::{CharacterPosition, CharacterStats};
use crystal_server_core::world::{self, WorldProvider};
use crystal_server_core::world::magic::{UserMagic as WorldUserMagic, encode_client_magic_bytes};
use crystal_server_core::item::{Equipment, Inventory};
use crystal_shared_proto::login::{
    CDeleteCharacter,
    CNewCharacter,
    CStartGame,
    SDeleteCharacter,
    SDeleteCharacterSuccess,
    SNewCharacter,
    SStartGame,
};
use crystal_shared_proto::map::{SMapChanged, SMapInformation};
use crystal_shared_proto::item::SNewItemInfo;
use crystal_shared_proto::scene::{
    SBaseStatsInfo,
    SObjectTeleportIn,
    SObjectTeleportOut,
    STeleportIn,
    SDefaultNpc,
};
use crystal_shared_proto::select::{SelectInfo, SNewCharacterSuccess};
use crystal_shared_proto::user::{SUserInformation, SUserLocation, SUserSlotsRefresh};

use super::{LoginConnection, Stage};

impl LoginConnection {
    pub(crate) fn handle_new_character(
        &mut self,
        msg: CNewCharacter,
        out: &mut Vec<Vec<u8>>,
    ) {
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

    pub(crate) fn handle_delete_character(
        &mut self,
        msg: CDeleteCharacter,
        out: &mut Vec<Vec<u8>>,
    ) {
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

    pub(crate) fn handle_start_game(
        &mut self,
        msg: CStartGame,
        out: &mut Vec<Vec<u8>>,
    ) {
        if let Some(ch) = self
            .characters
            .iter()
            .find(|c| c.index == msg.character_index)
            .cloned()
        {
            let (guild_name, guild_rank_name) = if let Some(ref account_id) = self.account_id {
                if let Ok(Some((name, rank_idx))) =
                    self.store.load_character_guild(account_id, ch.index)
                {
                    let rank = if rank_idx == 0 {
                        "Leader".to_string()
                    } else {
                        "Member".to_string()
                    };
                    (name, rank)
                } else {
                    (String::new(), String::new())
                }
            } else {
                (String::new(), String::new())
            };

            self.stage = Stage::InGame;
            self.current_char_index = Some(ch.index);

            {
                let mut map = self.player_summaries.lock().unwrap();
                map.insert(
                    self.session_id,
                    super::PlayerVisual {
                        name: ch.name.clone(),
                        guild_name: guild_name.clone(),
                        guild_rank_name: guild_rank_name.clone(),
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

            // Send ItemInfo definitions (equivalent to C# PlayerObject.GetItemInfo),
            // so the client can decode any UserItem instances we may send later.
            for info in &self.world_db.item_infos {
                if let Ok(pkt) = SNewItemInfo::from_item_info(info) {
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
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

            let experience_for_world = stats_opt
                .as_ref()
                .map(|s| s.experience)
                .unwrap_or(0);

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

                    // Validate the final spawn position against the loaded map, mirroring
                    // C# PlayerObject.StartGame's ValidPoint check. If a stored/bind
                    // position is out of bounds or non-walkable (e.g. inside a shop
                    // wall tile), fall back to the nearest walkable cell around the
                    // SafeZone StartPoint (or map centre when no SafeZone exists).
                    let spawn_out_of_bounds = spawn_x < 0
                        || spawn_y < 0
                        || spawn_x >= loaded_map.width as i32
                        || spawn_y >= loaded_map.height as i32;
                    let spawn_not_walkable = !spawn_out_of_bounds
                        && !loaded_map.is_walkable(spawn_x as u16, spawn_y as u16);

                    if spawn_out_of_bounds || spawn_not_walkable {
                        println!(
                            "[core] spawn position ({}, {}) on map {} not walkable; falling back to SafeZone centre",
                            spawn_x,
                            spawn_y,
                            loaded_map.info.index,
                        );

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

            let user_magics: Vec<WorldUserMagic> = if let Some(ref account_id) = self.account_id {
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
            let job = world::Job::from_u8(ch.class).unwrap_or(world::Job::Warrior);

            // Initialise / update player in world first.
            let events = {
                let mut world = self.world.lock().unwrap();
                world.handle_command(world::WorldCommand::StartGame {
                    session_id: self.session_id,
                    character_index: ch.index,
                    map_index: map_info_core.index,
                    x: spawn_x,
                    y: spawn_y,
                    direction: initial_direction,
                    job,
                    gender: ch.gender,
                    level: ch.level,
                    experience: experience_for_world,
                    magics: user_magics,
                })
            };
            self.handle_world_events(events, out);

            // Send BaseStatsInfo so the client has the same core stat
            // formulas (HP/MP, weights, etc.) as the server for this class.
            let base_stats_bytes = world::base_stats::encode_base_stats_for_job(job);
            let base_stats_pkt = SBaseStatsInfo {
                stats_bytes: base_stats_bytes,
            };
            out.push(Self::encode_raw(base_stats_pkt.encode()));

            if let Some(ref account_id) = self.account_id {
                if let Ok(Some((inv, eq))) =
                    self.store.load_character_items(account_id, ch.index)
                {
                    let mut world = self.world.lock().unwrap();
                    world.set_player_items(self.session_id, inv, eq);
                }
            }

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

            let raw_stats = stats_opt.unwrap_or(CharacterStats {
                hp: 100,
                mp: 50,
                experience: 0,
                gold: 0,
                credit: 0,
            });

            let (max_hp, max_mp) = {
                let world = self.world.lock().unwrap();
                world
                    .player_max_hp_mp(self.session_id)
                    .unwrap_or((raw_stats.hp.max(0), raw_stats.mp.max(0)))
            };

            let clamped_hp = raw_stats.hp.clamp(0, max_hp.max(0));
            let clamped_mp = raw_stats.mp.clamp(0, max_mp.max(0));

            let stats = CharacterStats {
                hp: clamped_hp,
                mp: clamped_mp,
                experience: raw_stats.experience,
                gold: raw_stats.gold,
                credit: raw_stats.credit,
            };

            self.current_stats = Some(stats.clone());

            let user = SUserInformation {
                object_id: self.session_id,
                real_id: self.session_id,
                name: ch.name,
                guild_name,
                guild_rank: guild_rank_name,
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

            let slots_refresh = {
                let world = self.world.lock().unwrap();
                let (inv, eq) = world
                    .player_items(self.session_id)
                    .unwrap_or((
                        Inventory::new_default(),
                        Equipment::new_default(),
                    ));
                SUserSlotsRefresh {
                    inventory: inv.slots,
                    equipment: eq.slots,
                }
            };
            if let Ok(raw) = slots_refresh.encode() {
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

            let default_npc = SDefaultNpc {
                object_id: super::LoginConnection::DEFAULT_NPC_ID,
            };
            if let Ok(raw) = default_npc.encode() {
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
            self.update_visibility(out);
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
