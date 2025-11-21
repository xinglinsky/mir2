use crate::stats::Stat;
use crate::world::map::{self, RespawnInfo};
use crate::world::monster::MonsterInstance;
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
    /// buffs and other timed systems. For now it implements a minimal time-
    /// based respawn scheduler mirroring the C# Map.ProcessRespawns logic,
    /// but without yet wiring spawn multipliers or death/despawn updates.
    pub fn update(&mut self, now_ms: i64) -> Vec<WorldEvent> {
        self.time_ms = now_ms;

        // First pass: decide for each respawn runtime whether it should
        // attempt to spawn this tick, and when its next spawn time should be.
        // We only handle time-based respawns (respawn_ticks == 0) here.
        let mut jobs: Vec<(i32, usize, u16)> = Vec::new(); // (map_index, respawn_idx, needed_count)

        let map_indices: Vec<i32> = self.respawns.keys().cloned().collect();

        for map_index in &map_indices {
            if let Some(runtimes) = self.respawns.get_mut(map_index) {
                for (idx, rt) in runtimes.iter_mut().enumerate() {
                    if rt.info.respawn_ticks != 0 {
                        continue;
                    }

                    // Initialise next_spawn_time_ms if this respawn has never been
                    // scheduled before.
                    if rt.next_spawn_time_ms == 0 {
                        let delay_minutes = if rt.info.delay == 0 {
                            1_i64
                        } else {
                            rt.info.delay as i64
                        };
                        let delay_ms = delay_minutes * 60_000;
                        rt.next_spawn_time_ms = now_ms.saturating_add(delay_ms);
                        continue;
                    }

                    // Not yet time to respawn.
                    if now_ms < rt.next_spawn_time_ms {
                        continue;
                    }

                    // Target total monsters for this respawn point. SpawnMultiplier
                    // is treated as 1.0 for now.
                    let target_total = rt.info.count as u16;

                    if rt.current_count < target_total {
                        let needed = target_total - rt.current_count;
                        if needed > 0 {
                            jobs.push((*map_index, idx, needed));
                        }
                    }

                    // Schedule the next respawn time even if no monsters are
                    // actually spawned this tick, mirroring the C# behaviour.
                    let delay_minutes = if rt.info.delay == 0 {
                        1_i64
                    } else {
                        rt.info.delay as i64
                    };
                    let delay_ms = delay_minutes * 60_000;
                    rt.next_spawn_time_ms = now_ms.saturating_add(delay_ms);
                }
            }
        }

        // Second pass: execute spawn jobs. This is separated from the first
        // pass to avoid holding mutable borrows into self.respawns while
        // calling other &mut self methods such as create_monsters_from_respawn.
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

            if spawned.is_empty() {
                continue;
            }

            let actual = spawned.len() as u16;
            let monsters_on_map = self.monsters.entry(map_index).or_default();
            monsters_on_map.extend(spawned);

            if let Some(runtimes) = self.respawns.get_mut(&map_index) {
                if let Some(rt) = runtimes.get_mut(rt_index) {
                    rt.current_count = rt.current_count.saturating_add(actual);
                    rt.error_count = 0;
                }
            }
        }

        Vec::new()
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
            });
        }

        result
    }

    /// Kill the monster closest to the given world position on the specified
    /// map. Returns the monster id if a target was found and removed.
    pub fn kill_nearest_monster(&mut self, map_index: i32, x: i32, y: i32) -> Option<u64> {
        let target_id = match self.monsters.get(&map_index) {
            Some(monsters) => {
                let mut best: Option<(i64, u64)> = None;
                for m in monsters {
                    let dx = m.x - x;
                    let dy = m.y - y;
                    let dist = (dx.abs() + dy.abs()) as i64;
                    match best {
                        None => best = Some((dist, m.id)),
                        Some((best_dist, _)) if dist < best_dist => {
                            best = Some((dist, m.id));
                        }
                        _ => {}
                    }
                }
                best.map(|(_, id)| id)
            }
            None => None,
        };

        if let Some(id) = target_id {
            self.mark_monster_dead(map_index, id);
            Some(id)
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

