use crystal_server_core::world;
use crystal_server_core::world::WorldProvider;
use crystal_server_core::world::configs::setup_config::setup_config;
use rand::{thread_rng, Rng};
use crystal_shared_proto::login::{
    CAttack,
    CChangeTrade,
    CPickUp,
    CRangeAttack,
    CTownRevive,
    CTurn,
    CWalk,
    CRun,
    CMagicKey,
    CMagic,
    CChangeAMode,
    CChangePMode,
    CSpellToggle,
    CHarvest,
    CFishingCast,
    CFishingChangeAutocast,
};
use crystal_shared_proto::map::SMapChanged;
use crystal_shared_proto::magic::{
    SMagic,
    SMagicCast,
    SMagicDelay,
    SObjectEffect,
    SObjectHidden,
    SObjectMagic,
    SObjectRangeAttack,
    SObjectSpell,
};
use crystal_shared_proto::scene::{
    SObjectAttack,
    SObjectRun,
    SObjectStruck,
    SObjectTurn,
    SObjectTurnWalkRun,
    SObjectWalk,
    SPushed,
    SObjectPushed,
    SUserDash,
    SObjectDash,
    SUserDashFail,
    SObjectDashFail,
    SDamageIndicator,
    SObjectHealth,
    SGainExperience,
    SGainedGold,
    SGainedItem,
    SObjectDied,
    SObjectItem,
    SObjectGold,
    SObjectRemove,
    SRevived,
    SObjectRevived,
    SSpellToggle,
    SAddBuff,
    SRemoveBuff,
    SPauseBuff,
    SObjectShow,
    SObjectHide,
    SLevelChanged,
    SObjectLeveled,
    SInTrapRock,
};
use crystal_shared_proto::user::{
    SChangeAMode,
    SChangePMode,
    SUserLocation,
    SHealthChanged,
    SUserSlotsRefresh,
};
use crystal_shared_proto::user::group::{
    SRequestReincarnation,
    SCancelReincarnation,
};
use tracing::debug;

use super::{hero_object_id, LoginConnection, PendingMove, PendingMoveKind, Stage};

impl LoginConnection {
    fn enqueue_pending_move(&mut self, kind: PendingMoveKind, direction: u8) {
        let (now_ms, due_time_ms) = {
            let world = self.world.lock().unwrap();
            let now_ms = world.current_time_ms();
            let due = world
                .player_next_action_time_ms(self.session_id)
                .unwrap_or(now_ms);
            (now_ms, due)
        };

        let due = due_time_ms.max(now_ms);
        self.pending_moves.push_back(PendingMove {
            due_time_ms: due,
            kind,
            direction,
        });

        // Avoid unbounded growth if the client spams movement during cooldown.
        while self.pending_moves.len() > 10 {
            self.pending_moves.pop_front();
        }
    }

    fn is_action_blocked_now(&self) -> bool {
        let world = self.world.lock().unwrap();
        let now_ms = world.current_time_ms();
        if let Some(next) = world.player_next_action_time_ms(self.session_id) {
            next != 0 && now_ms < next
        } else {
            false
        }
    }

    pub(crate) fn enqueue_for_viewers(&self, map_index: i32, x: i32, y: i32, raw: Vec<u8>) {
        let viewers = {
            let world = self.world.lock().unwrap();
            world.sessions_in_range_for_map(map_index, x, y, Self::DATA_RANGE)
        };

        let mut outboxes = self.outboxes.lock().unwrap();
        for sid in viewers {
            if sid != self.session_id {
                outboxes.entry(sid).or_default().push(raw.clone());
            }
        }
    }

    pub(crate) fn handle_change_attack_mode(
        &mut self,
        msg: CChangeAMode,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let mode = msg.mode;
        let mut world = self.world.lock().unwrap();
        world.set_player_attack_mode(self.session_id, mode);
        drop(world);

        // Notify the client so its local AMode and attack-mode UI stay in
        // sync with the server-side value, mirroring the original C#
        // MirConnection.ChangeAMode behaviour.
        let pkt = SChangeAMode { mode };
        let raw = pkt.encode();
        out.push(Self::encode_raw(raw));
    }

    pub(crate) fn handle_change_pet_mode(
        &mut self,
        msg: CChangePMode,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let mode = msg.mode;
        let mut world = self.world.lock().unwrap();
        world.set_player_pet_mode(self.session_id, mode);
        drop(world);

        // Mirror C# MirConnection.ChangePMode: echo the new pet mode back to
        // the client so its local PMode and pet-mode UI stay in sync.
        let pkt = SChangePMode { mode };
        let raw = pkt.encode();
        out.push(Self::encode_raw(raw));
    }

