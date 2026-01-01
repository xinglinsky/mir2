//! Legacy combat functions
//!
//! This module contains combat-related functions that are not part of the core
//! combat system but are still needed:
//! - Poison system (player <-> monster interactions)
//! - PvP damage application
//! - Player death drops
//!
//! Core combat functions have been moved to the `combat` module:
//! - Attack commands: `combat::commands`
//! - Damage calculation: `combat::damage`
//! - Buffs and effects: `combat::buffs`, `combat::effects`
//! - Utility functions: `combat::utils`

use rand::{thread_rng, Rng};

use crystal_shared_proto::item_types::UserItemData;

use crate::stats::{Stat, Stats};
use crate::world::provider::WorldProvider;
use crate::world::configs::setup_config;
use crate::world::map_item::MapItem;
use crate::world::types::{AttackMode, PoisonType, DamageType};
use crate::world::{SessionId, World, WorldEvent, PoisonInstance};

impl<P: WorldProvider> World<P> {

    /// Helper that mirrors C# MonsterObject.PoisonTarget for player targets
    /// by layering an outer poison-resist roll and chance-to-poison roll in
    /// front of the core ApplyPoison-style logic implemented by
    /// apply_poison_to_player_from_monster. This does not itself enforce any
    /// IsAttackTarget rules; callers should ensure the target is valid.
    #[allow(dead_code)]
    pub(crate) fn poison_target_player(
        &mut self,
        attacker_monster_id: u64,
        map_index: i32,
        target_session_id: SessionId,
        chance_to_poison: i32,
        poison_duration: i64,
        poison_type: PoisonType,
        poison_tick_speed_ms: i64,
        no_resist: bool,
        ignore_defence: bool,
    ) -> bool {
        if chance_to_poison <= 0 || poison_duration <= 0 || poison_tick_speed_ms <= 0 {
            return false;
        }

        // Fetch the attacking monster and build its SC-based stats, mirroring
        // the GetAttackPower(MinSC, MaxSC) call in C# MonsterObject.
        let (attacker_stats, _monster_index) = {
            let monsters = match self.monsters.get(&map_index) {
                Some(ms) => ms,
                None => return false,
            };

            let monster = match monsters
                .iter()
                .find(|m| m.id == attacker_monster_id && m.hp > 0)
            {
                Some(m) => m,
                None => return false,
            };

            let info = match self.provider.get_monster_info(monster.monster_index) {
                Some(i) => i,
                None => return false,
            };

            let mut stats: Stats = info.stats.clone();
            stats.add(&monster.buff_stats);
            (stats, monster.monster_index)
        };

        let min_sc = attacker_stats.get(Stat::MinSC);
        let max_sc = attacker_stats.get(Stat::MaxSC).max(min_sc);

        let mut rng = thread_rng();
        let value = if max_sc <= min_sc {
            min_sc
        } else {
            rng.gen_range(min_sc..=max_sc)
        };

        if value <= 0 {
            return false;
        }

        // Outer poison-resist roll taken directly from C# PoisonTarget:
        //
        // if (Envir.Random.Next(Settings.PoisonResistWeight) >= target.Stats[PoisonResist])
        //     { ... ApplyPoison ... }
        //
        // which is equivalent to skipping application when the roll is less
        // than PoisonResist.
        let cfg = setup_config();
        let poison_resist_weight = cfg.items.poison_resist_weight.max(1) as i32;
        let target_resist = self
            .players
            .get(&target_session_id)
            .map(|p| p.stats.total.get(Stat::PoisonResist).max(0))
            .unwrap_or(0);

        if target_resist > 0 && poison_resist_weight > 0 {
            let roll = rng.gen_range(0..poison_resist_weight.max(1));
            if roll < target_resist {
                return false;
            }
        }

        // chanceToPoison: only when Random.Next(chanceToPoison) == 0 do we
        // proceed to apply the poison.
        if chance_to_poison > 1 {
            let roll = rng.gen_range(0..chance_to_poison.max(1));
            if roll != 0 {
                return false;
            }
        } else if chance_to_poison == 1 {
            // Always attempt to poison once resist has passed.
        } else {
            return false;
        }

        self.apply_poison_to_player_from_monster(
            attacker_monster_id,
            map_index,
            target_session_id,
            poison_type,
            value,
            poison_duration,
            poison_tick_speed_ms,
            no_resist,
            ignore_defence,
        )
    }

