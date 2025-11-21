use rand::Rng;
use rand::thread_rng;
use tracing::debug;
use crate::stats::Stat;
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
            .map(|(&sid, p)| (sid, p.map_index, p.x, p.y, p.direction))
            .collect();

        for (map_index, monsters) in self.monsters.iter_mut() {
            let map_index = *map_index;

            for monster in monsters.iter_mut() {
                let Some(info) = self.provider.get_monster_info(monster.monster_index) else {
                    monster.ai_state = MonsterAiState::Idle;
                    monster.target_session_id = None;
                    continue;
                };

                let view_range = info.view_range as i32;
                if view_range <= 0 {
                    monster.ai_state = MonsterAiState::Idle;
                    monster.target_session_id = None;
                    continue;
                }

                // Pick nearest player in view on the same map.
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

                    let dist = dx.abs() + dy.abs();
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
                    continue;
                }

                if monster.ai_state != MonsterAiState::Chase {
                    continue;
                }

                let target_sid = match monster.target_session_id {
                    Some(sid) => sid,
                    None => continue,
                };

                if monster.next_move_time_ms != 0 && now_ms < monster.next_move_time_ms {
                    continue;
                }

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

                // Step at most one tile towards the target (8-directional).
                let step_x = (tx - monster.x).clamp(-1, 1);
                let step_y = (ty - monster.y).clamp(-1, 1);
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

                if new_x > i32::from(u16::MAX) || new_y > i32::from(u16::MAX) {
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
        let speed = move_speed.max(1) as i64;
        speed.saturating_mul(10)
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

        for i in 0..needed {
            let (x, y) = candidates[i % len];

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
}

