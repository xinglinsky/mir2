use crate::item::{Equipment, Inventory};
use crystal_shared_proto::item_types::{ItemInfoData, UserItemData};
use crate::stats::Stat;
use crate::stats_util::aggregate_equipment_stats;
use crate::world::magic::UserMagic;
use crate::world::buff::PlayerBuff;
use crate::world::party::PartyId;
use crate::world::provider::WorldProvider;
use crystal_shared_proto::io::{write_bool, write_i32_le, write_string};
use std::collections::{HashMap, HashSet};

use super::{Job, PlayerStats, SessionId, World};
use crate::quest::{QuestId, QuestProgress};
use crate::world::types::{AttackMode, PetMode};

#[derive(Clone, Debug)]
pub struct FriendEntry {
    pub index: i32,
    pub name: String,
    pub memo: String,
    pub blocked: bool,
}

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub name: String,
    pub session_id: SessionId,
    pub character_index: i32,
    pub hair: u8,
    pub map_index: i32,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub level: u16,
    pub experience: i64,
    pub job: Job,
    pub gender: u8,
    pub guild_name: String,
    pub main_pet_id: Option<u64>,
    pub pet_focus_target_monster_id: Option<u64>,
    pub allow_observe: bool,
    pub allow_group: bool,
    pub allow_trade: bool,
    pub spell_toggles: HashSet<u8>,
    pub party_id: Option<PartyId>,
    pub pending_group_invite_from: Option<SessionId>,
    pub pending_guild_invite_from: Option<String>,
    pub pending_trade_invite_from: Option<SessionId>,
    pub next_group_invite_time_ms: i64,
    pub magics: Vec<UserMagic>,
    pub active_buffs: Vec<PlayerBuff>,
    pub stats: PlayerStats,
    pub hp: i32,
    pub mp: i32,
    pub next_regen_time_ms: i64,
    pub dead: bool,
    pub hidden: bool,
    pub inventory: Inventory,
    pub equipment: Equipment,
    /// Refine slots for item refinement system (16 slots, matching C# CharacterInfo.Refine)
    pub refine_slots: Vec<Option<UserItemData>>,
    /// Current item being refined (matching C# CharacterInfo.CurrentRefine)
    pub current_refine: Option<UserItemData>,
    /// Time remaining for refine completion in milliseconds (matching C# CharacterInfo.RefineTimeRemaining)
    pub refine_time_remaining_ms: i64,
    pub riding_mount: bool,
    pub slaying_charged: bool,
    pub trade_partner: Option<SessionId>,
    pub trade_gold: u32,
    pub trade_locked: bool,
    pub trade: Vec<Option<UserItemData>>,
    pub pk_points: i32,
    pub brown_time_ms: i64,
    pub next_pk_decay_ms: i64,
    pub last_revival_time_ms: i64,
    pub attack_mode: u8,
    pub pet_mode: u8,
    pub next_action_time_ms: i64,
    pub next_spell_time_ms: i64,
    pub in_trap_rock: bool,
    /// Active poisons applied to this player. This mirrors the legacy C#
    /// MapObject.PoisonList and is processed by player_runtime.
    pub poisons: Vec<crate::world::PoisonInstance>,
    /// Bitmask of current poison types for quick comparison and client sync.
    pub current_poison_mask: u16,
    /// Per-player GameShop purchase counts keyed by GameShopItem GIndex.
    pub gs_purchases: HashMap<i32, i32>,
    pub npc_data: HashMap<String, String>,
    pub friends: Vec<FriendEntry>,
    pub quests: HashMap<QuestId, QuestProgress>,
    pub completed_quests: Vec<i32>,
    pub reincarnation_host_session_id: Option<SessionId>,
    pub reincarnation_ready: bool,
    pub reincarnation_target_session_id: Option<SessionId>,
    pub reincarnation_expire_time_ms: i64,
}

impl<P: WorldProvider> World<P> {
    pub(super) fn upsert_player(
        &mut self,
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
    ) -> &PlayerState {
        let is_new = !self.players.contains_key(&session_id);
        let player_name = name;

        let start_inventory = if is_new {
            Some(self.build_start_inventory(job, gender))
        } else {
            None
        };

        self.players
            .entry(session_id)
            .and_modify(|p| {
                p.character_index = character_index;
                p.name = player_name.clone();
                p.map_index = map_index;
                p.x = x;
                p.y = y;
                p.direction = direction;
                p.level = level;
                p.experience = experience;
                p.job = job;
                p.gender = gender;
                p.dead = false;
                p.reincarnation_host_session_id = None;
                p.reincarnation_ready = false;
                p.reincarnation_target_session_id = None;
                p.reincarnation_expire_time_ms = 0;
                p.stats.set_base_from_level(job, level);
                p.stats.recalc_if_dirty_for_job(job);
            })
            .or_insert_with(|| {
                let mut stats = PlayerStats::default();
                stats.set_base_from_level(job, level);
                stats.recalc_if_dirty_for_job(job);

                let max_hp = stats.total.get(Stat::HP).max(1);
                let max_mp = stats.total.get(Stat::MP).max(0);

                let inventory = start_inventory.unwrap_or_else(Inventory::new_default);
                let equipment = Equipment::new_default();

                PlayerState {
                    name: player_name.clone(),
                    session_id,
                    character_index,
                    hair: 0,
                    map_index,
                    x,
                    y,
                    direction,
                    level,
                    experience,
                    job,
                    gender,
                    guild_name: String::new(),
                    main_pet_id: None,
                    pet_focus_target_monster_id: None,
                    allow_observe: false,
                    allow_group: true,
                    allow_trade: true,
                    spell_toggles: HashSet::new(),
                    party_id: None,
                    pending_group_invite_from: None,
                    pending_guild_invite_from: None,
                    pending_trade_invite_from: None,
                    next_group_invite_time_ms: 0,
                    magics,
                    active_buffs: Vec::new(),
                    stats,
                    hp: max_hp,
                    mp: max_mp,
                    next_regen_time_ms: 0,
                    dead: false,
                    hidden: false,
                    inventory,
                    equipment,
                    refine_slots: vec![None; 16], // 16 refine slots, matching C# CharacterInfo.Refine
                    current_refine: None, // No item being refined initially
                    refine_time_remaining_ms: 0, // No refine time remaining initially
                    riding_mount: false,
                    slaying_charged: false,
                    trade_partner: None,
                    trade_gold: 0,
                    trade_locked: false,
                    trade: vec![None; 10],
                    pk_points: 0,
                    brown_time_ms: 0,
                    next_pk_decay_ms: 0,
                    last_revival_time_ms: 0,
                    attack_mode: 0,
                    pet_mode: 0,
                    next_action_time_ms: 0,
                    next_spell_time_ms: 0,
                    in_trap_rock: false,
                    poisons: Vec::new(),
                    current_poison_mask: 0,
                    gs_purchases: HashMap::new(),
                    npc_data: HashMap::new(),
                    friends: Vec::new(),
                    quests: HashMap::new(),
                    completed_quests: Vec::new(),
                    reincarnation_host_session_id: None,
                    reincarnation_ready: false,
                    reincarnation_target_session_id: None,
                    reincarnation_expire_time_ms: 0,
                }
            });

        self.players.get(&session_id).unwrap()
    }

    pub fn set_player_level_and_experience(
        &mut self,
        session_id: SessionId,
        level: u16,
        experience: i64,
    ) -> Option<()> {
        // Update the underlying PlayerState first.
        let updated = {
            if let Some(player) = self.players.get_mut(&session_id) {
                player.level = level;
                player.experience = experience;
                let job = player.job;
                player.stats.set_base_from_level(job, level);
                player.stats.recalc_if_dirty_for_job(job);

                // After recalculating stats for the new level, reset the
                // world-side current HP/MP to the new maxima. This mirrors
                // the C# LevelUp behaviour where RefreshStats is followed by
                // SetHP(Stats[HP]) and SetMP(Stats[MP]), ensuring that
                // subsequent monster damage and SHealthChanged packets operate
                // on the correct post-level values.
                let max_hp = player.stats.total.get(Stat::HP).max(1);
                let max_mp = player.stats.total.get(Stat::MP).max(0);
                player.hp = max_hp;
                player.mp = max_mp;
                true
            } else {
                false
            }
        };

        if !updated {
            return None;
        }

        // Refresh the in-memory ranking tables for this player so that
        // subsequent CGetRanking requests see the new level/experience.
        self.update_ranking_for_session(session_id);

        Some(())
    }

    pub fn set_player_guild_name(&mut self, session_id: SessionId, guild_name: &str) {
        if let Some(player) = self.players.get_mut(&session_id) {
            player.guild_name = guild_name.to_string();
        }
    }