    /// Apply a poison from a player to a monster, approximating the stacking
    /// rules from C# MonsterObject.ApplyPoison / PoisonTarget but without
    /// armour or resist checks for now. This function only initialises the
    /// poison instance and attaches it to the target monster; the actual
    /// tick damage is processed by monster_runtime.
    pub(crate) fn apply_poison_to_monster_from_player(
        &mut self,
        attacker_sid: SessionId,
        map_index: i32,
        target_monster_id: u64,
        poison_type: PoisonType,
        value: i32,
        duration: i64,
        tick_speed_ms: i64,
    ) -> bool {
        if duration <= 0 || tick_speed_ms <= 0 {
            return false;
        }

        let monsters = match self.monsters.get_mut(&map_index) {
            Some(m) => m,
            None => return false,
        };

        let monster = match monsters.iter_mut().find(|m| m.id == target_monster_id) {
            Some(m) => m,
            None => return false,
        };

        if monster.hp <= 0 {
            return false;
        }

        let new_poison = PoisonInstance {
            owner_session_id: Some(attacker_sid),
            poison_type,
            value,
            duration,
            time: 0,
            tick_time_ms: self.time_ms.saturating_add(tick_speed_ms.max(0)),
            tick_speed_ms: tick_speed_ms.max(0),
        };

        if let Some(idx) = monster
            .poisons
            .iter()
            .position(|p| p.poison_type == poison_type)
        {
            let existing = &monster.poisons[idx];

            // Mirror key C# stacking rules:
            // - For Green poison, a weaker (lower value) poison cannot
            //   overwrite a stronger one.
            if poison_type == PoisonType::Green && existing.value > new_poison.value {
                return false;
            }

            // - For non-Green poisons, a shorter duration cannot overwrite a
            //   longer remaining duration.
            if poison_type != PoisonType::Green
                && existing.duration.saturating_sub(existing.time) > new_poison.duration
            {
                return false;
            }

            // - Prevent permanent control from Frozen/Slow/Paralysis-style
            //   poisons by rejecting reapplications while they are active.
            if matches!(
                existing.poison_type,
                PoisonType::Frozen | PoisonType::Slow | PoisonType::Paralysis | PoisonType::LRParalysis
            ) {
                return false;
            }

            // - Ignore additional DelayedExplosion applications for now.
            if poison_type == PoisonType::DelayedExplosion {
                return false;
            }

            monster.poisons[idx] = new_poison;
        } else {
            monster.poisons.push(new_poison);
        }

        true
    }


