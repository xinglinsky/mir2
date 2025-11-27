use std::collections::HashMap;

use super::Job;
use crate::world::party::PartyManager;
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

#[derive(Clone, Debug, Default)]
struct CellOccupants {
    players: Vec<SessionId>,
    monsters: Vec<u64>,
}

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
        name: String,
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
    Magic {
        session_id: SessionId,
        spell: u8,
        direction: u8,
        target_id: u32,
        x: i32,
        y: i32,
    },
    DropItem {
        session_id: SessionId,
        unique_id: u64,
        count: u16,
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
    SetAllowGroup {
        session_id: SessionId,
        allow: bool,
    },
    InviteToParty {
        session_id: SessionId,
        target_name: String,
    },
    KickFromParty {
        session_id: SessionId,
        target_name: String,
    },
    RespondPartyInvite {
        session_id: SessionId,
        accept: bool,
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
        count: u16,
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
    pub(crate) occupancy: HashMap<i32, HashMap<(i32, i32), CellOccupants>>,
    /// Per-map respawn runtime state, mirroring C# MapRespawn in a simplified form.
    pub(crate) respawns: HashMap<i32, Vec<RespawnRuntime>>,
    pub(crate) respawn_tick_counter: u64,
    pub(crate) respawn_last_tick_ms: i64,
    pub(crate) respawn_base_spawn_rate_minutes: u8,
    pub(crate) spawn_multiplier: u16,
    pub(crate) drop_rate: f32,
    pub(crate) guilds: GuildManager,
    pub(crate) parties: PartyManager,
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
            occupancy: HashMap::new(),
            respawns: HashMap::new(),
            respawn_tick_counter: 0,
            respawn_last_tick_ms: 0,
            respawn_base_spawn_rate_minutes,
            spawn_multiplier,
            drop_rate,
            guilds: GuildManager::new(),
            parties: PartyManager::new(),
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

    fn occupancy_map_mut(
        &mut self,
        map_index: i32,
    ) -> &mut HashMap<(i32, i32), CellOccupants> {
        self.occupancy.entry(map_index).or_default()
    }

    pub(crate) fn is_cell_blocked(&self, map_index: i32, x: i32, y: i32) -> bool {
        self.occupancy
            .get(&map_index)
            .and_then(|m| m.get(&(x, y)))
            .map_or(false, |cell| !cell.players.is_empty() || !cell.monsters.is_empty())
    }

    pub(crate) fn add_player_to_occupancy(
        &mut self,
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
    ) {
        let occ_map = self.occupancy_map_mut(map_index);
        let entry = occ_map.entry((x, y)).or_default();
        if !entry.players.contains(&session_id) {
            entry.players.push(session_id);
        }
    }

    pub(crate) fn remove_player_from_occupancy(
        &mut self,
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
    ) {
        if let Some(map_occ) = self.occupancy.get_mut(&map_index) {
            if let Some(cell) = map_occ.get_mut(&(x, y)) {
                cell.players.retain(|&sid| sid != session_id);
                if cell.players.is_empty() && cell.monsters.is_empty() {
                    map_occ.remove(&(x, y));
                }
            }
            if map_occ.is_empty() {
                self.occupancy.remove(&map_index);
            }
        }
    }

    pub(crate) fn clear_player_from_occupancy(&mut self, session_id: SessionId) {
        if let Some(p) = self.players.get(&session_id) {
            self.remove_player_from_occupancy(session_id, p.map_index, p.x, p.y);
        }
    }

    pub(crate) fn add_monster_to_occupancy(
        &mut self,
        monster_id: u64,
        map_index: i32,
        x: i32,
        y: i32,
    ) {
        let occ_map = self.occupancy_map_mut(map_index);
        let entry = occ_map.entry((x, y)).or_default();
        if !entry.monsters.contains(&monster_id) {
            entry.monsters.push(monster_id);
        }
    }

    pub(crate) fn remove_monster_from_occupancy(
        &mut self,
        monster_id: u64,
        map_index: i32,
        x: i32,
        y: i32,
    ) {
        if let Some(map_occ) = self.occupancy.get_mut(&map_index) {
            if let Some(cell) = map_occ.get_mut(&(x, y)) {
                cell.monsters.retain(|&id| id != monster_id);
                if cell.players.is_empty() && cell.monsters.is_empty() {
                    map_occ.remove(&(x, y));
                }
            }
            if map_occ.is_empty() {
                self.occupancy.remove(&map_index);
            }
        }
    }

    /// Remove a player from the world and occupancy tracking. This is used when
    /// a connection fully disconnects so that offline characters no longer
    /// block movement.
    pub fn remove_player_from_world(&mut self, session_id: SessionId) {
        if let Some(p) = self.players.remove(&session_id) {
            self.remove_player_from_occupancy(session_id, p.map_index, p.x, p.y);
        }
    }

    pub fn handle_command(&mut self, cmd: WorldCommand) -> Vec<WorldEvent> {
        let mut events = Vec::new();

        match cmd {
            WorldCommand::StartGame {
                session_id,
                character_index,
                name,
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
                // Remove any existing occupancy entry for this session, then
                // upsert the player and re-add occupancy using a separate
                // scope to satisfy the borrow checker.
                self.clear_player_from_occupancy(session_id);
                self.players.remove(&session_id);
                if let Some(map) = self.get_or_load_map(map_index) {
                    self.spawn_monsters_for_map(map_index, &map);
                }

                let (sid, p_map, px, py, dir) = {
                    let p = self.upsert_player(
                        session_id,
                        character_index,
                        name,
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
                    (p.session_id, p.map_index, p.x, p.y, p.direction)
                };

                self.add_player_to_occupancy(sid, p_map, px, py);
                events.push(WorldEvent::UserLocation {
                    session_id: sid,
                    map_index: p_map,
                    x: px,
                    y: py,
                    direction: dir,
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
                    if let Some(mut p) = self.players.remove(&session_id) {
                        self.apply_step(&mut p, map, direction, 0);
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
            WorldCommand::Walk {
                session_id,
                direction,
            } => {
                println!("[world] Walk command: session={} dir={}", session_id, direction);
                let map_index = self.players.get(&session_id).map(|p| p.map_index);
                if let Some(map_index) = map_index {
                    let map = self.get_or_load_map(map_index);
                    if let Some(mut p) = self.players.remove(&session_id) {
                        self.apply_step(&mut p, map, direction, 1);
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
                        self.apply_step(&mut p, map, direction, 2);
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
            WorldCommand::Magic {
                session_id,
                spell,
                direction,
                target_id,
                x,
                y,
            } => {
                self.handle_magic_command(
                    session_id, spell, direction, target_id, x, y, &mut events,
                );
            }
            WorldCommand::DropItem {
                session_id,
                unique_id,
                count,
            } => {
                if count == 0 {
                    return events;
                }

                let (map_index, px, py) = match self.players.get(&session_id) {
                    Some(p) => (p.map_index, p.x, p.y),
                    None => return events,
                };

                // Respect map-level NoThrowItem flag, mirroring
                // C# PlayerObject.DropItem which rejects drops on such maps.
                if let Some(map_info) = self.provider.get_map_info(map_index) {
                    if map_info.no_throw_item {
                        return events;
                    }
                }

                let player = match self.players.get_mut(&session_id) {
                    Some(p) => p,
                    None => return events,
                };

                let inv_index = match player
                    .inventory
                    .slots
                    .iter()
                    .position(|s| s.as_ref().map(|i| i.unique_id) == Some(unique_id))
                {
                    Some(idx) => idx,
                    None => return events,
                };

                let item_in_slot = match player.inventory.slots[inv_index].clone() {
                    Some(it) => it,
                    None => return events,
                };

                if count as u32 > item_in_slot.count as u32 {
                    return events;
                }

                // Look up the ItemInfo so we can obtain the base index and
                // binding flags.
                let info = match self.provider.get_item_info(item_in_slot.item_index) {
                    Some(i) => i,
                    None => return events,
                };

                const BIND_DONT_DROP: i16 = 0x0002;
                const BIND_DESTROY_ON_DROP: i16 = 0x0080;

                // Mirror C# BindMode.DontDrop and rental BindingFlags.DontDrop
                // checks: when set, the item cannot be dropped at all.
                if (info.bind & BIND_DONT_DROP) != 0 {
                    return events;
                }
                if let Some(rental) = item_in_slot.rental_information.as_ref() {
                    if (rental.binding_flags & BIND_DONT_DROP) != 0 {
                        return events;
                    }
                }

                // DestroyOnDrop: dropping this item removes it from the
                // inventory but does not create a ground item, mirroring the
                // C# behaviour where DropItem(temp) is skipped when the flag
                // is present.
                let destroy_on_drop = (info.bind & BIND_DESTROY_ON_DROP) != 0
                    || item_in_slot
                        .rental_information
                        .as_ref()
                        .map_or(false, |r| (r.binding_flags & BIND_DESTROY_ON_DROP) != 0);

                // Decide which UserItemData should be placed on the ground:
                // - Full-stack drop: move the existing item (preserving all
                //   properties and UniqueID), matching C# DropItem(temp).
                // - Partial-stack drop: keep a reduced stack in inventory and
                //   create a fresh UserItemData for the dropped portion,
                //   matching C# temp2 = Envir.CreateFreshItem(temp.Info).
                let mut dropped_item: Option<UserItemData> = None;

                if count as u16 == item_in_slot.count {
                    // Full-stack: remove from inventory entirely.
                    player.inventory.slots[inv_index] = None;
                    if !destroy_on_drop {
                        dropped_item = Some(item_in_slot);
                    }
                } else {
                    // Partial-stack: keep remaining portion in inventory.
                    let mut remaining = item_in_slot.clone();
                    remaining.count = remaining.count.saturating_sub(count);
                    player.inventory.slots[inv_index] = Some(remaining);

                    if !destroy_on_drop {
                        // Fresh dropped stack with a derived unique_id so it
                        // remains distinct from existing items, mirroring the
                        // spirit of C# Envir.CreateFreshItem.
                        let map_item_id = self.next_map_item_id;
                        let unique_id =
                            ((session_id as u64) << 32) | (map_item_id & 0xFFFF_FFFF);
                        let fresh = create_fresh_user_item(info, unique_id, count);
                        dropped_item = Some(fresh);
                    }
                }

                let dropped_item = match dropped_item {
                    Some(it) => it,
                    None => {
                        // DestroyOnDrop consumed the item without creating a
                        // ground object.
                        return events;
                    }
                };

                // Create a new MapItem representing the dropped stack.
                let entry = self.map_items.entry(map_index).or_default();
                let map_item_id = self.next_map_item_id;
                self.next_map_item_id = self.next_map_item_id.wrapping_add(1);

                // Set expire time: default 5 minutes (300000 ms) after drop,
                // mirroring C# Settings.ItemTimeOut * Settings.Minute behavior.
                let item_timeout_ms: i64 = 300_000; // 5 minutes
                entry.push(MapItem {
                    id: map_item_id,
                    map_index,
                    x: px,
                    y: py,
                    item_index: Some(info.index),
                    gold: 0,
                    count: dropped_item.count,
                    item: Some(dropped_item.clone()),
                    expire_time_ms: self.time_ms + item_timeout_ms,
                });

                events.push(WorldEvent::ItemDropped {
                    object_id: map_item_id,
                    map_index,
                    x: px,
                    y: py,
                    item_index: info.index,
                    count: dropped_item.count,
                });
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
                            // Prefer to reuse the full UserItemData stored on
                            // the map item when available so that dropping and
                            // picking up a full item preserves all of its
                            // properties and UniqueID, mirroring C#
                            // ItemObject behaviour. Fall back to creating a
                            // fresh item only when no item payload was stored
                            // (e.g. older map entries or simple monster drops).
                            let user_item = if let Some(item) = map_item.item.clone() {
                                item
                            } else {
                                let info = match self.provider.get_item_info(item_index) {
                                    Some(i) => i,
                                    None => return events,
                                };
                                let count = if map_item.count == 0 {
                                    1
                                } else {
                                    map_item.count
                                };
                                let unique_id =
                                    ((session_id as u64) << 32) | (map_item.id & 0xFFFF_FFFF);
                                create_fresh_user_item(info, unique_id, count)
                            };

                            if let Some(player_state) =
                                self.players.get_mut(&session_id)
                            {
                                if let Some(slot) = player_state
                                    .inventory
                                    .slots
                                    .iter()
                                    .position(|s| s.is_none())
                                {
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
            WorldCommand::Teleport {
                session_id,
                map_index,
                x,
                y,
            } => {
                let map = match self.get_or_load_map(map_index) {
                    Some(m) => m,
                    None => return events,
                };

                if x < 0 || y < 0 {
                    return events;
                }
                let ux = x as u16;
                let uy = y as u16;
                if ux >= map.width || uy >= map.height || !map.is_walkable(ux, uy) {
                    return events;
                }

                self.spawn_monsters_for_map(map_index, &map);

                let (old_map, old_x, old_y, new_map, new_x, new_y, dir) = {
                    let p = match self.players.get_mut(&session_id) {
                        Some(p) => p,
                        None => return events,
                    };

                    let old_map = p.map_index;
                    let old_x = p.x;
                    let old_y = p.y;
                    p.map_index = map_index;
                    p.x = x;
                    p.y = y;

                    (old_map, old_x, old_y, p.map_index, p.x, p.y, p.direction)
                };

                self.remove_player_from_occupancy(session_id, old_map, old_x, old_y);
                self.add_player_to_occupancy(session_id, new_map, new_x, new_y);

                events.push(WorldEvent::MapChanged {
                    session_id,
                    map_index: new_map,
                    x: new_x,
                    y: new_y,
                    direction: dir,
                });
            }
            WorldCommand::SetAllowGroup { session_id, allow } => {
                if let Some(p) = self.players.get_mut(&session_id) {
                    p.allow_group = allow;
                }
            }
            WorldCommand::InviteToParty { .. } => {}
            WorldCommand::KickFromParty { .. } => {}
            WorldCommand::RespondPartyInvite { .. } => {}
        }

        events
    }
}
