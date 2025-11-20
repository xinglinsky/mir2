use std::collections::HashMap;

use tracing::debug;
use crate::stats::Stat;
use crate::world::config::WorldConfig;
use crate::world::map::{self, CellAttribute};
use crate::world::monster::MonsterInstance;
use crate::world::provider::WorldProvider;
use crate::world::magic::UserMagic;
use crate::world::monster_runtime::RespawnRuntime;
use crate::world::player::PlayerState;

pub type SessionId = u32;

#[derive(Clone, Debug)]
pub enum WorldCommand {
    StartGame {
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
        level: u16,
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
    Attack {
        session_id: SessionId,
        direction: u8,
        spell: u8,
    },
    Teleport {
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
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
    ObjectAttack {
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
        spell: u8,
        level: u8,
        attack_type: u8,
    },
    ObjectStruck {
        attacker_id: SessionId,
        target_id: u64,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
        damage: i32,
        damage_type: u8,
        health_percent: u8,
    },
}

#[derive(Clone, Debug)]
pub struct World<P: WorldProvider> {
    pub(crate) provider: P,
    pub(crate) config: WorldConfig,
    pub(crate) players: HashMap<SessionId, PlayerState>,
    pub(crate) maps: std::sync::Arc<std::sync::Mutex<HashMap<i32, map::Map>>>,
    /// Spawned monsters keyed by map_index.
    pub(crate) monsters: HashMap<i32, Vec<MonsterInstance>>,
    pub(crate) next_monster_id: u64,
    /// Per-map respawn runtime state, mirroring C# MapRespawn in a simplified form.
    pub(crate) respawns: HashMap<i32, Vec<RespawnRuntime>>,
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

    pub(crate) fn get_or_load_map(&self, map_index: i32) -> Option<map::Map> {
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


    pub fn handle_command(&mut self, cmd: WorldCommand) -> Vec<WorldEvent> {
        let mut events = Vec::new();

        match cmd {
            WorldCommand::StartGame {
                session_id,
                map_index,
                x,
                y,
                direction,
                level,
                magics,
            } => {
                if let Some(map) = self.get_or_load_map(map_index) {
                    self.spawn_monsters_for_map(map_index, &map);
                }
                let p = self.upsert_player(session_id, map_index, x, y, direction, level, magics);
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
            WorldCommand::Attack {
                session_id,
                direction,
                spell,
            } => {
                self.handle_attack_command(session_id, direction, spell, &mut events);
            }
            WorldCommand::Teleport {
                session_id,
                map_index,
                x,
                y,
            } => {
                if let Some(map) = self.get_or_load_map(map_index) {
                    self.spawn_monsters_for_map(map_index, &map);
                }

                if let Some(p) = self.players.get_mut(&session_id) {
                    p.map_index = map_index;
                    p.x = x;
                    p.y = y;

                    events.push(WorldEvent::MapChanged {
                        session_id: p.session_id,
                        map_index: p.map_index,
                        x: p.x,
                        y: p.y,
                        direction: p.direction,
                    });
                }
            }
        }

        events
    }
}