    /// Apply a poison from a monster to a player, approximating the core
    /// behaviour of C# MonsterObject.PoisonTarget together with
    /// HumanObject.ApplyPoison for Green/Red poisons. This handles poison
    /// resist checks, optional MAC-based reduction for Green poison and
    /// PoisonRecovery-based duration reduction, and reuses the same stacking
    /// rules as apply_poison_to_monster_from_player.
    #[allow(dead_code)]
    pub(crate) fn apply_poison_to_player_from_monster(
        &mut self,
        _attacker_monster_id: u64,
        map_index: i32,
        target_session_id: SessionId,
        poison_type: PoisonType,
        mut value: i32,
        mut duration: i64,
        tick_speed_ms: i64,
        no_resist: bool,
        ignore_defence: bool,
    ) -> bool {
        if duration <= 0 || tick_speed_ms <= 0 {
            return false;
        }

        let cfg = setup_config();
        let poison_resist_weight = cfg.items.poison_resist_weight.max(1) as i32;

        let player = match self.players.get_mut(&target_session_id) {
            Some(p) => p,
            None => return false,
        };

        if player.dead || player.hp <= 0 || player.map_index != map_index {
            return false;
        }

        // Poison resist check: mirror C# HumanObject.ApplyPoison where
        // Envir.Random.Next(Settings.PoisonResistWeight) < Stats[PoisonResist]
        // causes the poison application to be skipped. For monster-origin
        // poisons Caster.Race != Player so the PvP gating does not apply.
        if !no_resist {
            let poison_resist = player.stats.total.get(Stat::PoisonResist).max(0);
            if poison_resist > 0 && poison_resist_weight > 0 {
                let mut rng = thread_rng();
                let roll = rng.gen_range(0..poison_resist_weight.max(1));
                if roll < poison_resist {
                    return false;
                }
            }
        }

        // For Green poison, optionally reduce Value by the player's MAC
        // before applying, mirroring the C# ignoreDefence behaviour where a
        // high MAC can negate or reduce the poison entirely.
        if !ignore_defence && poison_type == PoisonType::Green {
            let stats = &player.stats.total;
            let min_mac = stats.get(Stat::MinMAC).max(0);
            let max_mac = stats.get(Stat::MaxMAC).max(min_mac);

            let armour = if max_mac <= min_mac {
                min_mac
            } else {
                let mut rng = thread_rng();
                rng.gen_range(min_mac..=max_mac)
            };

            if value < armour {
                // In C#, this sets PType = None, effectively cancelling the
                // poison application.
                return false;
            } else {
                value = value.saturating_sub(armour);
            }
        }

        // Apply PoisonRecovery to Green/Red duration, mirroring C# where
        // Duration is reduced by Stats[PoisonRecovery].
        if poison_type == PoisonType::Green || poison_type == PoisonType::Red {
            let recovery = player.stats.total.get(Stat::PoisonRecovery).max(0) as i64;
            if recovery > 0 {
                duration = duration.saturating_sub(recovery).max(0);
            }
            if duration <= 0 {
                return false;
            }
        }

        let new_poison = PoisonInstance {
            // For monster-origin poisons we do not currently attribute the
            // owner to a specific player session; poison damage is applied
            // directly to the player rather than via PendingMagicHit.
            owner_session_id: None,
            poison_type,
            value,
            duration,
            time: 0,
            tick_time_ms: self.time_ms.saturating_add(tick_speed_ms.max(0)),
            tick_speed_ms: tick_speed_ms.max(0),
        };

        if let Some(idx) = player
            .poisons
            .iter()
            .position(|p| p.poison_type == poison_type)
        {
            let existing = &player.poisons[idx];

            // Reuse the same stacking rules as monster poisons so that Green
            // poisons cannot be overwritten by weaker applications and
            // non-Green poisons preserve longer remaining durations.
            if poison_type == PoisonType::Green && existing.value > new_poison.value {
                return false;
            }

            if poison_type != PoisonType::Green
                && existing.duration.saturating_sub(existing.time) > new_poison.duration
            {
                return false;
            }

            if matches!(
                existing.poison_type,
                PoisonType::Frozen | PoisonType::Slow | PoisonType::Paralysis | PoisonType::LRParalysis
            ) {
                return false;
            }

            if poison_type == PoisonType::DelayedExplosion {
                return false;
            }

            player.poisons[idx] = new_poison;
        } else {
            player.poisons.push(new_poison);
        }

        true
    }

    /// Check if a player has a specific buff type
    // player_has_buff moved to combat::buffs

    /// Perform a counter attack from defender to attacker
    // perform_counter_attack and remove_player_buff moved to combat::buffs

    /// Apply equipment durability loss when taking damage
    // apply_equipment_durability_loss, heal_player moved to combat::damage
    pub(crate) fn apply_damage_to_player(
        &mut self,
        attacker_session_id: Option<SessionId>,
        target_session_id: SessionId,
        damage: i32,
        damage_type: DamageType,
        map_index: i32,
        events: &mut Vec<WorldEvent>,
    ) -> (i32, bool) {
        use crate::world::combat::damage;
        damage::apply_damage_to_player(
            self,
            attacker_session_id,
            target_session_id,
            damage,
            damage_type,
            map_index,
            events,
        )
    }

    // apply_damage_to_player_internal implementation moved to combat::damage
    // Old implementation removed - use apply_damage_to_player instead

