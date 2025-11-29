use rand::Rng;
use rand::thread_rng;
use tracing::{trace, debug};
use std::fs;
use crate::combat::compute_physical_melee_with_crit;
use crate::stats::{Stat, Stats};
use crate::world::map::{self, RespawnInfo};
use crate::world::monster::{MonsterAiState, MonsterInstance};
use crate::world::provider::WorldProvider;
use crate::world::types::{BuffProperty, BuffType};
use crate::world::Spell;

use super::{World, WorldEvent};

#[derive(Clone, Debug)]
pub struct RoutePoint {
    pub x: i32,
    pub y: i32,
    pub delay_ms: i64,
}

/// Runtime state for a single RespawnInfo entry on a map. This mirrors a subset
/// of the C# MapRespawn fields so that we can later drive timed respawns from
/// World::update without re-reading map metadata.
#[derive(Clone, Debug)]
pub struct RespawnRuntime {
    pub info: RespawnInfo,
    pub map_index: i32,
    /// Current number of monsters spawned for this respawn point. This will be
    /// kept in sync with the World.monsters list once death/despawn logic is
    /// implemented.
    pub current_count: u16,
    /// Next time (in ms since Unix epoch) when this respawn should attempt to
    /// spawn more monsters. This will be derived from RespawnInfo.delay and
    /// random_delay once World::update starts driving spawns.
    pub next_spawn_time_ms: i64,
    pub next_spawn_tick: u64,
    /// Simple error counter mirroring the C# MapRespawn.ErrorCount field; for
    /// now it is unused but reserved for future spawn failure backoff.
    pub error_count: u8,
    pub route_points: Option<Vec<RoutePoint>>,
}

impl<P: WorldProvider> World<P> {
    pub(super) fn spawn_monsters_for_map(&mut self, map_index: i32, map: &map::Map) {
        if self.monsters.contains_key(&map_index) {
            return;
        }

        let mut instances = Vec::new();

        // Initialise respawn runtime state for this map if we have not already
        // done so. For now we simply record the RespawnInfo entries together
        // with their initial Count; World::update will later evolve this into
        // a timed respawn system driven by delay/random_delay/respawn_ticks.
        if !self.respawns.contains_key(&map_index) {
            let mut runtimes = Vec::new();
            for respawn in &map.info.respawns {
                if respawn.monster_index <= 0 {
                    continue;
                }

                // Ensure the monster definition exists in the DB; if not, skip.
                if self
                    .provider
                    .get_monster_info(respawn.monster_index)
                    .is_none()
                {
                    continue;
                }

                runtimes.push(RespawnRuntime {
                    info: respawn.clone(),
                    map_index,
                    current_count: respawn.count,
                    next_spawn_time_ms: 0,
                    next_spawn_tick: 0,
                    error_count: 0,
                    route_points: self.load_route_points(&respawn.route_path),
                });
            }
            self.respawns.insert(map_index, runtimes);
        }

        for respawn in &map.info.respawns {
            if respawn.monster_index <= 0 {
                continue;
            }

            // Ensure the monster definition exists in the DB; if not, skip.
            if self
                .provider
                .get_monster_info(respawn.monster_index)
                .is_none()
            {
                continue;
            }

            instances.extend(self.create_monsters_from_respawn(map, respawn));
        }

        for m in &instances {
            self.add_monster_to_occupancy(m.id, m.map_index, m.x, m.y);
        }

        self.monsters.insert(map_index, instances);
    }

    fn load_route_points(&self, route_path: &str) -> Option<Vec<RoutePoint>> {
        if route_path.is_empty() {
            return None;
        }

        let file_name = format!("{route_path}.txt");
        let full_path = self.config.routes_path.join(file_name);

        let contents = fs::read_to_string(&full_path).ok()?;
        let mut points = Vec::new();

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() < 2 {
                continue;
            }

            let Ok(x) = parts[0].parse::<i32>() else { continue };
            let Ok(y) = parts[1].parse::<i32>() else { continue };
            let delay_ms = if parts.len() >= 3 {
                parts[2].parse::<i64>().unwrap_or(0)
            } else {
                0
            };

            points.push(RoutePoint { x, y, delay_ms });
        }

