use std::collections::HashMap;

use tracing::debug;
use crate::world::config::WorldConfig;
use crate::world::map::{self, CellAttribute, RespawnInfo};
use crate::world::monster::MonsterInstance;
use crate::world::provider::WorldProvider;
use crate::world::magic::UserMagic;

pub type SessionId = u32;

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub session_id: SessionId,
    pub map_index: i32,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub magics: Vec<UserMagic>,
}

#[derive(Clone, Debug)]
pub enum WorldCommand {
    StartGame {
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
        magics: Vec<UserMagic>,
    },
    Turn {
        session_id: SessionId,
        direction: u8,
    },
    Walk {
        session_id: SessionId,
        direction: u8,
    },
    Run {
        session_id: SessionId,
        direction: u8,
    },
}

#[derive(Clone, Debug)]
pub enum WorldEvent {
    UserLocation {
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
    },
    MapChanged {
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
    },
}

#[derive(Clone, Debug)]
pub struct World<P: WorldProvider> {
    pub provider: P,
    pub config: WorldConfig,
    pub players: HashMap<SessionId, PlayerState>,
    pub maps: std::sync::Arc<std::sync::Mutex<HashMap<i32, map::Map>>>,
    /// Spawned monsters keyed by map_index.
    pub monsters: HashMap<i32, Vec<MonsterInstance>>,
    pub next_monster_id: u64,
    /// Per-map respawn runtime state, mirroring C# MapRespawn in a simplified form.
    pub respawns: HashMap<i32, Vec<RespawnRuntime>>,
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
    /// Simple error counter mirroring the C# MapRespawn.ErrorCount field; for
    /// now it is unused but reserved for future spawn failure backoff.
    pub error_count: u8,
}

impl<P: WorldProvider> World<P> {
    pub fn new(provider: P, config: WorldConfig) -> Self {
        World {
            provider,
            config,
            players: HashMap::new(),
            maps: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            monsters: HashMap::new(),
            next_monster_id: 0,
            respawns: HashMap::new(),
        }
    }

    fn upsert_player(
        &mut self,
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
        magics: Vec<UserMagic>,
    ) -> &PlayerState {
        self.players
            .entry(session_id)
            .and_modify(|p| {
                p.map_index = map_index;
                p.x = x;
                p.y = y;
                p.direction = direction;
            })
            .or_insert(PlayerState {
                session_id,
                map_index,
                x,
                y,
                direction,
                magics,
            });

        self.players.get(&session_id).unwrap()
    }

    fn get_or_load_map(&self, map_index: i32) -> Option<map::Map> {
        let mut maps = self.maps.lock().unwrap();
        if let Some(m) = maps.get(&map_index) {
            return Some(m.clone());
        }

        let info = self.provider.get_map_info(map_index)?.clone();
        let map_dir = &self.config.map_path;
        let m = map::load_map_from_file(info, map_dir.as_path()).ok()?;
        maps.insert(map_index, m.clone());
        Some(m)
    }

