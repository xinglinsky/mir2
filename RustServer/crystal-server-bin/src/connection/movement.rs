use crystal_server_core::world;
use crystal_server_core::world::WorldProvider;
use crystal_shared_proto::login::{
    CAttack,
    CPickUp,
    CTownRevive,
    CTurn,
    CWalk,
    CRun,
    CMagicKey,
    CMagic,
};
use crystal_shared_proto::map::SMapChanged;
use crystal_shared_proto::magic::SObjectSpell;
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
    SGainedGold,
    SGainedItem,
    SLevelChanged,
    SObjectLeveled,
    SObjectDied,
    SObjectItem,
    SObjectGold,
    SObjectRemove,
    SRevived,
    SObjectRevived,
    SSpellToggle,
    SAddBuff,
    SRemoveBuff,
};
use crystal_shared_proto::user::{SUserLocation, SHealthChanged, SUserSlotsRefresh};

use super::{LoginConnection, Stage};

impl LoginConnection {
    pub(crate) fn handle_turn(&mut self, msg: CTurn, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        println!(
            "[ingame] handle_turn: session={} dir={} map={} pos=({}, {})",
            self.session_id,
            msg.direction,
            self.current_map_index,
            self.current_x,
            self.current_y,
        );

        let _ = self.apply_step(msg.direction, 0, out);
    }

    pub(crate) fn handle_walk(&mut self, msg: CWalk, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        println!(
            "[ingame] handle_walk: session={} dir={} map={} pos=({}, {})",
            self.session_id,
            msg.direction,
            self.current_map_index,
            self.current_x,
            self.current_y,
        );

        let map_changed = self.apply_step(msg.direction, 1, out);

        if map_changed {
            self.known_monsters.clear();
            self.known_npcs.clear();
        }

        self.update_visibility(out);
    }

    pub(crate) fn handle_magic_key(&mut self, msg: CMagicKey, _out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        // For now we only support main-player keys (no hero separation).
        let updated_magics = {
            let mut world = self.world.lock().unwrap();
            world.set_magic_key_for_player(self.session_id, msg.spell, msg.key)
        };

        if let (Some(magics), Some(ref account_id), Some(char_idx)) = (
            updated_magics,
            self.account_id.as_ref(),
            self.current_char_index,
        ) {
            let _ = self
                .store
                .save_character_magics(account_id, char_idx, &magics);
        }
    }

    pub(crate) fn handle_run(&mut self, msg: CRun, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        println!(
            "[ingame] handle_run: session={} dir={} map={} pos=({}, {})",
            self.session_id,
            msg.direction,
            self.current_map_index,
            self.current_x,
            self.current_y,
        );

        let map_changed = self.apply_step(msg.direction, 2, out);

        if map_changed {
            self.known_monsters.clear();
            self.known_npcs.clear();
        }

        self.update_visibility(out);
    }
    fn item_name_colour_for_grade(grade: u8) -> i32 {
        match grade {
            2 => 0xFF00BFFFu32 as i32,
            3 => 0xFFFF8C00u32 as i32,
            4 => 0xFFDDA0DDu32 as i32,
            5 => 0xFFFF0000u32 as i32,
            _ => 0xFFFFFFFFu32 as i32,
        }
    }

