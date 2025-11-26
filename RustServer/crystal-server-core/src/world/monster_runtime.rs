use rand::Rng;
use rand::thread_rng;
use tracing::debug;
use crate::combat::compute_physical_melee_with_crit;
use crate::stats::{Stat, Stats};
use crate::world::map::{self, RespawnInfo};
use crate::world::monster::{MonsterAiState, MonsterInstance};
use crate::world::provider::WorldProvider;

use super::{World, WorldEvent};

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

        self.monsters.insert(map_index, instances);
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

        // After maintaining respawn counts, run per-monster AI. In this
        // initial phase the AI only selects targets and updates internal
        // state; movement and active attacks will be layered on later.
        self.process_monster_ai(now_ms, &mut events);

        events
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

        // Clone map indices so we can load maps (&self) before borrowing
        // the monsters vector mutably for each map.
        let map_indices: Vec<i32> = self.monsters.keys().cloned().collect();

        for map_index in map_indices {
            // Load the map for this index so that movement checks can respect
            // walkability and bounds, mirroring the player movement logic.
            let map = match self.get_or_load_map(map_index) {
                Some(m) => m,
                None => continue,
            };

            let Some(monsters) = self.monsters.get_mut(&map_index) else {
                continue;
            };

            for monster in monsters.iter_mut() {
                let Some(info) = self.provider.get_monster_info(monster.monster_index) else {
                    monster.ai_state = MonsterAiState::Idle;
                    monster.target_session_id = None;
                    continue;
                };

                let ai = info.ai;
                if matches!(ai, 6 | 57 | 58 | 102 | 103 | 104 | 105 | 113) {
                    monster.ai_state = MonsterAiState::Idle;
                    monster.target_session_id = None;
                    continue;
                }

                // Use the monster's configured view range to decide when to
                // start chasing nearby players.
                let view_range = info.view_range as i32;
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

                                    monster.x = new_x;
                                    monster.y = new_y;

                                    let delay_ms =
                                        Self::compute_monster_move_delay_ms(info.move_speed);
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

                        pending_attacks.push((
                            monster.id,
                            map_index,
                            target_sid,
                            monster.monster_index,
                        ));

                        let delay_ms = Self::compute_monster_attack_delay_ms(info.attack_speed);
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

                // Step at most one tile towards the target (8-directional).
                let step_x = dx_full.clamp(-1, 1);
                let step_y = dy_full.clamp(-1, 1);
                if step_x == 0 && step_y == 0 {
                    continue;
                }

                let dir = match (step_x, step_y) {
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

                monster.x = new_x;
                monster.y = new_y;
                monster.direction = dir;

                let delay_ms = Self::compute_monster_move_delay_ms(info.move_speed);
                monster.next_move_time_ms = now_ms.saturating_add(delay_ms);

                events.push(WorldEvent::ObjectLocation {
                    object_id: monster.id,
                    map_index,
                    x: monster.x,
                    y: monster.y,
                    direction: monster.direction,
                });
            }
        }
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
            let attacker_stats: Stats = info.stats.clone();
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

    fn create_monsters_from_respawn(&mut self, map: &map::Map, respawn: &RespawnInfo) -> Vec<MonsterInstance> {
        let mut result = Vec::new();

        // Strictly mirror the C# logic:
        //   info.WalkableCells = WalkableCells.Where(x =>
        //       x.X <= Info.Location.X + Info.Spread &&
        //       x.X >= Info.Location.X - Info.Spread &&
        //       x.Y <= Info.Location.Y + Info.Spread &&
        //       x.Y >= Info.Location.Y - Info.Spread).ToList();
        //
        // and then MonsterObject.Spawn(MapRespawn) picks a random point from
        // Respawn.WalkableCells for each spawned monster. Here we build the
        // candidate list from map.walkable_cells and then distribute Count
        // monsters across those cells in a deterministic but equivalent way.

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

        // If there are no walkable cells in range, C# 的 MapRespawn 会得到
        // 一个空的 WalkableCells 列表，MonsterObject.Spawn 返回 false，
        // 该 Respawn 点不会刷怪，这里也保持相同行为：直接返回空列表。
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

        for _ in 0..needed {
            let idx = rng.gen_range(0..len);
            let (x, y) = candidates[idx];

            self.next_monster_id = self.next_monster_id.wrapping_add(1);

            result.push(MonsterInstance {
                id: self.next_monster_id,
                monster_index: respawn.monster_index,
                map_index: map.info.index,
                x,
                y,
                direction: respawn.direction,
                hp: base_hp,
                respawn_index: respawn.respawn_index,
                ai_state: MonsterAiState::Idle,
                target_session_id: None,
                next_move_time_ms: 0,
                next_attack_time_ms: 0,
                search_time_ms: 0,
                roam_time_ms: 0,
                alone: false,
                alone_time_ms: 0,
            });
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