    fn spawn_monsters_for_map(&mut self, map_index: i32, map: &map::Map) {
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

    /// World update tick. This will eventually drive respawns, monster AI,
    /// buffs and other timed systems. For now it implements a minimal time-
    /// based respawn scheduler mirroring the C# Map.ProcessRespawns logic,
    /// but without yet wiring spawn multipliers or death/despawn updates.
    pub fn update(&mut self, now_ms: i64) -> Vec<WorldEvent> {
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

    /// Learn a new magic for the given player session. If the magic is already
    /// present, this is a no-op and returns None.
    pub fn learn_magic_for_player(&mut self, session_id: SessionId, spell: u8) -> Option<UserMagic> {
        let player = self.players.get_mut(&session_id)?;
        if player.magics.iter().any(|m| m.spell == spell) {
            return None;
        }

        let magic = UserMagic::new(spell);
        player.magics.push(magic.clone());
        Some(magic)
    }

    /// Update the level and experience for an existing magic on the given
    /// player. Returns the updated magic if found.
    pub fn set_magic_level_for_player(
        &mut self,
        session_id: SessionId,
        spell: u8,
        level: u8,
        experience: u16,
    ) -> Option<UserMagic> {
        let player = self.players.get_mut(&session_id)?;
        let magic = player.magics.iter_mut().find(|m| m.spell == spell)?;
        magic.level = level;
        magic.experience = experience;
        Some(magic.clone())
    }

    fn create_monsters_from_respawn(&mut self, map: &map::Map, respawn: &RespawnInfo) -> Vec<MonsterInstance> {
        let mut result = Vec::new();

        for _ in 0..respawn.count {
            let x = respawn.location_x;
            let y = respawn.location_y;

            if x < 0 || y < 0 {
                continue;
            }

            let ux = x as u16;
            let uy = y as u16;

            let Some(cell) = map.cell(ux, uy) else {
                continue;
            };

            if !matches!(cell.attribute, CellAttribute::Walk) {
                continue;
            }

            self.next_monster_id = self.next_monster_id.wrapping_add(1);

            result.push(MonsterInstance {
                id: self.next_monster_id,
                monster_index: respawn.monster_index,
                map_index: map.info.index,
                x,
                y,
                direction: respawn.direction,
                respawn_index: respawn.respawn_index,
            });
        }

        result
    }

    /// Get a cloned list of learned magics for the given player session.
    pub fn player_magics(&self, session_id: SessionId) -> Vec<UserMagic> {
        self.players
            .get(&session_id)
            .map(|p| p.magics.clone())
            .unwrap_or_default()
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

    fn check_map_movement(&mut self, player: &mut PlayerState, events: &mut Vec<WorldEvent>) {
        let current_map_index = player.map_index;
        // Limit the lifetime of the immutable borrow from provider so that we can
        // subsequently borrow &mut self when loading maps and spawning monsters.
        let (dest_map_index, dest_x, dest_y) = {
            let Some(map_info) = self.provider.get_map_info(current_map_index) else {
                return;
            };

            let Some(movement) = map_info
                .movements
                .iter()
                .find(|m| m.source_x == player.x && m.source_y == player.y)
            else {
                return;
            };

            (movement.dest_map_index, movement.dest_x, movement.dest_y)
        };

        match self.get_or_load_map(dest_map_index) {
            Some(dest_map) => {
                self.spawn_monsters_for_map(dest_map_index, &dest_map);

                player.map_index = dest_map_index;
                player.x = dest_x;
                player.y = dest_y;

                events.push(WorldEvent::MapChanged {
                    session_id: player.session_id,
                    map_index: player.map_index,
                    x: player.x,
                    y: player.y,
                    direction: player.direction,
                });
            }
            None => {
                debug!(
                    "Map movement failed: could not load destination map {} from ({}, {})",
                    dest_map_index, player.x, player.y
                );
            }
        }
    }

    fn apply_step(player: &mut PlayerState, _map: Option<map::Map>, direction: u8, distance: i32) {
        let (dx, dy) = match direction {
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

        // Always update facing direction, even if movement is blocked.
        player.direction = direction;

        if distance <= 0 {
            return;
        }

        let steps = distance;
        let mut new_x = player.x;
        let mut new_y = player.y;

        for _ in 0..steps {
            let tx = new_x + dx;
            let ty = new_y + dy;

            if tx < 0 || ty < 0 || tx > u16::MAX as i32 || ty > u16::MAX as i32 {
                debug!("Move blocked: out of bounds ({}, {})", tx, ty);
                break;
            }

            // TEMP: ignore map cell attributes and treat everything within bounds as walkable.
            // Once map loading and cell attributes are fully validated, restore the map-based
            // collision checks below.
            // if let Some(ref m) = map {
            //     let ux = tx as u16;
            //     let uy = ty as u16;

            //     match m.cell(ux, uy) {
            //         Some(cell) if matches!(cell.attribute, CellAttribute::Walk) => {
            //             new_x = tx;
            //             new_y = ty;
            //         }
            //         Some(_) => {
            //             debug!("Move blocked: non-walkable cell at ({}, {})", tx, ty);
            //             break;
            //         }
            //         None => {
            //             debug!("Move blocked: no cell data at ({}, {})", tx, ty);
            //             break;
            //         }
            //     }
            // } else {
                new_x = tx;
                new_y = ty;
            // }
        }

        player.x = new_x;
        player.y = new_y;
    }

    pub fn handle_command(&mut self, cmd: WorldCommand) -> Vec<WorldEvent> {
        let mut events = Vec::new();

        match cmd {
            WorldCommand::StartGame {
                session_id,
                map_index,
                x,
                y,
                direction,
                magics,
            } => {
                if let Some(map) = self.get_or_load_map(map_index) {
                    self.spawn_monsters_for_map(map_index, &map);
                }
                let p = self.upsert_player(session_id, map_index, x, y, direction, magics);
                events.push(WorldEvent::UserLocation {
                    session_id: p.session_id,
                    map_index: p.map_index,
                    x: p.x,
                    y: p.y,
                    direction: p.direction,
                });
            }
            WorldCommand::Turn {
                session_id,
                direction,
            } => {
                let map_index = self.players.get(&session_id).map(|p| p.map_index);
                if let Some(map_index) = map_index {
                    let map = self.get_or_load_map(map_index);
                    if let Some(p) = self.players.get_mut(&session_id) {
                        Self::apply_step(p, map, direction, 0);
                        events.push(WorldEvent::UserLocation {
                            session_id: p.session_id,
                            map_index: p.map_index,
                            x: p.x,
                            y: p.y,
                            direction: p.direction,
                        });
                    }
                }
            }
            WorldCommand::Walk {
                session_id,
                direction,
            } => {
                let map_index = self.players.get(&session_id).map(|p| p.map_index);
                if let Some(map_index) = map_index {
                    let map = self.get_or_load_map(map_index);
                    if let Some(mut p) = self.players.remove(&session_id) {
                        Self::apply_step(&mut p, map, direction, 1);
                        self.check_map_movement(&mut p, &mut events);
                        events.push(WorldEvent::UserLocation {
                            session_id: p.session_id,
                            map_index: p.map_index,
                            x: p.x,
                            y: p.y,
                            direction: p.direction,
                        });
                        self.players.insert(session_id, p);
                    }
                }
            }
            WorldCommand::Run {
                session_id,
                direction,
            } => {
                let map_index = self.players.get(&session_id).map(|p| p.map_index);
                if let Some(map_index) = map_index {
                    let map = self.get_or_load_map(map_index);
                    if let Some(mut p) = self.players.remove(&session_id) {
                        Self::apply_step(&mut p, map, direction, 2);
                        self.check_map_movement(&mut p, &mut events);
                        events.push(WorldEvent::UserLocation {
                            session_id: p.session_id,
                            map_index: p.map_index,
                            x: p.x,
                            y: p.y,
                            direction: p.direction,
                        });
                        self.players.insert(session_id, p);
                    }
                }
            }
        }

        events
    }
}