        if points.is_empty() {
            None
        } else {
            Some(points)
        }
    }

    pub fn monsters_for_map(&self, map_index: i32) -> Vec<MonsterInstance> {
        self.monsters
            .get(&map_index)
            .cloned()
            .unwrap_or_default()
    }

    /// Query monsters on a given map within a rectangular view range around the
    /// provided centre, returning cloned MonsterInstance values. This mirrors
    /// the visibility logic currently used by the connection layer.
    pub fn monsters_in_view_for_map(
        &self,
        map_index: i32,
        centre_x: i32,
        centre_y: i32,
        range: i32,
    ) -> Vec<MonsterInstance> {
        self.monsters
            .get(&map_index)
            .into_iter()
            .flat_map(|monsters| monsters.iter())
            .filter(|m| {
                (m.x - centre_x).abs() <= range && (m.y - centre_y).abs() <= range
            })
            .cloned()
            .collect()
    }

    /// World update tick. This will eventually drive respawns, monster AI,
    /// buffs and other timed systems.
    pub fn update(&mut self, now_ms: i64) -> Vec<WorldEvent> {
        self.time_ms = now_ms;
        self.update_respawn_tick_counter(now_ms);

        let mut events: Vec<WorldEvent> = Vec::new();

        // Process expired map items: remove items that have exceeded their
        // expire_time_ms, mirroring C# ItemObject.Process() behavior.
        self.process_map_items(now_ms, &mut events);

        // Process player buffs: check for expiration
        self.process_player_buffs(now_ms, &mut events);

        // Natural HP/MP regeneration for players, mirroring the core of C#
        // HumanObject.ProcessRegen (RegenDelay, 3% max HP/MP and recovery
        // stats). This runs independently of SafeZone healing.
        self.process_player_regen(now_ms, &mut events);

        self.process_player_pk(now_ms);

        // SafeZoneHealing: periodically heal players standing inside any
        // configured SafeZone when Settings.SafeZoneHealing is enabled. This
        // mirrors the effect of C# Healing spell objects placed by
        // Map.CreateSafeZone but applies healing directly to player HP.
        if self.config.safe_zone_healing {
            const SAFEZONE_HEAL_INTERVAL_MS: i64 = 2_000;
            let due = self.safezone_heal_last_ms == 0
                || now_ms.saturating_sub(self.safezone_heal_last_ms) >= SAFEZONE_HEAL_INTERVAL_MS;
            if due {
                self.process_safezone_healing(&mut events);
                self.safezone_heal_last_ms = now_ms;
            }
        }

        self.process_monster_buffs(now_ms);

        let mut jobs: Vec<(i32, usize, u16)> = Vec::new();
        let mut total_spawned: u32 = 0;
        let mut total_jobs: u32 = 0;

        let map_indices: Vec<i32> = self.respawns.keys().cloned().collect();

        for map_index in &map_indices {
            if let Some(runtimes) = self.respawns.get_mut(map_index) {
                for (idx, rt) in runtimes.iter_mut().enumerate() {
                    let base_count = rt.info.count as u32;
                    let multiplier = self.spawn_multiplier.max(1) as u32;
                    let target_total =
                        base_count.saturating_mul(multiplier).min(u16::MAX as u32) as u16;

                    if rt.info.respawn_ticks != 0 {
                        let current_tick = self.respawn_tick_counter;

                        if rt.next_spawn_tick == 0 {
                            let delta = rt.info.respawn_ticks as u64;
                            rt.next_spawn_tick = current_tick.wrapping_add(delta);
                            continue;
                        }

                        if current_tick < rt.next_spawn_tick {
                            continue;
                        }

                        if rt.current_count < target_total {
                            let needed = target_total - rt.current_count;
                            if needed > 0 {
                                jobs.push((*map_index, idx, needed));
                            }
                        }

                        let delta = rt.info.respawn_ticks as u64;
                        rt.next_spawn_tick = current_tick.wrapping_add(delta);
                    } else {
                        if rt.next_spawn_time_ms == 0 {
                            let delay_ms = Self::compute_respawn_delay_ms(&rt.info);
                            rt.next_spawn_time_ms = now_ms.saturating_add(delay_ms);
                            continue;
                        }

                        if now_ms < rt.next_spawn_time_ms {
                            continue;
                        }

                        if rt.current_count < target_total {
                            let needed = target_total - rt.current_count;
                            if needed > 0 {
                                jobs.push((*map_index, idx, needed));
                            }
                        }

                        let delay_ms = Self::compute_respawn_delay_ms(&rt.info);
                        rt.next_spawn_time_ms = now_ms.saturating_add(delay_ms);
                    }
                }
            }
        }

        for (map_index, rt_index, needed) in jobs {
            let map = match self.get_or_load_map(map_index) {
                Some(m) => m,
                None => continue,
            };

            let base_info = match self
                .respawns
                .get(&map_index)
                .and_then(|v| v.get(rt_index))
            {
                Some(rt) => &rt.info,
                None => continue,
            };

            let mut tmp = base_info.clone();
            tmp.count = needed;
            let spawned = self.create_monsters_from_respawn(&map, &tmp);

            let actual = spawned.len() as u16;
            if actual == 0 {
                if let Some(runtimes) = self.respawns.get_mut(&map_index) {
                    if let Some(rt) = runtimes.get_mut(rt_index) {
                        if rt.info.respawn_ticks == 0 {
                            let minute_ms = 60_000_i64;
                            rt.next_spawn_time_ms =
                                self.time_ms.saturating_add(minute_ms);
                        }
                        if rt.error_count < u8::MAX {
                            rt.error_count = rt.error_count.saturating_add(1);
                        }
                    }
                }
                continue;
            }

            for m in &spawned {
                self.add_monster_to_occupancy(m.id, m.map_index, m.x, m.y);
            }

            let monsters_on_map = self.monsters.entry(map_index).or_default();
            monsters_on_map.extend(spawned);

            if let Some(runtimes) = self.respawns.get_mut(&map_index) {
                if let Some(rt) = runtimes.get_mut(rt_index) {
                    rt.current_count = rt.current_count.saturating_add(actual);
                    rt.error_count = 0;
                }
            }
            if actual > 0 {
                total_spawned = total_spawned.saturating_add(actual as u32);
                total_jobs = total_jobs.saturating_add(1);
            }
        }

        if total_spawned > 0 {
            debug!(
                "respawn_update: t={} tick={} multiplier={} base_min={} jobs={} spawned={}",
                self.time_ms,
                self.respawn_tick_counter,
                self.spawn_multiplier,
                self.respawn_base_spawn_rate_minutes,
                total_jobs,
                total_spawned,
            );
        }

        self.process_monster_ai(now_ms, &mut events);
        self.process_guard_ai(now_ms, &mut events);

        events
    }

    fn process_player_buffs(&mut self, now_ms: i64, events: &mut Vec<WorldEvent>) {
        for (&session_id, player) in &mut self.players {
            let mut removed_indices: Vec<usize> = Vec::new();
            let mut stats_changed = false;

            // Determine whether the player is currently standing inside any
            // configured SafeZone for their map. This mirrors the C#
            // MapObject.InSafeZone check used by BuffProperty.PauseInSafeZone
            // semantics.
            let in_safe_zone = self
                .provider
                .get_map_info(player.map_index)
                .map(|info| Self::point_in_safe_zone(info, player.x, player.y))
                .unwrap_or(false);

            for (idx, buff) in player.active_buffs.iter_mut().enumerate() {
                // PauseInSafeZone: if the BuffInfo has this property, toggle
                // the paused flag when the player is in/out of a SafeZone and
                // maintain pause_remaining_ms so that remaining duration is
                // preserved while paused.
                if let Some(info) = self.provider.get_buff_info(buff.buff_type) {
                    if info.has_property(BuffProperty::PauseInSafeZone) {
                        if in_safe_zone && !buff.paused {
                            // Transition: running -> paused.
                            if !buff.infinite {
                                let remaining = buff.expire_time_ms.saturating_sub(now_ms);
                                buff.pause_remaining_ms = remaining.max(0);
                            } else {
                                buff.pause_remaining_ms = 0;
                            }
                            buff.paused = true;
                            events.push(WorldEvent::PauseBuff {
                                session_id,
                                buff_type: buff.buff_type.as_u8(),
                                paused: true,
                            });
                        } else if !in_safe_zone && buff.paused {
                            // Transition: paused -> running.
                            buff.paused = false;
                            if !buff.infinite {
                                let remaining = if buff.pause_remaining_ms > 0 {
                                    buff.pause_remaining_ms
                                } else {
                                    buff.expire_time_ms.saturating_sub(now_ms).max(0)
                                };
                                buff.expire_time_ms = now_ms.saturating_add(remaining);
                            }
                            buff.pause_remaining_ms = 0;
                            events.push(WorldEvent::PauseBuff {
                                session_id,
                                buff_type: buff.buff_type.as_u8(),
                                paused: false,
                            });
                        }
                    }
                }

                // Expiration: only advance and expire buffs that are not
                // infinite and not currently paused.
                if !buff.infinite && !buff.paused && buff.expire_time_ms <= now_ms {
                    removed_indices.push(idx);
                }
            }

            if removed_indices.is_empty() {
                continue;
            }

            // Sort descending to remove from back.
            removed_indices.sort_unstable_by(|a, b| b.cmp(a));

            for idx in removed_indices {
                let buff = player.active_buffs.remove(idx);
                stats_changed = true;

                if buff.buff_type == BuffType::FlamingSword {
                    events.push(WorldEvent::SpellToggle {
                        session_id,
                        spell_id: Spell::FlamingSword as u8,
                        enabled: false,
                    });
                } else {
                    events.push(WorldEvent::RemoveBuff {
                        session_id,
                        buff_type: buff.buff_type as u8,
                    });
                }
            }

            if stats_changed {
                player.stats.buffs.clear();
                for b in &player.active_buffs {
                    player.stats.buffs.add(&b.stats);
                }
                player.stats.recalc_if_dirty_for_job(player.job);
            }
        }
    }

    fn process_monster_buffs(&mut self, now_ms: i64) {
        for monsters in self.monsters.values_mut() {
            for monster in monsters.iter_mut() {
                // Skip any monsters that have already been reduced to 0 HP by
                // previous combat logic (player skills, guards, etc.). Dead
                // monsters remain in the monsters list so that the client can
                // continue to render their corpses, but they should not run
                // further AI.
                if monster.hp <= 0 {
                    continue;
                }
                if monster.buffs.is_empty() {
                    continue;
                }

                let mut removed_indices: Vec<usize> = Vec::new();

                for (idx, buff) in monster.buffs.iter_mut().enumerate() {
                    if buff.infinite {
                        continue;
                    }
                    if buff.flag_for_removal
                        || (buff.expire_time_ms > 0 && buff.expire_time_ms <= now_ms)
                    {
                        removed_indices.push(idx);
                    }
                }

                if removed_indices.is_empty() {
                    continue;
                }

                removed_indices.sort_unstable_by(|a, b| b.cmp(a));

                for idx in removed_indices {
                    monster.buffs.remove(idx);
                }

                monster.buff_stats.clear();
                for b in &monster.buffs {
                    monster.buff_stats.add(&b.stats);
                }
            }
        }
    }

    fn process_player_regen(&mut self, now_ms: i64, events: &mut Vec<WorldEvent>) {
        const REGEN_INTERVAL_MS: i64 = 10_000; // C# HumanObject.RegenDelay
        const HEALTH_REGEN_WEIGHT: i32 = 10; // Settings.HealthRegenWeight
        const MANA_REGEN_WEIGHT: i32 = 10; // Settings.ManaRegenWeight

        for player in self.players.values_mut() {
            if player.dead {
                continue;
            }

            let max_hp = player.stats.total.get(Stat::HP).max(0);
            let max_mp = player.stats.total.get(Stat::MP).max(0);

            if max_hp <= 0 && max_mp <= 0 {
                continue;
            }

            // Mirror CanRegen/regen delay semantics using next_regen_time_ms.
            if player.next_regen_time_ms != 0 && now_ms < player.next_regen_time_ms {
                continue;
            }
            player.next_regen_time_ms = now_ms.saturating_add(REGEN_INTERVAL_MS);

            let mut hp_regen: i32 = 0;
            let mut mp_regen: i32 = 0;

            if player.hp < max_hp {
                // Base: 3% of Max HP + 1, mirroring `(Stats[HP] * 0.03F) + 1`.
                let base = ((max_hp as f32 * 0.03_f32) as i32).saturating_add(1);
                let recovery = player.stats.total.get(Stat::HealthRecovery).max(0);
                let bonus = if HEALTH_REGEN_WEIGHT > 0 {
                    (base as i64 * recovery as i64 / HEALTH_REGEN_WEIGHT as i64) as i32
                } else {
                    0
                };
                hp_regen = base.saturating_add(bonus);
            }

            if player.mp < max_mp {
                // Base: 3% of Max MP + 1, mirroring `(Stats[MP] * 0.03F) + 1`.
                let base = ((max_mp as f32 * 0.03_f32) as i32).saturating_add(1);
                let recovery = player.stats.total.get(Stat::SpellRecovery).max(0);
                let bonus = if MANA_REGEN_WEIGHT > 0 {
                    (base as i64 * recovery as i64 / MANA_REGEN_WEIGHT as i64) as i32
                } else {
                    0
                };
                mp_regen = base.saturating_add(bonus);
            }

            if hp_regen > 0 && max_hp > 0 {
                let old_hp = player.hp;
                let new_hp = old_hp.saturating_add(hp_regen).min(max_hp);
                let amount = new_hp.saturating_sub(old_hp);
                if amount > 0 {
                    player.hp = new_hp;
                    events.push(WorldEvent::PlayerHealed {
                        session_id: player.session_id,
                        map_index: player.map_index,
                        x: player.x,
                        y: player.y,
                        amount,
                        new_hp,
                    });
                }
            }

            if mp_regen > 0 && max_mp > 0 {
                let old_mp = player.mp;
                let new_mp = old_mp.saturating_add(mp_regen).min(max_mp);
                if new_mp > old_mp {
                    player.mp = new_mp;
                }
            }
        }
    }

    fn process_player_pk(&mut self, now_ms: i64) {
        const PK_DECAY_INTERVAL_MS: i64 = 12_000;

        for player in self.players.values_mut() {
            if player.pk_points > 0 {
                if player.next_pk_decay_ms == 0 || now_ms >= player.next_pk_decay_ms {
                    player.pk_points = player.pk_points.saturating_sub(1);
                    player.next_pk_decay_ms = now_ms.saturating_add(PK_DECAY_INTERVAL_MS);
                }
            } else {
                player.next_pk_decay_ms = 0;
            }
        }
    }

    fn process_monster_ai(&mut self, now_ms: i64, events: &mut Vec<WorldEvent>) {
        let player_positions: Vec<(u32, i32, i32, i32, u8)> = self
            .players
            .iter()
            .filter(|(_, p)| !p.dead && p.hp > 0)
            .map(|(&sid, p)| (sid, p.map_index, p.x, p.y, p.direction))
            .collect();

        let mut pending_attacks: Vec<(u64, i32, u32, i32)> = Vec::new();

        let mut rng = thread_rng();

        // Temporarily take ownership of the monsters map to avoid borrowing
        // conflicts while iterating. This allows us to call methods on `self`
        // (like get_or_load_map, is_cell_blocked, occupancy updates) inside
        // the loop.
        let mut monsters_map = std::mem::take(&mut self.monsters);

        for (map_index, monsters) in monsters_map.iter_mut() {
            let map_index = *map_index;

            // Load the map for this index so that movement checks can respect
            // walkability and bounds, mirroring the player movement logic.
            let map = match self.get_or_load_map(map_index) {
                Some(m) => m,
                None => continue,
            };

            for monster in monsters.iter_mut() {
                if monster.hp <= 0 {
                    monster.ai_state = MonsterAiState::Idle;
                    monster.target_session_id = None;
                    continue;
                }

                let (ai, view_range, move_speed, attack_speed) = {
                    let Some(info) = self.provider.get_monster_info(monster.monster_index) else {
                        monster.ai_state = MonsterAiState::Idle;
                        monster.target_session_id = None;
                        continue;
                    };
                    (
                        info.ai,
                        info.view_range as i32,
                        info.move_speed,
                        info.attack_speed,
                    )
                };

                if matches!(ai, 6 | 57 | 58 | 102 | 103 | 104 | 105 | 113) {
                    monster.ai_state = MonsterAiState::Idle;
                    monster.target_session_id = None;
                    continue;
                }

                // Use the monster's configured view range to decide when to
                // start chasing nearby players.
                if view_range <= 0 {
                    monster.ai_state = MonsterAiState::Idle;
                    monster.target_session_id = None;
                    continue;
                }

                // -----------------------------
                // CheckAlone: periodically determine whether this monster has
                // any players nearby on its map. Mirrors C#
                // MonsterObject.CheckAlone, which uses AloneTime/AloneDelay and
                // Globals.DataRange * 2 to decide if the monster should
                // consider itself "alone" and potentially skip AI work when no
                // players are around.
                // -----------------------------
                const ALONE_DELAY_MS: i64 = 3_000; // C# AloneDelay
                const ALONE_RANGE: i32 = 32; // Globals.DataRange (16) * 2

                if now_ms >= monster.alone_time_ms {
                    monster.alone_time_ms = now_ms.saturating_add(ALONE_DELAY_MS);

                    let mut has_player_on_map = false;
                    let mut near_player = false;
                    for (_sid, p_map, px, py, _dir) in player_positions.iter().copied() {
                        if p_map != map_index {
                            continue;
                        }

                        has_player_on_map = true;
                        let dx = px - monster.x;
                        let dy = py - monster.y;
                        if dx.abs() <= ALONE_RANGE && dy.abs() <= ALONE_RANGE {
                            near_player = true;
                            break;
                        }
                    }

                    if !has_player_on_map {
                        monster.alone = true;
                    } else {
                        monster.alone = !near_player;
                    }
                }

                // In C#: ProcessAI only runs ProcessSearch/ProcessRoam/ProcessTarget
                // when !Alone or Settings.MonsterProcessWhenAlone. We currently
                // treat MonsterProcessWhenAlone as false, so skip AI when alone
                // to match default behaviour.
                if monster.alone {
                    continue;
                }

                // -----------------------------
                // ProcessSearch: periodically (SearchDelay) try to acquire or
                // refresh a target, mirroring C# MonsterObject.ProcessSearch.
                // -----------------------------
                if now_ms >= monster.search_time_ms {
                    monster.search_time_ms = now_ms.saturating_add(3_000);

                    if !matches!(ai, 1 | 2 | 3) {
                        let should_search = monster.target_session_id.is_none()
                            || rng.gen_range(0..3) == 0;

                        if should_search {
                            let mut best: Option<(i32, u32)> = None;
                            for (sid, p_map, px, py, _dir) in player_positions.iter().copied() {
                                if p_map != map_index {
                                    continue;
                                }

                                let dx = px - monster.x;
                                let dy = py - monster.y;
                                if dx.abs() > view_range || dy.abs() > view_range {
                                    continue;
                                }

                                let dist = dx.abs().max(dy.abs());
                                match best {
                                    None => best = Some((dist, sid)),
                                    Some((best_dist, _)) if dist < best_dist => {
                                        best = Some((dist, sid));
                                    }
                                    _ => {}
                                }
                            }

                            if let Some((_dist, sid)) = best {
                                monster.ai_state = MonsterAiState::Chase;
                                monster.target_session_id = Some(sid);
                            } else {
                                monster.ai_state = MonsterAiState::Idle;
                                monster.target_session_id = None;
                            }
                        }
                    }
                }

                // -----------------------------
                // ProcessRoam: when no target, occasionally perform a random
                // turn or step, mirroring C# MonsterObject.ProcessRoam.
                // -----------------------------
                if monster.target_session_id.is_none() {
                    if now_ms >= monster.roam_time_ms {
                        monster.roam_time_ms = now_ms.saturating_add(1_000);

                        // Only roam 1/10 of the time when the timer fires.
                        if rng.gen_range(0..10) == 0 {
                            let choice = rng.gen_range(0..3);
                            if choice == 0 {
                                // Turn to a random direction without moving.
                                monster.direction = rng.gen_range(0..8);
                            } else {
                                // Walk one step in the current direction, but
                                // only if the destination tile is walkable on
                                // the current map, mirroring player movement.
                                let (step_x, step_y) = match monster.direction {
                                    0 => (0, -1),
                                    1 => (1, -1),
                                    2 => (1, 0),
                                    3 => (1, 1),
                                    4 => (0, 1),
                                    5 => (-1, 1),
                                    6 => (-1, 0),
                                    7 => (-1, -1),
                                    _ => (0, 0),
                                };

                                if step_x != 0 || step_y != 0 {
                                    let new_x = monster.x.saturating_add(step_x);
                                    let new_y = monster.y.saturating_add(step_y);

                                    // Reject moves that go outside the map
                                    // bounds. Monster positions are expected
                                    // to remain within 0..width/height.
                                    if new_x < 0 || new_y < 0 {
                                        continue;
                                    }

                                    if monster.x < 0 || monster.y < 0 {
                                        continue;
                                    }

                                    let from_x = monster.x as u16;
                                    let from_y = monster.y as u16;
                                    let to_x = new_x as u16;
                                    let to_y = new_y as u16;

                                    if to_x >= map.width || to_y >= map.height {
                                        continue;
                                    }

                                    if !map.can_move(from_x, from_y, to_x, to_y) {
                                        continue;
                                    }

                                    if self.is_cell_blocked(map_index, new_x, new_y) {
                                        continue;
                                    }

                                    self.remove_monster_from_occupancy(
                                        monster.id,
                                        map_index,
                                        monster.x,
                                        monster.y,
                                    );
                                    self.add_monster_to_occupancy(
                                        monster.id,
                                        map_index,
                                        new_x,
                                        new_y,
                                    );

                                    monster.x = new_x;
                                    monster.y = new_y;

                                    let delay_ms =
                                        Self::compute_monster_move_delay_ms(move_speed);
                                    monster.next_move_time_ms =
                                        now_ms.saturating_add(delay_ms);

                                    events.push(WorldEvent::ObjectLocation {
                                        object_id: monster.id,
                                        map_index,
                                        x: monster.x,
                                        y: monster.y,
                                        direction: monster.direction,
                                    });
                                }
                            }
                        }
                    }

                    // No target to chase this tick.
                    continue;
                }

                // -----------------------------
                // ProcessTarget: chase and melee attack logic when a target is
                // present, mirroring C# MonsterObject.ProcessTarget and
                // InAttackRange/MoveTo behaviour.
                // -----------------------------
                let target_sid = match monster.target_session_id {
                    Some(sid) => sid,
                    None => continue,
                };

                // Find the latest position of the current target.
                let mut target_pos: Option<(i32, i32)> = None;
                for (sid, p_map, px, py, _dir) in player_positions.iter().copied() {
                    if sid == target_sid && p_map == map_index {
                        target_pos = Some((px, py));
                        break;
                    }
                }

                let (tx, ty) = match target_pos {
                    Some(pos) => pos,
                    None => {
                        monster.ai_state = MonsterAiState::Idle;
                        monster.target_session_id = None;
                        continue;
                    }
                };

                let dx_full = tx - monster.x;
                let dy_full = ty - monster.y;
                let dist_full = dx_full.abs().max(dy_full.abs());

                // If in melee range (adjacent tile, not same cell), schedule an
                // attack instead of moving. This mirrors C#
                // MonsterObject.InAttackRange which checks
                //   Target.CurrentLocation != CurrentLocation &&
                //   Functions.InRange(CurrentLocation, Target.CurrentLocation, 1)
                // where InRange uses a Chebyshev distance. Attacks are gated by
                // a separate attack cooldown derived from MonsterInfo.AttackSpeed.
                if dist_full == 1 {
                    if now_ms >= monster.next_attack_time_ms {
                        let sx = dx_full.clamp(-1, 1);
                        let sy = dy_full.clamp(-1, 1);
                        let dir = match (sx, sy) {
                            (0, -1) => 0,
                            (1, -1) => 1,
                            (1, 0) => 2,
                            (1, 1) => 3,
                            (0, 1) => 4,
                            (-1, 1) => 5,
                            (-1, 0) => 6,
                            (-1, -1) => 7,
                            _ => monster.direction,
                        };
                        monster.direction = dir;

                        events.push(WorldEvent::ObjectAttack {
                            session_id: monster.id as u32,
                            map_index,
                            x: monster.x,
                            y: monster.y,
                            direction: dir,
                            spell: 0,
                            level: 0,
                            attack_type: 0,
                        });

                        pending_attacks.push((
                            monster.id,
                            map_index,
                            target_sid,
                            monster.monster_index,
                        ));

                        let delay_ms = Self::compute_monster_attack_delay_ms(attack_speed);
                        monster.next_attack_time_ms = now_ms.saturating_add(delay_ms);
                    }

                    // When already in attack range we do not move this tick.
                    continue;
                }

                // Step at most one tile towards the target (8-directional),
                // respecting the per-monster movement cooldown.
                if monster.next_move_time_ms != 0 && now_ms < monster.next_move_time_ms {
                    continue;
                }

                // Compute the primary direction towards the target using the
                // same 8-way scheme as the C# server, then attempt to walk in
                // that direction. If blocked (by map tiles or other
                // blocking objects), try up to 7 alternative directions by
                // rotating clockwise or counter-clockwise, mirroring
                // MonsterObject.MoveTo/Walk behaviour.

                let base_step_x = dx_full.clamp(-1, 1);
                let base_step_y = dy_full.clamp(-1, 1);

                // If we cannot determine a primary step, skip movement.
                if base_step_x == 0 && base_step_y == 0 {
                    continue;
                }

                let base_dir: u8 = match (base_step_x, base_step_y) {
                    (0, -1) => 0,
                    (1, -1) => 1,
                    (1, 0) => 2,
                    (1, 1) => 3,
                    (0, 1) => 4,
                    (-1, 1) => 5,
                    (-1, 0) => 6,
                    (-1, -1) => 7,
                    _ => monster.direction,
                };

                let mut candidate_dirs: [u8; 8] = [0; 8];
                candidate_dirs[0] = base_dir;

                // Randomise whether we explore clockwise or counter-clockwise
                // first, as in the C# MoveTo implementation.
                let mut dir = base_dir;
                if rng.gen_range(0..2) == 0 {
                    for i in 1..8 {
                        dir = (dir + 1) & 7;
                        candidate_dirs[i] = dir;
                    }
                } else {
                    for i in 1..8 {
                        dir = dir.wrapping_sub(1) & 7;
                        candidate_dirs[i] = dir;
                    }
                }

                let mut moved = false;

                for &dir in &candidate_dirs {
                    let (step_x, step_y) = match dir {
                        0 => (0, -1),
                        1 => (1, -1),
                        2 => (1, 0),
                        3 => (1, 1),
                        4 => (0, 1),
                        5 => (-1, 1),
                        6 => (-1, 0),
                        7 => (-1, -1),
                        _ => (0, 0),
                    };

                    if step_x == 0 && step_y == 0 {
                        continue;
                    }

                    let new_x = monster.x.saturating_add(step_x);
                    let new_y = monster.y.saturating_add(step_y);
                    if new_x < 0 || new_y < 0 {
                        continue;
                    }

                    if monster.x < 0 || monster.y < 0 {
                        continue;
                    }

                    let from_x = monster.x as u16;
                    let from_y = monster.y as u16;
                    let to_x = new_x as u16;
                    let to_y = new_y as u16;

                    if to_x >= map.width || to_y >= map.height {
                        continue;
                    }

                    if !map.can_move(from_x, from_y, to_x, to_y) {
                        continue;
                    }

                    if self.is_cell_blocked(map_index, new_x, new_y) {
                        continue;
                    }

                    self.remove_monster_from_occupancy(
                        monster.id,
                        map_index,
                        monster.x,
                        monster.y,
                    );
                    self.add_monster_to_occupancy(
                        monster.id,
                        map_index,
                        new_x,
                        new_y,
                    );

                    monster.x = new_x;
                    monster.y = new_y;
                    monster.direction = dir;

                    let delay_ms = Self::compute_monster_move_delay_ms(move_speed);
                    monster.next_move_time_ms = now_ms.saturating_add(delay_ms);

                    events.push(WorldEvent::ObjectLocation {
                        object_id: monster.id,
                        map_index,
                        x: monster.x,
                        y: monster.y,
                        direction: monster.direction,
                    });

                    moved = true;
                    break;
                }

                if !moved {
                    // All directions blocked this tick; monster stays put.
                }
            }
        }

        // Restore the monsters map to the world state.
        self.monsters = monsters_map;

        // Resolve pending monster attacks against players after we finish
        // iterating over the monsters map to avoid borrow conflicts.
        for (monster_id, map_index, target_sid, monster_index) in pending_attacks {
            let Some(player) = self.players.get_mut(&target_sid) else {
                continue;
            };

            if player.dead || player.hp <= 0 {
                continue;
            }

            let defender_stats: Stats = player.stats.total.clone();
            let max_hp = defender_stats.get(Stat::HP).max(1);
            if max_hp <= 0 {
                continue;
            }

            let Some(info) = self.provider.get_monster_info(monster_index) else {
                continue;
            };

            // Start from the static MonsterInfo stats and layer on any
            // per-instance buff_stats that are currently active on this
            // monster instance, mirroring C# MonsterObject.RefreshBuffs where
            // Buff.Stats are added on top of base stats.
            let mut attacker_stats: Stats = info.stats.clone();
            if let Some(monsters) = self.monsters.get(&map_index) {
                if let Some(m) = monsters.iter().find(|m| m.id == monster_id) {
                    attacker_stats.add(&m.buff_stats);
                }
            }
            let (hit, raw_damage, raw_damage_type) =
                compute_physical_melee_with_crit(&attacker_stats, &defender_stats);

            // Translate the helper's result into a concrete outcome.
            //
            // - When `hit` is false (Accuracy/Agility check failed) we emit a
            //   Miss (damage_type = 1, damage = 0) without changing the
            //   player's HP, mirroring C#'s BroadcastDamageIndicator(Miss).
            // - When `hit` is true but raw_damage <= 0 (e.g. very high AC/DR)
            //   we treat it as a fully absorbed hit and skip emitting any
            //   event.
            let (damage, damage_type, new_hp) = if !hit {
                let old_hp = player.hp.max(0);
                (0, 1_u8, old_hp)
            } else {
                if raw_damage <= 0 {
                    continue;
                }

                let damage = raw_damage;
                let old_hp = player.hp.max(0);
                let new_hp = old_hp.saturating_sub(damage).max(0);
                if new_hp == old_hp {
                    continue;
                }

                player.hp = new_hp;
                if new_hp <= 0 {
                    player.dead = true;
                }

                (damage, raw_damage_type, new_hp)
            };

            let health_percent = ((new_hp as i64 * 100) / max_hp as i64)
                .clamp(0, 100) as u8;

            events.push(WorldEvent::MonsterHitPlayer {
                attacker_monster_id: monster_id,
                session_id: target_sid,
                map_index,
                x: player.x,
                y: player.y,
                direction: player.direction,
                damage,
                damage_type,
                health_percent,
            });
        }
    }

    fn process_guard_ai(&mut self, now_ms: i64, events: &mut Vec<WorldEvent>) {
        const GUARD_AIS: [u8; 8] = [6, 57, 58, 102, 103, 104, 105, 113];
        let map_indices: Vec<i32> = self.monsters.keys().cloned().collect();
        let mut pending_kills: Vec<(i32, u64, i32, i32, u8)> = Vec::new();
        let player_positions: Vec<(u32, i32, i32, i32, u8, i32)> = self
            .players
            .iter()
            .filter(|(_, p)| !p.dead && p.hp > 0)
            .map(|(&sid, p)| (sid, p.map_index, p.x, p.y, p.direction, p.pk_points))
            .collect();
        let mut pending_guard_hits: Vec<(u64, i32, u32, i32)> = Vec::new();
        let mut pending_guard_moves: Vec<(u64, i32, i32, i32, i32, i32)> = Vec::new();

        for map_index in map_indices {
            // Load the map so that guard movement (both chase and patrol)
            // can respect bounds and walkability, mirroring the behaviour
            // used by the generic monster AI.
            let map = match self.get_or_load_map(map_index) {
                Some(m) => m,
                None => continue,
            };

            let Some(monsters) = self.monsters.get_mut(&map_index) else {
                continue;
            };

            let len = monsters.len();
            for i in 0..len {
                let (
                    attack_range,
                    attack_delay_ms,
                    guard_x,
                    guard_y,
                    guard_ai,
                    guard_monster_index,
                    guard_move_speed,
                ) = {
                    let m = &monsters[i];

                    if m.hp <= 0 {
                        continue;
                    }

                    let Some(info) = self.provider.get_monster_info(m.monster_index) else {
                        continue;
                    };

                    if !GUARD_AIS.contains(&info.ai) {
                        continue;
                    }

                    let range: i32 = match info.ai {
                        // IceGuard uses a fixed AttackRange of 8 in C#.
                        102 => 8,
                        // KingGuard uses a fixed AttackRange of 10.
                        105 => 10,
                        _ => info.view_range as i32,
                    }
                    .max(1);

                    let delay_ms = Self::compute_monster_attack_delay_ms(info.attack_speed);

                    (range, delay_ms, m.x, m.y, info.ai, m.monster_index, info.move_speed)
                };

                if now_ms < monsters[i].next_attack_time_ms {
                    continue;
                }

                // Select the best monster target within range (for instant kills).
                let mut best_monster: Option<(usize, i32)> = None;

                for j in 0..len {
                    if j == i {
                        continue;
                    }

                    let t = &monsters[j];
                    if t.hp <= 0 {
                        continue;
                    }

                    let Some(t_info) = self.provider.get_monster_info(t.monster_index) else {
                        continue;
                    };

                    if GUARD_AIS.contains(&t_info.ai) {
                        continue;
                    }

                    let dx = t.x - guard_x;
                    let dy = t.y - guard_y;
                    let dist = dx.abs().max(dy.abs());
                    if dist == 0 || dist > attack_range {
                        continue;
                    }

                    match best_monster {
                        None => best_monster = Some((j, dist)),
                        Some((_, best_dist)) if dist < best_dist => {
                            best_monster = Some((j, dist));
                        }
                        _ => {}
                    }
                }

                // Optionally select the best red-name player target for PK guards.
                let mut best_player: Option<(u32, i32, i32, u8, i32)> = None;
                let can_attack_players = matches!(guard_ai, 6 | 58 | 113);

                if can_attack_players {
                    for (sid, p_map, px, py, p_dir, pk_points) in
                        player_positions.iter().copied()
                    {
                        if p_map != map_index {
                            continue;
                        }

                        if pk_points < 200 {
                            continue;
                        }

                        let dx = px - guard_x;
                        let dy = py - guard_y;
                        let dist = dx.abs().max(dy.abs());
                        if dist == 0 || dist > attack_range {
                            continue;
                        }

                        match best_player {
                            None => best_player = Some((sid, px, py, p_dir, dist)),
                            Some((_, _, _, _, best_dist)) if dist < best_dist => {
                                best_player = Some((sid, px, py, p_dir, dist));
                            }
                            _ => {}
                        }
                    }
                }

                // Choose between a monster or player target based on distance
                // (prefer the closer one; on ties, prefer players to punish PK).
                let mut target_is_player = false;
                let mut target_id: u64 = 0;
                let mut target_sid: u32 = 0;
                let target_x: i32;
                let target_y: i32;
                let target_dir: u8;

                match (best_player, best_monster) {
                    (Some((sid, px, py, p_dir, p_dist)), Some((m_idx, m_dist))) => {
                        if p_dist <= m_dist {
                            target_is_player = true;
                            target_sid = sid;
                            target_x = px;
                            target_y = py;
                            target_dir = p_dir;
                        } else {
                            let t = &monsters[m_idx];
                            target_id = t.id;
                            target_x = t.x;
                            target_y = t.y;
                            target_dir = t.direction;
                        }
                    }
                    (Some((sid, px, py, p_dir, _)), None) => {
                        target_is_player = true;
                        target_sid = sid;
                        target_x = px;
                        target_y = py;
                        target_dir = p_dir;
                    }
                    (None, Some((m_idx, _))) => {
                        let t = &monsters[m_idx];
                        target_id = t.id;
                        target_x = t.x;
                        target_y = t.y;
                        target_dir = t.direction;
                    }
                    (None, None) => {
                        // No valid target this tick; attempt patrol for
                        // stationary guard types that are configured with
                        // a route (Guard/TownArcher/TaoGuard).
                        if matches!(guard_ai, 6 | 57 | 58) {
                            if let Some(respawns) = self.respawns.get(&map_index) {
                                let guard_respawn_index = monsters[i].respawn_index;
                                if let Some(rt) = respawns
                                    .iter()
                                    .find(|rt| rt.info.respawn_index == guard_respawn_index)
                                {
                                    if let Some(route_points) = &rt.route_points {
                                        if !route_points.is_empty() {
                                            let guard = &mut monsters[i];

                                            // Respect per-waypoint delay.
                                            if guard.route_wait_until_ms > 0
                                                && now_ms < guard.route_wait_until_ms
                                            {
                                                // Still waiting at current waypoint.
                                            } else {
                                                let len = route_points.len() as i32;
                                                if guard.route_index < 0
                                                    || guard.route_index >= len
                                                {
                                                    guard.route_index = 0;
                                                }

                                                let idx = guard.route_index as usize;
                                                let point = &route_points[idx];

                                                if guard.x == point.x && guard.y == point.y {
                                                    // Arrived: apply delay and advance to next
                                                    // waypoint.
                                                    if point.delay_ms > 0 {
                                                        guard.route_wait_until_ms = now_ms
                                                            .saturating_add(point.delay_ms);
                                                    } else {
                                                        guard.route_wait_until_ms = 0;
                                                    }

                                                    let mut next = guard.route_index + 1;
                                                    if next >= len {
                                                        next = 0;
                                                    }
                                                    guard.route_index = next;
                                                } else {
                                                    // Step one tile towards the current waypoint,
                                                    // gated by the monster's movement cooldown.
                                                    if guard.next_move_time_ms == 0
                                                        || now_ms >= guard.next_move_time_ms
                                                    {
                                                        let dx = point.x - guard.x;
                                                        let dy = point.y - guard.y;
                                                        let step_x = dx.clamp(-1, 1);
                                                        let step_y = dy.clamp(-1, 1);

                                                        if step_x != 0 || step_y != 0 {
                                                            let new_x = guard.x.saturating_add(step_x);
                                                            let new_y = guard.y.saturating_add(step_y);

                                                            // Reject patrol moves that go
                                                            // outside map bounds or into
                                                            // blocked cells, mirroring the
                                                            // generic monster movement checks.
                                                            if new_x < 0 || new_y < 0 {
                                                                continue;
                                                            }
                                                            if guard.x < 0 || guard.y < 0 {
                                                                continue;
                                                            }

                                                            let from_x = guard.x as u16;
                                                            let from_y = guard.y as u16;
                                                            let to_x = new_x as u16;
                                                            let to_y = new_y as u16;

                                                            if to_x >= map.width || to_y >= map.height {
                                                                continue;
                                                            }

                                                            if !map.can_move(from_x, from_y, to_x, to_y)
                                                            {
                                                                continue;
                                                            }

                                                            let old_x = guard.x;
                                                            let old_y = guard.y;
                                                            guard.x = new_x;
                                                            guard.y = new_y;

                                                            pending_guard_moves.push((
                                                                guard.id,
                                                                map_index,
                                                                old_x,
                                                                old_y,
                                                                new_x,
                                                                new_y,
                                                            ));

                                                            let delay = Self::
                                                                compute_monster_move_delay_ms(
                                                                    guard_move_speed,
                                                                );
                                                            guard.next_move_time_ms = now_ms
                                                                .saturating_add(delay);

                                                            events.push(
                                                                WorldEvent::ObjectLocation {
                                                                    object_id: guard.id,
                                                                    map_index,
                                                                    x: guard.x,
                                                                    y: guard.y,
                                                                    direction: guard.direction,
                                                                },
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        continue;
                    }
                }

                // For chase-type guards (IceGuard, ElementGuard, DemonGuard,
                // KingGuard), move one tile towards the target when it is
                // currently outside their attack range, mirroring the C#
                // MoveTo(Target.CurrentLocation) behaviour in
                // IceGuard/ElementGuard/KingGuard.ProcessTarget and the base
                // MonsterObject logic used by DemonGuard.
                if matches!(guard_ai, 102 | 103 | 104 | 105) {
                    let dist = (target_x - guard_x).abs().max((target_y - guard_y).abs());
                    if dist > attack_range {
                        let guard = &mut monsters[i];

                        if guard.next_move_time_ms == 0 || now_ms >= guard.next_move_time_ms {
                            let dx = target_x - guard.x;
                            let dy = target_y - guard.y;
                            let step_x = dx.clamp(-1, 1);
                            let step_y = dy.clamp(-1, 1);

                            if step_x != 0 || step_y != 0 {
                                let new_x = guard.x.saturating_add(step_x);
                                let new_y = guard.y.saturating_add(step_y);

                                // Respect map bounds and walkability when
                                // chasing, just like generic monsters do.
                                if new_x < 0 || new_y < 0 {
                                    continue;
                                }
                                if guard.x < 0 || guard.y < 0 {
                                    continue;
                                }

                                let from_x = guard.x as u16;
                                let from_y = guard.y as u16;
                                let to_x = new_x as u16;
                                let to_y = new_y as u16;

                                if to_x >= map.width || to_y >= map.height {
                                    continue;
                                }

                                if !map.can_move(from_x, from_y, to_x, to_y)
                                {
                                    continue;
                                }

                                let old_x = guard.x;
                                let old_y = guard.y;
                                guard.x = new_x;
                                guard.y = new_y;

                                pending_guard_moves.push((
                                    guard.id,
                                    map_index,
                                    old_x,
                                    old_y,
                                    new_x,
                                    new_y,
                                ));

                                let delay =
                                    Self::compute_monster_move_delay_ms(guard_move_speed);
                                guard.next_move_time_ms =
                                    now_ms.saturating_add(delay);

                                events.push(WorldEvent::ObjectLocation {
                                    object_id: guard.id,
                                    map_index,
                                    x: guard.x,
                                    y: guard.y,
                                    direction: guard.direction,
                                });
                            }
                        }

                        // Skip attacking this tick; the guard has spent its
                        // action moving towards the target instead.
                        continue;
                    }
                }

                let guard_id;
                let guard_x;
                let guard_y;
                let guard_dir;
                {
                    let guard = &mut monsters[i];
                    guard_id = guard.id;
                    guard_x = guard.x;
                    guard_y = guard.y;

                    let dx = target_x - guard.x;
                    let dy = target_y - guard.y;
                    let sx = dx.clamp(-1, 1);
                    let sy = dy.clamp(-1, 1);
                    guard.direction = match (sx, sy) {
                        (0, -1) => 0,
                        (1, -1) => 1,
                        (1, 0) => 2,
                        (1, 1) => 3,
                        (0, 1) => 4,
                        (-1, 1) => 5,
                        (-1, 0) => 6,
                        (-1, -1) => 7,
                        _ => guard.direction,
                    };
                    guard.next_attack_time_ms = now_ms.saturating_add(attack_delay_ms);
                    guard_dir = guard.direction;
                }

                // Emit the guard's attack using its own position and facing,
                // mirroring C# where ObjectAttack carries the attacker's
                // location and direction rather than the strike tile. The
                // client derives hit cells from this data.
                events.push(WorldEvent::ObjectAttack {
                    session_id: guard_id as u32,
                    map_index,
                    x: guard_x,
                    y: guard_y,
                    direction: guard_dir,
                    spell: 0,
                    level: 0,
                    attack_type: 0,
                });

                if target_is_player {
                    pending_guard_hits.push((
                        guard_id,
                        map_index,
                        target_sid,
                        guard_monster_index,
                    ));
                } else {
                    pending_kills.push((map_index, target_id, target_x, target_y, target_dir));
                }
            }
        }
        // Apply deferred occupancy updates for guards that moved along their
        // patrol routes. These must be done outside the main loop to avoid
        // borrowing self.monsters mutably at the same time as calling
        // occupancy helpers that also take &mut self.
        for (guard_id, map_index, old_x, old_y, new_x, new_y) in pending_guard_moves {
            self.remove_monster_from_occupancy(guard_id, map_index, old_x, old_y);
            self.add_monster_to_occupancy(guard_id, map_index, new_x, new_y);
        }
        for (map_index, target_id, x, y, direction) in pending_kills {
            // For guard-instakilled monsters, mark them as dead and clear
            // their AI/target state, but keep the MonsterInstance in the
            // monsters list so the client can continue to render a corpse
            // after receiving SObjectDied. We also remove dynamic occupancy so
            // that the corpse no longer blocks movement.
            if let Some(monsters) = self.monsters.get_mut(&map_index) {
                if let Some(m) = monsters.iter_mut().find(|m| m.id == target_id) {
                    if m.hp > 0 {
                        m.hp = 0;
                    }
                    m.ai_state = MonsterAiState::Idle;
                    m.target_session_id = None;
                }
            }

            // Use the strike coordinates captured in pending_kills for
            // occupancy removal to avoid needing a second mutable borrow of
            // self.monsters here.
            self.remove_monster_from_occupancy(target_id, map_index, x, y);

            events.push(WorldEvent::MonsterDied {
                object_id: target_id,
                map_index,
                x,
                y,
                direction,
            });
        }

        // Resolve pending guard hits against players using the same
        // MonsterHitPlayer model as generic monster AI.
        for (monster_id, map_index, target_sid, monster_index) in pending_guard_hits {
            let Some(player) = self.players.get_mut(&target_sid) else {
                continue;
            };

            if player.dead || player.hp <= 0 {
                continue;
            }

            let defender_stats: Stats = player.stats.total.clone();
            let max_hp = defender_stats.get(Stat::HP).max(1);
            if max_hp <= 0 {
                continue;
            }

            let Some(info) = self.provider.get_monster_info(monster_index) else {
                continue;
            };

            let mut attacker_stats: Stats = info.stats.clone();
            if let Some(monsters) = self.monsters.get(&map_index) {
                if let Some(m) = monsters.iter().find(|m| m.id == monster_id) {
                    attacker_stats.add(&m.buff_stats);
                }
            }

            let (hit, raw_damage, raw_damage_type) =
                compute_physical_melee_with_crit(&attacker_stats, &defender_stats);

            let (damage, damage_type, new_hp) = if !hit {
                let old_hp = player.hp.max(0);
                (0, 1_u8, old_hp)
            } else {
                if raw_damage <= 0 {
                    continue;
                }

                let damage = raw_damage;
                let old_hp = player.hp.max(0);
                let new_hp = old_hp.saturating_sub(damage).max(0);
                if new_hp == old_hp {
                    continue;
                }

                player.hp = new_hp;
                if new_hp <= 0 {
                    player.dead = true;
                }

                (damage, raw_damage_type, new_hp)
            };

            let health_percent = ((new_hp as i64 * 100) / max_hp as i64)
                .clamp(0, 100) as u8;

            events.push(WorldEvent::MonsterHitPlayer {
                attacker_monster_id: monster_id,
                session_id: target_sid,
                map_index,
                x: player.x,
                y: player.y,
                direction: player.direction,
                damage,
                damage_type,
                health_percent,
            });
        }
    }

    fn update_respawn_tick_counter(&mut self, now_ms: i64) {
        let base_minutes = i64::from(self.respawn_base_spawn_rate_minutes.max(1));
        let tick_ms = base_minutes.saturating_mul(60_000);
        if tick_ms <= 0 {
            return;
        }

        if self.respawn_last_tick_ms == 0 {
            self.respawn_last_tick_ms = now_ms;
            return;
        }

        if now_ms <= self.respawn_last_tick_ms {
            return;
        }

        let elapsed = now_ms.saturating_sub(self.respawn_last_tick_ms);
        if elapsed < tick_ms {
            return;
        }

        let ticks = (elapsed / tick_ms) as u64;
        if ticks == 0 {
            return;
        }

        self.respawn_tick_counter = self.respawn_tick_counter.wrapping_add(ticks);
        let advance = (ticks as i64).saturating_mul(tick_ms);
        self.respawn_last_tick_ms = self
            .respawn_last_tick_ms
            .saturating_add(advance);
    }

    fn compute_respawn_delay_ms(info: &RespawnInfo) -> i64 {
        let base = info.delay as i64;
        let random = info.random_delay as i64;

        let minutes = if random > 0 {
            let mut rng = thread_rng();
            let span = random.saturating_mul(2);
            let delta = rng.gen_range(0..span);
            let total = base.saturating_sub(random).saturating_add(delta);
            if total <= 0 {
                1_i64
            } else {
                total
            }
        } else if base <= 0 {
            1_i64
        } else {
            base
        };

        minutes.saturating_mul(60_000)
    }

    fn compute_monster_move_delay_ms(move_speed: u16) -> i64 {
        let speed = i64::from(move_speed.max(400));
        if speed <= 0 {
            400
        } else {
            speed
        }
    }

    fn compute_monster_attack_delay_ms(attack_speed: u16) -> i64 {
        // AttackSpeed in C# is in milliseconds and clamped to a minimum of
        // 400. We mirror that behaviour here.
        let speed = i64::from(attack_speed.max(400));
        if speed <= 0 {
            400
        } else {
            speed
        }
    }

    fn create_monsters_from_respawn(
        &mut self,
        map: &map::Map,
        respawn: &RespawnInfo,
    ) -> Vec<MonsterInstance> {
        let mut result = Vec::new();

        // Build candidate walkable cells within the configured spread.
        let spread = respawn.spread as i32;
        let mut candidates: Vec<(i32, i32)> = Vec::new();

        for &(wx, wy) in &map.walkable_cells {
            let x = wx as i32;
            let y = wy as i32;
            if x <= respawn.location_x + spread
                && x >= respawn.location_x - spread
                && y <= respawn.location_y + spread
                && y >= respawn.location_y - spread
            {
                candidates.push((x, y));
            }
        }

        // No walkable candidates in range: mirror C# by skipping this respawn.
        if candidates.is_empty() {
            debug!(
                "respawn: no walkable cells for map={} respawn_index={} loc=({}, {}) spread={}",
                map.info.index,
                respawn.respawn_index,
                respawn.location_x,
                respawn.location_y,
                respawn.spread,
            );
            return result;
        }

        let needed = respawn.count as usize;
        let len = candidates.len();
        if len == 0 || needed == 0 {
            return result;
        }

        // Determine base HP for this monster type from MonsterInfo stats.
        let base_hp: i32 = self
            .provider
            .get_monster_info(respawn.monster_index)
            .map(|info| info.stats.get(Stat::HP))
            .unwrap_or(1)
            .max(1);

        let mut rng = thread_rng();

        // For each monster we want to spawn, try a bounded number of times to
        // find a candidate cell that is not currently occupied by any player or
        // monster.
        for _ in 0..needed {
            let mut placed = false;

            for _attempt in 0..8 {
                let idx = rng.gen_range(0..len);
                let (x, y) = candidates[idx];

                if self.is_cell_blocked(map.info.index, x, y) {
                    continue;
                }

                self.next_monster_id = self.next_monster_id.wrapping_add(1);

                result.push(MonsterInstance {
                    id: self.next_monster_id,
                    monster_index: respawn.monster_index,
                    map_index: map.info.index,
                    x,
                    y,
                    home_x: respawn.location_x,
                    home_y: respawn.location_y,
                    direction: respawn.direction,
                    hp: base_hp,
                    respawn_index: respawn.respawn_index,
                    ai_state: MonsterAiState::Idle,
                    target_session_id: None,
                    next_move_time_ms: 0,
                    next_attack_time_ms: 0,
                    search_time_ms: 0,
                    roam_time_ms: 0,
                    route_index: 0,
                    route_wait_until_ms: 0,
                    alone: false,
                    alone_time_ms: 0,
                    buff_stats: Stats::default(),
                    buffs: Vec::new(),
                });

                placed = true;
                break;
            }

            if !placed {
                trace!(
                    "respawn: all candidate cells blocked for map={} respawn_index={} loc=({}, {}) spread={}",
                    map.info.index,
                    respawn.respawn_index,
                    respawn.location_x,
                    respawn.location_y,
                    respawn.spread,
                );
            }
        }

        result
    }

    /// Kill the monster closest to the given world position on the specified
    /// map. Returns the monster id and its experience value if a target was
    /// found and removed.
    pub fn kill_nearest_monster(&mut self, map_index: i32, x: i32, y: i32) -> Option<(u64, u32)> {
        let target = match self.monsters.get(&map_index) {
            Some(monsters) => {
                let mut best: Option<(i64, u64, i32)> = None;
                for m in monsters {
                    let dx = m.x - x;
                    let dy = m.y - y;
                    let dist = (dx.abs() + dy.abs()) as i64;
                    match best {
                        None => best = Some((dist, m.id, m.monster_index)),
                        Some((best_dist, _, _)) if dist < best_dist => {
                            best = Some((dist, m.id, m.monster_index));
                        }
                        _ => {}
                    }
                }
                best.map(|(_, id, monster_index)| (id, monster_index))
            }
            None => None,
        };

        if let Some((id, monster_index)) = target {
            self.mark_monster_dead(map_index, id);

            let exp = self
                .provider
                .get_monster_info(monster_index)
                .map(|info| info.experience)
                .unwrap_or(0);

            Some((id, exp))
        } else {
            None
        }
    }

    /// Record that a monster has died or despawned, removing it from the
    /// world state and decrementing the corresponding RespawnRuntime
    /// current_count if we can locate a matching respawn_index.
    pub fn mark_monster_dead(&mut self, map_index: i32, monster_id: u64) {
        if let Some(monsters) = self.monsters.get_mut(&map_index) {
            if let Some(pos) = monsters.iter().position(|m| m.id == monster_id) {
                let inst = monsters.remove(pos);

                // Clear dynamic occupancy so that the cell is no longer
                // blocking for players or other monsters, mirroring C# where
                // Dead monsters do not block movement.
                self.remove_monster_from_occupancy(monster_id, map_index, inst.x, inst.y);

                if let Some(runtimes) = self.respawns.get_mut(&map_index) {
                    if let Some(rt) = runtimes
                        .iter_mut()
                        .find(|rt| rt.info.respawn_index == inst.respawn_index)
                    {
                        if rt.current_count > 0 {
                            rt.current_count -= 1;
                        }
                    }
                }
            }
        }
    }

    /// Process map items: remove expired items and emit MapItemRemoved events.
    /// This mirrors C# ItemObject.Process() behavior where items are removed
    /// when Envir.Time > ExpireTime.
    fn process_map_items(&mut self, now_ms: i64, events: &mut Vec<WorldEvent>) {
        let map_indices: Vec<i32> = self.map_items.keys().cloned().collect();

        for map_index in map_indices {
            if let Some(items) = self.map_items.get_mut(&map_index) {
                let mut to_remove: Vec<usize> = Vec::new();

                for (idx, item) in items.iter().enumerate() {
                    if now_ms > item.expire_time_ms {
                        to_remove.push(idx);
                    }
                }

                // Remove items in reverse order to maintain indices
                for &idx in to_remove.iter().rev() {
                    let removed = items.remove(idx);
                    events.push(WorldEvent::MapItemRemoved {
                        object_id: removed.id,
                        map_index: removed.map_index,
                        x: removed.x,
                        y: removed.y,
                    });
                }
            }
        }
    }
}


