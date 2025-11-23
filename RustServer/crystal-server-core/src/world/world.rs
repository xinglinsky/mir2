use std::collections::HashMap;

use super::Job;
use crate::guild::{GuildInfo, GuildManager};
use crate::item::create_fresh_user_item;
use crate::world::config::WorldConfig;
use crate::world::map::{self};
use crate::world::map_item::MapItem;
use crate::world::magic::UserMagic;
use crate::world::monster::MonsterInstance;
use crate::world::monster_runtime::RespawnRuntime;
use crate::world::player::PlayerState;
use crate::world::provider::WorldProvider;
use crystal_shared_proto::item_types::UserItemData;

pub type SessionId = u32;

#[derive(Clone, Debug)]
pub struct CoreMetrics {
    pub players: u32,
    pub monsters: u32,
    pub connections: u32,
    pub blocked_ips: u32,
    pub uptime_seconds: u64,
    pub cycle_delay_ms: u32,
}

#[derive(Clone, Debug)]
pub struct CorePlayerInfo {
    pub session_id: SessionId,
    pub map_index: i32,
    pub x: i32,
    pub y: i32,
    pub level: u16,
    pub job: Job,
}

#[derive(Clone, Debug)]
pub enum WorldCommand {
    StartGame {
        session_id: SessionId,
        character_index: i32,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
        job: Job,
        gender: u8,
        level: u16,
        experience: i64,
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
    PickUp {
        session_id: SessionId,
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
    ObjectLocation {
        object_id: u64,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
    },
    GainExperience {
        session_id: SessionId,
        amount: u32,
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
    MonsterDied {
        object_id: u64,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
    },
    MonsterHitPlayer {
        attacker_monster_id: u64,
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
        damage: i32,
        damage_type: u8,
        health_percent: u8,
    },
    ItemDropped {
        object_id: u64,
        map_index: i32,
        x: i32,
        y: i32,
        item_index: i32,
    },
    GoldDropped {
        object_id: u64,
        map_index: i32,
        x: i32,
        y: i32,
        gold: u32,
    },
    PlayerGainedItem {
        session_id: SessionId,
        item: UserItemData,
    },
    PlayerGainedGold {
        session_id: SessionId,
        amount: u32,
    },
    MapItemRemoved {
        object_id: u64,
        map_index: i32,
        x: i32,
        y: i32,
    },
}

#[derive(Clone, Debug)]
pub struct World<P: WorldProvider> {
    pub(crate) provider: P,
    pub(crate) config: WorldConfig,
    pub(crate) time_ms: i64,
    pub(crate) players: HashMap<SessionId, PlayerState>,
    pub(crate) maps: std::sync::Arc<std::sync::Mutex<HashMap<i32, map::Map>>>,
    /// Spawned monsters keyed by map_index.
    pub(crate) monsters: HashMap<i32, Vec<MonsterInstance>>,
    pub(crate) next_monster_id: u64,
    /// Items and gold currently present on maps, keyed by map_index.
    pub(crate) map_items: HashMap<i32, Vec<MapItem>>,
    pub(crate) next_map_item_id: u64,
    /// Per-map respawn runtime state, mirroring C# MapRespawn in a simplified form.
    pub(crate) respawns: HashMap<i32, Vec<RespawnRuntime>>,
    pub(crate) respawn_tick_counter: u64,
    pub(crate) respawn_last_tick_ms: i64,
    pub(crate) respawn_base_spawn_rate_minutes: u8,
    pub(crate) spawn_multiplier: u16,
    pub(crate) drop_rate: f32,
    pub(crate) guilds: GuildManager,
}

impl<P: WorldProvider> World<P> {
    pub fn new(provider: P, config: WorldConfig) -> Self {
        let spawn_multiplier = config.spawn_multiplier;
        let respawn_base_spawn_rate_minutes = config.respawn_base_spawn_rate_minutes;
        let drop_rate = config.drop_rate;
        World {
            provider,
            config,
            time_ms: 0,
            players: HashMap::new(),
            maps: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            monsters: HashMap::new(),
            next_monster_id: 0,
            map_items: HashMap::new(),
            next_map_item_id: 1,
            respawns: HashMap::new(),
            respawn_tick_counter: 0,
            respawn_last_tick_ms: 0,
            respawn_base_spawn_rate_minutes,
            spawn_multiplier,
            drop_rate,
            guilds: GuildManager::new(),
        }
    }

    pub fn snapshot_metrics(&self, connections: u32) -> CoreMetrics {
        let players = self.players.len() as u32;
        let monsters = self
            .monsters
            .values()
            .map(|v| v.len() as u32)
            .fold(0_u32, |acc, v| acc.saturating_add(v));

        let uptime_seconds = if self.time_ms <= 0 {
            0_u64
        } else {
            (self.time_ms.max(0) as u64) / 1000_u64
        };

        let cycle_delay_ms = 50_u32;

        CoreMetrics {
            players,
            monsters,
            connections,
            blocked_ips: 0,
            uptime_seconds,
            cycle_delay_ms,
        }
    }

    pub fn snapshot_players(&self) -> Vec<CorePlayerInfo> {
        self
            .players
            .values()
            .map(|p| CorePlayerInfo {
                session_id: p.session_id,
                map_index: p.map_index,
                x: p.x,
                y: p.y,
                level: p.level,
                job: p.job,
            })
            .collect()
    }

    /// Look up the latest known position and facing of a monster on the given
    /// map. This is used by the connection layer when emitting visual attack
    /// packets (SObjectAttack) for monster melee swings.
    pub fn monster_position(&self, map_index: i32, monster_id: u64) -> Option<(i32, i32, u8)> {
        let monsters = self.monsters.get(&map_index)?;
        monsters
            .iter()
            .find(|m| m.id == monster_id)
            .map(|m| (m.x, m.y, m.direction))
    }

    /// Create a new guild with the given name if no existing guild uses the
    /// same name (case-insensitive). Returns a cloned GuildInfo on success.
    pub fn create_guild(&mut self, name: &str) -> Option<GuildInfo> {
        let exists = self
            .guilds
            .guilds()
            .any(|g| g.name.eq_ignore_ascii_case(name));

        if exists {
            return None;
        }

        let info = self.guilds.create_guild(name.to_string());
        Some(info.clone())
    }

    pub fn init_guilds_from_db(&mut self, guilds: Vec<GuildInfo>) {
        self.guilds = GuildManager::from_guilds(guilds);
    }

    pub fn set_spawn_config(
        &mut self,
        spawn_multiplier: u16,
        respawn_base_spawn_rate_minutes: u8,
        drop_rate: f32,
    ) {
        self.spawn_multiplier = spawn_multiplier.max(1);
        self.respawn_base_spawn_rate_minutes = respawn_base_spawn_rate_minutes.max(1);
        // Clamp drop_rate to a small positive value to avoid division by zero
        self.drop_rate = if drop_rate <= 0.0 { 0.0001 } else { drop_rate };

        // Keep config snapshot in sync for any callers that inspect it.
        self.config.spawn_multiplier = self.spawn_multiplier;
        self.config.respawn_base_spawn_rate_minutes = self.respawn_base_spawn_rate_minutes;
        self.config.drop_rate = self.drop_rate;
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
                character_index,
                map_index,
                x,
                y,
                direction,
                job,
                gender,
                level,
                experience,
                magics,
            } => {
                self.players.remove(&session_id);
                if let Some(map) = self.get_or_load_map(map_index) {
                    self.spawn_monsters_for_map(map_index, &map);
                }
                let p = self.upsert_player(
                    session_id,
                    character_index,
                    map_index,
                    x,
                    y,
                    direction,
                    job,
                    gender,
                    level,
                    experience,
                    magics,
                );
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
                println!("[world] Turn command: session={} dir={}", session_id, direction);
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
                println!("[world] Walk command: session={} dir={}", session_id, direction);
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
                println!("[world] Run command: session={} dir={}", session_id, direction);
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
            WorldCommand::PickUp { session_id } => {
                if let Some(player) = self.players.get(&session_id).cloned() {
                    let map_index = player.map_index;
                    let px = player.x;
                    let py = player.y;

                    let maybe_item = self
                        .map_items
                        .get(&map_index)
                        .and_then(|items| {
                            items
                                .iter()
                                .find(|mi| mi.x == px && mi.y == py)
                                .cloned()
                        });

                    if let Some(map_item) = maybe_item {
                        if map_item.gold > 0 && map_item.item_index.is_none() {
                            if map_item.gold > 0 {
                                events.push(WorldEvent::PlayerGainedGold {
                                    session_id,
                                    amount: map_item.gold,
                                });
                            }

                            if let Some(items) = self.map_items.get_mut(&map_index) {
                                if let Some(pos) =
                                    items.iter().position(|mi| mi.id == map_item.id)
                                {
                                    items.swap_remove(pos);
                                }
                            }

                            events.push(WorldEvent::MapItemRemoved {
                                object_id: map_item.id,
                                map_index,
                                x: map_item.x,
                                y: map_item.y,
                            });
                        } else if let Some(item_index) = map_item.item_index {
                            if let Some(info) = self.provider.get_item_info(item_index) {
                                if let Some(player_state) =
                                    self.players.get_mut(&session_id)
                                {
                                    if let Some(slot) = player_state
                                        .inventory
                                        .slots
                                        .iter()
                                        .position(|s| s.is_none())
                                    {
                                        let unique_id =
                                            ((session_id as u64) << 32) | (map_item.id & 0xFFFF_FFFF);
                                        let user_item =
                                            create_fresh_user_item(info, unique_id, 1);
                                        player_state.inventory.slots[slot] = Some(user_item.clone());

                                        if let Some(items) =
                                            self.map_items.get_mut(&map_index)
                                        {
                                            if let Some(pos) = items
                                                .iter()
                                                .position(|mi| mi.id == map_item.id)
                                            {
                                                items.swap_remove(pos);
                                            }
                                        }

                                        events.push(WorldEvent::PlayerGainedItem {
                                            session_id,
                                            item: user_item,
                                        });
                                        events.push(WorldEvent::MapItemRemoved {
                                            object_id: map_item.id,
                                            map_index,
                                            x: map_item.x,
                                            y: map_item.y,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
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