    pub(crate) fn enqueue_for_viewers(
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
        println!(
            "[conn] apply_step: session={} dir={} dist={} stage={:?} map={} pos=({}, {})",
            self.session_id,
            direction,
            distance,
            self.stage,
            self.current_map_index,
            self.current_x,
            self.current_y,
        );

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

                    // Emit decorative SafeZone border spells (TrapHexagon) for
                    // this map, mirroring C# Map.CreateSafeZone when
                    // Settings.SafeZoneBorder is enabled.
                    self.send_safezone_border_spells(map_index, out);
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
                world::WorldEvent::MonsterHitPlayer { .. } => {}
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

                    // Always persist updated experience so that gaining
                    // experience without leveling still survives logout or
                    // disconnect. Level changes are handled below when
                    // `leveled` is true.
                    if let (Some(ref account_id), Some(char_idx)) =
                        (self.account_id.as_ref(), self.current_char_index)
                    {
                        let _ = self
                            .store
                            .save_character_stats(account_id, char_idx, &*stats);
                    }

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

                    // Persist the new level immediately so that returning to
                    // character select or reconnecting uses the updated value.
                    if let (Some(ref account_id), Some(char_idx)) =
                        (self.account_id.as_ref(), self.current_char_index)
                    {
                        let _ = self
                            .store
                            .update_character_level(account_id, char_idx, level);
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
                world::WorldEvent::MonsterDied {
                    object_id,
                    map_index,
                    x,
                    y,
                    direction,
                } => {
                    // Monster is now dead; stop tracking it as a "known" live
                    // monster so that update_visibility does not immediately
                    // send SObjectRemove and erase the corpse.
                    self.known_monsters.remove(&object_id);

                    let pkt = SObjectDied {
                        object_id: object_id as u32,
                        location_x: x,
                        location_y: y,
                        direction,
                        death_type: 0,
                    };
                    if let Ok(raw) = pkt.encode() {
                        let encoded = Self::encode_raw(raw);
                        out.push(encoded.clone());
                        self.enqueue_for_viewers(map_index, x, y, encoded);
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
                            let encoded = Self::encode_raw(raw);
                            out.push(encoded.clone());
                            self.enqueue_for_viewers(map_index, x, y, encoded);
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
                    if gold == 0 {
                        continue;
                    }

                    let pkt = SObjectGold {
                        object_id: object_id as u32,
                        gold,
                        location_x: x,
                        location_y: y,
                    };
                    if let Ok(raw) = pkt.encode() {
                        let encoded = Self::encode_raw(raw);
                        out.push(encoded.clone());
                        self.enqueue_for_viewers(map_index, x, y, encoded);
                    }
                }
                world::WorldEvent::PlayerGainedItem { session_id, item } => {
                    if session_id != self.session_id {
                        continue;
                    }

                    if let Ok(pkt) = SGainedItem::from_user_item(&item) {
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
                world::WorldEvent::PlayerGainedGold { session_id, amount } => {
                    if session_id != self.session_id {
                        continue;
                    }

                    if let Some(mut stats) = self.current_stats.clone() {
                        let delta = amount as i64;
                        let new_gold = stats.gold.saturating_add(delta);
                        stats.gold = new_gold;

                        if let (Some(ref account_id), Some(char_idx)) =
                            (self.account_id.as_ref(), self.current_char_index)
                        {
                            let _ = self
                                .store
                                .save_character_stats(account_id, char_idx, &stats);
                        }

                        self.current_stats = Some(stats.clone());

                        let gained = SGainedGold { gold: amount };
                        if let Ok(raw) = gained.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
                world::WorldEvent::MapItemRemoved {
                    object_id,
                    map_index,
                    x,
                    y,
                } => {
                    let pkt = SObjectRemove {
                        object_id: object_id as u32,
                    };
                    if let Ok(raw) = pkt.encode() {
                        let encoded = Self::encode_raw(raw);
                        out.push(encoded.clone());
                        self.enqueue_for_viewers(map_index, x, y, encoded);
                    }
                }
                world::WorldEvent::SpellToggle {
                    session_id,
                    spell_id,
                    enabled,
                } => {
                    if session_id != self.session_id {
                        continue;
                    }

                    let pkt = SSpellToggle {
                        object_id: self.session_id,
                        spell: spell_id,
                        can_use: enabled,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
                world::WorldEvent::AddBuff {
                    session_id,
                    buff_bytes,
                } => {
                    if session_id != self.session_id {
                        continue;
                    }

                    let pkt = SAddBuff { buff_bytes };
                    let raw = pkt.encode();
                    out.push(Self::encode_raw(raw));
                }
                world::WorldEvent::RemoveBuff {
                    session_id,
                    buff_type,
                } => {
                    if session_id != self.session_id {
                        continue;
                    }

                    let pkt = SRemoveBuff {
                        buff_type,
                        object_id: self.session_id,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
                world::WorldEvent::PlayerHealed { .. } => {}
                world::WorldEvent::PartySystemMessage { session_id, message } => {
                    if session_id == self.session_id {
                        self.send_system_chat(&message, out);
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
        self.update_visibility(out);
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
        let (dest_map, dest_x, dest_y, dest_dir) = if let (
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

    fn send_safezone_border_spells(&self, map_index: i32, out: &mut Vec<Vec<u8>>) {
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
        (mi << 20) | (ux << 10) | uy
    }
}