    pub fn player_guild_name(&self, session_id: SessionId) -> Option<String> {
        self.players.get(&session_id).map(|p| p.guild_name.clone())
    }

    pub fn friends_for_player(&self, session_id: SessionId) -> Vec<FriendEntry> {
        self.players
            .get(&session_id)
            .map(|p| p.friends.clone())
            .unwrap_or_default()
    }

    pub fn add_friend_entry_for_player(
        &mut self,
        session_id: SessionId,
        friend_index: i32,
        friend_name: &str,
        blocked: bool,
    ) -> bool {
        let player = match self.players.get_mut(&session_id) {
            Some(p) => p,
            None => return false,
        };

        if player.friends.iter().any(|f| f.index == friend_index) {
            return false;
        }

        player.friends.push(FriendEntry {
            index: friend_index,
            name: friend_name.to_string(),
            memo: String::new(),
            blocked,
        });
        true
    }

    pub fn remove_friend_for_player(&mut self, session_id: SessionId, friend_index: i32) -> bool {
        let player = match self.players.get_mut(&session_id) {
            Some(p) => p,
            None => return false,
        };

        let before = player.friends.len();
        player.friends.retain(|f| f.index != friend_index);
        player.friends.len() != before
    }

    pub fn set_friend_memo_for_player(
        &mut self,
        session_id: SessionId,
        friend_index: i32,
        memo: String,
    ) -> bool {
        if memo.is_empty() || memo.chars().count() > 200 {
            return false;
        }

        let player = match self.players.get_mut(&session_id) {
            Some(p) => p,
            None => return false,
        };

        if let Some(entry) = player.friends.iter_mut().find(|f| f.index == friend_index) {
            entry.memo = memo;
            true
        } else {
            false
        }
    }

    pub fn encode_friends_bytes_for_player(&self, session_id: SessionId) -> Vec<u8> {
        let mut buf = Vec::new();
        let friends = match self.players.get(&session_id) {
            Some(p) => &p.friends,
            None => {
                let _ = write_i32_le(&mut buf, 0);
                return buf;
            }
        };

        let count = friends.len().min(i32::MAX as usize) as i32;
        let _ = write_i32_le(&mut buf, count);

        for f in friends {
            let _ = write_i32_le(&mut buf, f.index);
            let _ = write_string(&mut buf, &f.name);
            let _ = write_string(&mut buf, &f.memo);
            let _ = write_bool(&mut buf, f.blocked);

            // Online flag: true if any active player session currently uses
            // this character index. This mirrors ClientFriend.Online in the
            // legacy C# server.
            let online = self
                .players
                .values()
                .any(|p| p.character_index == f.index);
            let _ = write_bool(&mut buf, online);
        }

        buf
    }

    pub fn set_player_npc_data(&mut self, session_id: SessionId, key: &str, value: String) {
        if let Some(player) = self.players.get_mut(&session_id) {
            player.npc_data.insert(key.to_string(), value);
        }
    }

    pub fn get_player_npc_data(&self, session_id: SessionId, key: &str) -> Option<String> {
        self.players
            .get(&session_id)
            .and_then(|p| p.npc_data.get(key).cloned())
    }

    pub fn set_player_attack_mode(&mut self, session_id: SessionId, mode: u8) {
        let amode = AttackMode::from_u8(mode);
        if let Some(player) = self.players.get_mut(&session_id) {
            player.attack_mode = amode.as_u8();
        }
    }

    pub fn set_player_pet_mode(&mut self, session_id: SessionId, mode: u8) {
        if let Some(player) = self.players.get_mut(&session_id) {
            let pmode = PetMode::from_u8(mode);
            player.pet_mode = pmode.as_u8();
        }
    }

