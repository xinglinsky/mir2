use std::collections::HashMap;

use super::Job;
use crate::stats::{Stat, Stats};
use crate::world::{BuffProperty, BuffStackType, BuffType, Spell};
use crate::world::party::{Party, PartyManager, MAX_GROUP_SIZE};
use crate::guild::{GuildInfo, GuildManager};
use super::configs::{guild_max_experience_for_level, guild_member_cap_for_level, guild_settings, pet_template};
use crate::item::create_fresh_user_item;
use crate::world::config::WorldConfig;
use crate::world::map::{self};
use crate::world::map_item::MapItem;
use crate::world::magic::UserMagic;
use crate::world::monster::{MonsterAiState, MonsterInstance};
use crate::world::monster_runtime::RespawnRuntime;
use crate::world::player::PlayerState;
use crate::world::provider::WorldProvider;
use crate::world::types::PetKind;
use crystal_shared_proto::item_types::{ItemInfoData, UserItemData};

pub type SessionId = u32;

#[derive(Clone, Debug, Default)]
pub(crate) struct CellOccupants {
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
pub enum GuildJoinError {
    NotFound,
    Full,
}

#[derive(Clone, Debug)]
pub struct GuildExpGainResult {
    pub guild: GuildInfo,
    pub exp_gained: u32,
    pub leveled: bool,
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
    ObjectMagic {
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
        spell: u8,
        level: u8,
        target_id: u32,
        target_x: i32,
        target_y: i32,
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
    MagicLeveled {
        session_id: SessionId,
        spell_id: u8,
        level: u8,
        experience: u16,
    },
    /// Notify the client that the cooldown (Delay) for a magic has changed,
    /// typically after the magic levels up. Mirrors C# S.MagicDelay.
    MagicDelay {
        session_id: SessionId,
        spell_id: u8,
        delay: i64,
    },
    /// Notify the client that a magic has just been cast so it can update the
    /// per-spell CastTime used for button cooldowns. Mirrors C# S.MagicCast.
    MagicCast {
        session_id: SessionId,
        spell_id: u8,
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
    PauseBuff {
        session_id: SessionId,
        buff_type: u8, // BuffType as u8
        paused: bool,
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
    /// In-memory GameShop purchase log keyed by GameShopItem GIndex.
    /// This approximates Envir.GameshopLog in the legacy C# server.
    pub(crate) gameshop_log: HashMap<i32, i32>,
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
            next_map_item_id: 1_000_000_000,
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
            gameshop_log: HashMap::new(),
            buyback: HashMap::new(),
        }
    }

    /// Increment experience for the given player's magic and, if the
    /// underlying UserMagic changes, emit a MagicLeveled world event so the
    /// connection layer can notify the client via SMagicLeveled.
    pub(crate) fn level_up_magic_for_player(
        &mut self,
        session_id: SessionId,
        spell: u8,
        events: &mut Vec<WorldEvent>,
    ) {
        use crate::world::magic::level_up_magic_simple;

        let (new_level, new_exp, delay) = {
            let player = match self.players.get_mut(&session_id) {
                Some(p) => p,
                None => return,
            };

            let magic = match player.magics.iter_mut().find(|m| m.spell == spell) {
                Some(m) => m,
                None => return,
            };

            let info = match self.provider.get_magic_info(spell) {
                Some(i) => i,
                None => return,
            };

            let player_level = player.level;
            let skill_mult = player.stats.total.get(Stat::SkillGainMultiplier);

            let changed = level_up_magic_simple(info, magic, player_level, skill_mult);
            if !changed {
                return;
            }

            let new_level = magic.level;
            let new_exp = magic.experience;

            // Compute the new cooldown (Delay) for this magic using the same
            // formula as ClientMagic.Save: DelayBase - Level * DelayReduction.
            let delay: i64 = info.delay_base as i64
                - (new_level as i64 * info.delay_reduction as i64);

            (new_level, new_exp, delay)
        };

        events.push(WorldEvent::MagicLeveled {
            session_id,
            spell_id: spell,
            level: new_level,
            experience: new_exp,
        });

        events.push(WorldEvent::MagicDelay {
            session_id,
            spell_id: spell,
            delay,
        });
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

    pub fn monster_position(&self, map_index: i32, monster_id: u64) -> Option<(i32, i32, u8)> {
        let monsters = self.monsters.get(&map_index)?;
        monsters
            .iter()
            .find(|m| m.id == monster_id)
            .map(|m| (m.x, m.y, m.direction))
    }

    pub fn player_position(&self, session_id: SessionId) -> Option<(i32, i32, i32, u8)> {
        self.players
            .get(&session_id)
            .map(|p| (p.map_index, p.x, p.y, p.direction))
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

    /// Inspect which guild (if any) currently has a pending invite targeted
    /// at the given session. This mirrors C# PendingGuildInvite behaviour.
    pub fn pending_guild_invite(&self, session_id: SessionId) -> Option<String> {
        self.players
            .get(&session_id)
            .and_then(|p| p.pending_guild_invite_from.clone())
    }

    /// Set or overwrite the pending guild invite for the given session.
    pub fn set_pending_guild_invite(&mut self, session_id: SessionId, guild_name: &str) {
        if let Some(p) = self.players.get_mut(&session_id) {
            p.pending_guild_invite_from = Some(guild_name.to_string());
        }
    }

    /// Clear any pending guild invite for the given session.
    pub fn clear_pending_guild_invite(&mut self, session_id: SessionId) {
        if let Some(p) = self.players.get_mut(&session_id) {
            p.pending_guild_invite_from = None;
        }
    }

    /// Take and clear the pending guild invite for the given session,
    /// returning the guild name if present.
    pub fn take_pending_guild_invite(&mut self, session_id: SessionId) -> Option<String> {
        if let Some(p) = self.players.get_mut(&session_id) {
            p.pending_guild_invite_from.take()
        } else {
            None
        }
    }

    /// Inspect who, if anyone, has a pending trade invite targeted at the
    /// given session. This mirrors C# trade invitation behaviour.
    pub fn pending_trade_invite_from(&self, session_id: SessionId) -> Option<SessionId> {
        self.players
            .get(&session_id)
            .and_then(|p| p.pending_trade_invite_from)
    }

    /// Set or overwrite the pending trade invite for the given session.
    pub fn set_pending_trade_invite(&mut self, session_id: SessionId, from: SessionId) {
        if let Some(p) = self.players.get_mut(&session_id) {
            p.pending_trade_invite_from = Some(from);
        }
    }

    /// Clear any pending trade invite for the given session.
    pub fn clear_pending_trade_invite(&mut self, session_id: SessionId) {
        if let Some(p) = self.players.get_mut(&session_id) {
            p.pending_trade_invite_from = None;
        }
    }

    /// Take and clear the pending trade invite for the given session,
    /// returning the inviter session id if present.
    pub fn take_pending_trade_invite(&mut self, session_id: SessionId) -> Option<SessionId> {
        if let Some(p) = self.players.get_mut(&session_id) {
            p.pending_trade_invite_from.take()
        } else {
            None
        }
    }

    pub fn trade_partner_for(&self, session_id: SessionId) -> Option<SessionId> {
        self.players.get(&session_id).and_then(|p| p.trade_partner)
    }

    pub fn is_trade_locked(&self, session_id: SessionId) -> Option<bool> {
        self.players.get(&session_id).map(|p| p.trade_locked)
    }

    pub fn trade_gold_for(&self, session_id: SessionId) -> Option<u32> {
        self.players.get(&session_id).map(|p| p.trade_gold)
    }

    pub fn set_trade_partner_pair(&mut self, a: SessionId, b: SessionId) -> bool {
        if a == b {
            return false;
        }
        let (exists_a, exists_b) = (self.players.contains_key(&a), self.players.contains_key(&b));
        if !exists_a || !exists_b {
            return false;
        }

        if let Some(p) = self.players.get_mut(&a) {
            p.trade_partner = Some(b);
            p.trade_gold = 0;
            p.trade_locked = false;
        }
        if let Some(p) = self.players.get_mut(&b) {
            p.trade_partner = Some(a);
            p.trade_gold = 0;
            p.trade_locked = false;
        }
        true
    }

    pub fn clear_trade_session(&mut self, session_id: SessionId) {
        let partner = match self.players.get(&session_id) {
            Some(p) => p.trade_partner,
            None => return,
        };

        if let Some(p) = self.players.get_mut(&session_id) {
            p.trade_partner = None;
            p.trade_gold = 0;
            p.trade_locked = false;
        }

        if let Some(partner_id) = partner {
            if let Some(p) = self.players.get_mut(&partner_id) {
                p.trade_partner = None;
                p.trade_gold = 0;
                p.trade_locked = false;
            }
        }
    }

    pub fn add_trade_gold(&mut self, session_id: SessionId, amount: u32) -> Option<u32> {
        let p = self.players.get_mut(&session_id)?;
        let new = p.trade_gold.saturating_add(amount);
        p.trade_gold = new;
        Some(new)
    }

    pub fn set_trade_locked(&mut self, session_id: SessionId, locked: bool) {
        if let Some(p) = self.players.get_mut(&session_id) {
            p.trade_locked = locked;
        }
    }

    /// Clear the trade_locked flag for the given session and its trade
    /// partner (if any). This mirrors the C# TradeUnlock behaviour for the
    /// lock state only; item and gold rollback are handled separately.
    pub fn trade_unlock(&mut self, session_id: SessionId) {
        let partner = match self.players.get(&session_id) {
            Some(p) => p.trade_partner,
            None => return,
        };

        self.set_trade_locked(session_id, false);

        if let Some(partner_id) = partner {
            self.set_trade_locked(partner_id, false);
        }
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

        // Initialize member cap and max experience for level 0 from
        // GuildSettings.ini so that newly created guilds follow the same
        // growth rules as the legacy C# server.
        let level = info.level;
        let cap = guild_member_cap_for_level(level);
        if cap > 0 {
            info.member_cap = cap;
        }
        let max_exp = guild_max_experience_for_level(level);
        if max_exp > 0 {
            info.max_experience = max_exp;
        }

        Some(info.clone())
    }

    pub fn init_guilds_from_db(&mut self, guilds: Vec<GuildInfo>) {
        self.guilds = GuildManager::from_guilds(guilds);
    }

    /// Get a cloned snapshot of GuildInfo for the guild with the specified
    /// name, if it exists. This is primarily used by the connection layer
    /// when building guild member lists for the legacy client.
    pub fn get_guild_info_by_name(&self, guild_name: &str) -> Option<GuildInfo> {
        self.guilds.get_guild_by_name(guild_name).cloned()
    }

    /// Replace the notice (bulletin) text for the given guild and return the
    /// updated GuildInfo snapshot on success.
    pub fn guild_update_notice(
        &mut self,
        guild_name: &str,
        notice: Vec<String>,
    ) -> Option<GuildInfo> {
        let guild = self.guilds.get_guild_by_name_mut(guild_name)?;
        guild.notice = notice;
        Some(guild.clone())
    }

    /// Apply raw experience to the specified guild, using GuildSettings
    /// (exp_rate, points_per_level, exp_by_level, member_cap_by_level) to
    /// determine the effective gain and any level-ups. This mirrors the core
    /// behaviour of C# GuildObject.GainExp, but leaves rate/broadcast
    /// throttling to the caller.
    pub fn guild_gain_exp(
        &mut self,
        guild_name: &str,
        base_amount: u32,
    ) -> Option<GuildExpGainResult> {
        let guild = self.guilds.get_guild_by_name_mut(guild_name)?;

        // No progression configured or already at cap.
        if guild.max_experience <= 0 {
            return None;
        }

        let settings = guild_settings();

        if settings.exp_rate <= 0.0 {
            return None;
        }

        // Effective guild exp gain uses ExpRate multiplier, mirroring the C#
        // cast from double to uint which truncates toward zero.
        let exp_amount: u32 = ((base_amount as f64) * (settings.exp_rate as f64)) as u32;
        if exp_amount == 0 {
            return None;
        }

        guild.experience = guild
            .experience
            .saturating_add(exp_amount as i64);

        let mut experience = guild.experience;
        let mut leveled = false;

        // Loop while the cumulative experience exceeds the current level's
        // MaxExperience, advancing levels and updating MaxExperience from
        // GuildSettings. This matches the C# pattern of working on a local
        // "experience" variable without mutating Info.Experience back down
        // to the band.
        while guild.max_experience > 0 && experience > guild.max_experience {
            if guild.level == u8::MAX {
                break;
            }

            leveled = true;
            guild.level = guild.level.saturating_add(1);

            let spare_total = (guild.spare_points as u16)
                .saturating_add(settings.points_per_level as u16);
            guild.spare_points = spare_total.min(u8::MAX as u16) as u8;

            experience = experience.saturating_sub(guild.max_experience);

            let next_max = guild_max_experience_for_level(guild.level);
            guild.max_experience = next_max;
            if guild.max_experience <= 0 {
                break;
            }
        }

        if leveled {
            let cap = guild_member_cap_for_level(guild.level);
            if cap > 0 {
                guild.member_cap = cap;
            }
        }

        Some(GuildExpGainResult {
            guild: guild.clone(),
            exp_gained: exp_amount,
            leveled,
        })
    }

    pub fn guild_add_member_by_name(
        &mut self,
        guild_name: &str,
    ) -> Result<GuildInfo, GuildJoinError> {
        let guild = match self.guilds.get_guild_by_name_mut(guild_name) {
            Some(g) => g,
            None => return Err(GuildJoinError::NotFound),
        };

        if !guild.has_room() {
            return Err(GuildJoinError::Full);
        }

        guild.member_count = guild.member_count.saturating_add(1);
        Ok(guild.clone())
    }

    /// Remove a member with the given name from the specified guild. Returns
    /// the updated GuildInfo and the rank index from which the member was
    /// removed on success.
    pub fn guild_remove_member_by_name(
        &mut self,
        guild_name: &str,
        member_name: &str,
    ) -> Option<(GuildInfo, u8)> {
        let guild = self.guilds.get_guild_by_name_mut(guild_name)?;
        let (rank_index, _member) = guild.remove_member_by_name(member_name)?;
        Some((guild.clone(), rank_index))
    }

    /// Move a member to a different rank within the specified guild. The
    /// new_rank_index is interpreted as the logical rank index as stored in
    /// GuildRank.index, but will fall back to treating it as a zero-based
    /// vector index if no matching GuildRank.index is found. Returns the
    /// updated GuildInfo on success.
    pub fn guild_change_member_rank(
        &mut self,
        guild_name: &str,
        member_name: &str,
        new_rank_index: u8,
    ) -> Option<GuildInfo> {
        let guild = self.guilds.get_guild_by_name_mut(guild_name)?;

        if guild.ranks.is_empty() {
            return None;
        }

        let dest_pos = guild
            .ranks
            .iter()
            .position(|r| r.index == new_rank_index)
            .or_else(|| {
                let idx = new_rank_index as usize;
                if idx < guild.ranks.len() {
                    Some(idx)
                } else {
                    None
                }
            })?;

        let mut moved: Option<crate::guild::GuildMember> = None;
        for rank in &mut guild.ranks {
            if let Some(i) = rank
                .members
                .iter()
                .position(|m| m.name.eq_ignore_ascii_case(member_name))
            {
                let m = rank.members.remove(i);
                moved = Some(m);
                break;
            }
        }

        let member = moved?;
        guild.ranks[dest_pos].members.push(member);
        Some(guild.clone())
    }

    /// Insert a new rank into the specified guild, mirroring the C#
    /// GuildObject.NewRank behaviour. Returns the updated GuildInfo together
    /// with a cloned copy of the newly created rank on success. If the guild
    /// already has 255 ranks, this is a no-op.
    pub fn guild_new_rank(
        &mut self,
        guild_name: &str,
    ) -> Option<(GuildInfo, crate::guild::GuildRank)> {
        let guild = self.guilds.get_guild_by_name_mut(guild_name)?;

        if guild.ranks.len() >= u8::MAX as usize {
            return None;
        }

        let new_index: u8 = if guild.ranks.len() > 1 {
            (guild.ranks.len() - 1) as u8
        } else {
            1
        };

        let new_rank = crate::guild::GuildRank {
            index: new_index,
            name: format!("Rank-{}", new_index),
            options: 0,
            members: Vec::new(),
        };

        let new_rank_clone = new_rank.clone();

        let insert_pos = new_index as usize;
        if insert_pos <= guild.ranks.len() {
            guild.ranks.insert(insert_pos, new_rank);
        } else {
            guild.ranks.push(new_rank);
        }

        let len = guild.ranks.len();
        if len > 0 {
            let last_idx = len - 1;
            if let Some(last) = guild.ranks.get_mut(last_idx) {
                last.index = last_idx as u8;
            }
        }

        Some((guild.clone(), new_rank_clone))
    }

    /// Change the name of an existing rank within the specified guild.
    /// Returns the updated GuildInfo on success.
    pub fn guild_change_rank_name(
        &mut self,
        guild_name: &str,
        rank_index: u8,
        new_name: &str,
    ) -> Option<GuildInfo> {
        let guild = self.guilds.get_guild_by_name_mut(guild_name)?;
        let idx = rank_index as usize;
        if idx >= guild.ranks.len() {
            return None;
        }

        guild.ranks[idx].name = new_name.to_string();
        Some(guild.clone())
    }

    /// Change a single option flag on a rank within the specified guild.
    /// The option parameter selects the bit (0-7). When enabled is true the
    /// bit is set; when false, the bit is toggled, mirroring the C#
    /// GuildObject.ChangeRankOption behaviour.
    pub fn guild_change_rank_option(
        &mut self,
        guild_name: &str,
        rank_index: u8,
        option: u8,
        enabled: bool,
    ) -> Option<GuildInfo> {
        let guild = self.guilds.get_guild_by_name_mut(guild_name)?;
        let idx = rank_index as usize;
        if idx >= guild.ranks.len() || option > 7 {
            return None;
        }

        let mask = 1u8 << option;
        let rank = &mut guild.ranks[idx];
        if enabled {
            rank.options |= mask;
        } else {
            rank.options ^= mask;
        }

        Some(guild.clone())
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

    /// Find a suitable drop location around the given centre tile, mirroring
    /// the C# ItemObject.Drop behaviour. The search expands in rings from the
    /// centre up to max_distance, preferring:
    ///   1) The first completely empty, walkable, non-movement, non-blocked
    ///      cell it finds.
    ///   2) Otherwise, the cell with the fewest existing map items, subject
    ///      to a maximum stack size, so that drops spread out when many items
    ///      are present.
    pub(crate) fn find_drop_location(
        &self,
        map_index: i32,
        center_x: i32,
        center_y: i32,
        max_distance: i32,
    ) -> Option<(i32, i32)> {
        let map = self.get_or_load_map(map_index)?;
        let info = &map.info;

        let mut best_location: Option<(i32, i32)> = None;
        let mut best_count: usize = 0;

        let items_on_map = self.map_items.get(&map_index);

        let max_d = if max_distance < 0 { 0 } else { max_distance };

        for d in 0..=max_d {
            for y in (center_y - d)..=(center_y + d) {
                if y < 0 || y >= map.height as i32 {
                    continue;
                }

                let dy = (y - center_y).abs();
                let step = if d == 0 || dy == d { 1 } else { (d * 2).max(1) };

                let mut x = center_x - d;
                while x <= center_x + d {
                    if x < 0 || x >= map.width as i32 {
                        x += step;
                        continue;
                    }

                    let ux = x as u16;
                    let uy = y as u16;

                    // Only consider walkable tiles, approximating
                    // CurrentMap.ValidPoint in C#.
                    if !map.is_walkable(ux, uy) {
                        x += step;
                        continue;
                    }

                    // Skip movement source tiles so players do not drop items
                    // directly on teleports, mirroring the C# MovementInfo
                    // check in ItemObject.Drop.
                    if info
                        .movements
                        .iter()
                        .any(|m| m.source_x == x && m.source_y == y)
                    {
                        x += step;
                        continue;
                    }

                    // Treat any occupancy by players or monsters as
                    // blocking, approximating MapObject.Blocking.
                    if self.is_cell_blocked(map_index, x, y) {
                        x += step;
                        continue;
                    }

                    // Count existing map items on this tile for stack-size
                    // enforcement and best-cell selection.
                    let mut count: usize = 0;
                    if let Some(items) = items_on_map {
                        for mi in items.iter().filter(|mi| mi.x == x && mi.y == y) {
                            let _ = mi;
                            count = count.saturating_add(1);
                        }
                    }

                    // Match C# Settings.DropStackSize default of 5.
                    const DROP_STACK_SIZE: usize = 5;
                    if count >= DROP_STACK_SIZE {
                        x += step;
                        continue;
                    }

                    // Prefer the first completely empty tile.
                    if count == 0 {
                        return Some((x, y));
                    }

                    // Otherwise track the tile with the smallest stack so far.
                    if best_location.is_none() || count < best_count {
                        best_location = Some((x, y));
                        best_count = count;
                    }

                    x += step;
                }
            }
        }

        best_location
    }

    /// Choose an inventory slot for a newly gained item, mirroring the C#
    /// HumanObject.AddItem behaviour:
    ///
    /// - Potions, scrolls and scripts with Effect == 1 prefer the potion
    ///   belt slots.
    /// - Amulets prefer the amulet belt slots.
    /// - All other items prefer the main bag region first (slots at or above
    ///   BeltSize), only falling back to belt slots when the bag is full.
    fn pickup_slot_for_item(player: &PlayerState, info: &ItemInfoData) -> Option<usize> {
        // Item type numeric values mirrored from C# ItemType enum.
        const ITEM_TYPE_AMULET: u8 = 8;
        const ITEM_TYPE_POTION: u8 = 13;
        const ITEM_TYPE_SCROLL: u8 = 17;
        const ITEM_TYPE_SCRIPT: u8 = 21;

        // Belt layout mirrored from C# HumanObject.PotionBeltMinimum /
        // PotionBeltMaximum / AmuletBeltMinimum / AmuletBeltMaximum /
        // BeltSize for players (not heroes):
        // - Potion belt: indices 0..4
        // - Amulet belt: indices 4..6
        // - Main bag:    indices 6..inventory.len()
        const POTION_BELT_MIN: usize = 0;
        const POTION_BELT_MAX: usize = 4; // exclusive
        const AMULET_BELT_MIN: usize = 4;
        const AMULET_BELT_MAX: usize = 6; // exclusive
        const BELT_SIZE: usize = 6;

        let slots = &player.inventory.slots;

        let is_quick_potion_scroll_or_script = info.item_type == ITEM_TYPE_POTION
            || info.item_type == ITEM_TYPE_SCROLL
            || (info.item_type == ITEM_TYPE_SCRIPT && info.effect == 1);

        if is_quick_potion_scroll_or_script {
            // First preference: potion belt slots.
            for idx in POTION_BELT_MIN..POTION_BELT_MAX {
                if idx < slots.len() && slots[idx].is_none() {
                    return Some(idx);
                }
            }
        } else if info.item_type == ITEM_TYPE_AMULET {
            // Amulets prefer the dedicated amulet belt slots.
            for idx in AMULET_BELT_MIN..AMULET_BELT_MAX {
                if idx < slots.len() && slots[idx].is_none() {
                    return Some(idx);
                }
            }
        } else {
            // All other items (equipment, etc.) prefer the main bag region
            // first, to avoid filling belt quick-slots with gear.
            for idx in BELT_SIZE..slots.len() {
                if slots[idx].is_none() {
                    return Some(idx);
                }
            }
        }

        // Fallback: any free slot, including belt, when the preferred region
        // is full. This matches the final loop in C# AddItem.
        for (idx, slot) in slots.iter().enumerate() {
            if slot.is_none() {
                return Some(idx);
            }
        }

        None
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

    pub fn spawn_pet_for_player(
        &mut self,
        session_id: SessionId,
        pet_kind: PetKind,
    ) -> Option<u64> {
        let (map_index, x, y, direction, job, current_pet) = {
            let player = self.players.get(&session_id)?;
            (
                player.map_index,
                player.x,
                player.y,
                player.direction,
                player.job,
                player.main_pet_id,
            )
        };

        if job != Job::Taoist {
            return None;
        }

        if current_pet.is_some() {
            return None;
        }

        let template = pet_template(pet_kind)?;
        if template.monster_index <= 0 {
            return None;
        }

        let info = match self.provider.get_monster_info(template.monster_index) {
            Some(i) => i,
            None => return None,
        };

        self.next_monster_id = self.next_monster_id.wrapping_add(1);
        let hp = info.stats.get(Stat::HP).max(1);

        let instance = MonsterInstance {
            id: self.next_monster_id,
            monster_index: template.monster_index,
            map_index,
            x,
            y,
            home_x: x,
            home_y: y,
            direction,
            hp,
            is_pet: true,
            owner_session_id: Some(session_id),
            pet_kind: Some(pet_kind),
            respawn_index: 0,
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
        };

        self.add_monster_to_occupancy(instance.id, map_index, instance.x, instance.y);

        let monsters = self.monsters.entry(map_index).or_default();
        monsters.push(instance);

        if let Some(player) = self.players.get_mut(&session_id) {
            player.main_pet_id = Some(self.next_monster_id);
        }

        Some(self.next_monster_id)
    }

    pub fn remove_all_pets_for_session(&mut self, session_id: SessionId) {
        let mut to_remove = Vec::new();

        for (map_index, monsters) in &self.monsters {
            for m in monsters {
                if m.is_pet && m.owner_session_id == Some(session_id) {
                    to_remove.push((*map_index, m.id, m.x, m.y));
                }
            }
        }

        for (map_index, monster_id, x, y) in to_remove {
            if let Some(monsters) = self.monsters.get_mut(&map_index) {
                if let Some(idx) = monsters.iter().position(|m| m.id == monster_id) {
                    monsters.remove(idx);
                }
            }
            self.remove_monster_from_occupancy(monster_id, map_index, x, y);
        }

        if let Some(player) = self.players.get_mut(&session_id) {
            player.main_pet_id = None;
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
    
    /// Query the total GameShop purchases for a given GIndex across the
    /// entire server. This mirrors Envir.GameshopLog in the C# server.
    pub fn gameshop_purchased_global(&self, g_index: i32) -> i32 {
        *self.gameshop_log.get(&g_index).unwrap_or(&0)
    }

    /// Query the per-player GameShop purchases for a given GIndex. This
    /// approximates CharacterInfo.GSpurchases in the C# server but is kept
    /// purely in-memory for now.
    pub fn gameshop_purchased_for_player(&self, session_id: SessionId, g_index: i32) -> i32 {
        self
            .players
            .get(&session_id)
            .and_then(|p| p.gs_purchases.get(&g_index).copied())
            .unwrap_or(0)
    }

    /// Increment the per-player GameShop purchase count for a given GIndex
    /// by the specified quantity.
    pub fn increment_gameshop_purchases_for_player(
        &mut self,
        session_id: SessionId,
        g_index: i32,
        quantity: i32,
    ) {
        if quantity <= 0 {
            return;
        }
        if let Some(p) = self.players.get_mut(&session_id) {
            let entry = p.gs_purchases.entry(g_index).or_insert(0);
            *entry = entry.saturating_add(quantity);
        }
    }

    /// Increment the global GameShop purchase log for a given GIndex by the
    /// specified quantity.
    pub fn increment_gameshop_log(&mut self, g_index: i32, quantity: i32) {
        if quantity <= 0 {
            return;
        }
        let entry = self.gameshop_log.entry(g_index).or_insert(0);
        *entry = entry.saturating_add(quantity);
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

                // Use a C#-style ItemObject.Drop search to find a nearby
                // valid tile for the dropped stack, matching Settings.DropRange
                // (4) for the search radius.
                let (drop_x, drop_y) = match self.find_drop_location(map_index, px, py, 4) {
                    Some(pos) => pos,
                    None => return events,
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
                    x: drop_x,
                    y: drop_y,
                    item_index: Some(info.index),
                    gold: 0,
                    count: dropped_item.count,
                    item: Some(dropped_item.clone()),
                    expire_time_ms: self.time_ms + item_timeout_ms,
                });

                events.push(WorldEvent::ItemDropped {
                    object_id: map_item_id,
                    map_index,
                    x: drop_x,
                    y: drop_y,
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
                            tracing::debug!(
                                "[pickup] gold map_item: session={} map={} pos=({}, {}) object_id={} gold={}",
                                session_id,
                                map_index,
                                map_item.x,
                                map_item.y,
                                map_item.id,
                                map_item.gold
                            );

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

                            tracing::trace!(
                                "[pickup] MapItemRemoved queued: object_id={} map={} pos=({}, {})",
                                map_item.id,
                                map_index,
                                map_item.x,
                                map_item.y
                            );

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

                            // Choose an inventory slot based on item type so
                            // that consumables go to the belt quick-slots and
                            // equipment/general items go to the main bag
                            // first, mirroring the C# AddItem logic.
                            let info = match self
                                .provider
                                .get_item_info(user_item.item_index)
                            {
                                Some(i) => i.clone(),
                                None => return events,
                            };

                            if let Some(player_state) =
                                self.players.get_mut(&session_id)
                            {
                                if let Some(slot) =
                                    Self::pickup_slot_for_item(player_state, &info)
                                {
                                    player_state.inventory.slots[slot] =
                                        Some(user_item.clone());

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

    pub fn add_player_buff(
        &mut self,
        session_id: SessionId,
        buff_type: BuffType,
        duration_ms: i64,
        stats: Stats,
        values: Vec<i32>,
        events: &mut Vec<WorldEvent>,
    ) {
        let now_ms = self.time_ms;

        let player = match self.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let buff_info = match self.provider.get_buff_info(buff_type) {
            Some(info) => info,
            None => return,
        };

        // Determine whether the player is currently inside any SafeZone on
        // their map so that PauseInSafeZone buffs can start in paused state
        // when applied inside town, mirroring C# MapObject.AddBuff.
        let in_safe_zone = self
            .provider
            .get_map_info(player.map_index)
            .map(|info| Self::point_in_safe_zone(info, player.x, player.y))
            .unwrap_or(false);

        if matches!(buff_type, BuffType::MagicShield | BuffType::ElementalBarrier) {
            tracing::debug!(
                "add_player_buff: start buff_type={:?} session_id={} now_ms={} duration_ms={} in_safe_zone={} active_buffs_before={}",
                buff_type,
                session_id,
                now_ms,
                duration_ms,
                in_safe_zone,
                player.active_buffs.len(),
            );
        }

        let infinite = matches!(buff_info.stack_type, BuffStackType::Infinite);

        // Find any existing buff of this type on the player.
        let existing_index = player
            .active_buffs
            .iter()
            .position(|b| b.buff_type == buff_type);

        if let Some(idx) = existing_index {
            let buff = &mut player.active_buffs[idx];

            buff.infinite = infinite;

            // Update duration/stat behaviour based on BuffStackType, closely
            // mirroring C# MapObject.AddBuff.
            match buff_info.stack_type {
                BuffStackType::ResetDuration => {
                    if !infinite && duration_ms > 0 {
                        buff.expire_time_ms = now_ms.saturating_add(duration_ms);
                    }
                }
                BuffStackType::StackDuration => {
                    if !infinite && duration_ms > 0 {
                        let base = buff.expire_time_ms.max(now_ms);
                        buff.expire_time_ms = base.saturating_add(duration_ms);
                    }
                }
                BuffStackType::StackStat => {
                    buff.stats.add(&stats);
                }
                BuffStackType::StackStatAndDuration => {
                    buff.stats.add(&stats);
                    if !infinite && duration_ms > 0 {
                        let base = buff.expire_time_ms.max(now_ms);
                        buff.expire_time_ms = base.saturating_add(duration_ms);
                    }
                }
                BuffStackType::ResetStat => {
                    buff.stats = stats;
                }
                BuffStackType::ResetStatAndDuration => {
                    buff.stats = stats;
                    if !infinite && duration_ms > 0 {
                        buff.expire_time_ms = now_ms.saturating_add(duration_ms);
                    }
                }
                BuffStackType::Infinite | BuffStackType::None => {}
            }

            buff.values = values;
            buff.visible = buff_info.visible;

            if buff_info.has_property(BuffProperty::PauseInSafeZone) && in_safe_zone {
                buff.paused = true;
                if !buff.infinite {
                    let remaining = buff.expire_time_ms.saturating_sub(now_ms).max(0);
                    buff.pause_remaining_ms = remaining;
                } else {
                    buff.pause_remaining_ms = 0;
                }
            }
        } else {
            let expire_time_ms = if infinite || duration_ms <= 0 {
                0
            } else {
                now_ms.saturating_add(duration_ms)
            };

            let mut buff = crate::world::buff::PlayerBuff::new(buff_type, expire_time_ms);
            buff.stats = stats;
            buff.values = values;
            buff.visible = buff_info.visible;
            buff.infinite = infinite;
            buff.caster_id = Some(session_id);

            if buff_info.has_property(BuffProperty::PauseInSafeZone) && in_safe_zone {
                buff.paused = true;
                if !buff.infinite && duration_ms > 0 {
                    buff.pause_remaining_ms = duration_ms.max(0);
                }
            }

            player.active_buffs.push(buff);
        }

        if matches!(buff_type, BuffType::MagicShield | BuffType::ElementalBarrier) {
            if let Some(p) = self.players.get(&session_id) {
                tracing::debug!(
                    "add_player_buff: after mutation buff_type={:?} session_id={} active_buffs_now={} has_magicshield={}",
                    buff_type,
                    session_id,
                    p.active_buffs.len(),
                    p.active_buffs.iter().any(|b| b.buff_type == BuffType::MagicShield),
                );
            }
        }

        // Recalculate buff-derived stats after mutating the active buff list.
        if let Some(p) = self.players.get_mut(&session_id) {
            p.stats.buffs.clear();
            for b in &p.active_buffs {
                p.stats.buffs.add(&b.stats);
            }
            p.stats.recalc_if_dirty_for_job(p.job);
        }

        // Emit an AddBuff world event so the connection layer can send
        // SAddBuff to the client. Only visible buffs are serialized.
        if let Some(p) = self.players.get(&session_id) {
            if let Some(buff) = p.active_buffs.iter().find(|b| b.buff_type == buff_type) {
                if buff.visible {
                    let mut buff_bytes = Vec::new();
                    buff.encode(&mut buff_bytes, session_id);
                    if matches!(buff_type, BuffType::MagicShield | BuffType::ElementalBarrier) {
                        tracing::debug!(
                            "add_player_buff: pushing AddBuff event buff_type={:?} session_id={} visible={} buff_len={}",
                            buff_type,
                            session_id,
                            buff.visible,
                            buff_bytes.len(),
                        );
                    }
                    events.push(WorldEvent::AddBuff {
                        session_id,
                        buff_bytes,
                    });
                }
            }
        }
    }

    pub fn add_monster_buff(
        &mut self,
        map_index: i32,
        monster_id: u64,
        buff_type: BuffType,
        duration_ms: i64,
        stats: Stats,
    ) {
        let now_ms = self.time_ms;

        let buff_info = match self.provider.get_buff_info(buff_type) {
             Some(info) => info,
             None => return,
        };

        let monsters = match self.monsters.get_mut(&map_index) {
             Some(list) => list,
             None => return,
        };

        let monster = match monsters.iter_mut().find(|m| m.id == monster_id) {
             Some(m) => m,
             None => return,
        };

        let infinite = matches!(buff_info.stack_type, BuffStackType::Infinite);

        let existing_index = monster
            .buffs
            .iter()
            .position(|b| b.buff_type == buff_type);

        if let Some(idx) = existing_index {
            let buff = &mut monster.buffs[idx];

            buff.infinite = infinite;

            match buff_info.stack_type {
                BuffStackType::ResetDuration => {
                    if !infinite && duration_ms > 0 {
                        buff.expire_time_ms = now_ms.saturating_add(duration_ms);
                    }
                }
                BuffStackType::StackDuration => {
                    if !infinite && duration_ms > 0 {
                        let base = buff.expire_time_ms.max(now_ms);
                        buff.expire_time_ms = base.saturating_add(duration_ms);
                    }
                }
                BuffStackType::StackStat => {
                    buff.stats.add(&stats);
                }
                BuffStackType::StackStatAndDuration => {
                    buff.stats.add(&stats);
                    if !infinite && duration_ms > 0 {
                        let base = buff.expire_time_ms.max(now_ms);
                        buff.expire_time_ms = base.saturating_add(duration_ms);
                    }
                }
                BuffStackType::ResetStat => {
                    buff.stats = stats;
                }
                BuffStackType::ResetStatAndDuration => {
                    buff.stats = stats;
                    if !infinite && duration_ms > 0 {
                        buff.expire_time_ms = now_ms.saturating_add(duration_ms);
                    }
                }
                BuffStackType::Infinite | BuffStackType::None => {}
            }
        } else {
            let expire_time_ms = if infinite || duration_ms <= 0 {
                0
            } else {
                now_ms.saturating_add(duration_ms)
            };

            let buff = crate::world::monster::MonsterBuff {
                buff_type,
                expire_time_ms,
                stats,
                infinite,
                flag_for_removal: false,
            };

            monster.buffs.push(buff);
        }

        monster.buff_stats.clear();
        for b in &monster.buffs {
            monster.buff_stats.add(&b.stats);
        }
    }

    pub(crate) fn point_in_safe_zone(info: &map::MapInfo, x: i32, y: i32) -> bool {
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
