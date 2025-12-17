use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crystal_server_core::account::{
    AccountStorage,
    CharacterPosition,
    CharacterStats,
    CharacterSummary,
    StoredMail,
    StoredFriend,
};
use crystal_server_core::world::{self, WorldProvider};
use crystal_server_core::world::configs::base_stats;
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
use crystal_shared_proto::map::SMapInformation;
use crystal_shared_proto::quest::{SChangeQuest, SCompleteQuest};
use crystal_shared_proto::item::{SNewItemInfo, SNewRecipeInfo, SUserStorage, SResizeStorage};
use crystal_shared_proto::scene::{
    SBaseStatsInfo,
    SHeroBaseStatsInfo,
    SObjectTeleportIn,
    SObjectTeleportOut,
    SObjectTurn,
    SObjectTurnWalkRun,
    STeleportIn,
    SDefaultNpc,
    SObjectHidden,
    SObjectHealth,
    SSpellToggle,
};
use crystal_shared_proto::select::{SelectInfo, SNewCharacterSuccess};
use crystal_shared_proto::notice::{NoticeData, SUpdateNotice};
use crystal_shared_proto::shop::SGameShopInfo;
use crystal_shared_proto::mail::SReceiveMail;
use crystal_shared_proto::user::{
    SChangeAMode,
    SChangePMode,
    SHealthChanged,
    SHeroHealthChanged,
    STimeOfDay,
    SSwitchGroup,
    SUserInformation,
    SUserSlotsRefresh,
};
use crystal_shared_proto::io::{
    write_bool,
    write_i32_le,
    write_i64_le,
    write_u16_le,
    write_u32_le,
    write_string,
    write_u64_le,
};
use crystal_shared_proto::hero::{SHeroInformation, SUpdateHeroSpawnState};

use super::{LoginConnection, Stage};

