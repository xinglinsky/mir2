use crate::item::{Equipment, Inventory};
use crystal_shared_proto::item_types::{ItemInfoData, UserItemData};
use crate::stats::Stat;
use crate::stats_util::aggregate_equipment_stats;
use crate::world::magic::UserMagic;
use crate::world::buff::PlayerBuff;
use crate::world::party::PartyId;
use crate::world::provider::WorldProvider;
use crystal_shared_proto::io::{write_bool, write_i32_le, write_string};
use std::collections::HashMap;

use super::{Job, PlayerStats, SessionId, World};
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
    pub allow_group: bool,
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
    /// Active poisons applied to this player. This mirrors the legacy C#
    /// MapObject.PoisonList and is processed by player_runtime.
    pub poisons: Vec<crate::world::PoisonInstance>,
    /// Bitmask of current poison types for quick comparison and client sync.
    pub current_poison_mask: u16,
    /// Per-player GameShop purchase counts keyed by GameShopItem GIndex.
    pub gs_purchases: HashMap<i32, i32>,
    pub npc_data: HashMap<String, String>,
    pub friends: Vec<FriendEntry>,
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
                    allow_group: true,
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
                    poisons: Vec::new(),
                    current_poison_mask: 0,
                    gs_purchases: HashMap::new(),
                    npc_data: HashMap::new(),
                    friends: Vec::new(),
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

