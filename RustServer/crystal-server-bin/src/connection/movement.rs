use crystal_server_core::world;
use crystal_server_core::world::WorldProvider;
use crystal_shared_proto::map::SMapChanged;
use crystal_shared_proto::scene::{
    SObjectAttack,
    SObjectRun,
    SObjectStruck,
    SObjectTurn,
    SObjectTurnWalkRun,
    SObjectWalk,
    SDamageIndicator,
    SObjectHealth,
    SGainExperience,
    SLevelChanged,
    SObjectLeveled,
};
use crystal_shared_proto::user::{SUserLocation, SHealthChanged};

use super::LoginConnection;

impl LoginConnection {
    fn enqueue_for_viewers(
        &self,
        map_index: i32,
        x: i32,
        y: i32,
        payload: Vec<u8>,
    ) {
        let viewers: Vec<world::SessionId> = {
            let world = self.world.lock().unwrap();
            world.sessions_in_range_for_map(map_index, x, y, Self::DATA_RANGE)
        };

        let mut outboxes = self.outboxes.lock().unwrap();
        for sid in viewers {
            if sid == self.session_id {
                continue;
            }
            outboxes.entry(sid).or_default().push(payload.clone());
        }
    }

    pub(crate) fn apply_step(&mut self, direction: u8, distance: i32, out: &mut Vec<Vec<u8>>) -> bool {
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

        // Remember the movement type (0=turn,1=walk,2=run) for this command so
        // that handle_world_events can emit the appropriate SObjectTurn/Walk/Run
        // broadcast to nearby players.
        self.last_move_kind = Some(distance as u8);

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(cmd)
        };
        self.handle_world_events(events, out)
    }

    pub(crate) fn handle_world_events(
        &mut self,
        events: Vec<world::WorldEvent>,
        out: &mut Vec<Vec<u8>>,
    ) -> bool {
        let mut map_changed = false;

        // Take and clear the last movement kind associated with this batch of
        // world events so we can send a matching ObjectTurn/Walk/Run to
        // observers when processing the UserLocation update.
        let movement_kind = self.last_move_kind.take();

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

                    if let Some(kind) = movement_kind {
                        let base = SObjectTurnWalkRun {
                            object_id: self.session_id,
                            location_x: x,
                            location_y: y,
                            direction,
                        };

                        let pkt_res = match kind {
                            0 => SObjectTurn(base).encode(),
                            1 => SObjectWalk(base).encode(),
                            2 => SObjectRun(base).encode(),
                            _ => return map_changed,
                        };

                        if let Ok(pkt) = pkt_res {
                            let raw = Self::encode_raw(pkt);
                            self.enqueue_for_viewers(map_index, x, y, raw);
                        }
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
                    map_index,
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
                    if let Ok(pkt) = attack.encode() {
                        let raw = Self::encode_raw(pkt);
                        out.push(raw.clone());
                        self.enqueue_for_viewers(map_index, x, y, raw);
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
                } => {
                    if attacker_id != self.session_id {
                        continue;
                    }

                    let object_id = target_id as u32;

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
                        expire: 2,
                    };
                    if let Ok(pkt) = health.encode() {
                        let raw = Self::encode_raw(pkt);
                        out.push(raw.clone());
                        self.enqueue_for_viewers(map_index, x, y, raw);
                    }
                }
                world::WorldEvent::ObjectLocation { .. } => {}
                world::WorldEvent::GainExperience {
                    session_id,
                    amount,
                } => {
                    if session_id != self.session_id {
                        continue;
                    }

                    // Always notify the client about gained experience, matching
                    // C# PlayerObject.GainExp which enqueues S.GainExperience
                    // before performing any level-up checks.
                    let gain_pkt = SGainExperience { amount };
                    if let Ok(raw) = gain_pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }

                    // If we don't have a stats snapshot yet, we cannot safely
                    // compute level-ups. In that case we mirror the packet
                    // behaviour only.
                    let Some(stats) = self.current_stats.as_mut() else {
                        continue;
                    };

                    // Determine the current character level from the cached
                    // SelectInfo list. This mirrors the source for Level used
                    // when starting the game.
                    let mut level: u16 = 0;
                    if let Some(char_idx) = self.current_char_index {
                        if let Some(ch) = self.characters.iter().find(|c| c.index == char_idx) {
                            level = ch.level;
                        }
                    }

                    if level == 0 {
                        level = 1;
                    }

                    // Apply the experience gain locally. World combat logic
                    // already increments PlayerState.experience by `amount`;
                    // below we will normalise both world and connection-side
                    // state to the same post-level-up values.
                    let mut exp = stats
                        .experience
                        .saturating_add(amount as i64);

                    let mut leveled = false;

                    loop {
                        let lvl_usize = level as usize;
                        if lvl_usize == 0 {
                            break;
                        }

                        let max_exp = *self
                            .exp_table
                            .get(lvl_usize.saturating_sub(1))
                            .unwrap_or(&0_i64);

                        if max_exp <= 0 {
                            break;
                        }

                        if exp < max_exp {
                            break;
                        }

                        if level >= u16::MAX {
                            break;
                        }

                        exp = exp.saturating_sub(max_exp);
                        level = level.saturating_add(1);
                        leveled = true;
                    }

                    // Persist the remaining experience after any level-ups.
                    stats.experience = exp;

                    if !leveled {
                        continue;
                    }

                    // Update world-side player state (level + experience) and
                    // trigger stat recalculation, mirroring C# LevelUp's
                    // RefreshStats behaviour.
                    {
                        let mut world = self.world.lock().unwrap();
                        let _ = world.set_player_level_and_experience(
                            self.session_id,
                            level,
                            exp,
                        );
                    }

                    // After stats are recalculated, fetch the new HP/MP caps
                    // from the world and reset the connection-side values to
                    // those maxima, mirroring SetHP(Stats[HP]) / SetMP(Stats[MP]).
                    let (max_hp, max_mp) = {
                        let world = self.world.lock().unwrap();
                        world
                            .player_max_hp_mp(self.session_id)
                            .unwrap_or((stats.hp.max(0), stats.mp.max(0)))
                    };
                    stats.hp = max_hp.max(0);
                    stats.mp = max_mp.max(0);

                    // Notify the leveling player of their new HP/MP values so
                    // the client UI can update. This mirrors C#
                    // HumanObject.SendHealthChanged which enqueues
                    // S.HealthChanged after SetHP/SetMP.
                    let hpmp_pkt = SHealthChanged {
                        hp: stats.hp,
                        mp: stats.mp,
                    };
                    if let Ok(raw) = hpmp_pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }

                    // Keep the cached SelectInfo level in sync so that future
                    // character lists and admin views show the correct level.
                    if let Some(char_idx) = self.current_char_index {
                        if let Some(ch) = self
                            .characters
                            .iter_mut()
                            .find(|c| c.index == char_idx)
                        {
                            ch.level = level;
                        }
                    }

                    // Compute MaxExperience for the new level using the same
                    // exp_table and indexing convention as handle_start_game.
                    let max_experience = if level == 0 {
                        0_i64
                    } else {
                        *self
                            .exp_table
                            .get(level as usize - 1)
                            .unwrap_or(&0_i64)
                    };

                    // Notify the leveling player.
                    let lvl_pkt = SLevelChanged {
                        level,
                        experience: exp,
                        max_experience,
                    };
                    if let Ok(raw) = lvl_pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }

                    // Broadcast ObjectLeveled to self and nearby observers,
                    // mirroring C# HumanObject.LevelUp's Broadcast.
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
                }
            }
        }

        map_changed
    }
}