impl LoginConnection {
    pub(crate) fn handle_new_character(
        &mut self,
        msg: CNewCharacter,
        out: &mut Vec<Vec<u8>>,
    ) {
        if let Some(acc_id) = &self.account_id {
            if let Ok(Some(_)) = self.store.find_character_by_name(&msg.name) {
                out.push(Self::encode_raw(SNewCharacter { result: 5 }.encode()));
                return;
            }
            match self.store.create_character(acc_id, msg.name, msg.class, msg.gender) {
                Ok(ch) => {
                    let info = SelectInfo {
                        index: ch.index,
                        name: ch.name,
                        level: ch.level,
                        class: ch.class,
                        gender: ch.gender,
                        // Stored as Unix ms in the database; convert to
                        // .NET DateTime.ToBinary-compatible ticks for the
                        // legacy C# client.
                        last_access_binary: Self::unix_ms_to_dotnet_binary(ch.last_access_binary),
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

            let (hair, allow_observe) = {
                let world = self.world.lock().unwrap();
                (
                    world.player_hair(self.session_id).unwrap_or(0),
                    world.player_allow_observe(self.session_id).unwrap_or(false),
                )
            };

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
                        hair,
                    },
                );
            }

            let ok = SStartGame { result: 4, resolution: 1024 };
            if let Ok(raw) = ok.encode() {
                out.push(Self::encode_raw(raw));
            }

            // After sending StartGame (which switches the client into the
            // GameScene), mirror the C# PlayerObject.StartGameSuccess
            // behaviour for server notices: if Envir/Notice.txt exists and its
            // last modification time is more recent than the character's last
            // logout, send an UpdateNotice packet so the client shows the
            // welcome/notice dialog on first login after a change.
            if let Some(ref account_id) = self.account_id {
                let last_logout_unix_ms = self
                    .store
                    .list_characters(account_id)
                    .ok()
                    .and_then(|chars: Vec<CharacterSummary>| {
                        chars
                            .into_iter()
                            .find(|cs| cs.index == ch.index)
                            .map(|cs| cs.last_access_binary)
                    })
                    .unwrap_or(0);

                if let Some((notice, notice_last_update_ms)) = Self::load_notice_from_file() {
                    if notice_last_update_ms > last_logout_unix_ms {
                        let pkt = SUpdateNotice { notice };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
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

            {
                let world = self.world.lock().unwrap();
                for recipe in &self.world_db.recipe_infos {
                    if let Some(uid) =
                        world.get_runtime_recipe_uid_by_item_index(recipe.item_index)
                    {
                        if let Some(runtime) = world.get_runtime_recipe_by_uid(uid) {
                            let pkt = SNewRecipeInfo {
                                recipe_bytes: runtime.client_bytes.clone(),
                            };
                            if let Ok(raw) = pkt.encode() {
                                out.push(Self::encode_raw(raw));
                            }
                        }
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

            let stored_friends: Vec<StoredFriend> = if let Some(ref account_id) = self.account_id {
                self
                    .store
                    .load_character_friends(account_id, ch.index)
                    .unwrap_or_else(|e| {
                        tracing::debug!(
                            "load_character_friends failed for account_id={} idx={} err={:?}",
                            account_id,
                            ch.index,
                            e,
                        );
                        Vec::new()
                    })
            } else {
                Vec::new()
            };

            let mut account_storage = if let Some(ref account_id) = self.account_id {
                match self.store.load_account_storage(account_id) {
                    Ok(Some(s)) => s,
                    Ok(None) => {
                        let s = AccountStorage {
                            slots: vec![None; 80],
                            has_expanded_storage: false,
                            expanded_storage_expiry_binary: 0,
                        };
                        let _ = self.store.save_account_storage(account_id, &s);
                        s
                    }
                    Err(_) => AccountStorage {
                        slots: vec![None; 80],
                        has_expanded_storage: false,
                        expanded_storage_expiry_binary: 0,
                    },
                }
            } else {
                AccountStorage {
                    slots: vec![None; 80],
                    has_expanded_storage: false,
                    expanded_storage_expiry_binary: 0,
                }
            };

            // If expanded storage has expired while the account was offline,
            // clear the flag before sending UserInformation/ResizeStorage so
            // the client does not see stale expanded pages. This mirrors the
            // C# PlayerObject.Process expiry behaviour at login time.
            if account_storage.has_expanded_storage
                && account_storage.expanded_storage_expiry_binary > 0
            {
                let expiry_ms = super::LoginConnection::dotnet_binary_to_unix_ms(
                    account_storage.expanded_storage_expiry_binary,
                );
                let now_ms = super::LoginConnection::now_millis();
                if expiry_ms > 0 && now_ms > expiry_ms {
                    account_storage.has_expanded_storage = false;
                    if let Some(ref account_id) = self.account_id {
                        let _ = self
                            .store
                            .save_account_storage(account_id, &account_storage);
                    }
                }
            }

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
                    tracing::debug!(
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
                        tracing::debug!(
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
                    tracing::debug!(
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

            // Mirror the C# CharacterInfo loader behaviour where UserMagic.CastTime
            // is reset on login (magic.CastTime = int.MinValue). Our cooldown
            // gate in World::check_and_update_magic_cooldown only treats
            // positive cast_time values as active cooldowns, so setting this to
            // zero ensures that all spells start without an artificial
            // cross-session cooldown window after a server restart.
            for um in &mut user_magics {
                um.cast_time = 0;
            }

            let mut magic_bytes = Vec::new();
            for um in &user_magics {
                if let Some(mi) = self.world_db.get_magic_info(um.spell) {
                    // Use the magic's current cast_time value as the `now` parameter so that
                    // the encoded CastTime offset is always zero on login. This avoids
                    // situations where a persisted absolute cast_time from a previous
                    // server session results in excessively long client-side cooldowns
                    // (e.g. "You cannot cast ThunderBolt for another 110 seconds.").
                    if let Ok(bytes) = encode_client_magic_bytes(mi, um, um.cast_time) {
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
                let events = world.handle_command(world::WorldCommand::StartGame {
                    session_id: self.session_id,
                    character_index: ch.index,
                    name: ch.name.clone(),
                    map_index: map_info_core.index,
                    x: spawn_x,
                    y: spawn_y,
                    direction: initial_direction,
                    job,
                    gender: ch.gender,
                    level: ch.level,
                    experience: experience_for_world,
                    magics: user_magics,
                });

                if !guild_name.is_empty() {
                    world.set_player_guild_name(self.session_id, &guild_name);
                }

                // Hydrate in-memory friend list from stored friends, mirroring
                // C# CharacterInfo.Friends / FriendInfo.CreateClientFriend.
                for f in &stored_friends {
                    let _ = world.add_friend_entry_for_player(
                        self.session_id,
                        f.friend_index,
                        &f.name,
                        f.blocked,
                    );
                    if !f.memo.is_empty() {
                        let _ = world.set_friend_memo_for_player(
                            self.session_id,
                            f.friend_index,
                            f.memo.clone(),
                        );
                    }
                }

                events
            };
            self.handle_world_events(events, out);

            if let Some(ref account_id) = self.account_id {
                if let Ok(Some((inv, eq))) =
                    self.store.load_character_items(account_id, ch.index)
                {
                    let mut world = self.world.lock().unwrap();
                    world.set_player_items(self.session_id, inv, eq);
                }
            }

            self.current_map_index = map_info_core.index;
            self.current_x = spawn_x;
            self.current_y = spawn_y;
            self.direction = initial_direction;

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
                real_id: ch.index as u32,
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
                hair,
                hp: stats.hp,
                mp: stats.mp,
                experience: stats.experience,
                max_experience,
                level_effects: 0,
                has_hero: true,
                hero_behaviour: 0,
                gold: stats.gold as u32,
                credit: stats.credit as u32,
                has_expanded_storage: account_storage.has_expanded_storage,
                expanded_storage_expiry_binary: account_storage.expanded_storage_expiry_binary,
                magics: magic_bytes.clone(),
                summoned_creature_type: 0,
                creature_summoned: false,
                allow_observe,
                observer: false,
            };
            if let Ok(raw) = user.encode() {
                out.push(Self::encode_raw(raw));
            }

            // Minimal hero bootstrap: send HeroInformation so the client can
            // construct GameScene.Hero (UserHeroObject) and open hero UI.
            {
                let hero_id = super::hero_object_id(self.session_id);
                let mut core_bytes = Vec::new();
                if write_u32_le(&mut core_bytes, hero_id).is_ok()
                    && write_string(&mut core_bytes, "Hero").is_ok()
                {
                    core_bytes.push(ch.class);
                    core_bytes.push(ch.gender);
                    let _ = write_u16_le(&mut core_bytes, ch.level);
                    core_bytes.push(hair);

                    let _ = write_i32_le(&mut core_bytes, stats.hp);
                    let _ = write_i32_le(&mut core_bytes, stats.mp);

                    let _ = write_i64_le(&mut core_bytes, stats.experience);
                    let _ = write_i64_le(&mut core_bytes, max_experience);

                    // Inventory (fixed 46 slots)
                    let _ = write_bool(&mut core_bytes, true);
                    let _ = write_i32_le(&mut core_bytes, 46);
                    for _ in 0..46 {
                        let _ = write_bool(&mut core_bytes, false);
                    }

                    // Equipment (fixed 14 slots)
                    let _ = write_bool(&mut core_bytes, true);
                    let _ = write_i32_le(&mut core_bytes, 14);
                    for _ in 0..14 {
                        let _ = write_bool(&mut core_bytes, false);
                    }

                    // Magics: count + raw ClientMagic bytes
                    let _ = write_i32_le(&mut core_bytes, magic_bytes.len() as i32);
                    for m in &magic_bytes {
                        core_bytes.extend_from_slice(m);
                    }

                    let hero_pkt = SHeroInformation {
                        core_bytes,
                        auto_pot: false,
                        auto_hp_percent: 0,
                        auto_mp_percent: 0,
                        hp_item_index: -1,
                        mp_item_index: -1,
                    };
                    if let Ok(raw) = hero_pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }

                    let hero_base_stats_bytes = base_stats::encode_base_stats_for_job(job);
                    let hero_base_stats_pkt = SHeroBaseStatsInfo {
                        stats_bytes: hero_base_stats_bytes,
                    };
                    out.push(Self::encode_raw(hero_base_stats_pkt.encode()));

                    let hero_hc_pkt = SHeroHealthChanged {
                        hp: stats.hp,
                        mp: stats.mp,
                    };
                    if let Ok(raw) = hero_hc_pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }

                    // Tell the client that the hero is spawned so hero panels
                    // become visible.
                    let state_pkt = SUpdateHeroSpawnState { state: 2 };
                    out.push(Self::encode_raw(state_pkt.encode()));
                }
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

            let base = SObjectTurnWalkRun {
                object_id: self.session_id,
                location_x: self.current_x,
                location_y: self.current_y,
                direction: self.direction,
            };
            let pkt = SObjectTurn(base);
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }

            let hc_pkt = SHealthChanged {
                hp: stats.hp,
                mp: stats.mp,
            };
            if let Ok(raw) = hc_pkt.encode() {
                tracing::debug!(
                    "start_game: send SHealthChanged -> session_id={} hp={} mp={}",
                    self.session_id,
                    stats.hp,
                    stats.mp,
                );
                out.push(Self::encode_raw(raw));
            }

            let base_stats_bytes = base_stats::encode_base_stats_for_job(job);
            let base_stats_pkt = SBaseStatsInfo {
                stats_bytes: base_stats_bytes,
            };
            out.push(Self::encode_raw(base_stats_pkt.encode()));

            self.send_safezone_border_spells(map_info_core.index, out);
            self.known_monsters.clear();
            self.known_npcs.clear();
            self.known_players.clear();
            self.known_heroes.clear();
            self.update_visibility(out);

            let initial_hp_percent: u8 = if max_hp > 0 {
                ((stats.hp as i64 * 100 / max_hp as i64).clamp(0, 100)) as u8
            } else {
                0
            };
            let self_hp_pkt = SObjectHealth {
                object_id: self.session_id,
                percent: initial_hp_percent,
                expire: 5,
            };
            if let Ok(raw) = self_hp_pkt.encode() {
                tracing::debug!(
                    "start_game: send initial SObjectHealth -> session_id={} object_id={} percent={} expire={}",
                    self.session_id,
                    self.session_id,
                    initial_hp_percent,
                    5,
                );
                out.push(Self::encode_raw(raw));
            }

            // Explicitly clear any stale Hidden state for this session's
            // player object so that the client renders them as visible on
            // login, even if previous buffs or reconnections left the
            // server-side flag set. The legacy C# server assumes Hidden is
            // false on spawn; sending an explicit ObjectHidden(false) here is
            // a safe approximation.
            let unhide_pkt = SObjectHidden {
                object_id: self.session_id,
                hidden: false,
            };
            if let Ok(raw) = unhide_pkt.encode() {
                tracing::debug!(
                    "start_game: send SObjectHidden(false) -> session_id={} object_id={}",
                    self.session_id,
                    self.session_id,
                );
                out.push(Self::encode_raw(raw));
            }

            // Mirror the C# StartGameSuccess sequence: after BaseStatsInfo,
            // send TimeOfDay, ChangeAMode, ChangePMode and SwitchGroup so the
            // client's UI (light level, attack mode, pet mode, group toggle)
            // matches the server-side defaults.
            let tod_pkt = STimeOfDay {
                lights: map_info_core.light,
            };
            let tod_raw = tod_pkt.encode();
            out.push(Self::encode_raw(tod_raw));

            // Initialise the client-side attack mode UI to match the
            // server-side default (Peace/0), mirroring the C#
            // PlayerObject.StartGame behaviour which enqueues
            // S.ChangeAMode with the current AMode (persisted in
            // CharacterInfo.AMode). We currently default to Peace/0.
            let amode_pkt = SChangeAMode { mode: 0 };
            let amode_raw = amode_pkt.encode();
            out.push(Self::encode_raw(amode_raw));

            // PetMode defaults to Both/0 for now. The legacy C# server
            // persists PMode per-character; once that is mirrored in the
            // Rust world state we can load and send the stored value here.
            let pmode_pkt = SChangePMode { mode: 0 };
            let pmode_raw = pmode_pkt.encode();
            out.push(Self::encode_raw(pmode_raw));

            // Group toggle: default to allowing group invites, matching the
            // initial AllowGroup behaviour on the C# server. The dedicated
            // group connection handler will keep this in sync when the
            // player toggles the option.
            let switch_group_pkt = SSwitchGroup { allow_group: true };
            if let Ok(raw) = switch_group_pkt.encode() {
                out.push(Self::encode_raw(raw));
            }

            let default_npc = SDefaultNpc {
                object_id: super::LoginConnection::DEFAULT_NPC_ID,
            };
            if let Ok(raw) = default_npc.encode() {
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

            // After the player has spawned and inventory/equipment have been
            // synchronised, mirror the C# StartGameSuccess quest sync by
            // sending the completed quest list followed by all active quest
            // progresses as ChangeQuest(Add) packets.
            {
                let (now_ms, active_quests, completed_quests) = {
                    let world = self.world.lock().unwrap();
                    let now_ms = world.current_time_ms();
                    let quests = world.quests_for_player(self.session_id);
                    let completed = world.completed_quests_for_player(self.session_id);
                    (now_ms, quests, completed)
                };

                if !completed_quests.is_empty() {
                    let pkt = SCompleteQuest {
                        completed_quests,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }

                const QUEST_STATE_ADD: u8 = 0;

                for progress in active_quests {
                    let quest_bytes = match progress.to_client_progress_bytes(now_ms) {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            tracing::debug!(
                                "start_game: failed to encode quest progress for session_id={} quest_id=? err={:?}",
                                self.session_id,
                                e,
                            );
                            continue;
                        }
                    };

                    let pkt = SChangeQuest {
                        quest_bytes,
                        quest_state: QUEST_STATE_ADD,
                        track_quest: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }

            let mut storage_bytes = Vec::new();
            let has_storage_array = !account_storage.slots.is_empty();
            let _ = write_bool(&mut storage_bytes, has_storage_array);
            if has_storage_array {
                let _ = write_i32_le(&mut storage_bytes, account_storage.slots.len() as i32);
                for slot in &account_storage.slots {
                    let _ = write_bool(&mut storage_bytes, slot.is_some());
                    if let Some(item) = slot {
                        if let Ok(bytes) = item.encode_to_bytes() {
                            storage_bytes.extend_from_slice(&bytes);
                        }
                    }
                }
            }

            let storage_pkt = SUserStorage {
                storage_bytes,
            };
            if let Ok(raw) = storage_pkt.encode() {
                out.push(Self::encode_raw(raw));
            }

            let resize_pkt = SResizeStorage {
                size: account_storage.slots.len() as i32,
                has_expanded_storage: account_storage.has_expanded_storage,
                expiry_time_binary: account_storage.expanded_storage_expiry_binary,
            };
            if let Ok(raw) = resize_pkt.encode() {
                out.push(Self::encode_raw(raw));
            }

            let toggles = {
                let world = self.world.lock().unwrap();
                world.player_spell_toggles(self.session_id)
            };
            for spell_id in toggles {
                let pkt = SSpellToggle {
                    object_id: self.session_id,
                    spell: spell_id,
                    can_use: true,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }

            // Send the GameShop list to the client, mirroring the C#
            // PlayerObject.GetGameShop behaviour that enqueues a
            // GameShopInfo packet for each GameShopItem when a player
            // enters the game. For now we do not track purchases, so the
            // stock level is taken directly from the mir.db GameShopItem
            // record.
            self.send_full_gameshop(out);

            // Send any stored mail for this character, approximating the
            // C# behaviour where pending mail is delivered on login via
            // a ReceiveMail packet containing ClientMail blobs.
            self.send_full_mailbox(ch.index, out);
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

    fn load_notice_from_file() -> Option<(NoticeData, i64)> {
        let path_deploy = Path::new("./deploy/Envir/Notice.txt");
        let path_plain = Path::new("./Envir/Notice.txt");
        let path = if path_deploy.exists() { path_deploy } else { path_plain };
        if !path.exists() {
            return None;
        }

        let metadata = match fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                tracing::debug!("load_notice_from_file: failed to stat {:?}: {:?}", path, e);
                return None;
            }
        };

        let modified: SystemTime = match metadata.modified() {
            Ok(t) => t,
            Err(e) => {
                tracing::debug!(
                    "load_notice_from_file: failed to get modified time for {:?}: {:?}",
                    path,
                    e,
                );
                return None;
            }
        };

        let notice_last_update_ms = match modified.duration_since(UNIX_EPOCH) {
            Ok(dur) => dur.as_millis() as i64,
            Err(_) => 0,
        };

        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                tracing::debug!("load_notice_from_file: failed to read {:?}: {:?}", path, e);
                return None;
            }
        };

        let mut title = String::new();
        let mut message_lines: Vec<String> = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();
            let upper = trimmed.to_ascii_uppercase();
            if upper.starts_with("TITLE") && trimmed.contains('=') {
                if let Some((_, value)) = trimmed.split_once('=') {
                    title = value.to_string();
                    continue;
                }
            }

            message_lines.push(line.to_string());
        }

        if title.is_empty() && message_lines.is_empty() {
            return None;
        }

        let mut message = message_lines.join("\r\n");
        if !message.is_empty() {
            message.push_str("\r\n");
        }

        let notice = NoticeData { title, message };

        Some((notice, notice_last_update_ms))
    }

    fn encode_game_shop_item_bytes(
        &self,
        rec: &world::map::GameShopItemRecord,
        stock_level: i32,
    ) -> Option<Vec<u8>> {
        let info = self.world_db.get_item_info(rec.item_index)?;

        let mut buf = Vec::new();

        if write_i32_le(&mut buf, rec.item_index).is_err() {
            return None;
        }
        if write_i32_le(&mut buf, rec.g_index).is_err() {
            return None;
        }

        if info.encode(&mut buf).is_err() {
            return None;
        }

        if write_u32_le(&mut buf, rec.gold_price).is_err() {
            return None;
        }
        if write_u32_le(&mut buf, rec.credit_price).is_err() {
            return None;
        }
        if write_u16_le(&mut buf, rec.count).is_err() {
            return None;
        }

        if write_string(&mut buf, &rec.class).is_err() {
            return None;
        }
        if write_string(&mut buf, &rec.category).is_err() {
            return None;
        }

        if write_i32_le(&mut buf, rec.stock).is_err() {
            return None;
        }
        if write_bool(&mut buf, rec.i_stock).is_err() {
            return None;
        }
        if write_bool(&mut buf, rec.deal).is_err() {
            return None;
        }
        if write_bool(&mut buf, rec.top_item).is_err() {
            return None;
        }

        if write_i64_le(&mut buf, rec.date_binary).is_err() {
            return None;
        }
        if write_bool(&mut buf, rec.can_buy_credit).is_err() {
            return None;
        }
        if write_bool(&mut buf, rec.can_buy_gold).is_err() {
            return None;
        }

        if write_i32_le(&mut buf, stock_level).is_err() {
            return None;
        }

        Some(buf)
    }

    /// Send the full GameShop item list to the client, approximating the
    /// C# PlayerObject.GetGameShop method. Each GameShopItemRecord is
    /// converted into GameShopItem.Save(writer, true) bytes followed by
    /// StockLevel and wrapped in an SGameShopInfo packet.
    pub(crate) fn send_full_gameshop(&self, out: &mut Vec<Vec<u8>>) {
        for rec in &self.world_db.game_shop_items {
            let stock_level = if rec.stock != 0 {
                // Mirror the C# logic:
                //  * For individual stock (iStock == true), subtract the
                //    per-player GSpurchases from Stock.
                //  * For server stock (iStock == false), subtract the
                //    global GameshopLog value from Stock.
                let (per_player, global) = {
                    let world = self.world.lock().unwrap();
                    let per_player = world.gameshop_purchased_for_player(self.session_id, rec.g_index);
                    let global = world.gameshop_purchased_global(rec.g_index);
                    (per_player, global)
                };

                let purchased = if rec.i_stock { per_player } else { global };
                let remaining = rec.stock - purchased;
                if remaining < 0 {
                    continue;
                }
                remaining
            } else {
                rec.stock
            };

            if let Some(info_bytes) = self.encode_game_shop_item_bytes(rec, stock_level) {
                let pkt = SGameShopInfo { info_bytes };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }

    /// Load all stored mail for the given character and, if any exists,
    /// encode it into a ReceiveMail payload using the same layout as
    /// ClientMail.Save(writer) in the C# client, then send it via
    /// SReceiveMail.
    pub(crate) fn send_full_mailbox(&self, char_index: i32, out: &mut Vec<Vec<u8>>) {
        let account_id = match &self.account_id {
            Some(id) => id,
            None => return,
        };

        let mails: Vec<StoredMail> = match self.store.load_character_mail(account_id, char_index) {
            Ok(m) => m,
            Err(e) => {
                tracing::debug!("send_full_mailbox: failed to load mail for account_id={} idx={} err={:?}", account_id, char_index, e);
                return;
            }
        };

        if mails.is_empty() {
            return;
        }

        let mut buf = Vec::new();

        if write_i32_le(&mut buf, mails.len() as i32).is_err() {
            return;
        }

        for mail in &mails {
            if write_u64_le(&mut buf, mail.mail_id).is_err() {
                return;
            }
            if write_string(&mut buf, &mail.sender).is_err() {
                return;
            }
            if write_string(&mut buf, &mail.message).is_err() {
                return;
            }
            if write_bool(&mut buf, mail.opened).is_err() {
                return;
            }
            if write_bool(&mut buf, mail.locked).is_err() {
                return;
            }
            if write_bool(&mut buf, mail.can_reply).is_err() {
                return;
            }
            if write_bool(&mut buf, mail.collected).is_err() {
                return;
            }

            if write_i64_le(&mut buf, mail.date_sent_binary).is_err() {
                return;
            }

            if write_u32_le(&mut buf, mail.gold).is_err() {
                return;
            }

            let item_count: i32 = match mail.items.len().try_into() {
                Ok(v) => v,
                Err(_) => return,
            };
            if write_i32_le(&mut buf, item_count).is_err() {
                return;
            }

            for item_bytes in &mail.items {
                buf.extend_from_slice(item_bytes);
            }
        }

        let pkt = SReceiveMail { mail_bytes: buf };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
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
