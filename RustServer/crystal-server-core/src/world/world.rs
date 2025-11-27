use std::collections::HashMap;

use super::Job;
use crate::world::Spell;
use crate::world::party::{Party, PartyManager, MAX_GROUP_SIZE};
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
pub struct BuyBackEntry {
    /// Snapshot of the sold item at the time it was added to BuyBack.
    pub item: UserItemData,
    /// Server time in milliseconds when this entry was created.
    pub added_ms: i64,
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
    PlayerHealed {
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
        amount: i32,
        new_hp: i32,
    },
    SpellToggle {
        session_id: SessionId,
        spell_id: u8,
        enabled: bool,
    },
    AddBuff {
        session_id: SessionId,
        buff_bytes: Vec<u8>,
    },
    RemoveBuff {
        session_id: SessionId,
        buff_type: u8, // BuffType as u8
    },
    /// A free-form system message destined for a specific player session,
    /// typically used for errors or feedback from world-side social logic
    /// such as the party system.
    PartySystemMessage {
        session_id: SessionId,
        message: String,
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
    pub(crate) map_spells: HashMap<i32, HashMap<(i32, i32), Vec<u8>>>,
    /// Per-map respawn runtime state, mirroring C# MapRespawn in a simplified form.
    pub(crate) respawns: HashMap<i32, Vec<RespawnRuntime>>,
    pub(crate) respawn_tick_counter: u64,
    pub(crate) respawn_last_tick_ms: i64,
    /// Last time (in ms) when SafeZone healing was processed.
    pub(crate) safezone_heal_last_ms: i64,
    pub(crate) respawn_base_spawn_rate_minutes: u8,
    pub(crate) spawn_multiplier: u16,
    pub(crate) drop_rate: f32,
    pub(crate) guilds: GuildManager,
    pub(crate) parties: PartyManager,
    /// In-memory BuyBack storage keyed by (session, map_index, npc_index).
    /// This approximates C# NPCObject.BuyBack per player and per NPC.
    pub(crate) buyback: HashMap<(SessionId, i32, i32), Vec<BuyBackEntry>>,
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
            map_spells: HashMap::new(),
            respawns: HashMap::new(),
            respawn_tick_counter: 0,
            respawn_last_tick_ms: 0,
            safezone_heal_last_ms: 0,
            respawn_base_spawn_rate_minutes,
            spawn_multiplier,
            drop_rate,
            guilds: GuildManager::new(),
            parties: PartyManager::new(),
            buyback: HashMap::new(),
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

    /// Look up the canonical player name for a given session.
    pub fn player_name(&self, session_id: SessionId) -> Option<String> {
        self.players.get(&session_id).map(|p| p.name.clone())
    }

    /// Find a player session by character name using a case-insensitive
    /// comparison. This mirrors C# party and social lookup behaviour.
    pub fn find_session_by_name(&self, name: &str) -> Option<SessionId> {
        self
            .players
            .iter()
            .find(|(_, p)| p.name.eq_ignore_ascii_case(name))
            .map(|(&sid, _)| sid)
    }

    /// Inspect who, if anyone, has a pending group invite targeted at the
    /// given session. This mirrors C# PendingGroupInviteFrom behaviour.
    pub fn pending_group_invite_from(&self, session_id: SessionId) -> Option<SessionId> {
        self.players
            .get(&session_id)
            .and_then(|p| p.pending_group_invite_from)
    }

    /// Helper to snapshot current party members (online only) for a given
    /// session, returning a list of (SessionId, Name) pairs.
    pub fn party_members_for_session(
        &self,
        session_id: SessionId,
    ) -> Option<Vec<(SessionId, String)>> {
        let party_id = self.players.get(&session_id).and_then(|p| p.party_id)?;
        let party = self.parties.parties.get(&party_id)?;

        let mut members = Vec::new();
        for &sid in &party.members {
            if let Some(p) = self.players.get(&sid) {
                members.push((sid, p.name.clone()));
            }
        }
        Some(members)
    }

    /// Helper for MovementInfo.NeedHole: returns true if the given map cell
    /// has a "hole" spell (DigOutZombie or DigOutArmadillo) recorded in the
    /// map_spells occupancy, approximating C# Cell.Objects SpellObject check.
    pub(crate) fn cell_has_hole_spell(&self, map_index: i32, x: i32, y: i32) -> bool {
        if let Some(map_spells) = self.map_spells.get(&map_index) {
            if let Some(spells) = map_spells.get(&(x, y)) {
                return spells.iter().any(|&s| {
                    s == Spell::DigOutZombie as u8 || s == Spell::DigOutArmadillo as u8
                });
            }
        }
        false
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
        self.occupancy.entry(map_index).or_insert_with(HashMap::new)
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
        let map = self.occupancy_map_mut(map_index);
        let cell = map.entry((x, y)).or_insert_with(CellOccupants::default);
        if !cell.players.contains(&session_id) {
            cell.players.push(session_id);
        }
    }

    pub(crate) fn remove_player_from_occupancy(
        &mut self,
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
    ) {
        if let Some(map) = self.occupancy.get_mut(&map_index) {
            if let Some(cell) = map.get_mut(&(x, y)) {
                cell.players.retain(|&sid| sid != session_id);
                if cell.players.is_empty() && cell.monsters.is_empty() {
                    map.remove(&(x, y));
                }
            }
        }
    }

    pub(crate) fn clear_player_from_occupancy(&mut self, session_id: SessionId) {
        for map in self.occupancy.values_mut() {
            let mut to_remove = Vec::new();
            for (&coord, cell) in map.iter_mut() {
                cell.players.retain(|&sid| sid != session_id);
                if cell.players.is_empty() && cell.monsters.is_empty() {
                    to_remove.push(coord);
                }
            }
            for coord in to_remove {
                map.remove(&coord);
            }
        }
    }

    pub(crate) fn add_monster_to_occupancy(
        &mut self,
        monster_id: u64,
        map_index: i32,
        x: i32,
        y: i32,
    ) {
        let map = self.occupancy_map_mut(map_index);
        let cell = map.entry((x, y)).or_insert_with(CellOccupants::default);
        if !cell.monsters.contains(&monster_id) {
            cell.monsters.push(monster_id);
        }
    }

    pub(crate) fn remove_monster_from_occupancy(
        &mut self,
        monster_id: u64,
        map_index: i32,
        x: i32,
        y: i32,
    ) {
        if let Some(map) = self.occupancy.get_mut(&map_index) {
            if let Some(cell) = map.get_mut(&(x, y)) {
                cell.monsters.retain(|&id| id != monster_id);
                if cell.players.is_empty() && cell.monsters.is_empty() {
                    map.remove(&(x, y));
                }
            }
        }
    }
    
    /// Append a sold item to the BuyBack list for the given player, map and
    /// NPC. This mirrors the C# behaviour where each NPC keeps a per-player
    /// list of recently sold items that can be repurchased via @BUYBACK.
    pub fn add_buyback_item(
        &mut self,
        session_id: SessionId,
        map_index: i32,
        npc_index: i32,
        item: UserItemData,
    ) {
        let key = (session_id, map_index, npc_index);
        let entry = BuyBackEntry {
            item,
            added_ms: self.time_ms,
        };
        self.buyback.entry(key).or_default().push(entry);
    }

    /// Get a cloned list of BuyBack items for a player at a specific NPC on
    /// the given map. This is used by the connection layer to populate the
    /// @BUYBACK panel.
    pub fn buyback_items_for(
        &self,
        session_id: SessionId,
        map_index: i32,
        npc_index: i32,
    ) -> Vec<UserItemData> {
        let key = (session_id, map_index, npc_index);
        self
            .buyback
            .get(&key)
            .map(|v| v.iter().map(|e| e.item.clone()).collect())
            .unwrap_or_default()
    }

    pub fn leave_party(&mut self, session_id: SessionId) {
        let party_id = match self
            .players
            .get(&session_id)
            .and_then(|p| p.party_id)
        {
            Some(pid) => pid,
            None => return,
        };

        let members_snapshot = match self.parties.parties.get(&party_id) {
            Some(p) => p.members.clone(),
            None => return,
        };

        let mut disband = false;
        if let Some(party) = self.parties.parties.get_mut(&party_id) {
            party.members.retain(|&sid| sid != session_id);
            if party.members.is_empty() {
                disband = true;
            } else if party.leader == session_id {
                if let Some(&new_leader) = party.members.first() {
                    party.leader = new_leader;
                }
            }
        } else {
            return;
        }

        if let Some(p) = self.players.get_mut(&session_id) {
            if p.party_id == Some(party_id) {
                p.party_id = None;
            }
        }

        if disband {
            for sid in members_snapshot {
                if let Some(p) = self.players.get_mut(&sid) {
                    if p.party_id == Some(party_id) {
                        p.party_id = None;
                    }
                }
            }
            self.parties.parties.remove(&party_id);
        }
    }

    /// Remove a player from the world and occupancy tracking. This is used
    /// when a connection fully disconnects so that offline characters no
    /// longer block movement.
    pub fn remove_player_from_world(&mut self, session_id: SessionId) {
        if let Some(p) = self.players.remove(&session_id) {
            self.remove_player_from_occupancy(session_id, p.map_index, p.x, p.y);
        }
    }

    pub fn handle_command(&mut self, cmd: WorldCommand) -> Vec<WorldEvent> {
        let mut events: Vec<WorldEvent> = Vec::new();

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
            WorldCommand::InviteToParty {
                session_id,
                target_name,
            } => {
                // Look up the inviter first so we can apply leadership and
                // cooldown checks.
                let inviter = match self.players.get(&session_id) {
                    Some(p) => p,
                    None => return events,
                };

                // If already in a party, only the leader may invite.
                if let Some(pid) = inviter.party_id {
                    if let Some(party) = self.parties.parties.get(&pid) {
                        if party.leader != session_id {
                            events.push(WorldEvent::PartySystemMessage {
                                session_id,
                                message:
                                    "你不是队长，不能邀请其他玩家加入队伍。".to_string(),
                            });
                            return events;
                        }

                        if party.members.len() >= MAX_GROUP_SIZE {
                            events.push(WorldEvent::PartySystemMessage {
                                session_id,
                                message: "你的队伍人数已经达到上限。".to_string(),
                            });
                            return events;
                        }
                    }
                }

                // Simple cooldown: mirror C# NextGroupInviteTime 行为，静默丢弃过
                // 于频繁的邀请，不发送任何提示。
                if inviter.next_group_invite_time_ms > self.time_ms {
                    return events;
                }

                // 禁止给自己发邀请。
                if inviter.name.eq_ignore_ascii_case(&target_name) {
                    events.push(WorldEvent::PartySystemMessage {
                        session_id,
                        message: "你不能把自己加入队伍。".to_string(),
                    });
                    return events;
                }

                // 从名字解析目标玩家。
                let target_session_id = match self.find_session_by_name(&target_name) {
                    Some(sid) => sid,
                    None => {
                        events.push(WorldEvent::PartySystemMessage {
                            session_id,
                            message: format!("未找到玩家 {}。", target_name),
                        });
                        return events;
                    }
                };

                // 只读检查目标玩家状态，决定是否可以发送邀请。
                let target_name_canonical;
                {
                    let target = match self.players.get(&target_session_id) {
                        Some(p) => p,
                        None => {
                            events.push(WorldEvent::PartySystemMessage {
                                session_id,
                                message: format!("未找到玩家 {}。", target_name),
                            });
                            return events;
                        }
                    };

                    target_name_canonical = target.name.clone();

                    if !target.allow_group {
                        events.push(WorldEvent::PartySystemMessage {
                            session_id,
                            message: format!(
                                "{} 未开启允许组队。",
                                target_name_canonical
                            ),
                        });
                        return events;
                    }

                    if target.party_id.is_some() {
                        events.push(WorldEvent::PartySystemMessage {
                            session_id,
                            message: format!(
                                "{} 已经在其他队伍中了。",
                                target_name_canonical
                            ),
                        });
                        return events;
                    }

                    if target.pending_group_invite_from.is_some() {
                        events.push(WorldEvent::PartySystemMessage {
                            session_id,
                            message: format!(
                                "{} 已经在处理另一位玩家的组队邀请。",
                                target_name_canonical
                            ),
                        });
                        return events;
                    }
                }

                // 所有检查通过，记录这次待处理的组队邀请并设置冷却。
                if let Some(target) = self.players.get_mut(&target_session_id) {
                    target.pending_group_invite_from = Some(session_id);
                }

                if let Some(inviter) = self.players.get_mut(&session_id) {
                    let cooldown_ms: i64 = 10_000;
                    inviter.next_group_invite_time_ms =
                        self.time_ms.saturating_add(cooldown_ms);
                }
            }
            WorldCommand::KickFromParty {
                session_id,
                target_name,
            } => {
                let kicker = match self.players.get(&session_id) {
                    Some(p) => p,
                    None => return events,
                };

                let party_id = match kicker.party_id {
                    Some(pid) => pid,
                    None => {
                        events.push(WorldEvent::PartySystemMessage {
                            session_id,
                            message: "你当前不在任何队伍中。".to_string(),
                        });
                        return events;
                    }
                };

                let party = match self.parties.parties.get(&party_id) {
                    Some(p) => p,
                    None => {
                        events.push(WorldEvent::PartySystemMessage {
                            session_id,
                            message: "你当前不在任何队伍中。".to_string(),
                        });
                        return events;
                    }
                };

                if party.leader != session_id {
                    events.push(WorldEvent::PartySystemMessage {
                        session_id,
                        message:
                            "你不是队长，不能将成员移出队伍。".to_string(),
                    });
                    return events;
                }

                let target_session_id = match self.find_session_by_name(&target_name) {
                    Some(sid) => sid,
                    None => {
                        events.push(WorldEvent::PartySystemMessage {
                            session_id,
                            message: format!(
                                "玩家 {} 不在你的队伍中。",
                                target_name
                            ),
                        });
                        return events;
                    }
                };

                if !party.members.contains(&target_session_id) {
                    events.push(WorldEvent::PartySystemMessage {
                        session_id,
                        message: format!(
                            "玩家 {} 不在你的队伍中。",
                            target_name
                        ),
                    });
                    return events;
                }

                self.leave_party(target_session_id);
            }
            WorldCommand::RespondPartyInvite { session_id, accept } => {
                let (inviter_session_id, invitee_party_id, inviter_party_id, inviter_name, invitee_name, inviter_allow_group) =
                    {
                        let invitee = match self.players.get(&session_id) {
                            Some(p) => p,
                            None => return events,
                        };

                        let inviter_session_id = match invitee.pending_group_invite_from {
                            Some(sid) => sid,
                            None => {
                                events.push(WorldEvent::PartySystemMessage {
                                    session_id,
                                    message:
                                        "你当前没有收到任何组队邀请。".to_string(),
                                });
                                return events;
                            }
                        };

                        let inviter = match self.players.get(&inviter_session_id) {
                            Some(p) => p,
                            None => {
                                events.push(WorldEvent::PartySystemMessage {
                                    session_id,
                                    message: "组队发起者已下线。".to_string(),
                                });
                                if let Some(invitee_mut) =
                                    self.players.get_mut(&session_id)
                                {
                                    if invitee_mut.pending_group_invite_from
                                        == Some(inviter_session_id)
                                    {
                                        invitee_mut.pending_group_invite_from = None;
                                    }
                                }
                                return events;
                            }
                        };

                        (
                            inviter_session_id,
                            invitee.party_id,
                            inviter.party_id,
                            inviter.name.clone(),
                            invitee.name.clone(),
                            inviter.allow_group,
                        )
                    };

                if !accept {
                    // 通知发起人：对方拒绝了组队邀请。
                    events.push(WorldEvent::PartySystemMessage {
                        session_id: inviter_session_id,
                        message: format!(
                            "{} 拒绝了你的组队邀请。",
                            invitee_name
                        ),
                    });

                    if let Some(invitee) = self.players.get_mut(&session_id) {
                        if invitee.pending_group_invite_from
                            == Some(inviter_session_id)
                        {
                            invitee.pending_group_invite_from = None;
                        }
                    }
                    return events;
                }

                // 接受前的各种合法性检查，尽量贴近 C# 行为。
                if let Some(pid) = invitee_party_id {
                    if let Some(party) = self.parties.parties.get(&pid) {
                        if party.members.contains(&session_id) {
                            if let Some(invitee) =
                                self.players.get_mut(&session_id)
                            {
                                invitee.pending_group_invite_from = None;
                            }
                            events.push(WorldEvent::PartySystemMessage {
                                session_id,
                                message: format!(
                                    "你已经在队伍中，无法加入 {} 的队伍。",
                                    inviter_name
                                ),
                            });
                            return events;
                        }
                    }
                }

                if let Some(pid) = inviter_party_id {
                    if let Some(party) = self.parties.parties.get(&pid) {
                        if party.leader != inviter_session_id {
                            if let Some(invitee) =
                                self.players.get_mut(&session_id)
                            {
                                if invitee.pending_group_invite_from
                                    == Some(inviter_session_id)
                                {
                                    invitee.pending_group_invite_from = None;
                                }
                            }
                            events.push(WorldEvent::PartySystemMessage {
                                session_id,
                                message: format!(
                                    "{} 已经不是该队伍的队长了。",
                                    inviter_name
                                ),
                            });
                            return events;
                        }

                        if party.members.len() >= MAX_GROUP_SIZE {
                            if let Some(invitee) =
                                self.players.get_mut(&session_id)
                            {
                                if invitee.pending_group_invite_from
                                    == Some(inviter_session_id)
                                {
                                    invitee.pending_group_invite_from = None;
                                }
                            }
                            events.push(WorldEvent::PartySystemMessage {
                                session_id,
                                message: format!(
                                    "{} 的队伍人数已经达到上限。",
                                    inviter_name
                                ),
                            });
                            return events;
                        }
                    }
                }

                if !inviter_allow_group {
                    if let Some(invitee) = self.players.get_mut(&session_id) {
                        if invitee.pending_group_invite_from
                            == Some(inviter_session_id)
                        {
                            invitee.pending_group_invite_from = None;
                        }
                    }
                    events.push(WorldEvent::PartySystemMessage {
                        session_id,
                        message: format!(
                            "{} 当前未开启允许组队。",
                            inviter_name
                        ),
                    });
                    return events;
                }

                // 通过所有检查后，按原有逻辑创建或加入队伍。
                let party_id = {
                    if let Some(pid) = inviter_party_id {
                        if let Some(party) = self.parties.parties.get_mut(&pid) {
                            if !party.members.contains(&session_id) {
                                party.members.push(session_id);
                            }
                        }
                        pid
                    } else {
                        let pid = self.parties.next_id;
                        let next = self.parties.next_id.wrapping_add(1);
                        self.parties.next_id = if next == 0 { 1 } else { next };

                        let party = Party {
                            id: pid,
                            leader: inviter_session_id,
                            members: vec![inviter_session_id, session_id],
                        };
                        self.parties.parties.insert(pid, party);
                        pid
                    }
                };

                if let Some(invitee) = self.players.get_mut(&session_id) {
                    invitee.party_id = Some(party_id);
                    invitee.pending_group_invite_from = None;
                }

                if let Some(inviter) = self.players.get_mut(&inviter_session_id) {
                    inviter.party_id = Some(party_id);
                }
            }
        }

        events
    }

    fn point_in_safe_zone(info: &map::MapInfo, x: i32, y: i32) -> bool {
        for sz in &info.safe_zones {
            let dx = x - sz.location_x;
            let dy = y - sz.location_y;
            if dx.abs() <= sz.size as i32 && dy.abs() <= sz.size as i32 {
                return true;
            }
        }
        false
    }

    pub(crate) fn process_safezone_healing(&mut self, events: &mut Vec<WorldEvent>) {
        const HEAL_AMOUNT: i32 = 25;

        for player in self.players.values_mut() {
            if player.dead || player.hp <= 0 {
                continue;
            }

            let Some(map_info) = self.provider.get_map_info(player.map_index) else {
                continue;
            };

            if !Self::point_in_safe_zone(map_info, player.x, player.y) {
                continue;
            }

            let max_hp = player.stats.total.get(crate::stats::Stat::HP).max(1);
            if player.hp >= max_hp {
                continue;
            }

            let old_hp = player.hp;
            let new_hp = (old_hp + HEAL_AMOUNT).min(max_hp);
            if new_hp <= old_hp {
                continue;
            }

            let amount = new_hp - old_hp;
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
}