    pub(crate) fn handle_turn(&mut self, msg: CTurn, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        if self.is_action_blocked_now() {
            self.enqueue_pending_move(PendingMoveKind::Turn, msg.direction);
            return;
        }

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::Turn {
                session_id: self.session_id,
                direction: msg.direction,
            })
        };

        let _ = self.handle_world_events(events, out);

        let base = SObjectTurnWalkRun {
            object_id: self.session_id,
            location_x: self.current_x,
            location_y: self.current_y,
            direction: self.direction,
        };
        let pkt = SObjectTurn(base);
        if let Ok(raw) = pkt.encode() {
            self.enqueue_for_viewers(
                self.current_map_index,
                self.current_x,
                self.current_y,
                Self::encode_raw(raw),
            );
        }

        if self.hero_spawn_state >= 2 {
            let hero_base = SObjectTurnWalkRun {
                object_id: hero_object_id(self.session_id),
                location_x: self.current_x,
                location_y: self.current_y,
                direction: self.direction,
            };
            let hero_pkt = SObjectTurn(hero_base);
            if let Ok(raw) = hero_pkt.encode() {
                self.enqueue_for_viewers(
                    self.current_map_index,
                    self.current_x,
                    self.current_y,
                    Self::encode_raw(raw),
                );
            }
        }
    }

    pub(crate) fn handle_walk(&mut self, msg: CWalk, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        if self.is_action_blocked_now() {
            self.enqueue_pending_move(PendingMoveKind::Walk, msg.direction);
            return;
        }

        let old_x = self.current_x;
        let old_y = self.current_y;

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::Walk {
                session_id: self.session_id,
                direction: msg.direction,
            })
        };

        let _ = self.handle_world_events(events, out);

        if self.current_x == old_x && self.current_y == old_y {
            return;
        }

        let base = SObjectTurnWalkRun {
            object_id: self.session_id,
            location_x: self.current_x,
            location_y: self.current_y,
            direction: self.direction,
        };
        let pkt = SObjectWalk(base);
        if let Ok(raw) = pkt.encode() {
            self.enqueue_for_viewers(
                self.current_map_index,
                self.current_x,
                self.current_y,
                Self::encode_raw(raw),
            );
        }

        if self.hero_spawn_state >= 2 {
            let hero_base = SObjectTurnWalkRun {
                object_id: hero_object_id(self.session_id),
                location_x: self.current_x,
                location_y: self.current_y,
                direction: self.direction,
            };
            let hero_pkt = SObjectWalk(hero_base);
            if let Ok(raw) = hero_pkt.encode() {
                self.enqueue_for_viewers(
                    self.current_map_index,
                    self.current_x,
                    self.current_y,
                    Self::encode_raw(raw),
                );
            }
        }
    }

    pub(crate) fn handle_run(&mut self, msg: CRun, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        if self.is_action_blocked_now() {
            self.enqueue_pending_move(PendingMoveKind::Run, msg.direction);
            return;
        }

        let old_x = self.current_x;
        let old_y = self.current_y;

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::Run {
                session_id: self.session_id,
                direction: msg.direction,
            })
        };

        let _ = self.handle_world_events(events, out);

        if self.current_x == old_x && self.current_y == old_y {
            return;
        }

        let base = SObjectTurnWalkRun {
            object_id: self.session_id,
            location_x: self.current_x,
            location_y: self.current_y,
            direction: self.direction,
        };
        let pkt = SObjectRun(base);
        if let Ok(raw) = pkt.encode() {
            self.enqueue_for_viewers(
                self.current_map_index,
                self.current_x,
                self.current_y,
                Self::encode_raw(raw),
            );
        }

        if self.hero_spawn_state >= 2 {
            let hero_base = SObjectTurnWalkRun {
                object_id: hero_object_id(self.session_id),
                location_x: self.current_x,
                location_y: self.current_y,
                direction: self.direction,
            };
            let hero_pkt = SObjectRun(hero_base);
            if let Ok(raw) = hero_pkt.encode() {
                self.enqueue_for_viewers(
                    self.current_map_index,
                    self.current_x,
                    self.current_y,
                    Self::encode_raw(raw),
                );
            }
        }
    }

    pub(crate) fn handle_magic_key(&mut self, msg: CMagicKey, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let mut world = self.world.lock().unwrap();
        let _ = world.set_magic_key_for_player(self.session_id, msg.spell, msg.key);
    }

    pub(crate) fn handle_world_events(
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
                    if session_id == self.session_id {
                        self.current_map_index = map_index;
                        self.current_x = x;
                        self.current_y = y;
                        self.direction = direction;

                        let pkt = SUserLocation {
                            location_x: x,
                            location_y: y,
                            direction,
                        };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        // Also broadcast updated party member locations so the
                        // big map view stays in sync with movement, mirroring
                        // C# HumanObject.GetPlayerLocation.
                        self.broadcast_group_locations_for_self(out);
                    }
                }
                world::WorldEvent::UserDash {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                } => {
                    debug!(
                        "world_event: UserDash -> local_sid={} event_sid={} map={} pos=({}, {}) dir={}",
                        self.session_id,
                        session_id,
                        map_index,
                        x,
                        y,
                        direction
                    );
                    if session_id == self.session_id {
                        self.current_map_index = map_index;
                        self.current_x = x;
                        self.current_y = y;
                        self.direction = direction;

                        let pkt = SUserDash {
                            location_x: x,
                            location_y: y,
                            direction,
                        };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }

                    let base = SObjectTurnWalkRun {
                        object_id: session_id,
                        location_x: x,
                        location_y: y,
                        direction,
                    };
                    let pkt = SObjectDash(base);
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);
                        self.enqueue_for_viewers(map_index, x, y, bytes);
                    }
                }
                world::WorldEvent::UserDashFail {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                } => {
                    debug!(
                        "world_event: UserDashFail -> local_sid={} event_sid={} map={} pos=({}, {}) dir={}",
                        self.session_id,
                        session_id,
                        map_index,
                        x,
                        y,
                        direction
                    );
                    if session_id == self.session_id {
                        self.current_map_index = map_index;
                        self.current_x = x;
                        self.current_y = y;
                        self.direction = direction;

                        let pkt = SUserDashFail {
                            location_x: x,
                            location_y: y,
                            direction,
                        };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }

                    let base = SObjectTurnWalkRun {
                        object_id: session_id,
                        location_x: x,
                        location_y: y,
                        direction,
                    };
                    let pkt = SObjectDashFail(base);
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);
                        self.enqueue_for_viewers(map_index, x, y, bytes);
                    }
                }
                world::WorldEvent::PlayerPushed {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                } => {
                    debug!(
                        "world_event: PlayerPushed -> local_sid={} pushed_sid={} map={} pos=({}, {}) dir={}",
                        self.session_id,
                        session_id,
                        map_index,
                        x,
                        y,
                        direction
                    );
                    if session_id == self.session_id {
                        self.current_map_index = map_index;
                        self.current_x = x;
                        self.current_y = y;
                        self.direction = direction;

                        let pkt = SPushed {
                            location_x: x,
                            location_y: y,
                            direction,
                        };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }

                    let base = SObjectTurnWalkRun {
                        object_id: session_id,
                        location_x: x,
                        location_y: y,
                        direction,
                    };
                    let pkt = SObjectPushed(base);
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);
                        self.enqueue_for_viewers(map_index, x, y, bytes);
                    }
                }
                world::WorldEvent::ObjectPushed {
                    object_id,
                    map_index,
                    x,
                    y,
                    direction,
                } => {
                    debug!(
                        "world_event: ObjectPushed -> local_sid={} object_id={} map={} pos=({}, {}) dir={}",
                        self.session_id,
                        object_id,
                        map_index,
                        x,
                        y,
                        direction
                    );
                    let base = SObjectTurnWalkRun {
                        object_id: object_id as u32,
                        location_x: x,
                        location_y: y,
                        direction,
                    };
                    let pkt = SObjectPushed(base);
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);
                        self.enqueue_for_viewers(map_index, x, y, bytes);
                    }
                }
                world::WorldEvent::InTrapRock { session_id, trapped } => {
                    if session_id != self.session_id {
                        continue;
                    }
                    let pkt = SInTrapRock { trapped };
                    if let Ok(raw) = pkt.encode() {
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
                    let old_map_index = self.current_map_index;
                    if session_id == self.session_id {
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

                        // After changing maps, refresh party member map names
                        // and locations for all members, mirroring C#
                        // GroupMemberMapNameChanged + GetPlayerLocation.
                        self.broadcast_group_maps_for_self(out);
                        self.broadcast_group_locations_for_self(out);

                        // Emit decorative SafeZone border spells (TrapHexagon)
                        // only when changing to a different map. This avoids
                        // duplicate ObjectSpell bursts when a MapChanged event
                        // is triggered for a reposition within the same map
                        // (e.g. login/start-game positioning).
                        if old_map_index != map_index {
                            self.send_safezone_border_spells(map_index, out);
                        }
                    }
                }
                world::WorldEvent::TeleportToBindRequested {
                    session_id,
                    spell_id,
                    level,
                } => {
                    if session_id != self.session_id {
                        continue;
                    }

                    // Determine the bind location for this character. If no
                    // bind is stored yet, fall back to the current
                    // map/position, mirroring the behaviour used by town
                    // teleport scrolls and TownRevive.
                    let (bind_map, bind_x, bind_y, _bind_dir) = if let (
                        Some(ref account_id),
                        Some(char_idx),
                    ) = (self.account_id.as_ref(), self.current_char_index)
                    {
                        if let Ok(Some(pos)) =
                            self.store.load_character_bind(account_id, char_idx)
                        {
                            (pos.map_index, pos.x, pos.y, pos.direction)
                        } else {
                            (
                                self.current_map_index,
                                self.current_x,
                                self.current_y,
                                self.direction,
                            )
                        }
                    } else {
                        (
                            self.current_map_index,
                            self.current_x,
                            self.current_y,
                            self.direction,
                        )
                    };

                    // Implement the C# PlayerObject.MagicTeleport algorithm:
                    // sample up to 200 random points around BindLocation
                    // within a rectangle sized by map_width/(level+1) and
                    // map_height/(level+1), stopping at the first walkable
                    // destination.
                    let mut dest_opt: Option<(i32, i32)> = None;

                    if let Some(info) = self.world_db.get_map_info(bind_map).cloned() {
                        let dir = &self.world_config.map_path;
                        if let Ok(map) =
                            crystal_server_core::world::map::load_map_from_file(
                                info,
                                dir.as_path(),
                            )
                        {
                            let width: i32 = map.width as i32;
                            let height: i32 = map.height as i32;

                            let denom: i32 = (level as i32).saturating_add(1).max(1);
                            let map_size_x: i32 = (width / denom).max(1);
                            let map_size_y: i32 = (height / denom).max(1);

                            let mut rng = thread_rng();

                            for _ in 0..200 {
                                let dx = rng.gen_range(-map_size_x..map_size_x);
                                let dy = rng.gen_range(-map_size_y..map_size_y);

                                let x = bind_x.saturating_add(dx);
                                let y = bind_y.saturating_add(dy);

                                if x < 0 || y < 0 {
                                    continue;
                                }

                                let ux = x as u16;
                                let uy = y as u16;
                                if ux >= map.width || uy >= map.height {
                                    continue;
                                }

                                if !map.is_walkable(ux, uy) {
                                    continue;
                                }

                                dest_opt = Some((x, y));
                                break;
                            }
                        }
                    }

                    if let Some((tx, ty)) = dest_opt {
                        let events2 = {
                            let mut world = self.world.lock().unwrap();
                            let mut events2 = world.handle_command(world::WorldCommand::Teleport {
                                session_id: self.session_id,
                                map_index: bind_map,
                                x: tx,
                                y: ty,
                            });

                            world.apply_teleport_skill_outcome(
                                self.session_id,
                                spell_id,
                                &mut events2,
                            );

                            events2
                        };

                        let map_changed2 = self.handle_world_events(events2, out);
                        if map_changed2 {
                            self.known_monsters.clear();
                            self.known_npcs.clear();
                            self.known_players.clear();
                            self.known_heroes.clear();
                            self.update_visibility(out);
                        }
                    }
                }
                world::WorldEvent::ObjectLocation { .. } => {}
                world::WorldEvent::Poisoned { .. } => {}
                world::WorldEvent::ObjectPoisoned { .. } => {}
                world::WorldEvent::GainExperience {
                    session_id,
                    amount,
                } => {
                    if session_id == self.session_id {
                        let pkt = SGainExperience { amount };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        // Persist updated experience to the character stats so that
                        // gains survive logout and character switching, mirroring the
                        // C# PlayerObject.GainExp behaviour where CharacterInfo.Experience
                        // is updated whenever experience is gained.
                        if let Some(ref mut stats) = self.current_stats {
                            let add = amount as i64;
                            let new_exp = stats.experience.saturating_add(add);
                            stats.experience = new_exp;

                            if let (Some(ref account_id), Some(char_idx)) =
                                (self.account_id.as_ref(), self.current_char_index)
                            {
                                let _ = self
                                    .store
                                    .save_character_stats(account_id, char_idx, stats);
                            }
                        }

                        // Mirror C# PlayerObject.GainExp guild experience
                        // behaviour: whenever a player gains experience and
                        // is in a non-newbie guild, award guild experience as
                        // well. The effective guild gain (after
                        // GuildSettings.ExpRate) is computed by
                        // World::guild_gain_exp, and we broadcast
                        // SGuildExpGain so all online members see the
                        // progress.
                        let guild_name = {
                            let map = self.player_summaries.lock().unwrap();
                            map.get(&self.session_id)
                                .map(|v| v.guild_name.clone())
                                .unwrap_or_default()
                        };

                        if !guild_name.is_empty() {
                            let newbie_name = &setup_config().game.newbie_guild;
                            if !guild_name.eq_ignore_ascii_case(newbie_name) {
                                let outcome_opt = {
                                    let mut world = self.world.lock().unwrap();
                                    world.guild_gain_exp(&guild_name, amount)
                                };

                                if let Some(outcome) = outcome_opt {
                                    let _ = self.store.save_guild(&outcome.guild);
                                    self.broadcast_guild_exp_gain(&guild_name, outcome.exp_gained);
                                    if outcome.leveled {
                                        self.broadcast_guild_status_for_guild(&outcome.guild);
                                    }
                                }
                            }
                        }
                    }
                }
                world::WorldEvent::PlayerLevelChanged {
                    session_id,
                    level,
                    experience,
                    max_experience,
                } => {
                    if session_id == self.session_id {
                        // Sync in-memory stats with the new level-band experience so that
                        // subsequent logouts persist the correct value even when the level
                        // changes due to normal gameplay or GM commands.
                        if let Some(ref mut stats) = self.current_stats {
                            stats.experience = experience;

                            if let (Some(ref account_id), Some(char_idx)) =
                                (self.account_id.as_ref(), self.current_char_index)
                            {
                                let _ = self
                                    .store
                                    .save_character_stats(account_id, char_idx, stats);
                            }
                        }

                        // Notify the owner about their new level/experience
                        // band using SLevelChanged, matching the behaviour
                        // in the GM /level command.
                        let lvl_pkt = SLevelChanged {
                            level,
                            experience,
                            max_experience,
                        };
                        if let Ok(raw) = lvl_pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }

                        // Broadcast ObjectLeveled to self and nearby
                        // observers so clients can play level-up effects,
                        // mirroring C# PlayerObject.LevelUp.
                        let obj_pkt = SObjectLeveled {
                            object_id: self.session_id,
                        };
                        if let Ok(pkt) = obj_pkt.encode() {
                            let raw = Self::encode_raw(pkt);
                            out.push(raw.clone());
                            self.enqueue_for_viewers(
                                self.current_map_index,
                                self.current_x,
                                self.current_y,
                                raw,
                            );
                        }

                        // Call default NPC LevelUp page after level up
                        // C#: CallDefaultNPC(DefaultNPCType.LevelUp)
                        // C# generates key as: "LevelUp" (no parameters)
                        // Then wraps it as: string.Format("[@_{0}]", key) -> "[@_LevelUp]"
                        use crystal_shared_proto::npc::CCallNPC;
                        let level_up_key = "LevelUp".to_string();
                        let call_npc_msg = CCallNPC {
                            object_id: super::LoginConnection::DEFAULT_NPC_ID,
                            key: level_up_key,
                        };
                        // Call handle_call_npc directly (it's pub(crate) and part of LoginConnection impl)
                        self.handle_call_npc(call_npc_msg, out);
                    }
                }
                world::WorldEvent::ObjectAttack {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                    spell,
                    level,
                    attack_type,
                } => {
                    let pkt = SObjectAttack {
                        object_id: session_id,
                        location_x: x,
                        location_y: y,
                        direction,
                        spell,
                        level,
                        attack_type,
                    };
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);
                        self.enqueue_for_viewers(map_index, x, y, bytes);
                    }
                }
                world::WorldEvent::ObjectRangeAttack {
                    object_id,
                    map_index,
                    x,
                    y,
                    direction,
                    target_id,
                    target_x,
                    target_y,
                    spell,
                    level,
                    attack_type,
                } => {
                    let pkt = SObjectRangeAttack {
                        object_id: object_id as u32,
                        location_x: x,
                        location_y: y,
                        direction,
                        target_id: target_id as u32,
                        target_x,
                        target_y,
                        attack_type,
                        spell,
                        level,
                    };
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);
                        self.enqueue_for_viewers(map_index, x, y, bytes);
                    }
                }
                world::WorldEvent::ObjectMagic {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                    spell,
                    level,
                    target_id,
                    target_x,
                    target_y,
                } => {
                    let pkt = SObjectMagic {
                        object_id: session_id,
                        location_x: x,
                        location_y: y,
                        direction,
                        spell,
                        target_id,
                        target_x,
                        target_y,
                        cast: true,
                        level,
                        // Mirror the C# behaviour for player-cast spells
                        // where the local client is responsible for playing
                        // its own spell animation. When SelfBroadcast is
                        // false, the C# client ignores S.ObjectMagic for the
                        // UserObject but still processes it for other
                        // viewers. This prevents the caster from seeing a
                        // duplicated spell-cast animation while nearby
                        // players still see the effect.
                        self_broadcast: false,
                        secondary_target_ids: Vec::new(),
                    };
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);

                        // Only broadcast to other sessions in range; the
                        // caster already enqueues a local MirAction.Spell via
                        // input handling, so sending S.ObjectMagic back to
                        // them would cause a second, duplicate animation.
                        self.enqueue_for_viewers(map_index, x, y, bytes);
                    }
                }
                world::WorldEvent::ObjectStruck {
                    attacker_id,
                    target_id,
                    map_index,
                    x,
                    y,
                    direction,
                    damage,
                    damage_type,
                    health_percent,
                    show_struck,
                } => {
                    // If this session caused the hit, broadcast visual effects to
                    // everyone in range except the attacker (who might get separate feedback).
                    // Wait, C# broadcasts to everyone.
                    if attacker_id == self.session_id {
                        let object_id = target_id as u32;

                        if show_struck {
                            let struck = SObjectStruck {
                                object_id,
                                attacker_id: attacker_id as u32,
                                location_x: x,
                                location_y: y,
                                direction,
                            };
                            if let Ok(pkt) = struck.encode() {
                                let raw = Self::encode_raw(pkt);
                                out.push(raw.clone());
                                self.enqueue_for_viewers(map_index, x, y, raw);
                            }
                        }

                        let dmg = SDamageIndicator {
                            damage,
                            damage_type,
                            object_id,
                        };
                        if let Ok(pkt) = dmg.encode() {
                            let raw = Self::encode_raw(pkt);
                            out.push(raw.clone());
                            self.enqueue_for_viewers(map_index, x, y, raw);
                        }

                        let health = SObjectHealth {
                            object_id,
                            percent: health_percent,
                            expire: 5,
                        };
                        if let Ok(pkt) = health.encode() {
                            let raw = Self::encode_raw(pkt);
                            tracing::debug!(
                                "move: send SObjectHealth(ObjectStruck/self-attack) -> sid={} object_id={} percent={} expire={} map={} pos=({}, {})",
                                self.session_id,
                                object_id,
                                health_percent,
                                5,
                                map_index,
                                x,
                                y,
                            );
                            out.push(raw.clone());
                            self.enqueue_for_viewers(map_index, x, y, raw);
                        }
                    }
                }
                world::WorldEvent::Struck {
                    attacker_session_id,
                    target_id,
                } => {
                    // C#: Enqueue(new S.Struck { AttackerID = attacker.ObjectID });
                    // This is sent only to the attacker to notify them of a successful hit
                    // Note: C# uses target's ObjectID as AttackerID in the packet (confusing naming)
                    if attacker_session_id == self.session_id {
                        use crystal_shared_proto::user::SStruck;
                        let struck = SStruck {
                            attacker_id: target_id as u32,
                        };
                        if let Ok(pkt) = struck.encode() {
                            out.push(Self::encode_raw(pkt));
                        }
                    }
                }
                world::WorldEvent::MonsterDied {
                    object_id,
                    map_index,
                    x,
                    y,
                    direction,
                } => {
                    let pkt = SObjectDied {
                        object_id: object_id as u32,
                        location_x: x,
                        location_y: y,
                        direction,
                        death_type: 0,
                    };
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);
                        out.push(bytes.clone());
                        self.enqueue_for_viewers(map_index, x, y, bytes);
                    }
                }
                world::WorldEvent::ObjectHidden {
                    object_id,
                    map_index,
                    x,
                    y,
                    hidden,
                } => {
                    let pkt = SObjectHidden {
                        object_id,
                        hidden,
                    };
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);

                        tracing::debug!(
                            "move: send SObjectHidden -> sid={} object_id={} hidden={} map={} pos=({}, {})",
                            self.session_id,
                            object_id,
                            hidden,
                            map_index,
                            x,
                            y,
                        );

                        if object_id == self.session_id {
                            out.push(bytes.clone());
                        }

                        self.enqueue_for_viewers(map_index, x, y, bytes);
                    }
                }
                world::WorldEvent::ObjectShow {
                    object_id,
                    map_index,
                    x,
                    y,
                } => {
                    debug!(
                        "movement: ObjectShow object_id={} map={} pos=({}, {})",
                        object_id,
                        map_index,
                        x,
                        y
                    );

                    let pkt = SObjectShow {
                        object_id: object_id as u32,
                    };
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);
                        out.push(bytes.clone());
                        self.enqueue_for_viewers(map_index, x, y, bytes);
                    }
                }
                world::WorldEvent::ObjectHide {
                    object_id,
                    map_index,
                    x,
                    y,
                } => {
                    debug!(
                        "movement: ObjectHide object_id={} map={} pos=({}, {})",
                        object_id,
                        map_index,
                        x,
                        y
                    );

                    let pkt = SObjectHide {
                        object_id: object_id as u32,
                    };
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);
                        out.push(bytes.clone());
                        self.enqueue_for_viewers(map_index, x, y, bytes);
                    }
                }
                world::WorldEvent::MonsterHitPlayer {
                    attacker_monster_id,
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                    damage,
                    damage_type,
                    health_percent,
                    show_struck,
                } => {
                    if session_id == self.session_id {
                        let object_id = self.session_id;
                        let attacker_id = attacker_monster_id as u32;

                        if show_struck {
                            let struck = SObjectStruck {
                                object_id,
                                attacker_id,
                                location_x: x,
                                location_y: y,
                                direction,
                            };
                            if let Ok(pkt) = struck.encode() {
                                let raw = Self::encode_raw(pkt);
                                out.push(raw.clone());
                                self.enqueue_for_viewers(map_index, x, y, raw);
                            }
                        }

                        let dmg = SDamageIndicator {
                            damage,
                            damage_type,
                            object_id,
                        };
                        if let Ok(pkt) = dmg.encode() {
                            let raw = Self::encode_raw(pkt);
                            out.push(raw.clone());
                            self.enqueue_for_viewers(map_index, x, y, raw);
                        }

                        let health = SObjectHealth {
                            object_id,
                            percent: health_percent,
                            expire: 5,
                        };
                        if let Ok(pkt) = health.encode() {
                            let raw = Self::encode_raw(pkt);
                            out.push(raw.clone());
                            self.enqueue_for_viewers(map_index, x, y, raw);
                        }
                    }
                }
                world::WorldEvent::ItemDropped {
                    object_id,
                    map_index,
                    x,
                    y,
                    item_index,
                    count,
                } => {
                    if let Some(info) = self.world_db.get_item_info(item_index) {
                        let base_name = info.friendly_name();
                        let name = if count > 1 {
                            format!("{} ({})", base_name, count)
                        } else {
                            base_name
                        };

                        let pkt = SObjectItem {
                            object_id: object_id as u32,
                            name,
                            name_colour_argb: Self::item_name_colour_for_grade(info.grade),
                            location_x: x,
                            location_y: y,
                            image: info.image,
                            grade: info.grade,
                        };
                        if let Ok(raw) = pkt.encode() {
                            let bytes = Self::encode_raw(raw);
                            out.push(bytes.clone());
                            self.enqueue_for_viewers(map_index, x, y, bytes);
                        }
                    }
                }
                world::WorldEvent::GoldDropped {
                    object_id,
                    map_index,
                    x,
                    y,
                    gold,
                } => {
                    let pkt = SObjectGold {
                        object_id: object_id as u32,
                        gold,
                        location_x: x,
                        location_y: y,
                    };
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);
                        out.push(bytes.clone());
                        self.enqueue_for_viewers(map_index, x, y, bytes);
                    }
                }
                world::WorldEvent::PlayerGainedItem { session_id, item } => {
                    if session_id == self.session_id {
                        if let Ok(pkt) = SGainedItem::from_user_item(&item) {
                            if let Ok(raw) = pkt.encode() {
                                out.push(Self::encode_raw(raw));
                            }
                        }
                    }
                }
                world::WorldEvent::Magic {
                    session_id,
                    spell_id,
                    target_id,
                    x,
                    y,
                    cast,
                    level,
                    secondary_target_ids,
                } => {
                    if session_id == self.session_id {
                        let pkt = SMagic {
                            spell: spell_id,
                            target_id,
                            target_x: x,
                            target_y: y,
                            cast,
                            level,
                            secondary_target_ids,
                        };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
                world::WorldEvent::MagicLeveled {
                    session_id,
                    spell_id,
                    level,
                    experience,
                } => {
                    if session_id == self.session_id {
                        self.send_magic_leveled(spell_id, level, experience, out);
                    }
                }
                world::WorldEvent::MagicDelay {
                    session_id,
                    spell_id,
                    delay,
                } => {
                    if session_id == self.session_id {
                        let pkt = SMagicDelay {
                            object_id: self.session_id,
                            spell: spell_id,
                            delay,
                        };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
                world::WorldEvent::MagicCast {
                    session_id,
                    spell_id,
                } => {
                    if session_id == self.session_id {
                        let pkt = SMagicCast { spell: spell_id };
                        let raw = pkt.encode();
                        out.push(Self::encode_raw(raw));
                    }
                }
                world::WorldEvent::PlayerGainedGold { session_id, amount } => {
                    if session_id == self.session_id {
                        if let Some(ref mut stats) = self.current_stats {
                            let add = amount as i64;
                            let new_gold = stats.gold.saturating_add(add);
                            stats.gold = new_gold;

                            if let (Some(ref account_id), Some(char_idx)) =
                                (self.account_id.as_ref(), self.current_char_index)
                            {
                                let _ = self
                                    .store
                                    .save_character_stats(account_id, char_idx, stats);
                            }
                        }

                        let pkt = SGainedGold { gold: amount };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
                world::WorldEvent::PlayerGainedCredit { session_id, amount } => {
                    if session_id == self.session_id {
                        if let Some(ref mut stats) = self.current_stats {
                            let add = amount as i64;
                            let new_credit = stats.credit.saturating_add(add);
                            stats.credit = new_credit;

                            if let (Some(ref account_id), Some(char_idx)) =
                                (self.account_id.as_ref(), self.current_char_index)
                            {
                                let _ = self
                                    .store
                                    .save_character_stats(account_id, char_idx, stats);
                            }
                        }

                        use crystal_shared_proto::user::status::SGainedCredit;
                        let pkt = SGainedCredit { credit: amount };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
                world::WorldEvent::ObjectEffect { session_id, effect } => {
                    // Generic object effect visual, used for SpellEffect-style
                    // animations such as StormEscape. We mirror the handling
                    // pattern used for Healing and MagicShieldUp/Down: send
                    // to the owner (if this connection matches) and broadcast
                    // to nearby viewers around the player's current
                    // location.
                    let eff_pkt = SObjectEffect {
                        object_id: session_id,
                        effect,
                        effect_type: 0,
                        delay_time: 0,
                        time: 0,
                    };

                    if let Ok(raw) = eff_pkt.encode() {
                        let bytes = Self::encode_raw(raw);

                        if session_id == self.session_id {
                            out.push(bytes.clone());
                        }

                        self.enqueue_for_viewers(
                            self.current_map_index,
                            self.current_x,
                            self.current_y,
                            bytes,
                        );
                    }
                }
                world::WorldEvent::MapItemRemoved {
                    object_id,
                    map_index,
                    x,
                    y,
                } => {
                    tracing::debug!(
                        "[pickup] MapItemRemoved dispatch: session={} object_id={} map={} pos=({}, {})",
                        self.session_id,
                        object_id,
                        map_index,
                        x,
                        y
                    );

                    let pkt = SObjectRemove {
                        object_id: object_id as u32,
                    };
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);
                        out.push(bytes.clone());
                        self.enqueue_for_viewers(map_index, x, y, bytes);
                    }
                }
                world::WorldEvent::MapSpellAdded {
                    map_index,
                    x,
                    y,
                    spell,
                    direction,
                    param,
                } => {
                    let object_id = Self::safezone_spell_object_id(map_index, x, y);
                    let pkt = SObjectSpell {
                        object_id,
                        location_x: x,
                        location_y: y,
                        spell,
                        direction,
                        param,
                    };
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);
                        out.push(bytes.clone());
                        self.enqueue_for_viewers(map_index, x, y, bytes);
                    }
                }
                world::WorldEvent::MapSpellRemoved {
                    map_index,
                    x,
                    y,
                    spell: _,
                } => {
                    let object_id = Self::safezone_spell_object_id(map_index, x, y);
                    let pkt = SObjectRemove { object_id };
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);
                        out.push(bytes.clone());
                        self.enqueue_for_viewers(map_index, x, y, bytes);
                    }
                }
                world::WorldEvent::PlayerHealed {
                    session_id,
                    map_index,
                    x,
                    y,
                    amount: _,
                    new_hp,
                    show_healing_effect,
                } => {
                    // Update HP/MP for the healed player on their own
                    // connection, mirroring the behaviour of the original
                    // server's SHealthChanged packet.
                    if session_id == self.session_id {
                        if let Some(ref mut stats) = self.current_stats {
                            stats.hp = new_hp;
                            let pkt = SHealthChanged {
                                hp: stats.hp,
                                mp: stats.mp,
                            };
                            if let Ok(raw) = pkt.encode() {
                                out.push(Self::encode_raw(raw));
                            }
                        }
                    }

                    // Broadcast an ObjectHealth update so that nearby
                    // clients can render the healed player's head HP bar,
                    // mirroring C# MapObject.BroadcastHealthChange for
                    // regen/healing.
                    let percent_opt = {
                        let world = self.world.lock().unwrap();
                        if let Some((max_hp, _)) = world.player_max_hp_mp(session_id) {
                            if max_hp > 0 {
                                let clamped = new_hp.max(0).min(max_hp);
                                let pct =
                                    ((clamped as i64 * 100 / max_hp as i64).clamp(0, 100)) as u8;
                                Some(pct)
                            } else {
                                Some(0)
                            }
                        } else {
                            None
                        }
                    };

                    if let Some(percent) = percent_opt {
                        let health = SObjectHealth {
                            object_id: session_id,
                            percent,
                            expire: 5,
                        };
                        if let Ok(pkt) = health.encode() {
                            let raw = Self::encode_raw(pkt);

                            // Also send to the current connection (typically
                            // the caster) so it can render the healed
                            // player's head HP bar update immediately.
                            if session_id != self.session_id {
                                out.push(raw.clone());
                            }

                            // Always send to the healed player when this
                            // connection corresponds to them.
                            if session_id == self.session_id {
                                out.push(raw.clone());
                            }

                            // And broadcast to other nearby viewers around
                            // the healed player's map/x/y.
                            self.enqueue_for_viewers(map_index, x, y, raw);
                        }
                    }

                    // Optionally emit a Healing visual effect around the
                    // healed player when requested by the world event (e.g.
                    // Taoist Healing / MassHealing). This mirrors
                    // SpellEffect.Healing from C# Healing/HealingCircle
                    // SpellObjects without showing visuals for passive regen.
                    if show_healing_effect {
                        const HEALING_EFFECT: u8 = 3; // SpellEffect.Healing
                        let eff_pkt = SObjectEffect {
                            object_id: session_id,
                            effect: HEALING_EFFECT,
                            effect_type: 0,
                            delay_time: 0,
                            time: 0,
                        };

                        if let Ok(raw) = eff_pkt.encode() {
                            let bytes = Self::encode_raw(raw);

                            // Also send to the current connection (typically
                            // the caster) so it can render the healing visual
                            // effect even when healing other players.
                            if session_id != self.session_id {
                                out.push(bytes.clone());
                            }

                            // Always send the effect to the healed player if this
                            // connection corresponds to them.
                            if session_id == self.session_id {
                                out.push(bytes.clone());
                            }

                            // And broadcast to other nearby viewers around the
                            // healed player's location.
                            self.enqueue_for_viewers(map_index, x, y, bytes);
                        }
                    }
                }
                world::WorldEvent::MonsterHealed {
                    monster_id,
                    map_index,
                    x,
                    y,
                    amount: _,
                    new_hp: _,
                    health_percent,
                    show_healing_effect,
                } => {
                    let object_id = monster_id as u32;

                    let health = SObjectHealth {
                        object_id,
                        percent: health_percent,
                        expire: 5,
                    };

                    if let Ok(pkt) = health.encode() {
                        let raw = Self::encode_raw(pkt);
                        out.push(raw.clone());
                        self.enqueue_for_viewers(map_index, x, y, raw);
                    }

                    if show_healing_effect {
                        const HEALING_EFFECT: u8 = 3; // SpellEffect.Healing
                        let eff_pkt = SObjectEffect {
                            object_id,
                            effect: HEALING_EFFECT,
                            effect_type: 0,
                            delay_time: 0,
                            time: 0,
                        };

                        if let Ok(raw) = eff_pkt.encode() {
                            let bytes = Self::encode_raw(raw);
                            out.push(bytes.clone());
                            self.enqueue_for_viewers(map_index, x, y, bytes);
                        }
                    }
                }
                world::WorldEvent::SpellToggle {
                    session_id,
                    spell_id,
                    enabled,
                } => {
                    let pkt = SSpellToggle {
                        object_id: session_id,
                        spell: spell_id,
                        can_use: enabled,
                    };
                    if let Ok(raw) = pkt.encode() {
                        let bytes = Self::encode_raw(raw);
                        out.push(bytes.clone());

                        if let Some((map_index, x, y, _dir)) = {
                            let world = self.world.lock().unwrap();
                            world.player_position(session_id)
                        } {
                            self.enqueue_for_viewers(map_index, x, y, bytes);
                        }
                    }
                }
                world::WorldEvent::AddBuff {
                    session_id,
                    buff_bytes,
                } => {
                    if session_id == self.session_id {
                        // First byte of buff_bytes is BuffType as u8.
                        let buff_type_u8 = buff_bytes.first().copied().unwrap_or(0);

                        tracing::debug!(
                            "connection: AddBuff event for session_id={} local_session_id={} buff_type_u8={}",
                            session_id,
                            self.session_id,
                            buff_type_u8,
                        );

                        let pkt = SAddBuff { buff_bytes };
                        let raw = pkt.encode();
                        out.push(Self::encode_raw(raw));

                        // For MagicShield / ElementalBarrier, also emit
                        // a persistent shield visual using SObjectEffect
                        // with SpellEffect.MagicShieldUp / ElementalBarrierUp.
                        if buff_type_u8 == world::BuffType::MagicShield as u8
                            || buff_type_u8 == world::BuffType::ElementalBarrier as u8
                        {
                            tracing::debug!(
                                "connection: sending MagicShield/ElementalBarrier Up effect session_id={} buff_type_u8={}",
                                session_id,
                                buff_type_u8,
                            );
                            let effect: u8 = if buff_type_u8 == world::BuffType::MagicShield as u8 {
                                6 // SpellEffect.MagicShieldUp
                            } else {
                                13 // SpellEffect.ElementalBarrierUp
                            };

                            let eff_pkt = SObjectEffect {
                                object_id: self.session_id,
                                effect,
                                effect_type: 0,
                                delay_time: 0,
                                time: 0,
                            };

                            if let Ok(raw) = eff_pkt.encode() {
                                let bytes = Self::encode_raw(raw);

                                // Send to self
                                out.push(bytes.clone());

                                // And broadcast to nearby viewers around
                                // the player's current location.
                                self.enqueue_for_viewers(
                                    self.current_map_index,
                                    self.current_x,
                                    self.current_y,
                                    bytes,
                                );
                            }
                        }
                    }
                }
                world::WorldEvent::RemoveBuff {
                    session_id,
                    buff_type,
                } => {
                    if session_id != self.session_id {
                        continue;
                    }

                    tracing::debug!(
                        "connection: RemoveBuff event for session_id={} buff_type={} (local_session_id={})",
                        session_id,
                        buff_type,
                        self.session_id,
                    );

                    let pkt = SRemoveBuff {
                        buff_type,
                        object_id: self.session_id,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }

                    // For MagicShield / ElementalBarrier, also emit the
                    // corresponding "Down" visual to stop the shield
                    // animation on the client.
                    if buff_type == world::BuffType::MagicShield as u8
                        || buff_type == world::BuffType::ElementalBarrier as u8
                    {
                        tracing::debug!(
                            "connection: sending MagicShield/ElementalBarrier Down effect session_id={} buff_type={}",
                            session_id,
                            buff_type,
                        );
                        let effect: u8 = if buff_type == world::BuffType::MagicShield as u8 {
                            7 // SpellEffect.MagicShieldDown
                        } else {
                            14 // SpellEffect.ElementalBarrierDown
                        };

                        let eff_pkt = SObjectEffect {
                            object_id: self.session_id,
                            effect,
                            effect_type: 0,
                            delay_time: 0,
                            time: 0,
                        };

                        if let Ok(raw) = eff_pkt.encode() {
                            let bytes = Self::encode_raw(raw);

                            // Send to self
                            out.push(bytes.clone());

                            // And broadcast to nearby viewers around the
                            // player's current location.
                            self.enqueue_for_viewers(
                                self.current_map_index,
                                self.current_x,
                                self.current_y,
                                bytes,
                            );
                        }
                    }
                }
                world::WorldEvent::PauseBuff {
                    session_id,
                    buff_type,
                    paused,
                } => {
                    if session_id != self.session_id {
                        continue;
                    }

                    let pkt = SPauseBuff {
                        buff_type,
                        object_id: self.session_id,
                        paused,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
                world::WorldEvent::PartySystemMessage { session_id, message } => {
                    if session_id == self.session_id {
                        self.send_system_chat(&message, out);
                    }
                }
                world::WorldEvent::ReincarnationRequested {
                    host_session_id: _,
                    target_session_id,
                } => {
                    if target_session_id == self.session_id {
                        let pkt = SRequestReincarnation;
                        let raw = pkt.encode();
                        out.push(Self::encode_raw(raw));
                    }
                }
                world::WorldEvent::ReincarnationCancelled { session_id } => {
                    if session_id == self.session_id {
                        let pkt = SCancelReincarnation;
                        let raw = pkt.encode();
                        out.push(Self::encode_raw(raw));
                    }
                }
                world::WorldEvent::QuestShared { session_id, quest_id, sharer_name } => {
                    if session_id == self.session_id {
                        // Send SShareQuest packet to notify client about shared quest
                        use crystal_shared_proto::quest::SShareQuest;
                        let pkt = SShareQuest {
                            quest_index: quest_id,
                            sharer_name,
                        };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
                world::WorldEvent::QuestItemRemoved { session_id, unique_id, count } => {
                    if session_id == self.session_id {
                        // Send SDeleteQuestItem packet to notify client about quest item removal
                        // Mirrors C# S.DeleteQuestItem behavior
                        use crystal_shared_proto::quest::SDeleteQuestItem;
                        let pkt = SDeleteQuestItem {
                            unique_id,
                            count,
                        };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                        
                        // Since Rust server uses regular inventory instead of QuestInventory,
                        // we also send SUserSlotsRefresh to ensure inventory is synchronized
                        let (inv, eq) = {
                            let world = self.world.lock().unwrap();
                            world
                                .player_items(self.session_id)
                                .unwrap_or((
                                    crystal_server_core::item::Inventory::new_default(),
                                    crystal_server_core::item::Equipment::new_default(),
                                ))
                        };
                        let refresh = SUserSlotsRefresh {
                            inventory: inv.slots,
                            equipment: eq.slots,
                        };
                        if let Ok(raw) = refresh.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
                world::WorldEvent::PlayerDied { session_id } => {
                    if session_id == self.session_id {
                        // Call default NPC Die page after player death
                        // C#: CallDefaultNPC(DefaultNPCType.Die)
                        // C# generates key as: "Die" (no parameters)
                        // Then wraps it as: string.Format("[@_{0}]", key) -> "[@_Die]"
                        use crystal_shared_proto::npc::CCallNPC;
                        let die_key = "Die".to_string();
                        let call_npc_msg = CCallNPC {
                            object_id: super::LoginConnection::DEFAULT_NPC_ID,
                            key: die_key,
                        };
                        // Call handle_call_npc directly (it's pub(crate) and part of LoginConnection impl)
                        self.handle_call_npc(call_npc_msg, out);
                    }
                }
            }
        }

        map_changed
    }

    pub(crate) fn handle_magic(&mut self, msg: CMagic, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::Magic {
                session_id: self.session_id,
                spell: msg.spell,
                direction: msg.direction,
                target_id: msg.target_id,
                x: msg.x,
                y: msg.y,
            })
        };

        let _ = self.handle_world_events(events, out);

        // 施法后主动从世界读取当前 HP/MP，并发一帧 SHealthChanged，
        // 确保客户端蓝条与服务器同步，匹配 C# MirConnection.Magic 的行为。
        let (hp, mp) = {
            let world = self.world.lock().unwrap();
            world
                .player_current_hp_mp(self.session_id)
                .unwrap_or((0, 0))
        };

        if let Some(stats) = self.current_stats.as_mut() {
            stats.hp = hp;
            stats.mp = mp;
        }

        let hc = SHealthChanged { hp, mp };
        if let Ok(raw) = hc.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_attack(&mut self, msg: CAttack, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::Attack {
                session_id: self.session_id,
                direction: msg.direction,
                spell: msg.spell,
            })
        };

        let _ = self.handle_world_events(events, out);

        // 对通过普通攻击键触发的技能（包括消耗 MP 的近战/魔法）也在攻击后
        // 主动同步一次 HP/MP，避免仅依赖后续伤害或回复事件间接刷新。
        let (hp, mp) = {
            let world = self.world.lock().unwrap();
            world
                .player_current_hp_mp(self.session_id)
                .unwrap_or((0, 0))
        };

        if let Some(stats) = self.current_stats.as_mut() {
            stats.hp = hp;
            stats.mp = mp;
        }

        let hc = SHealthChanged { hp, mp };
        if let Ok(raw) = hc.encode() {
            out.push(Self::encode_raw(raw));
        }
        self.update_visibility(out);
    }

    pub(crate) fn handle_range_attack(&mut self, msg: CRangeAttack, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Route through world so ranged attacks share validation and future
        // damage logic with other combat commands.
        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::RangeAttack {
                session_id: self.session_id,
                direction: msg.direction,
                target_id: msg.target_id,
                target_x: msg.target_x,
                target_y: msg.target_y,
            })
        };

        let _ = self.handle_world_events(events, out);
        self.update_visibility(out);
    }

    pub(crate) fn handle_spell_toggle(&mut self, msg: CSpellToggle, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // C# semantics:
        // - canUse > None (-1): apply to player
        // - else (== None): apply to hero if spawned
        // Player toggles are now persisted in world state so the setting
        // affects subsequent logic and survives beyond a single packet.
        if msg.can_use_state > -1 {
            let events = {
                let mut world = self.world.lock().unwrap();
                world.handle_command(world::WorldCommand::SpellToggle {
                    session_id: self.session_id,
                    spell_id: msg.spell,
                    can_use_state: msg.can_use_state,
                })
            };
            let _ = self.handle_world_events(events, out);
            return;
        }

        // Hero toggles are not yet represented in world state; keep a local
        // echo so the UI remains responsive.
        let enabled = if self.hero_spell_toggles.contains(&msg.spell) {
            self.hero_spell_toggles.remove(&msg.spell);
            false
        } else {
            self.hero_spell_toggles.insert(msg.spell);
            true
        };
        let target_object_id: u32 = if self.hero_spawn_state >= 2 {
            hero_object_id(self.session_id)
        } else {
            self.session_id
        };

        let pkt = SSpellToggle {
            object_id: target_object_id,
            spell: msg.spell,
            can_use: enabled,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_harvest(&mut self, msg: CHarvest, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement harvest logic
        // This should:
        // 1. Check if player can move (not dead, not in action delay)
        // 2. Set action time delay
        // 3. Update player direction
        // 4. Send UserLocation update
        // 5. Broadcast ObjectHarvest to nearby players
        // 6. Search for dead monsters in front of player
        // 7. If found, harvest the monster and send ObjectHarvested
        // 8. Handle harvest rewards (items, experience, etc.)
        
        // For now, send a placeholder response
        tracing::debug!("Harvest packet received: direction={}", msg.direction);
    }

    pub(crate) fn handle_change_trade(&mut self, msg: CChangeTrade, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        {
            let mut world = self.world.lock().unwrap();
            world.set_player_allow_trade(self.session_id, msg.allow_trade);
        }

        debug!(
            "ChangeTrade received: session_id={} allow_trade={}",
            self.session_id,
            msg.allow_trade
        );
    }

    pub(crate) fn handle_fishing_cast(&mut self, msg: CFishingCast, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement fishing cast logic
        // This should:
        // 1. Check if player has a fishing rod equipped
        // 2. Check if fishing rod has durability
        // 3. Validate fishing location (must be water)
        // 4. If cast_out is true:
        //    - Check if player has bait
        //    - Consume bait
        //    - Start fishing process
        // 5. If cast_out is false:
        //    - Stop fishing if in progress
        //    - Check if fish was found and attempt to catch
        //    - Handle fishing rewards
        // 6. Send appropriate response packets
        
        tracing::debug!("FishingCast packet received: cast_out={}", msg.cast_out);
    }

    pub(crate) fn handle_fishing_change_autocast(&mut self, msg: CFishingChangeAutocast, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // TODO: Implement fishing autocast change logic
        // This should:
        // 1. Check if player has a fishing rod equipped
        // 2. Check if fishing rod has a reel in the reel slot
        // 3. Set fishing autocast flag
        // 4. Update player state
        
        tracing::debug!("FishingChangeAutocast packet received: auto_cast={}", msg.auto_cast);
    }

    pub(crate) fn handle_pick_up(&mut self, _msg: CPickUp, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(world::WorldCommand::PickUp {
                session_id: self.session_id,
            })
        };

        let _ = self.handle_world_events(events, out);

        // After applying pickup results, refresh inventory/equipment so that
        // the client does not display stale items or ghost highlights. This
        // mirrors the explicit slot refresh we perform after DropItem.
        let (inv_after, eq_after) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        let refresh = SUserSlotsRefresh {
            inventory: inv_after.slots,
            equipment: eq_after.slots,
        };
        if let Ok(raw) = refresh.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_town_revive(&mut self, _msg: CTownRevive, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // Determine the bind location for this character (equivalent to C#
        // BindMapIndex/BindLocation). If none is stored, fall back to the
        // current map/position.
        let (mut dest_map, mut dest_x, mut dest_y, dest_dir) = if let (
            Some(ref account_id),
            Some(char_idx),
        ) = (self.account_id.as_ref(), self.current_char_index)
        {
            if let Ok(Some(pos)) = self.store.load_character_bind(account_id, char_idx) {
                (pos.map_index, pos.x, pos.y, pos.direction)
            } else {
                (
                    self.current_map_index,
                    self.current_x,
                    self.current_y,
                    self.direction,
                )
            }
        } else {
            (
                self.current_map_index,
                self.current_x,
                self.current_y,
                self.direction,
            )
        };

        // Mirror C# PlayerObject.TownRevive red-player behaviour: when
        // PKPoints >= 200, attempt to revive at PKTownMapName /
        // PKTownPositionX/Y from Setup.ini. If the configured PKTown map
        // cannot be resolved, fall back to the normal bind location above.
        let is_red = {
            let world = self.world.lock().unwrap();
            world
                .player_pk_points(self.session_id)
                .map(|pk| pk >= 200)
                .unwrap_or(false)
        };

        if is_red {
            let cfg = setup_config();
            if let Some(mi) = self
                .world_db
                .get_map_info_by_file_name(&cfg.pktown.map_name)
            {
                dest_map = mi.index;
                dest_x = cfg.pktown.position_x;
                dest_y = cfg.pktown.position_y;
            }
        }

        // Revive the player at the chosen bind location on the world side and
        // then issue a Teleport command so that a MapChanged event is emitted,
        // mirroring the C# TownRevive behaviour of reviving at town.
        let (map_index, x, y, direction, hp, mp, events) = {
            let mut world = self.world.lock().unwrap();
            let Some((map_index, x, y, direction, hp, mp)) = world.revive_player_to_position(
                self.session_id,
                dest_map,
                dest_x,
                dest_y,
                dest_dir,
            ) else {
                return;
            };
            let events = world.handle_command(world::WorldCommand::Teleport {
                session_id: self.session_id,
                map_index,
                x,
                y,
            });
            (map_index, x, y, direction, hp, mp, events)
        };

        // Let the shared world-event handler emit SMapChanged and update
        // visibility state for the new town map.
        let map_changed = self.handle_world_events(events, out);
        if map_changed {
            self.known_monsters.clear();
            self.known_npcs.clear();
            self.known_players.clear();
            self.known_heroes.clear();
            self.update_visibility(out);
        }

        self.current_map_index = map_index;
        self.current_x = x;
        self.current_y = y;
        self.direction = direction;

        if let Some(stats) = self.current_stats.as_mut() {
            stats.hp = hp;
            stats.mp = mp;
        }

        let hc = SHealthChanged { hp, mp };
        if let Ok(raw) = hc.encode() {
            out.push(Self::encode_raw(raw));
        }

        let revived = SRevived;
        let raw = revived.encode();
        out.push(Self::encode_raw(raw));

        let obj_revived = SObjectRevived {
            object_id: self.session_id,
            effect: true,
        };
        if let Ok(pkt) = obj_revived.encode() {
            let raw = Self::encode_raw(pkt);
            out.push(raw.clone());
            self.enqueue_for_viewers(map_index, x, y, raw);
        }
    }

    pub(crate) fn handle_accept_reincarnation(&mut self, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let (map_index, x, y, direction, hp, mp) = {
            let mut world = self.world.lock().unwrap();
            let Some((map_index, x, y, direction, hp, mp)) =
                world.accept_reincarnation_for_session(self.session_id)
            else {
                drop(world);
                // Mirror the legacy behaviour where the target is notified
                // when a Reincarnation attempt is no longer valid.
                self.send_system_chat("Reincarnation failed.", out);
                return;
            };
            (map_index, x, y, direction, hp, mp)
        };

        self.current_map_index = map_index;
        self.current_x = x;
        self.current_y = y;
        self.direction = direction;

        if let Some(stats) = self.current_stats.as_mut() {
            stats.hp = hp;
            stats.mp = mp;
        }

        let hc = SHealthChanged { hp, mp };
        if let Ok(raw) = hc.encode() {
            out.push(Self::encode_raw(raw));
        }

        let revived = SRevived;
        let raw = revived.encode();
        out.push(Self::encode_raw(raw));

        let obj_revived = SObjectRevived {
            object_id: self.session_id,
            effect: true,
        };
        if let Ok(pkt) = obj_revived.encode() {
            let raw = Self::encode_raw(pkt);
            out.push(raw.clone());
            self.enqueue_for_viewers(map_index, x, y, raw);
        }
    }

    pub(crate) fn handle_cancel_reincarnation(&mut self, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let mut world = self.world.lock().unwrap();
        world.expire_reincarnation_from_session(self.session_id);
    }

    pub(crate) fn send_safezone_border_spells(&self, map_index: i32, out: &mut Vec<Vec<u8>>) {
        if !self.world_config.safe_zone_border {
            return;
        }

        let Some(info) = self.world_db.get_map_info(map_index) else {
            return;
        };

        for sz in &info.safe_zones {
            let cx = sz.location_x;
            let cy = sz.location_y;
            let size = sz.size as i32;

            let min_y = cy - size;
            let max_y = cy + size;
            for y in min_y..=max_y {
                let dy = (y - cy).abs();
                let step = if dy == size { 1 } else { (size * 2).max(1) };

                let min_x = cx - size;
                let max_x = cx + size;
                let mut x = min_x;
                while x <= max_x {
                    let object_id = Self::safezone_spell_object_id(map_index, x, y);
                    let pkt = SObjectSpell {
                        object_id,
                        location_x: x,
                        location_y: y,
                        spell: world::Spell::TrapHexagon as u8,
                        direction: 0,
                        param: false,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }

                    x += step;
                }
            }
        }
    }

    fn safezone_spell_object_id(map_index: i32, x: i32, y: i32) -> u32 {
        let mi = map_index as u32 & 0xFFF; // 12 bits for map index
        let ux = x.max(0) as u32 & 0x3FF; // 10 bits for x
        let uy = y.max(0) as u32 & 0x3FF; // 10 bits for y

        // Place safe-zone border spell object IDs into a high, reserved range
        // so they do not collide with real runtime object IDs such as player
        // SessionId or monster/NPC ids. We keep the original bit-packing for
        // map/x/y and then force the top bit on.
        let base = (mi << 20) | (ux << 10) | uy;
        base | 0x8000_0000
    }
}