    // can_attack_player moved to combat::utils

    /// Handle combat buffs and counter-attacks after damage
    // handle_combat_buffs_and_counters moved to combat::buffs

    /// Apply player death-drop logic for the given target session on the
    /// specified map. This approximates C# PlayerObject.Die ->
    /// DeathDrop/RedDeathDrop by selecting a subset of equipment and
    /// inventory items to drop on the ground near the corpse and assigning a
    /// longer expire timeout based on GameConfig.player_died_item_timeout.
    pub(crate) fn apply_player_death_drops(
        &mut self,
        target_sid: SessionId,
        map_index: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        // Snapshot basic position/PK state and the player's name without
        // holding a mutable borrow so we can still mutate world structures
        // below.
        let (px, py, pk_points, player_name) = match self.players.get(&target_sid) {
            Some(p) => (p.x, p.y, p.pk_points, p.name.clone()),
            None => return,
        };

        let map_info = match self.provider.get_map_info(map_index) {
            Some(info) => info,
            None => return,
        };

        // Honour NoDropPlayer like C# Map.Info.NoDropPlayer.
        if map_info.no_drop_player {
            return;
        }

        // Use the configured PlayerDiedItemTimeOut (in seconds) from
        // Setup.ini. This is the extended timeout used by C# ItemObject for
        // player-death drops.
        let cfg = setup_config();
        let timeout_secs = cfg.game.player_died_item_timeout.max(0) as i64;
        let item_timeout_ms = timeout_secs.saturating_mul(1_000);
        let now = self.time_ms.max(0);

        let mut rng = thread_rng();
        let is_red = pk_points > 200;

        // Mirror the C# behaviour where non-red players only death-drop when
        // not in a SafeZone, while red players (PKPoints > 200) can drop
        // regardless of SafeZone.
        if !is_red && self.player_in_safe_zone(target_sid) {
            return;
        }

        // Collect candidate drops as (is_equipment, slot_index, item_clone)
        // and items that should be destroyed (BreakOnDeath) as
        // (is_equipment, slot_index).
        let mut drops: Vec<(bool, usize, UserItemData)> = Vec::new();
        let mut destroys: Vec<(bool, usize)> = Vec::new();

        const BIND_DONT_DEATHDROP: i16 = 0x0001;
        const BIND_BREAK_ON_DEATH: i16 = 0x0100;

        if let Some(p) = self.players.get(&target_sid) {
            // First, approximate C# ItemSets behaviour for the Spirit set so
            // we can mirror the logic that destroys Spirit items when the set
            // is incomplete. In the C# server this is driven by ItemSets and
            // SetComplete; here we approximate it by counting how many
            // equipped items belong to the Spirit set (ItemInfo.set ==
            // ItemSet.Spirit). If the player has at least one but fewer than
            // SPIRIT_FULL_SET_PIECES Spirit items equipped, we consider the
            // set incomplete and destroy all Spirit equipment on death.
            let mut spirit_pieces: i32 = 0;
            for opt in p.equipment.slots.iter() {
                if let Some(it) = opt {
                    if let Some(info) = self.provider.get_item_info(it.item_index) {
                        if info.set == 1 {
                            spirit_pieces += 1;
                        }
                    }
                }
            }
            const SPIRIT_FULL_SET_PIECES: i32 = 4;
            let destroy_spirit_equipment = spirit_pieces > 0 && spirit_pieces < SPIRIT_FULL_SET_PIECES;

            // Equipment: approximate DeathDrop/RedDeathDrop percentages and
            // BindMode semantics.
            for (idx, opt) in p.equipment.slots.iter().enumerate() {
                let item = match opt {
                    Some(it) => it,
                    None => continue,
                };

                let info = match self.provider.get_item_info(item.item_index) {
                    Some(i) => i,
                    None => continue,
                };

                // Skip DontDeathdrop-bound items and rental equivalents.
                if (info.bind & BIND_DONT_DEATHDROP) != 0 {
                    continue;
                }
                if item
                    .rental_information
                    .as_ref()
                    .map_or(false, |r| (r.binding_flags & BIND_DONT_DEATHDROP) != 0)
                {
                    continue;
                }

                // Skip wedding rings (C# item.WeddingRing != -1). We treat
                // any non-zero wedding_ring as a bound marriage ring.
                if item.wedding_ring != 0 {
                    continue;
                }

                // Skip sealed items entirely for now while the SealedInfo
                // model is not yet fully wired.
                if item.sealed_info.is_some() {
                    continue;
                }

                // Approximate C# Spirit set death behaviour: when the player
                // has an incomplete Spirit set equipped, destroy all Spirit
                // set equipment on death instead of dropping it.
                if destroy_spirit_equipment && info.set == 1 {
                    destroys.push((true, idx));
                    continue;
                }

                let break_on_death =
                    (info.bind & BIND_BREAK_ON_DEATH) != 0
                        || item
                            .rental_information
                            .as_ref()
                            .map_or(false, |r| (r.binding_flags & BIND_BREAK_ON_DEATH) != 0);

                if break_on_death {
                    // Destroy the item on death without dropping a
                    // ground item, mirroring BindMode.BreakOnDeath.
                    destroys.push((true, idx));
                    continue;
                }

                let mut drop_count: u16 = 0;
                if item.count > 1 {
                    // Stacks: choose a random 1–8 (or 4–10 for red) percent
                    // of the stack, similar to HumanObject.DeathDrop and
                    // PlayerObject.RedDeathDrop.
                    let percent = if is_red {
                        rng.gen_range(4..=10)
                    } else {
                        rng.gen_range(1..=8)
                    } as u32;
                    let total = item.count as u32;
                    let calc = ((total * percent + 9) / 10) as u16; // ceil
                    if calc > 0 {
                        drop_count = calc.min(item.count);
                    }
                } else {
                    // Single equipment item: 1/30 for normal, 1/10 for red.
                    let chance = if is_red { 10 } else { 30 };
                    if rng.gen_range(0..chance) == 0 {
                        // For single-count rental items, approximate C#
                        // DeathDrop/RedDeathDrop behaviour by returning the
                        // item to its owner instead of dropping it on the
                        // ground.
                        if item.rental_information.is_some() {
                            let name = info.friendly_name();
                            events.push(WorldEvent::PartySystemMessage {
                                session_id: target_sid,
                                message: format!(
                                    "You died and {} has been returned to it's owner.",
                                    name
                                ),
                            });
                            destroys.push((true, idx));
                        } else {
                            drop_count = 1;
                        }
                    }
                }

                if drop_count > 0 {
                    let mut clone = item.clone();
                    clone.count = drop_count;
                    drops.push((true, idx, clone));
                }
            }

            // Inventory: similar probabilities but with a higher base chance
            // for singles in normal (1/10) and red (1/5) cases.
            for (idx, opt) in p.inventory.slots.iter().enumerate() {
                let item = match opt {
                    Some(it) => it,
                    None => continue,
                };

                let info = match self.provider.get_item_info(item.item_index) {
                    Some(i) => i,
                    None => continue,
                };

                if (info.bind & BIND_DONT_DEATHDROP) != 0 {
                    continue;
                }
                if item
                    .rental_information
                    .as_ref()
                    .map_or(false, |r| (r.binding_flags & BIND_DONT_DEATHDROP) != 0)
                {
                    continue;
                }

                if item.wedding_ring != 0 {
                    continue;
                }

                if item.sealed_info.is_some() {
                    continue;
                }

                let break_on_death =
                    (info.bind & BIND_BREAK_ON_DEATH) != 0
                        || item
                            .rental_information
                            .as_ref()
                            .map_or(false, |r| (r.binding_flags & BIND_BREAK_ON_DEATH) != 0);

                if break_on_death {
                    destroys.push((false, idx));
                    continue;
                }

                let mut drop_count: u16 = 0;
                if item.count > 1 {
                    let percent = if is_red {
                        rng.gen_range(4..=10)
                    } else {
                        rng.gen_range(1..=8)
                    } as u32;
                    let total = item.count as u32;
                    let calc = ((total * percent + 9) / 10) as u16;
                    if calc > 0 {
                        drop_count = calc.min(item.count);
                    }
                } else {
                    // Single inventory item: 1/10 for normal, 1/5 for red.
                    let chance = if is_red { 5 } else { 10 };
                    if rng.gen_range(0..chance) == 0 {
                        if item.rental_information.is_some() {
                            let name = info.friendly_name();
                            events.push(WorldEvent::PartySystemMessage {
                                session_id: target_sid,
                                message: format!(
                                    "You died and {} has been returned to it's owner.",
                                    name
                                ),
                            });
                            destroys.push((false, idx));
                        } else {
                            drop_count = 1;
                        }
                    }
                }

                if drop_count > 0 {
                    let mut clone = item.clone();
                    clone.count = drop_count;
                    drops.push((false, idx, clone));
                }
            }
        }

        if drops.is_empty() {
            return;
        }

        // Place map items near the corpse using the same drop location search
        // as normal item drops, but with the extended death-drop timeout.
        let mut placed: Vec<(bool, usize, UserItemData)> = Vec::new();

        for (is_eq, idx, item) in drops.into_iter() {
            let info = match self.provider.get_item_info(item.item_index) {
                Some(i) => i,
                None => continue,
            };

            if let Some((dx, dy)) = self.find_drop_location(map_index, px, py, 4) {
                let entry = self.map_items.entry(map_index).or_default();
                let map_item_id = self.next_map_item_id;
                self.next_map_item_id = self.next_map_item_id.wrapping_add(1);

                entry.push(MapItem {
                    id: map_item_id,
                    map_index,
                    x: dx,
                    y: dy,
                    item_index: Some(info.index),
                    gold: 0,
                    count: item.count,
                    item: Some(item.clone()),
                    expire_time_ms: now.saturating_add(item_timeout_ms),
                });

                events.push(WorldEvent::ItemDropped {
                    object_id: map_item_id,
                    map_index,
                    x: dx,
                    y: dy,
                    item_index: info.index,
                    count: item.count,
                });

                // If this item is flagged for global drop notification,
                // approximate the C# GlobalDropNotify behaviour by sending a
                // system chat message to all online players indicating that
                // the dead player dropped this item.
                if info.global_drop_notify {
                    let base_name = info.friendly_name();
                    let name = if item.count > 1 {
                        format!("{} ({})", base_name, item.count)
                    } else {
                        base_name
                    };
                    let text = format!("{} has dropped {}.", player_name, name);
                    for (&sid, _) in self.players.iter() {
                        events.push(WorldEvent::PartySystemMessage {
                            session_id: sid,
                            message: text.clone(),
                        });
                    }
                }

                placed.push((is_eq, idx, item));
            }
        }

        // Finally, remove the dropped or destroyed items from the player's
        // equipment / inventory slots.
        if let Some(p) = self.players.get_mut(&target_sid) {
            // Apply stack reductions for items that produced ground drops.
            for (is_eq, idx, item) in placed.into_iter() {
                let slots = if is_eq {
                    &mut p.equipment.slots
                } else {
                    &mut p.inventory.slots
                };

                if let Some(slot_opt) = slots.get_mut(idx) {
                    if let Some(slot_item) = slot_opt {
                        if item.count >= slot_item.count {
                            *slot_opt = None;
                        } else {
                            slot_item.count = slot_item
                                .count
                                .saturating_sub(item.count);
                        }
                    }
                }
            }

            // Remove items flagged as BreakOnDeath regardless of whether a
            // ground drop was produced.
            for (is_eq, idx) in destroys.into_iter() {
                let slots = if is_eq {
                    &mut p.equipment.slots
                } else {
                    &mut p.inventory.slots
                };

                if let Some(slot_opt) = slots.get_mut(idx) {
                    *slot_opt = None;
                }
            }
        }
    }