    pub fn set_player_pet_focus_target_monster(
        &mut self,
        session_id: SessionId,
        target_id: Option<u64>,
    ) {
        if let Some(player) = self.players.get_mut(&session_id) {
            player.pet_focus_target_monster_id = target_id;
        }
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

    /// Update the quickbar key binding for a magic on the given player.
    /// This mirrors the C# MirConnection.MagicKey handler where a given
    /// key index is unique per actor, so any existing magic using the
    /// same key is cleared before assigning the new one.
    pub fn set_magic_key_for_player(
        &mut self,
        session_id: SessionId,
        spell: u8,
        key: u8,
    ) -> Option<Vec<UserMagic>> {
        let player = self.players.get_mut(&session_id)?;
        for magic in &mut player.magics {
            if magic.spell != spell {
                if magic.key == key {
                    magic.key = 0;
                }
                continue;
            }

            magic.key = key;
        }
        Some(player.magics.clone())
    }

    /// Get a cloned list of learned magics for the given player session.
    pub fn player_magics(&self, session_id: SessionId) -> Vec<UserMagic> {
        self.players
            .get(&session_id)
            .map(|p| p.magics.clone())
            .unwrap_or_default()
    }

    pub fn recalc_player_equipment_stats(&mut self, session_id: SessionId) {
        let (job, equip_stats) = if let Some(player) = self.players.get(&session_id) {
            let mut equipped = Vec::new();
            for slot in &player.equipment.slots {
                if let Some(user_item) = slot {
                    if let Some(info) = self.provider.get_item_info(user_item.item_index) {
                        equipped.push((info.clone(), user_item.clone()));
                    }
                }
            }

            let stats = aggregate_equipment_stats(&equipped);
            (player.job, stats)
        } else {
            return;
        };

        if let Some(player) = self.players.get_mut(&session_id) {
            player.stats.set_equip_stats(&equip_stats);
            player.stats.recalc_if_dirty_for_job(job);
        }
    }

    /// Query players on a given map within a rectangular view range around the
    /// provided centre, returning (session_id, x, y, direction) tuples. This
    /// mirrors the visibility logic currently used by the connection layer.
    pub fn players_in_view_for_map(
        &self,
        map_index: i32,
        centre_x: i32,
        centre_y: i32,
        range: i32,
        exclude_session: Option<SessionId>,
    ) -> Vec<(SessionId, i32, i32, u8)> {
        self.players
            .iter()
            .filter_map(|(&sid, p)| {
                if let Some(ex) = exclude_session {
                    if sid == ex {
                        return None;
                    }
                }

                if p.map_index != map_index {
                    return None;
                }

                if (p.x - centre_x).abs() > range || (p.y - centre_y).abs() > range {
                    return None;
                }

                Some((sid, p.x, p.y, p.direction))
            })
            .collect()
    }

    /// Query session IDs on a given map within a rectangular view range around
    /// the provided centre. This is used by the connection layer to decide
    /// which clients should receive broadcast events such as attacks.
    pub fn sessions_in_range_for_map(
        &self,
        map_index: i32,
        centre_x: i32,
        centre_y: i32,
        range: i32,
    ) -> Vec<SessionId> {
        self.players
            .iter()
            .filter_map(|(&sid, p)| {
                if p.map_index != map_index {
                    return None;
                }

                if (p.x - centre_x).abs() > range || (p.y - centre_y).abs() > range {
                    return None;
                }

                Some(sid)
            })
            .collect()
    }

    pub fn player_max_hp_mp(&self, session_id: SessionId) -> Option<(i32, i32)> {
        self.players.get(&session_id).map(|p| {
            let hp = p.stats.total.get(Stat::HP).max(0);
            let mp = p.stats.total.get(Stat::MP).max(0);
            (hp, mp)
        })
    }

    pub fn player_current_hp_mp(&self, session_id: SessionId) -> Option<(i32, i32)> {
        self.players.get(&session_id).map(|p| (p.hp, p.mp))
    }

    pub fn set_player_hp_mp(
        &mut self,
        session_id: SessionId,
        hp: i32,
        mp: i32,
    ) -> Option<()> {
        let player = self.players.get_mut(&session_id)?;

        let max_hp = player.stats.total.get(Stat::HP).max(1);
        let max_mp = player.stats.total.get(Stat::MP).max(0);

        let new_hp = hp.max(0).min(max_hp);
        let new_mp = mp.max(0).min(max_mp);

        player.hp = new_hp;
        player.mp = new_mp;
        player.dead = new_hp <= 0;
        Some(())
    }

    /// Revive a dead player at their current map/x/y/direction without
    /// changing position. Returns the new location and HP/MP.
    pub fn revive_player_in_place(
        &mut self,
        session_id: SessionId,
    ) -> Option<(i32, i32, i32, u8, i32, i32)> {
        let (map_index, x, y, direction) = {
            let player = self.players.get(&session_id)?;
            (player.map_index, player.x, player.y, player.direction)
        };

        self.revive_player_to_position(session_id, map_index, x, y, direction)
    }

    /// Revive a dead player and move them to a specific map and location,
    /// resetting HP/MP to their maximum values and clearing the dead flag.
    pub fn revive_player_to_position(
        &mut self,
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
    ) -> Option<(i32, i32, i32, u8, i32, i32)> {
        let player = self.players.get_mut(&session_id)?;

        if !player.dead && player.hp > 0 {
            return None;
        }

        player.map_index = map_index;
        player.x = x;
        player.y = y;
        player.direction = direction;

        let max_hp = player.stats.total.get(Stat::HP).max(1);
        let max_mp = player.stats.total.get(Stat::MP).max(0);

        player.hp = max_hp;
        player.mp = max_mp;
        player.dead = false;

        Some((
            player.map_index,
            player.x,
            player.y,
            player.direction,
            player.hp,
            player.mp,
        ))
    }

    pub fn accept_reincarnation_for_session(
        &mut self,
        session_id: SessionId,
    ) -> Option<(i32, i32, i32, u8, i32, i32)> {
        let now = self.time_ms;

        let host_session_id = {
            let player = self.players.get(&session_id)?;

            if !player.dead && player.hp > 0 {
                return None;
            }

            if player.reincarnation_expire_time_ms > 0
                && now > player.reincarnation_expire_time_ms
            {
                return None;
            }

            match player.reincarnation_host_session_id {
                Some(h) => h,
                None => return None,
            }
        };

        {
            let host = self.players.get(&host_session_id)?;

            if !host.reincarnation_ready {
                return None;
            }

            if host.reincarnation_target_session_id != Some(session_id) {
                return None;
            }

            if host.reincarnation_expire_time_ms > 0
                && now > host.reincarnation_expire_time_ms
            {
                return None;
            }
        }

        let (map_index, x, y, direction, hp, mp) = {
            let player = self.players.get_mut(&session_id)?;

            let max_hp = player.stats.total.get(Stat::HP).max(1);
            let new_hp = (max_hp / 2).max(1);
            let new_mp = player.mp;

            player.hp = new_hp;
            player.dead = false;
            player.reincarnation_host_session_id = None;
            player.reincarnation_expire_time_ms = 0;

            (
                player.map_index,
                player.x,
                player.y,
                player.direction,
                new_hp,
                new_mp,
            )
        };

        if let Some(host) = self.players.get_mut(&host_session_id) {
            if host.reincarnation_target_session_id == Some(session_id) {
                host.reincarnation_target_session_id = None;
            }
            host.reincarnation_ready = false;
            host.reincarnation_expire_time_ms = 0;
        }

        Some((map_index, x, y, direction, hp, mp))
    }

    pub fn cancel_reincarnation_for_session(
        &mut self,
        session_id: SessionId,
    ) -> Option<SessionId> {
        let (host_session_id, target_session_id) = {
            let player = self.players.get(&session_id)?;

            if let Some(host) = player.reincarnation_host_session_id {
                (host, session_id)
            } else if let Some(target) = player.reincarnation_target_session_id {
                (session_id, target)
            } else {
                return None;
            }
        };

        if let Some(host) = self.players.get_mut(&host_session_id) {
            if host.reincarnation_target_session_id == Some(target_session_id) {
                host.reincarnation_target_session_id = None;
            }
            host.reincarnation_ready = false;
            host.reincarnation_expire_time_ms = 0;
        }

        if let Some(target) = self.players.get_mut(&target_session_id) {
            if target.reincarnation_host_session_id == Some(host_session_id) {
                target.reincarnation_host_session_id = None;
            }
            target.reincarnation_expire_time_ms = 0;
        }

        Some(host_session_id)
    }

    /// Mark an in-progress Taoist Reincarnation attempt as expired from the
    /// perspective of the given session without immediately clearing all
    /// state. This mirrors the legacy client CancelReincarnation behaviour
    /// where only ReincarnationExpireTime is moved forward; the actual
    /// cleanup and S.CancelReincarnation emission happens when the world
    /// processes timers.
    pub fn expire_reincarnation_from_session(&mut self, session_id: SessionId) {
        let mut host_session_id: Option<SessionId> = None;

        if let Some(player) = self.players.get(&session_id) {
            if let Some(host) = player.reincarnation_host_session_id {
                host_session_id = Some(host);
            } else if player.reincarnation_target_session_id.is_some() {
                host_session_id = Some(session_id);
            }
        }

        if let Some(host_sid) = host_session_id {
            if let Some(host) = self.players.get_mut(&host_sid) {
                if host.reincarnation_ready && host.reincarnation_target_session_id.is_some() {
                    host.reincarnation_expire_time_ms = self.time_ms;
                }
            }
        }
    }

    pub fn player_items(&self, session_id: SessionId) -> Option<(Inventory, Equipment)> {
        self.players
            .get(&session_id)
            .map(|p| (p.inventory.clone(), p.equipment.clone()))
    }

    pub fn player_level(&self, session_id: SessionId) -> Option<u16> {
        self.players.get(&session_id).map(|p| p.level)
    }

    pub fn player_pk_points(&self, session_id: SessionId) -> Option<i32> {
        self.players.get(&session_id).map(|p| p.pk_points)
    }

    pub fn set_player_items(
        &mut self,
        session_id: SessionId,
        inventory: Inventory,
        equipment: Equipment,
    ) {
        if let Some(player) = self.players.get_mut(&session_id) {
            player.inventory = inventory;
            player.equipment = equipment;
            self.recalc_player_equipment_stats(session_id);
        }
    }

    /// Get player's refine slots
    pub fn player_refine_slots(&self, session_id: SessionId) -> Option<Vec<Option<UserItemData>>> {
        self.players
            .get(&session_id)
            .map(|p| p.refine_slots.clone())
    }

    /// Set player's refine slots
    pub fn set_player_refine_slots(
        &mut self,
        session_id: SessionId,
        refine_slots: Vec<Option<UserItemData>>,
    ) {
        if let Some(player) = self.players.get_mut(&session_id) {
            player.refine_slots = refine_slots;
        }
    }

    /// Get player's current refine item
    pub fn player_current_refine(&self, session_id: SessionId) -> Option<UserItemData> {
        self.players
            .get(&session_id)
            .and_then(|p| p.current_refine.clone())
    }

    /// Set player's current refine item
    pub fn set_player_current_refine(
        &mut self,
        session_id: SessionId,
        current_refine: Option<UserItemData>,
    ) {
        if let Some(player) = self.players.get_mut(&session_id) {
            player.current_refine = current_refine;
        }
    }

    /// Get player's refine time remaining
    pub fn player_refine_time_remaining(&self, session_id: SessionId) -> Option<i64> {
        self.players
            .get(&session_id)
            .map(|p| p.refine_time_remaining_ms)
    }

    /// Set player's refine time remaining
    pub fn set_player_refine_time_remaining(
        &mut self,
        session_id: SessionId,
        refine_time_remaining_ms: i64,
    ) {
        if let Some(player) = self.players.get_mut(&session_id) {
            player.refine_time_remaining_ms = refine_time_remaining_ms;
        }
    }

    pub fn deposit_trade_item_for_player(
        &mut self,
        session_id: SessionId,
        from: i32,
        to: i32,
    ) -> bool {
        let player = match self.players.get_mut(&session_id) {
            Some(p) => p,
            None => return false,
        };

        if from < 0 || to < 0 {
            return false;
        }
        let from = from as usize;
        let to = to as usize;

        if from >= player.inventory.len() || to >= player.trade.len() {
            return false;
        }

        let item = match player.inventory.slots[from].as_ref() {
            Some(it) => it.clone(),
            None => return false,
        };

        let info = match self.provider.get_item_info(item.item_index) {
            Some(i) => i,
            None => return false,
        };

        const BIND_DONT_TRADE: i16 = 0x0010;

        if (info.bind & BIND_DONT_TRADE) != 0 {
            return false;
        }

        if item
            .rental_information
            .as_ref()
            .map_or(false, |r| (r.binding_flags & BIND_DONT_TRADE) != 0)
        {
            return false;
        }

        if player.trade[to].is_some() {
            return false;
        }

        player.trade[to] = Some(item);
        player.inventory.slots[from] = None;
        true
    }

    pub fn retrieve_trade_item_for_player(
        &mut self,
        session_id: SessionId,
        from: i32,
        to: i32,
    ) -> bool {
        let player = match self.players.get_mut(&session_id) {
            Some(p) => p,
            None => return false,
        };

        if from < 0 || to < 0 {
            return false;
        }
        let from = from as usize;
        let to = to as usize;

        if from >= player.trade.len() || to >= player.inventory.len() {
            return false;
        }

        let item = match player.trade[from].as_ref() {
            Some(it) => it.clone(),
            None => return false,
        };

        if player.inventory.slots[to].is_some() {
            return false;
        }

        player.inventory.slots[to] = Some(item);
        player.trade[from] = None;
        true
    }

    pub fn player_trade_items(&self, session_id: SessionId) -> Vec<Option<UserItemData>> {
        self.players
            .get(&session_id)
            .map(|p| p.trade.clone())
            .unwrap_or_else(|| vec![None; 10])
    }

    pub fn move_item_in_grid(
        &mut self,
        session_id: SessionId,
        grid: u8,
        from: i32,
        to: i32,
    ) -> bool {
        let player = match self.players.get_mut(&session_id) {
            Some(p) => p,
            None => return false,
        };

        if grid != 1 {
            return false;
        }

        if from < 0 || to < 0 {
            return false;
        }
        let from = from as usize;
        let to = to as usize;

        if from >= player.inventory.len() || to >= player.inventory.len() {
            return false;
        }

        if player.inventory.slots[from].is_none() {
            return false;
        }

        player.inventory.slots.swap(from, to);
        true
    }

    pub fn equip_item_for_player(
        &mut self,
        session_id: SessionId,
        grid: u8,
        unique_id: u64,
        to: i32,
    ) -> bool {
        if grid != 1 {
            return false;
        }

        if to < 0 {
            return false;
        }
        let slot = to as usize;

        if !self.can_equip_item_for_player_by_uid(session_id, unique_id, slot) {
            return false;
        }

        let player = match self.players.get_mut(&session_id) {
            Some(p) => p,
            None => return false,
        };

        if slot >= player.equipment.len() {
            return false;
        }

        let from_index = match player
            .inventory
            .slots
            .iter()
            .position(|s| s.as_ref().map(|i| i.unique_id) == Some(unique_id))
        {
            Some(idx) => idx,
            None => return false,
        };

        let from_item = player.inventory.slots[from_index].take();
        let to_item = player.equipment.slots[slot].take();
        player.equipment.slots[slot] = from_item;
        player.inventory.slots[from_index] = to_item;

        if let Some(equipped) = player.equipment.slots[slot].as_mut() {
            if let Some(info) = self.provider.get_item_info(equipped.item_index) {
                if info.need_identify && !equipped.identified {
                    equipped.identified = true;
                }

                // BindOnEquip: if the item is not yet soul-bound (<= 0), bind it to this character.
                if (info.bind & 0x0200i16) != 0 && equipped.soul_bound_id <= 0 {
                    equipped.soul_bound_id = player.character_index;
                }
            }
        }

        self.recalc_player_equipment_stats(session_id);
        true
    }

    pub fn remove_item_for_player(
        &mut self,
        session_id: SessionId,
        grid: u8,
        unique_id: u64,
        to: i32,
    ) -> bool {
        let player = match self.players.get_mut(&session_id) {
            Some(p) => p,
            None => return false,
        };

        if grid != 1 {
            return false;
        }

        if to < 0 {
            return false;
        }
        let to = to as usize;
        if to >= player.inventory.len() {
            return false;
        }

        if player.inventory.slots[to].is_some() {
            return false;
        }

        let from_index = match player
            .equipment
            .slots
            .iter()
            .position(|s| s.as_ref().map(|i| i.unique_id) == Some(unique_id))
        {
            Some(idx) => idx,
            None => return false,
        };

        let from_item = player.equipment.slots[from_index].take();
        if from_item.is_none() {
            return false;
        }

        player.inventory.slots[to] = from_item;

        self.recalc_player_equipment_stats(session_id);
        true
    }

    pub fn merge_item_for_player(
        &mut self,
        session_id: SessionId,
        grid_from: u8,
        grid_to: u8,
        id_from: u64,
        id_to: u64,
    ) -> bool {
        // For now, only support Inventory -> Inventory merge
        // Note: Storage is handled in connection layer, so we only handle Inventory here
        if grid_from != 1 || grid_to != 1 {
            return false;
        }

        let player = match self.players.get_mut(&session_id) {
            Some(p) => p,
            None => return false,
        };

        // Find the source item
        let from_index = match player
            .inventory
            .slots
            .iter()
            .position(|s| s.as_ref().map(|i| i.unique_id) == Some(id_from))
        {
            Some(idx) => idx,
            None => return false,
        };

        let from_item = match player.inventory.slots[from_index].as_ref() {
            Some(it) => it.clone(),
            None => return false,
        };

        // Find the target item
        let to_index = match player
            .inventory
            .slots
            .iter()
            .position(|s| s.as_ref().map(|i| i.unique_id) == Some(id_to))
        {
            Some(idx) => idx,
            None => return false,
        };

        let to_item = match player.inventory.slots[to_index].as_ref() {
            Some(it) => it.clone(),
            None => return false,
        };

        // Check if items can be merged (same item type, stackable)
        if from_item.item_index != to_item.item_index {
            return false;
        }

        let info = match self.provider.get_item_info(from_item.item_index) {
            Some(i) => i,
            None => return false,
        };

        // Check stack size
        if info.stack_size <= 1 {
            return false;
        }

        // Check if target is already at max stack
        if to_item.count >= info.stack_size as u16 {
            return false;
        }

        // Calculate how much can be merged
        let available_space = (info.stack_size as u16) - to_item.count;
        let merge_amount = from_item.count.min(available_space);

        // Update target item count
        if let Some(ref mut target) = player.inventory.slots[to_index] {
            target.count += merge_amount;
        }

        // Update or remove source item
        if from_item.count <= merge_amount {
            // Source item is completely merged
            player.inventory.slots[from_index] = None;
        } else {
            // Source item still has remaining count
            if let Some(ref mut source) = player.inventory.slots[from_index] {
                source.count -= merge_amount;
            }
        }

        true
    }

    pub fn split_item_for_player(
        &mut self,
        session_id: SessionId,
        grid: u8,
        unique_id: u64,
        count: u16,
    ) -> bool {
        // For now, only support Inventory split
        // Note: Storage is handled in connection layer, so we only handle Inventory here
        if grid != 1 {
            return false;
        }

        if count == 0 {
            return false;
        }

        let player = match self.players.get_mut(&session_id) {
            Some(p) => p,
            None => return false,
        };

        // Find the item to split
        let from_index = match player
            .inventory
            .slots
            .iter()
            .position(|s| s.as_ref().map(|i| i.unique_id) == Some(unique_id))
        {
            Some(idx) => idx,
            None => return false,
        };

        let from_item = match player.inventory.slots[from_index].as_ref() {
            Some(it) => it.clone(),
            None => return false,
        };

        // Check if split count is valid
        if count >= from_item.count {
            return false;
        }

        // Find an empty slot for the split item
        let to_index = match player
            .inventory
            .slots
            .iter()
            .position(|s| s.is_none())
        {
            Some(idx) => idx,
            None => return false, // No free space
        };

        // Create the split item with a new unique ID
        // Generate unique ID similar to other item creation: use session_id and timestamp/counter
        let mut split_item = from_item.clone();
        split_item.count = count;
        // Generate a new unique ID by combining session_id with a counter
        // Use a simple approach: session_id in upper bits, timestamp-based counter in lower bits
        let time_based = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        split_item.unique_id = ((session_id as u64) << 32) | (time_based & 0xFFFF_FFFF);

        // Update source item count
        if let Some(ref mut source) = player.inventory.slots[from_index] {
            source.count -= count;
        }

        // Place split item in empty slot
        player.inventory.slots[to_index] = Some(split_item);

        true
    }

    pub fn remove_slot_item_for_player(
        &mut self,
        session_id: SessionId,
        grid: u8,
        grid_to: u8,
        unique_id: u64,
        to: i32,
        from_unique_id: u64,
    ) -> bool {
        // Support Inventory (1) and Storage (2) grid_to
        // Storage is handled in connection layer, so we only handle Inventory here
        if grid_to != 1 {
            return false;
        }

        let player = match self.players.get_mut(&session_id) {
            Some(p) => p,
            None => return false,
        };

        if to < 0 {
            return false;
        }
        let to_index = to as usize;
        if to_index >= player.inventory.len() {
            return false;
        }

        if player.inventory.slots[to_index].is_some() {
            return false; // Target slot must be empty
        }

        // Find the parent item that contains the slot item
        // Grid types: Mount=13, Fishing=7, Socket=8
        let parent_item: Option<&mut UserItemData> = match grid {
            13 => {
                // Mount: get from equipment slot 13
                player.equipment.slots.get_mut(13).and_then(|s| s.as_mut())
            }
            7 => {
                // Fishing: get from equipment slot 7 (Weapon)
                player.equipment.slots.get_mut(7).and_then(|s| s.as_mut())
            }
            8 => {
                // Socket: find by from_unique_id in equipment or inventory
                player
                    .equipment
                    .slots
                    .iter_mut()
                    .find(|s| s.as_ref().map(|i| i.unique_id) == Some(from_unique_id))
                    .and_then(|s| s.as_mut())
                    .or_else(|| {
                        player
                            .inventory
                            .slots
                            .iter_mut()
                            .find(|s| s.as_ref().map(|i| i.unique_id) == Some(from_unique_id))
                            .and_then(|s| s.as_mut())
                    })
            }
            _ => return false,
        };

        let parent_item = match parent_item {
            Some(p) => p,
            None => return false,
        };

        // Validate parent item has slots
        if parent_item.slots.is_empty() {
            return false;
        }

        // Find the slot item to remove
        let slot_index = match parent_item
            .slots
            .iter()
            .position(|s| s.as_ref().and_then(|b| Some(b.unique_id)) == Some(unique_id))
        {
            Some(idx) => idx,
            None => return false,
        };

        let slot_item = match parent_item.slots[slot_index].take() {
            Some(boxed_item) => *boxed_item,
            None => return false,
        };

        // Check if slot item is cursed (cannot remove)
        if slot_item.cursed {
            // Put it back
            parent_item.slots[slot_index] = Some(Box::new(slot_item));
            return false;
        }

        // Check if slot item has wedding ring (cannot remove)
        if slot_item.wedding_ring != -1 {
            // Put it back
            parent_item.slots[slot_index] = Some(Box::new(slot_item));
            return false;
        }

        // Move slot item to inventory
        player.inventory.slots[to_index] = Some(slot_item);

        // Recalculate stats if equipment changed
        if grid == 13 || grid == 7 {
            self.recalc_player_equipment_stats(session_id);
        }

        true
    }

    pub fn equip_slot_item_for_player(
        &mut self,
        session_id: SessionId,
        grid: u8,
        grid_to: u8,
        unique_id: u64,
        to: i32,
        to_unique_id: u64,
    ) -> bool {
        let player = match self.players.get_mut(&session_id) {
            Some(p) => p,
            None => return false,
        };

        // Find source item index first (before any mutable borrows)
        let source_item_index = match grid {
            1 => {
                // Inventory
                player
                    .inventory
                    .slots
                    .iter()
                    .position(|s| s.as_ref().map(|i| i.unique_id) == Some(unique_id))
            }
            2 => {
                // Storage - handled in connection layer (requires store access)
                return false;
            }
            _ => return false,
        };

        let source_item_index = match source_item_index {
            Some(idx) => idx,
            None => return false,
        };

        // Get source item for validation (immutable borrow)
        let (source_item_index_to_use, source_item_idx, source_soul_bound, source_cursed, source_wedding_ring) = {
            let source_item = match player.inventory.slots[source_item_index].as_ref() {
                Some(item) => item,
                None => return false,
            };
            (source_item_index, source_item.item_index, source_item.soul_bound_id, source_item.cursed, source_item.wedding_ring)
        };

        // Get source item info
        let source_info = match self.provider.get_item_info(source_item_idx) {
            Some(info) => info,
            None => return false,
        };

        // Find target item based on grid_to
        // Grid types: Mount=13, Fishing=7, Socket=8
        let (target_item_idx, target_item_slot_idx, target_equipment_slot) = match grid_to {
            13 => {
                // Mount: get from equipment slot 13
                match player.equipment.slots.get(13).and_then(|s| s.as_ref()) {
                    Some(item) => (item.item_index, 13, true),
                    None => return false,
                }
            }
            7 => {
                // Fishing: get from equipment slot 7 (Weapon)
                match player.equipment.slots.get(7).and_then(|s| s.as_ref()) {
                    Some(item) => (item.item_index, 7, true),
                    None => return false,
                }
            }
            8 => {
                // Socket: find by to_unique_id in equipment or inventory
                let mut found_idx = None;
                let mut found_slot = None;
                let mut found_in_equipment = false;

                // Search in equipment first
                for (idx, slot) in player.equipment.slots.iter().enumerate() {
                    if let Some(item) = slot.as_ref() {
                        if item.unique_id == to_unique_id {
                            found_idx = Some(item.item_index);
                            found_slot = Some(idx);
                            found_in_equipment = true;
                            break;
                        }
                    }
                }

                // If not found in equipment, search in inventory
                if found_idx.is_none() {
                    for (idx, slot) in player.inventory.slots.iter().enumerate() {
                        if let Some(item) = slot.as_ref() {
                            if item.unique_id == to_unique_id {
                                found_idx = Some(item.item_index);
                                found_slot = Some(idx);
                                found_in_equipment = false;
                                break;
                            }
                        }
                    }
                }

                match (found_idx, found_slot) {
                    (Some(idx), Some(slot)) => (idx, slot, found_in_equipment),
                    _ => return false,
                }
            }
            _ => return false,
        };

        // Get target item info
        let target_info = match self.provider.get_item_info(target_item_idx) {
            Some(info) => info,
            None => return false,
        };

        // Validate based on grid_to type
        match grid_to {
            7 => {
                // Fishing: target must be a fishing rod
                // Note: ItemInfoData doesn't have IsFishingRod field yet, so we check it's a weapon
                // In C#, this checks: item.Info.Type == ItemType.Weapon && item.Info.IsFishingRod
                if target_info.item_type != 0 {
                    // ItemType.Weapon = 0
                    return false;
                }
                // TODO: Add IsFishingRod field to ItemInfoData and check it here
            }
            8 => {
                // Socket: source must be ItemType.Socket
                if source_info.item_type != 12 {
                    // ItemType.Socket = 12
                    return false;
                }
            }
            _ => {}
        }

        // Check soul bound
        if source_soul_bound != -1 && source_soul_bound != player.character_index {
            return false;
        }

        // Check if source item is cursed (cannot equip to slot if cursed, unless UnlockCurse is active)
        // Note: UnlockCurse is not yet implemented in PlayerState, so we just check cursed flag
        if source_cursed {
            // TODO: Check UnlockCurse flag when implemented
            return false;
        }

        // Check if source item is a wedding ring (cannot equip to slot)
        if source_wedding_ring != -1 {
            return false;
        }

        // Validate source item shape for socket type
        if grid_to == 8 {
            // Socket: validate shape restrictions
            match source_info.shape {
                1 => {
                    // Only for weapons
                    if target_info.item_type != 0 {
                        return false;
                    }
                }
                2 => {
                    // Only for armour
                    if target_info.item_type != 1 {
                        return false;
                    }
                }
                3 => {
                    // Only for rings/bracelets/necklaces
                    if target_info.item_type != 2
                        && target_info.item_type != 3
                        && target_info.item_type != 4
                    {
                        return false;
                    }
                }
                _ => {}
            }
        }

        // Validate slot index before mutable borrows
        let slot_index = if to < 0 {
            return false;
        } else {
            to as usize
        };

        // Validate target item has slots and slot is empty
        // We need to check this before mutable borrows
        let target_has_slots_and_empty = match grid_to {
            13 => {
                player.equipment.slots.get(13)
                    .and_then(|s| s.as_ref())
                    .map(|item| !item.slots.is_empty() && slot_index < item.slots.len() && item.slots[slot_index].is_none())
                    .unwrap_or(false)
            }
            7 => {
                player.equipment.slots.get(7)
                    .and_then(|s| s.as_ref())
                    .map(|item| !item.slots.is_empty() && slot_index < item.slots.len() && item.slots[slot_index].is_none())
                    .unwrap_or(false)
            }
            8 => {
                if target_equipment_slot {
                    player.equipment.slots.get(target_item_slot_idx)
                        .and_then(|s| s.as_ref())
                        .map(|item| !item.slots.is_empty() && slot_index < item.slots.len() && item.slots[slot_index].is_none())
                        .unwrap_or(false)
                } else {
                    player.inventory.slots.get(target_item_slot_idx)
                        .and_then(|s| s.as_ref())
                        .map(|item| !item.slots.is_empty() && slot_index < item.slots.len() && item.slots[slot_index].is_none())
                        .unwrap_or(false)
                }
            }
            _ => false,
        };

        if !target_has_slots_and_empty {
            return false;
        }

        // Now perform the move - handle different cases to avoid borrow conflicts
        let source_item = player.inventory.slots[source_item_index_to_use].take();
        if let Some(item) = source_item {
            // Place item in target slot based on grid_to
            match grid_to {
                13 => {
                    if let Some(target_item) = player.equipment.slots.get_mut(13).and_then(|s| s.as_mut()) {
                        target_item.slots[slot_index] = Some(Box::new(item));
                    } else {
                        // Put source item back if target is invalid
                        player.inventory.slots[source_item_index_to_use] = Some(item);
                        return false;
                    }
                }
                7 => {
                    if let Some(target_item) = player.equipment.slots.get_mut(7).and_then(|s| s.as_mut()) {
                        target_item.slots[slot_index] = Some(Box::new(item));
                    } else {
                        // Put source item back if target is invalid
                        player.inventory.slots[source_item_index_to_use] = Some(item);
                        return false;
                    }
                }
                8 => {
                    if target_equipment_slot {
                        if let Some(target_item) = player.equipment.slots.get_mut(target_item_slot_idx).and_then(|s| s.as_mut()) {
                            target_item.slots[slot_index] = Some(Box::new(item));
                        } else {
                            // Put source item back if target is invalid
                            player.inventory.slots[source_item_index_to_use] = Some(item);
                            return false;
                        }
                    } else {
                        // Target is in inventory - need to handle carefully to avoid borrow conflicts
                        // Since we already took source_item, we can safely borrow target
                        if let Some(target_item) = player.inventory.slots.get_mut(target_item_slot_idx).and_then(|s| s.as_mut()) {
                            target_item.slots[slot_index] = Some(Box::new(item));
                        } else {
                            // Put source item back if target is invalid
                            player.inventory.slots[source_item_index_to_use] = Some(item);
                            return false;
                        }
                    }
                }
                _ => {
                    // Put source item back
                    player.inventory.slots[source_item_index_to_use] = Some(item);
                    return false;
                }
            }
            self.recalc_player_equipment_stats(session_id);
            true
        } else {
            false
        }
    }

    pub fn combine_item_for_player(
        &mut self,
        session_id: SessionId,
        grid: u8,
        id_from: u64,
        id_to: u64,
    ) -> (bool, bool) {
        // Returns (success, destroy)
        let player = match self.players.get_mut(&session_id) {
            Some(p) => p,
            None => return (false, false),
        };

        // Check if player is dead
        if player.dead {
            return (false, false);
        }

        // Find items in inventory (for now, only support Inventory grid)
        let (from_index, to_index) = match grid {
            1 => {
                // Inventory
                let from_idx = player
                    .inventory
                    .slots
                    .iter()
                    .position(|s| s.as_ref().map(|i| i.unique_id) == Some(id_from));
                let to_idx = player
                    .inventory
                    .slots
                    .iter()
                    .position(|s| s.as_ref().map(|i| i.unique_id) == Some(id_to));

                match (from_idx, to_idx) {
                    (Some(f), Some(t)) => (f, t),
                    _ => return (false, false),
                }
            }
            _ => return (false, false), // TODO: Support HeroInventory
        };

        // Get items
        let from_item = match player.inventory.slots[from_index].as_ref() {
            Some(item) => item.clone(),
            None => return (false, false),
        };

        let to_item = match player.inventory.slots[to_index].as_ref() {
            Some(item) => item.clone(),
            None => return (false, false),
        };

        // Get item infos
        let from_info = match self.provider.get_item_info(from_item.item_index) {
            Some(info) => info,
            None => return (false, false),
        };

        let to_info = match self.provider.get_item_info(to_item.item_index) {
            Some(info) => info,
            None => return (false, false),
        };

        // Check if source item is cursed (cannot combine if cursed, unless UnlockCurse is active)
        // Note: UnlockCurse is not yet implemented in PlayerState
        if from_item.cursed {
            // TODO: Check UnlockCurse flag when implemented
            return (false, false);
        }

        // Check if source item is a wedding ring (cannot combine)
        if from_item.wedding_ring != -1 {
            return (false, false);
        }

        // Source must be a gem (ItemType.Gem = 12)
        if from_info.item_type != 12 {
            return (false, false);
        }

        // Target must be a valid equipment type (1-11)
        if to_info.item_type < 1 || to_info.item_type > 11 {
            return (false, false);
        }

        // Check if target item is cursed (cannot combine if cursed, unless UnlockCurse is active)
        // Note: UnlockCurse is not yet implemented in PlayerState
        if to_item.cursed {
            // TODO: Check UnlockCurse flag when implemented
            return (false, false);
        }

        // Check if target item is a wedding ring (cannot combine)
        if to_item.wedding_ring != -1 {
            return (false, false);
        }

        // Handle different gem shapes
        match from_info.shape {
            1 | 2 | 5 | 6 => {
                // Repair tools (BoneHammer, SewingSupplies, SpecialHammer, SpecialSewingSupplies)
                // Check if target can be repaired
                const BIND_DONT_REPAIR: i16 = 0x0010;
                if (to_info.bind & BIND_DONT_REPAIR) != 0 {
                    return (false, false);
                }

                // Check if repair tool matches item type
                // ItemType: Weapon=0, Armour=1, Helmet=5, Boots=6, Belt=7, Necklace=2, Ring=3, Bracelet=4
                let can_repair = match to_info.item_type {
                    0 | 2 | 3 | 4 => {
                        // Weapon, Necklace, Ring, Bracelet - use hammer (shape 1 or 5)
                        from_info.shape == 1 || from_info.shape == 5
                    }
                    1 | 5 | 6 | 7 => {
                        // Armour, Helmet, Boots, Belt - use sewing supplies (shape 2 or 6)
                        from_info.shape == 2 || from_info.shape == 6
                    }
                    _ => false,
                };

                if !can_repair {
                    return (false, false);
                }

                // Check if item needs repair
                if to_item.current_dura >= to_item.max_dura {
                    return (false, false);
                }

                // Perform repair - restore durability
                // For now, restore to full durability (can be made more complex later)
                if let Some(target_item) = player.inventory.slots[to_index].as_mut() {
                    target_item.current_dura = target_item.max_dura;
                    
                    // Remove source item
                    player.inventory.slots[from_index] = None;
                    
                    return (true, false);
                }
            }
            3 | 4 => {
                // Gems/Orbs - upgrade stats
                // Check if target can be upgraded
                const BIND_DONT_UPGRADE: i16 = 0x0020;
                if (to_info.bind & BIND_DONT_UPGRADE) != 0 {
                    return (false, false);
                }

                // Check gem count limits
                // from_info.stats[Stat.CriticalDamage] is max gem count
                // from_info.stats[Stat.HPDrainRatePercent] is max stat count
                let max_gem_count = from_info
                    .stats
                    .entries
                    .iter()
                    .find(|(stat_id, _)| *stat_id == 20) // Stat.CriticalDamage = 20
                    .map(|(_, val)| *val as u16)
                    .unwrap_or(255);

                if to_item.gem_count >= max_gem_count {
                    return (false, false);
                }

                // Calculate success chance
                // Base success chance is from_info.stats[Stat.Reflect]
                let base_success = from_info
                    .stats
                    .entries
                    .iter()
                    .find(|(stat_id, _)| *stat_id == 19) // Stat.Reflect = 19
                    .map(|(_, val)| *val)
                    .unwrap_or(100);

                // Adjust success chance based on gem count (simplified version)
                // In C#, this is more complex with GemStatIndependent setting
                let mut success_chance = base_success;
                success_chance = success_chance.saturating_sub((to_item.gem_count as i32) * 10);
                success_chance = success_chance.max(0).min(100);

                // Roll for success
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let roll = rng.gen_range(0..100);
                let succeeded = roll < success_chance;

                // Apply stat upgrades if succeeded
                let should_recalc = if succeeded {
                    if let Some(target_item) = player.inventory.slots[to_index].as_mut() {
                        // Increment gem count
                        target_item.gem_count += 1;

                        // Apply stat upgrades from gem
                        // Find the first non-zero stat in the gem and apply it
                        for (stat_id, stat_value) in &from_info.stats.entries {
                            if *stat_value > 0 {
                                // Find or add this stat to target item's added_stats
                                let existing_idx = target_item
                                    .added_stats
                                    .entries
                                    .iter()
                                    .position(|(id, _)| id == stat_id);

                                if let Some(idx) = existing_idx {
                                    target_item.added_stats.entries[idx].1 += stat_value;
                                } else {
                                    target_item.added_stats.entries.push((*stat_id, *stat_value));
                                }
                                break; // Only apply first stat (matching C# logic)
                            }
                        }
                        true // Need to recalc stats
                    } else {
                        false
                    }
                } else {
                    // Failure handling
                    if from_info.shape == 3 {
                        // Gem (shape 3) has 20% chance to destroy item on failure
                        let destroy_roll = rng.gen_range(0..15);
                        if destroy_roll < 3 {
                            // Destroy target item
                            player.inventory.slots[to_index] = None;
                            // Remove source item
                            player.inventory.slots[from_index] = None;
                            return (false, true);
                        }
                    }
                    false // No need to recalc on failure
                };

                // Remove source item
                player.inventory.slots[from_index] = None;

                // Recalculate player stats if needed (after releasing player borrow)
                if should_recalc {
                    drop(player); // Explicitly drop the borrow
                    self.recalc_player_equipment_stats(session_id);
                }
                
                return (succeeded, false);
            }
            7 => {
                // Slot upgrade - add socket
                const BIND_DONT_UPGRADE: i16 = 0x0020;
                if (to_info.bind & BIND_DONT_UPGRADE) != 0 {
                    return (false, false);
                }

                // Check rental information
                if let Some(ref rental) = to_item.rental_information {
                    if (rental.binding_flags & BIND_DONT_UPGRADE) != 0 {
                        return (false, false);
                    }
                }

                // Validate gem type matches item type (ValidGemForItem)
                // Check if gem's unique flag matches item type
                // Note: C# uses 1-based item types in ValidGemForItem, but Rust uses 0-based
                // C#: 1=Weapon, 2=Armour, 4=Helmet, 5=Necklace, 6=Bracelet, 7=Ring, 8=Amulet, 9=Belt, 10=Boots, 11=Stone, 12=Torch
                // Rust: 0=Weapon, 1=Armour, 2=Necklace, 3=Ring, 4=Bracelet, 5=Helmet, 6=Boots, 7=Belt, etc.
                let gem_valid = match to_info.item_type {
                    0 => {
                        // Weapon - gem must have Paralize flag (C# case 1)
                        (from_info.unique & 0x0001) != 0 // SpecialItemMode.Paralize = 0x0001
                    }
                    1 => {
                        // Armour - gem must have Teleport flag (C# case 2)
                        (from_info.unique & 0x0002) != 0 // SpecialItemMode.Teleport = 0x0002
                    }
                    2 => {
                        // Necklace - gem must have Protection flag (C# case 5)
                        (from_info.unique & 0x0008) != 0 // SpecialItemMode.Protection = 0x0008
                    }
                    3 => {
                        // Ring - gem must have Muscle flag (C# case 7)
                        (from_info.unique & 0x0020) != 0 // SpecialItemMode.Muscle = 0x0020
                    }
                    4 => {
                        // Bracelet - gem must have Revival flag (C# case 6)
                        (from_info.unique & 0x0010) != 0 // SpecialItemMode.Revival = 0x0010
                    }
                    5 => {
                        // Helmet - gem must have ClearRing flag (C# case 4)
                        (from_info.unique & 0x0004) != 0 // SpecialItemMode.ClearRing = 0x0004
                    }
                    6 => {
                        // Boots - gem must have Probe flag (C# case 10)
                        (from_info.unique & 0x0100) != 0 // SpecialItemMode.Probe = 0x0100
                    }
                    7 => {
                        // Belt - gem must have Healing flag (C# case 9)
                        (from_info.unique & 0x0080) != 0 // SpecialItemMode.Healing = 0x0080
                    }
                    8 => {
                        // Amulet - gem must have Flame flag (C# case 8)
                        (from_info.unique & 0x0040) != 0 // SpecialItemMode.Flame = 0x0040
                    }
                    9 => {
                        // Stone - gem must have Skill flag (C# case 11)
                        (from_info.unique & 0x0200) != 0 // SpecialItemMode.Skill = 0x0200
                    }
                    10 => {
                        // Torch - gem must have NoDuraLoss flag (C# case 12)
                        (from_info.unique & 0x0400) != 0 // SpecialItemMode.NoDuraLoss = 0x0400
                    }
                    _ => false,
                };

                if !gem_valid {
                    return (false, false);
                }

                // Check if item can have more slots
                // If random_stats_id is 0, item cannot have slots
                if to_info.random_stats_id == 0 {
                    return (false, false);
                }

                // Get SlotMaxStat from RandomItemStats config
                use crate::world::configs::random_item_stat_for_id;
                let slot_max = match random_item_stat_for_id(to_info.random_stats_id) {
                    Some(stat) => stat.slot_max_stat,
                    None => return (false, false),
                };

                // Check if item already has max slots
                if to_item.slots.len() >= slot_max as usize {
                    return (false, false);
                }

                // Add slot to item
                if let Some(target_item) = player.inventory.slots[to_index].as_mut() {
                    target_item.slots.push(None);
                }

                // Remove source item
                player.inventory.slots[from_index] = None;
                
                return (true, false);
            }
            8 => {
                // Seal item
                const BIND_DONT_UPGRADE: i16 = 0x0020;
                if (to_info.bind & BIND_DONT_UPGRADE) != 0 {
                    return (false, false);
                }

                // Check if item is already sealed and not expired
                if let Some(ref sealed) = to_item.sealed_info {
                    // Check if seal is still active (ExpiryDate > now)
                    if sealed.expiry_binary > self.time_ms {
                        return (false, false);
                    }
                    // If expired, allow re-sealing (but check NextSealDate)
                    if sealed.next_seal_binary > self.time_ms {
                        // Cannot seal yet - must wait until NextSealDate
                        return (false, false);
                    }
                }

                // Get seal duration from gem's current_dura (in minutes)
                let seal_minutes = from_item.current_dura as i64;
                
                // Calculate expiry date (current time + seal_minutes)
                let now_ms = self.time_ms;
                let expiry_ms = now_ms + (seal_minutes * 60 * 1000);
                
                // NextSealDate = expiry + ItemSealDelay (default 24 hours = 86400000 ms)
                let seal_delay_ms: i64 = 24 * 60 * 60 * 1000; // 24 hours in milliseconds
                let next_seal_ms = expiry_ms + seal_delay_ms;

                // Apply seal
                if let Some(target_item) = player.inventory.slots[to_index].as_mut() {
                    use crystal_shared_proto::item_types::SealedInfoData;
                    target_item.sealed_info = Some(SealedInfoData {
                        expiry_binary: expiry_ms,
                        next_seal_binary: next_seal_ms,
                    });
                }

                // Remove source item
                player.inventory.slots[from_index] = None;
                
                return (true, false);
            }
            _ => {
                return (false, false);
            }
        }

        (false, false)
    }

    fn can_equip_item_for_player_by_uid(
        &self,
        session_id: SessionId,
        unique_id: u64,
        slot: usize,
    ) -> bool {
        let player = match self.players.get(&session_id) {
            Some(p) => p,
            None => return false,
        };

        let item = match player
            .inventory
            .slots
            .iter()
            .find(|s| s.as_ref().map(|i| i.unique_id) == Some(unique_id))
        {
            Some(Some(i)) => i,
            _ => return false,
        };

        // SoulBound check: if the item is already bound to a different character,
        // it cannot be equipped by this player. Treat non-positive IDs as unbound
        // for compatibility with existing data.
        if item.soul_bound_id > 0 && item.soul_bound_id != player.character_index {
            return false;
        }

        self.can_equip_item_for_player(player, item, slot)
    }

    pub fn can_use_item_for_player(
        &self,
        session_id: SessionId,
        item: &UserItemData,
    ) -> bool {
        let player = match self.players.get(&session_id) {
            Some(p) => p,
            None => return false,
        };

        if player.dead {
            return false;
        }

        // Check if item is sealed and not expired
        if let Some(ref sealed) = item.sealed_info {
            if sealed.expiry_binary > self.time_ms {
                // Item is sealed and not expired - cannot use
                return false;
            }
        }

        let info = match self.provider.get_item_info(item.item_index) {
            Some(i) => i,
            None => return false,
        };

        let gender_ok = match player.gender {
            0 => (info.required_gender & 1) != 0,
            1 => (info.required_gender & 2) != 0,
            _ => true,
        };
        if !gender_ok {
            return false;
        }

        let class_bit = match player.job {
            Job::Warrior => 1,
            Job::Wizard => 2,
            Job::Taoist => 4,
            Job::Assassin => 8,
            Job::Archer => 16,
        };
        if info.required_class != 0 && (info.required_class & class_bit) == 0 {
            return false;
        }

        let req_amt = info.required_amount as i32;
        match info.required_type {
            0 => {
                if (player.level as i32) < req_amt {
                    return false;
                }
            }
            1 => {
                if player.stats.total.get(Stat::MaxAC) < req_amt {
                    return false;
                }
            }
            2 => {
                if player.stats.total.get(Stat::MaxMAC) < req_amt {
                    return false;
                }
            }
            3 => {
                if player.stats.total.get(Stat::MaxDC) < req_amt {
                    return false;
                }
            }
            4 => {
                if player.stats.total.get(Stat::MaxMC) < req_amt {
                    return false;
                }
            }
            5 => {
                if player.stats.total.get(Stat::MaxSC) < req_amt {
                    return false;
                }
            }
            6 => {
                if (player.level as i32) > req_amt {
                    return false;
                }
            }
            7 => {
                if player.stats.total.get(Stat::MinAC) < req_amt {
                    return false;
                }
            }
            8 => {
                if player.stats.total.get(Stat::MinMAC) < req_amt {
                    return false;
                }
            }
            9 => {
                if player.stats.total.get(Stat::MinDC) < req_amt {
                    return false;
                }
            }
            10 => {
                if player.stats.total.get(Stat::MinMC) < req_amt {
                    return false;
                }
            }
            11 => {
                if player.stats.total.get(Stat::MinSC) < req_amt {
                    return false;
                }
            }
            _ => {}
        }

        if let Some(map_info) = self.provider.get_map_info(player.map_index) {
            if info.item_type == 13 {
                if map_info.no_drug {
                    return false;
                }
            } else if info.item_type == 17 {
                match info.shape {
                    0 => {
                        if map_info.no_escape {
                            return false;
                        }
                    }
                    1 => {
                        if map_info.no_town_teleport {
                            return false;
                        }
                    }
                    2 => {
                        if map_info.no_random {
                            return false;
                        }
                    }
                    6 => {
                        if map_info.no_reincarnation {
                            return false;
                        }
                    }
                    _ => {}
                }
            }
        }

        true
    }

    fn can_equip_item_for_player(
        &self,
        player: &PlayerState,
        item: &UserItemData,
        slot: usize,
    ) -> bool {
        // Check if item is sealed and not expired
        if let Some(ref sealed) = item.sealed_info {
            if sealed.expiry_binary > self.time_ms {
                // Item is sealed and not expired - cannot equip
                return false;
            }
        }

        let info = match self.provider.get_item_info(item.item_index) {
            Some(i) => i,
            None => return false,
        };

        if !Self::equipment_slot_matches_item(slot, &info) {
            return false;
        }

        let gender_ok = match player.gender {
            0 => (info.required_gender & 1) != 0,
            1 => (info.required_gender & 2) != 0,
            _ => true,
        };
        if !gender_ok {
            return false;
        }

        let class_bit = match player.job {
            Job::Warrior => 1,
            Job::Wizard => 2,
            Job::Taoist => 4,
            Job::Assassin => 8,
            Job::Archer => 16,
        };
        if (info.required_class & class_bit) == 0 {
            return false;
        }

        let req_amt = info.required_amount as i32;
        match info.required_type {
            0 => {
                if (player.level as i32) < req_amt {
                    return false;
                }
            }
            1 => {
                if player.stats.total.get(Stat::MaxAC) < req_amt {
                    return false;
                }
            }
            2 => {
                if player.stats.total.get(Stat::MaxMAC) < req_amt {
                    return false;
                }
            }
            3 => {
                if player.stats.total.get(Stat::MaxDC) < req_amt {
                    return false;
                }
            }
            4 => {
                if player.stats.total.get(Stat::MaxMC) < req_amt {
                    return false;
                }
            }
            5 => {
                if player.stats.total.get(Stat::MaxSC) < req_amt {
                    return false;
                }
            }
            6 => {
                if (player.level as i32) > req_amt {
                    return false;
                }
            }
            7 => {
                if player.stats.total.get(Stat::MinAC) < req_amt {
                    return false;
                }
            }
            8 => {
                if player.stats.total.get(Stat::MinMAC) < req_amt {
                    return false;
                }
            }
            9 => {
                if player.stats.total.get(Stat::MinDC) < req_amt {
                    return false;
                }
            }
            10 => {
                if player.stats.total.get(Stat::MinMC) < req_amt {
                    return false;
                }
            }
            11 => {
                if player.stats.total.get(Stat::MinSC) < req_amt {
                    return false;
                }
            }
            _ => {}
        }

        let (mut hand_weight, mut wear_weight) = self.current_equipment_weights(player);

        if let Some(current) = player.equipment.get(slot) {
            if let Some(curr_info) = self.provider.get_item_info(current.item_index) {
                if Self::is_hand_item(&curr_info) {
                    hand_weight -= curr_info.weight as i32;
                } else {
                    wear_weight -= curr_info.weight as i32;
                }
            }
        }

        if Self::is_hand_item(&info) {
            hand_weight += info.weight as i32;
            if hand_weight > player.stats.total.get(Stat::HandWeight) {
                return false;
            }
        } else {
            wear_weight += info.weight as i32;
            if wear_weight > player.stats.total.get(Stat::WearWeight) {
                return false;
            }
        }

        true
    }

    fn current_equipment_weights(&self, player: &PlayerState) -> (i32, i32) {
        let mut hand = 0i32;
        let mut wear = 0i32;
        for slot in &player.equipment.slots {
            if let Some(item) = slot {
                if let Some(info) = self.provider.get_item_info(item.item_index) {
                    if Self::is_hand_item(&info) {
                        hand += info.weight as i32;
                    } else {
                        wear += info.weight as i32;
                    }
                }
            }
        }
        (hand, wear)
    }

    fn is_hand_item(info: &ItemInfoData) -> bool {
        info.item_type == 1 || info.item_type == 12
    }

    fn equipment_slot_matches_item(slot: usize, info: &ItemInfoData) -> bool {
        match slot {
            0 => info.item_type == 1,
            1 => info.item_type == 2,
            2 => info.item_type == 4,
            3 => info.item_type == 12,
            4 => info.item_type == 5,
            5 => info.item_type == 6,
            6 => info.item_type == 6 || info.item_type == 8,
            7 | 8 => info.item_type == 7,
            9 => info.item_type == 8,
            10 => info.item_type == 9,
            11 => info.item_type == 10,
            12 => info.item_type == 11,
            13 => info.item_type == 19,
            _ => false,
        }
    }

    fn build_start_inventory(&self, job: Job, gender: u8) -> Inventory {
        let mut inventory = Inventory::new_default();

        let class_bit = match job {
            Job::Warrior => 1,
            Job::Wizard => 2,
            Job::Taoist => 4,
            Job::Assassin => 8,
            Job::Archer => 16,
        };

        let gender_bit = match gender {
            0 => 1,
            1 => 2,
            _ => 1 | 2,
        };

        // tracing::debug!(
        //     "[world] build_start_inventory: job={} gender={} class_bit={} gender_bit={}",
        //     job.as_u8(),
        //     gender,
        //     class_bit,
        //     gender_bit
        // );

        let mut counter: u64 = 0;

        for info in self.provider.item_infos() {
            let matches_class = (info.required_class & class_bit) != 0;
            let matches_gender = (info.required_gender & gender_bit) != 0;

            // if info.start_item {
            //     tracing::debug!(
            //         "[world]  candidate idx={} name={} start_item={} req_class={} req_gender={} matches_class={} matches_gender={}",
            //         info.index,
            //         info.name,
            //         info.start_item,
            //         info.required_class,
            //         info.required_gender,
            //         matches_class,
            //         matches_gender
            //     );
            // }

            if !info.start_item {
                continue;
            }

            // Mirror C# HumanObject.CorrectStartItem: RequiredClass *must* include
            // the current class bit; items with RequiredClass == 0 are never
            // valid start items for any class.
            if !matches_class {
                continue;
            }

            // Same for RequiredGender: it must explicitly include the
            // character's gender flag; items with RequiredGender == 0 are not
            // valid start items for any gender.
            if !matches_gender {
                continue;
            }

            if let Some(slot) = inventory
                .slots
                .iter_mut()
                .skip(6)
                .find(|s| s.is_none())
            {
                counter = counter.saturating_add(1);
                let unique_id = ((job.as_u8() as u64) << 56)
                    | ((gender as u64) << 48)
                    | counter;
                let item = crate::item::create_fresh_user_item(info, unique_id, 1);
                *slot = Some(item);
            } else {
                break;
            }
        }

        for (i, slot) in inventory.slots.iter().enumerate() {
            if let Some(item) = slot {
                tracing::debug!(
                    "[world]  result slot={} item_index={} for job={} gender={}",
                    i,
                    item.item_index,
                    job.as_u8(),
                    gender
                );
            }
        }

        inventory
    }
}

