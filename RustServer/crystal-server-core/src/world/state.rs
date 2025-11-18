use std::collections::HashMap;

use crate::world::config::WorldConfig;
use crate::world::provider::WorldProvider;

pub type SessionId = u32;

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub session_id: SessionId,
    pub map_index: i32,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
}

#[derive(Clone, Debug)]
pub enum WorldCommand {
    StartGame {
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
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
}

#[derive(Clone, Debug)]
pub struct World<P: WorldProvider> {
    pub provider: P,
    pub config: WorldConfig,
    pub players: HashMap<SessionId, PlayerState>,
}

impl<P: WorldProvider> World<P> {
    pub fn new(provider: P, config: WorldConfig) -> Self {
        World {
            provider,
            config,
            players: HashMap::new(),
        }
    }

    fn upsert_player(
        &mut self,
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
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
            });

        self.players.get(&session_id).unwrap()
    }

    fn apply_step_internal(player: &mut PlayerState, direction: u8, distance: i32) {
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

        player.direction = direction;
        player.x += dx * distance;
        player.y += dy * distance;
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
            } => {
                let p = self.upsert_player(session_id, map_index, x, y, direction);
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
                if let Some(p) = self.players.get_mut(&session_id) {
                    Self::apply_step_internal(p, direction, 0);
                    events.push(WorldEvent::UserLocation {
                        session_id: p.session_id,
                        map_index: p.map_index,
                        x: p.x,
                        y: p.y,
                        direction: p.direction,
                    });
                }
            }
            WorldCommand::Walk {
                session_id,
                direction,
            } => {
                if let Some(p) = self.players.get_mut(&session_id) {
                    Self::apply_step_internal(p, direction, 1);
                    events.push(WorldEvent::UserLocation {
                        session_id: p.session_id,
                        map_index: p.map_index,
                        x: p.x,
                        y: p.y,
                        direction: p.direction,
                    });
                }
            }
            WorldCommand::Run {
                session_id,
                direction,
            } => {
                if let Some(p) = self.players.get_mut(&session_id) {
                    Self::apply_step_internal(p, direction, 2);
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

        events
    }
}