    /// Apply damage from one player to another, updating HP/death,
    /// PKPoints/BrownTime and emitting ObjectStruck, mirroring the core C#
    /// PlayerObject.Attacked + PK rules. `damage` may be zero to represent a
    /// miss; in that case we still emit ObjectStruck but do not change HP.
    pub(crate) fn apply_player_hit_from_player(
        &mut self,
        attacker_sid: SessionId,
        target_sid: SessionId,
        map_index: i32,
        damage: i32,
        damage_type: u8,
        strike_x_override: Option<i32>,
        strike_y_override: Option<i32>,
        strike_dir_override: Option<u8>,
        allow_pk_points: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        if attacker_sid == target_sid {
            return;
        }

        let damage = damage.max(0);

        let (old_pk_points, old_brown_time) = if let Some(t) = self.players.get(&target_sid) {
            (t.pk_points, t.brown_time_ms)
        } else {
            return;
        };

        let (max_hp, strike_x, strike_y, strike_dir, new_hp, dead) =
            if let Some(t) = self.players.get_mut(&target_sid) {
                if t.dead || t.hp <= 0 {
                    return;
                }

                let max_hp = t.stats.total.get(Stat::HP).max(1);

                let mut strike_x = t.x;
                let mut strike_y = t.y;
                let mut strike_dir = t.direction;

                if let Some(sx) = strike_x_override {
                    strike_x = sx;
                }
                if let Some(sy) = strike_y_override {
                    strike_y = sy;
                }
                if let Some(sd) = strike_dir_override {
                    strike_dir = sd;
                }

                let old_hp = t.hp.max(0);
                let mut new_hp = old_hp;
                let mut dead = false;

                if damage > 0 {
                    if damage >= t.hp {
                        t.hp = 0;
                        dead = true;
                    } else {
                        t.hp -= damage;
                    }

                    new_hp = t.hp.max(0);
                }

                (max_hp, strike_x, strike_y, strike_dir, new_hp, dead)
            } else {
                return;
            };

        let health_percent = if max_hp > 0 {
            ((new_hp as i64 * 100) / max_hp as i64).clamp(0, 100) as u8
        } else {
            0
        };

        events.push(WorldEvent::ObjectStruck {
            attacker_id: attacker_sid,
            target_id: target_sid as u64,
            map_index,
            x: strike_x,
            y: strike_y,
            direction: strike_dir,
            damage,
            damage_type,
            health_percent,
            show_struck: true,
        });

        // If the hit killed the target, first attempt revival via Revival
        // rings before applying death buffs/drops. If revival succeeds, skip
        // further death handling (no PKPoints, no drops).
        if dead {
            if self.try_revive_with_revival_ring(target_sid, events) {
                return;
            }

            // 玩家真正死亡时，先清理该玩家的所有宠物/召唤物，再按 C#
            // HumanObject.Die 语义清理带 RemoveOnDeath 属性的 Buff，并
            // 处理死亡掉落逻辑。
            self.remove_all_pets_for_session(target_sid);
            self.clear_player_buffs_on_death(target_sid, events);
            self.apply_player_death_drops(target_sid, map_index, events);
            
            // Call default NPC Die page after player death
            // C#: CallDefaultNPC(DefaultNPCType.Die)
            // C# generates key as: "Die" (no parameters)
            // Then wraps it as: string.Format("[@_{0}]", key) -> "[@_Die]"
            events.push(WorldEvent::PlayerDied {
                session_id: target_sid,
            });
        }

        if !allow_pk_points || damage <= 0 || !dead {
            return;
        }

        let mut murder = false;

        if let Some(map_info) = self.provider.get_map_info(map_index) {
            if !map_info.fight {
                // PK/BrownTime logic: mirror existing melee path, only outside fight/war zones.
                if let Some(t) = self.players.get_mut(&target_sid) {
                    t.brown_time_ms = self.time_ms;
                }

                let eligible = old_pk_points < 200 && self.time_ms > old_brown_time;
                if eligible {
                    if let Some(att) = self.players.get_mut(&attacker_sid) {
                        att.pk_points = att.pk_points.saturating_add(100);
                    }
                    murder = true;
                }
            }
        }

        if murder {
            use crate::world::combat::effects;
            effects::apply_weapon_luck_curse(self, attacker_sid, events);
        }
    }


}

