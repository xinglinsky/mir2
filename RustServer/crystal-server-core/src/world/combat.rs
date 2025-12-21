use rand::{thread_rng, Rng};

use crystal_shared_proto::item_types::UserItemData;

use crate::combat::compute_physical_melee_with_crit;
use crate::world::magic::magic_power;
use crate::stats::{Stat, Stats};
use crate::world::monster::MonsterAiState;
use crate::world::player::PlayerState;
use crate::world::provider::WorldProvider;
use crate::world::configs::setup_config;
use crate::world::map_item::MapItem;
use crate::world::skills::{
    apply_attack_spell_scaling,
    apply_fatal_sword_and_undead,
    compute_magic_mana_cost,
    compute_pure_magic_attack_damage,
    fatal_sword_level_for_player,
    is_pure_magic_attack,
    resolve_attack_spell_and_level_for_player,
};
use crate::world::skills::assassin::{apply_moon_dark_bonus, resolve_moon_dark_opening, cast_swift_feet, cast_haste};
use crate::world::skills::warrior::{
    cast_counter_attack,
    cast_cross_half_moon,
    cast_flaming_sword,
    cast_half_moon,
    cast_immortal_skin,
    cast_rage,
    cast_fury,
    cast_shoulder_dash,
    is_thrusting_spell,
    resolve_slaying_for_attack,
    thrusting_max_range,
};
use crate::world::skills::wizard::{
    cast_fire_bang_ice_storm,
    cast_fire_wall,
    cast_lightning,
    cast_hell_fire,
    cast_magic_shield,
    cast_thunder_storm_flame_field,
    cast_blizzard,
    cast_blink,
    cast_storm_escape,
    cast_teleport,
};
use crate::world::skills::taoist::{
    cast_blessed_armour,
    cast_energy_shield,
    cast_hallucination,
    cast_healing,
    cast_hiding,
    cast_mass_healing,
    cast_mass_hiding,
    cast_poisoning,
    cast_reincarnation,
    cast_soul_shield,
    cast_summon_holy_deva,
    cast_summon_shinsu,
    cast_summon_skeleton,
    cast_ultimate_enhancer,
};
use crate::world::types::{AttackMode, BuffType, PetMode, PoisonType, DamageType};
use crate::world::{PendingMagicHit, SessionId, World, WorldEvent, Spell, PoisonInstance};

/// Helper struct for player damage calculation data
struct PlayerDamageData {
    hp: i32,
    stats: Stats,
    current_poison_mask: u16,
    dead: bool,
}

/// Helper struct for player position data
struct PlayerPosition {
    x: i32,
    y: i32,
    direction: u8,
}

impl<P: WorldProvider> World<P> {
    fn can_attack(_player: &PlayerState) -> bool {
        true
    }

    pub(super) fn handle_range_attack_command(
        &mut self,
        session_id: SessionId,
        direction: u8,
        target_id: u32,
        target_x: i32,
        target_y: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let (map_index, x, y, attacker_stats) = match self.players.get_mut(&session_id) {
            Some(p) => {
                if !Self::can_attack(p) {
                    return;
                }
                p.direction = direction;
                (p.map_index, p.x, p.y, p.stats.total.clone())
            }
            None => return,
        };

        // Always emit the ranged-attack visual so the client feels responsive.
        events.push(WorldEvent::ObjectRangeAttack {
            object_id: session_id as u64,
            map_index,
            x,
            y,
            direction,
            target_id: target_id as u64,
            target_x,
            target_y,
            spell: 0,
            level: 0,
            attack_type: 0,
        });

        // Minimal playable damage: if the packet provides a target monster id
        // within a reasonable range, schedule an immediate PendingMagicHit so
        // that HP, drops and experience reuse the existing monster-runtime
        // pipeline.
        if target_id == 0 {
            return;
        }

        let target_monster_id = target_id as u64;
        if !self.can_attack_monster(session_id, map_index, target_monster_id) {
            return;
        }

        // Locate the monster and build defender stats.
        let (monster_index, mx, my, defender_stats) = match self.monsters.get(&map_index) {
            Some(monsters) => {
                if let Some(m) = monsters.iter().find(|m| m.id == target_monster_id && m.hp > 0) {
                    let monster_index = m.monster_index;
                    let mut stats = self
                        .provider
                        .get_monster_info(monster_index)
                        .map(|info| info.stats.clone())
                        .unwrap_or_default();
                    stats.add(&m.buff_stats);
                    (monster_index, m.x, m.y, stats)
                } else {
                    return;
                }
            }
            None => return,
        };

        let dx = mx - x;
        let dy = my - y;
        let dist = dx.abs().max(dy.abs());
        let max_range: i32 = 8;
        if dist <= 0 || dist > max_range {
            return;
        }

        let (hit, raw_damage, damage_type) =
            compute_physical_melee_with_crit(&attacker_stats, &defender_stats);
        if !hit || raw_damage <= 0 {
            return;
        }

        let base_time = self.time_ms.max(0);
        self.pending_magic_hits.push(PendingMagicHit {
            due_time_ms: base_time,
            attacker_session_id: session_id,
            map_index,
            target_monster_id,
            monster_index,
            spell_id: 0,
            damage: raw_damage,
            damage_type,
        });
    }

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

    /// When a player in FocusMasterTarget pet mode acquires a valid monster
    /// target (via melee or attack-command driven magic), update their pets'
    /// focus target so that pet AI can mirror C# PetMode.FocusMasterTarget
    /// behaviour and only attack that monster.
    fn maybe_update_pet_focus_target_for_player_on_monster(
        &mut self,
        session_id: SessionId,
        monster_id: u64,
    ) {
        let pmode = match self.players.get(&session_id) {
            Some(p) => PetMode::from_u8(p.pet_mode),
            None => return,
        };

        if pmode == PetMode::FocusMasterTarget {
            self.set_player_pet_focus_target_monster(session_id, Some(monster_id));
        }
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
    fn player_has_buff(&self, session_id: SessionId, buff_type: BuffType) -> bool {
        if let Some(player) = self.players.get(&session_id) {
            player.active_buffs.iter().any(|buff| buff.buff_type == buff_type)
        } else {
            false
        }
    }

    /// Perform a counter attack from defender to attacker
    fn perform_counter_attack(
        &mut self,
        defender_sid: SessionId,
        attacker_sid: SessionId,
        map_index: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let (defender_stats, attacker_x, attacker_y) = match (
            self.players.get(&defender_sid),
            self.players.get(&attacker_sid),
        ) {
            (Some(defender), Some(attacker)) => {
                (defender.stats.total.clone(), attacker.x, attacker.y)
            }
            _ => return,
        };

        // Calculate counter attack damage based on defender's DC
        let min_dc = defender_stats.get(Stat::MinDC).max(0);
        let max_dc = defender_stats.get(Stat::MaxDC).max(min_dc);
        let base_damage = if max_dc == min_dc {
            min_dc
        } else {
            let mut rng = thread_rng();
            rng.gen_range(min_dc..=max_dc)
        };

        // Apply counter attack damage
        let _ = self.apply_damage_to_player(
            Some(defender_sid),
            attacker_sid,
            base_damage,
            DamageType::Physical,
            map_index,
            events,
        );

        // Send counter attack animation
        events.push(WorldEvent::ObjectMagic {
            session_id: defender_sid,
            map_index,
            direction: 0, // Will be set based on attacker position
            x: 0,
            y: 0,
            spell: crate::world::Spell::CounterAttack as u8,
            level: 0,
            target_id: attacker_sid as u32,
            target_x: attacker_x,
            target_y: attacker_y,
        });
    }

    /// Remove a specific buff from a player
    fn remove_player_buff(&mut self, session_id: SessionId, buff_type: BuffType) {
        if let Some(player) = self.players.get_mut(&session_id) {
            player.active_buffs.retain(|buff| buff.buff_type != buff_type);
        }
    }

    /// Apply equipment durability loss when taking damage
    fn apply_equipment_durability_loss(
        &mut self,
        session_id: SessionId,
        damage_taken: i32,
        _events: &mut Vec<WorldEvent>,
    ) {
        // Durability loss is typically based on damage taken
        // Higher damage = more durability loss
        // Common formula: damage / 100 (minimum 1)
        let durability_loss = (damage_taken / 100).max(1) as u16;
        
        if let Some(player) = self.players.get_mut(&session_id) {
            let mut equipment_changed = false;
            
            // Apply durability loss to all equipped items
            for slot_opt in player.equipment.slots.iter_mut() {
                if let Some(item) = slot_opt {
                    if item.current_dura > 0 {
                        let old_dura = item.current_dura;
                        item.current_dura = item.current_dura.saturating_sub(durability_loss);
                        
                        if item.current_dura != old_dura {
                            equipment_changed = true;
                            
                            // If item reached 0 durability, it might break
                            if item.current_dura == 0 {
                                // Item broke - could add special handling here
                                // For now, just leave it at 0
                            }
                        }
                    }
                }
            }
            
            // Recalculate equipment stats if durability changed
            if equipment_changed {
                self.recalc_player_equipment_stats(session_id);
            }
        }
    }

    /// Heal a player by a certain amount
    fn heal_player(&mut self, session_id: SessionId, heal_amount: i32, events: &mut Vec<WorldEvent>) {
        if let Some(player) = self.players.get_mut(&session_id) {
            if player.dead || heal_amount <= 0 {
                return;
            }

            let max_hp = player.stats.total.get(Stat::HP).max(1);
            let old_hp = player.hp;
            let new_hp = (old_hp + heal_amount).min(max_hp);
            
            if new_hp > old_hp {
                player.hp = new_hp;
                let amount = new_hp - old_hp;
                
                events.push(WorldEvent::PlayerHealed {
                    session_id,
                    map_index: player.map_index,
                    x: player.x,
                    y: player.y,
                    amount,
                    new_hp,
                    show_healing_effect: true,
                });
            }
        }
    }

    /// Apply damage to a player, mirroring C# PlayerObject.Struck behavior.
    /// Returns (actual_damage, is_dead) tuple.
    pub(crate) fn apply_damage_to_player(
        &mut self,
        attacker_session_id: Option<SessionId>,
        target_session_id: SessionId,
        damage: i32,
        damage_type: DamageType,
        map_index: i32,
        events: &mut Vec<WorldEvent>,
    ) -> (i32, bool) {
        self.apply_damage_to_player_internal(
            attacker_session_id,
            target_session_id,
            damage,
            damage_type,
            map_index,
            events,
            false, // is_reflected flag to prevent infinite loops
        )
    }

    /// Internal damage application with reflection guard
    fn apply_damage_to_player_internal(
        &mut self,
        attacker_session_id: Option<SessionId>,
        target_session_id: SessionId,
        damage: i32,
        damage_type: DamageType,
        map_index: i32,
        events: &mut Vec<WorldEvent>,
        is_reflected: bool,
    ) -> (i32, bool) {
        // Extract player data first to avoid borrow checker issues
        let (player_data, player_pos) = match self.players.get(&target_session_id) {
            Some(p) => {
                if p.dead || p.hp <= 0 {
                    return (0, false);
                }
                (
                    PlayerDamageData {
                        hp: p.hp,
                        stats: p.stats.total.clone(),
                        current_poison_mask: p.current_poison_mask,
                        dead: p.dead,
                    },
                    PlayerPosition {
                        x: p.x,
                        y: p.y,
                        direction: p.direction,
                    }
                )
            }
            None => return (0, false),
        };

        // Calculate damage with all reductions
        let (final_damage, is_dead) = self.calculate_damage_result(
            damage,
            damage_type,
            &player_data,
        );

        // Apply the damage result to the player
        self.apply_damage_result_to_player(
            target_session_id,
            final_damage,
            is_dead,
            attacker_session_id,
            damage_type,
            map_index,
            player_pos,
            events,
            is_reflected,
        );

        (final_damage, is_dead)
    }

    /// Calculate damage result without modifying player state
    fn calculate_damage_result(
        &self,
        mut damage: i32,
        damage_type: DamageType,
        player_data: &PlayerDamageData,
    ) -> (i32, bool) {
        // Apply AC/MAC damage reduction based on damage type
        let defence = match damage_type {
            DamageType::Physical => {
                let min_ac = player_data.stats.get(Stat::MinAC).max(0);
                let max_ac = player_data.stats.get(Stat::MaxAC).max(min_ac);
                if max_ac <= min_ac {
                    min_ac
                } else {
                    let mut rng = thread_rng();
                    rng.gen_range(min_ac..=max_ac)
                }
            }
            DamageType::Magical | DamageType::Poison => {
                let min_mac = player_data.stats.get(Stat::MinMAC).max(0);
                let max_mac = player_data.stats.get(Stat::MaxMAC).max(min_mac);
                if max_mac <= min_mac {
                    min_mac
                } else {
                    let mut rng = thread_rng();
                    rng.gen_range(min_mac..=max_mac)
                }
            }
            // Elemental damage uses MAC reduction
            _ if damage_type.is_elemental() => {
                let min_mac = player_data.stats.get(Stat::MinMAC).max(0);
                let max_mac = player_data.stats.get(Stat::MaxMAC).max(min_mac);
                if max_mac <= min_mac {
                    min_mac
                } else {
                    let mut rng = thread_rng();
                    rng.gen_range(min_mac..=max_mac)
                }
            }
            _ => 0,
        };

        damage = damage.saturating_sub(defence);

        // Apply elemental resistance after MAC reduction
        if damage_type.is_elemental() {
            let resistance_stat = damage_type.get_resistance_stat();
            let resistance = player_data.stats.get(resistance_stat).max(0);
            
            if resistance > 0 {
                // Resistance reduces damage by percentage
                // Formula: damage * (1 - resistance / weight)
                // Default weight is 10, so 10 resistance = 100% immunity
                const RESISTANCE_WEIGHT: i32 = 10;
                let reduction_percent = (resistance * 100) / RESISTANCE_WEIGHT;
                let reduction_percent = reduction_percent.min(100); // Cap at 100%
                
                let reduction_amount = (damage * reduction_percent) / 100;
                damage = damage.saturating_sub(reduction_amount);
            }
        }

        // Enforce minimum damage (at least 1)
        damage = damage.max(1);

        // Check if player would die
        let is_dead = player_data.hp.saturating_sub(damage) <= 0;

        (damage, is_dead)
    }

    /// Apply calculated damage to player and handle all side effects
    fn apply_damage_result_to_player(
        &mut self,
        target_session_id: SessionId,
        damage: i32,
        is_dead: bool,
        attacker_session_id: Option<SessionId>,
        damage_type: DamageType,
        map_index: i32,
        player_pos: PlayerPosition,
        events: &mut Vec<WorldEvent>,
        is_reflected: bool,
    ) {
        // Apply damage to player HP
        let (hp, dead) = if let Some(player) = self.players.get_mut(&target_session_id) {
            player.hp = player.hp.saturating_sub(damage);
            let dead = is_dead || player.hp <= 0;
            
            if dead {
                player.dead = true;
                // Note: death_time_ms field doesn't exist - would need to be added to PlayerState
            }
            
            (player.hp, dead)
        } else {
            return;
        };

        // Send damage packet to nearby players
        events.push(WorldEvent::ObjectStruck {
            attacker_id: attacker_session_id.unwrap_or(target_session_id),
            target_id: target_session_id as u64,
            map_index,
            x: player_pos.x,
            y: player_pos.y,
            direction: player_pos.direction,
            damage,
            damage_type: damage_type.as_u8(),
            health_percent: if let Some(player) = self.players.get(&target_session_id) {
                ((player.hp as i64 * 100) / player.stats.total.get(Stat::HP).max(1) as i64).clamp(0, 100) as u8
            } else {
                0
            },
        });

        // Apply equipment durability loss
        self.apply_equipment_durability_loss(target_session_id, damage, events);

        // Apply death drops if player died
        if dead {
            self.apply_player_death_drops(target_session_id, map_index, events);
        }

        // Handle buffs and counter-attacks
        self.handle_combat_buffs_and_counters(
            attacker_session_id,
            target_session_id,
            damage,
            map_index,
            events,
            is_reflected,
        );
    }

    /// Handle combat buffs and counter-attacks after damage
    fn handle_combat_buffs_and_counters(
        &mut self,
        attacker_session_id: Option<SessionId>,
        target_session_id: SessionId,
        damage: i32,
        map_index: i32,
        events: &mut Vec<WorldEvent>,
        is_reflected: bool,
    ) {
        // Apply LifeSteal吸血 if the attacker has the buff
        if let Some(attacker_sid) = attacker_session_id {
            if self.player_has_buff(attacker_sid, BuffType::LifeSteal) {
                // LifeSteal typically heals a percentage of damage dealt
                // Common values are 10-30% depending on the spell level
                let lifesteal_percent = 15; // 15% lifesteal as default
                let heal_amount = (damage * lifesteal_percent) / 100;
                
                if heal_amount > 0 {
                    self.heal_player(attacker_sid, heal_amount, events);
                }
            }

            // Apply ThornReflect反伤 if the defender has the buff
            if !is_reflected && self.player_has_buff(target_session_id, BuffType::ThornReflect) {
                // ThornReflect typically returns a percentage of damage taken
                // Common values are 10-30% depending on the spell level
                let reflect_percent = 20; // 20% reflection as default
                let reflect_damage = (damage * reflect_percent) / 100;
                
                if reflect_damage > 0 {
                    // Apply reflected damage back to attacker with reflection guard
                    let _ = self.apply_damage_to_player_internal(
                        Some(target_session_id), // Defender is now the attacker
                        attacker_sid,
                        reflect_damage,
                        DamageType::Physical,
                        map_index,
                        events,
                        true, // Mark as reflected to prevent infinite loops
                    );
                }
            }
            
            // Apply CounterAttack if the defender has the buff
            if self.player_has_buff(target_session_id, BuffType::CounterAttack) {
                // Get the CounterAttack buff to check its level
                if let Some(player) = self.players.get(&target_session_id) {
                    if let Some(counter_buff) = player.active_buffs.iter()
                        .find(|b| b.buff_type == BuffType::CounterAttack) {
                        // Counter attack chance: 10 - (spell_level + 6)
                        // So level 0 = 40% chance, level 3 = 10% chance
                        let spell_level = counter_buff.values.get(0).copied().unwrap_or(0);
                        let chance_to_counter = 10 - (spell_level + 6);
                        
                        if chance_to_counter > 0 {
                            let mut rng = thread_rng();
                            if rng.gen_range(0..10) < chance_to_counter {
                                // Perform counter attack
                                self.perform_counter_attack(
                                    target_session_id,
                                    attacker_sid,
                                    map_index,
                                    events,
                                );
                                
                                // Remove the CounterAttack buff after use
                                self.remove_player_buff(target_session_id, BuffType::CounterAttack);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Determine whether `attacker_sid` is allowed to attack `target_sid`
    /// according to the C# PlayerObject.IsAttackTarget(HumanObject attacker)
    /// rules. This inspects map NoFight, SafeZone membership and the
    /// attacker's AttackMode, together with party and guild membership and
    /// the target's PK status (red/brown).
    pub(crate) fn can_attack_player(&self, attacker_sid: SessionId, target_sid: SessionId) -> bool {
        if attacker_sid == target_sid {
            return false;
        }

        let attacker = match self.players.get(&attacker_sid) {
            Some(p) => p,
            None => return false,
        };
        let target = match self.players.get(&target_sid) {
            Some(p) => p,
            None => return false,
        };

        if attacker.dead || target.dead || attacker.hp <= 0 || target.hp <= 0 {
            return false;
        }

        if attacker.map_index != target.map_index {
            return false;
        }
        let map_index = attacker.map_index;

        let map_info = match self.provider.get_map_info(map_index) {
            Some(info) => info,
            None => return false,
        };

        // Mirror C# CurrentMap.Info.NoFight and InSafeZone checks.
        if map_info.no_fight {
            return false;
        }

        let attacker_in_safe = Self::point_in_safe_zone(map_info, attacker.x, attacker.y);
        let target_in_safe = Self::point_in_safe_zone(map_info, target.x, target.y);
        if attacker_in_safe || target_in_safe {
            return false;
        }

        let mode = AttackMode::from_u8(attacker.attack_mode);

        match mode {
            AttackMode::Peace => false,
            AttackMode::All => true,
            AttackMode::Group => {
                // C#: Group => GroupMembers == null || !GroupMembers.Contains(attacker)
                // Here: only disallow when both are in the same party.
                if let (Some(a_pid), Some(t_pid)) = (attacker.party_id, target.party_id) {
                    a_pid != t_pid
                } else {
                    true
                }
            }
            AttackMode::Guild => {
                // C#: Guild => MyGuild == null || MyGuild != attacker.MyGuild
                // i.e. cannot attack same-guild members when both have guilds.
                if target.guild_name.is_empty() || attacker.guild_name.is_empty() {
                    true
                } else {
                    attacker.guild_name != target.guild_name
                }
            }
            AttackMode::EnemyGuild => {
                // C#: EnemyGuild => MyGuild != null && MyGuild.IsEnemy(attacker.MyGuild)
                // Until full guild war state is implemented, approximate this
                // as: both have non-empty, different guild names.
                if target.guild_name.is_empty() || attacker.guild_name.is_empty() {
                    false
                } else {
                    attacker.guild_name != target.guild_name
                }
            }
            AttackMode::RedBrown => {
                // C#: RedBrown => PKPoints >= 200 || Envir.Time < BrownTime
                target.pk_points >= 200 || self.time_ms < target.brown_time_ms
            }
        }
    }

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

    /// Determine whether `attacker_sid` is allowed to attack the specified
    /// monster according to the C# MonsterObject.IsAttackTarget(HumanObject
    /// attacker) rules. This primarily affects pets (monsters with a player
    /// owner) and mirrors the interaction with AttackMode/party/guild/PK
    /// state, while leaving wild monsters unrestricted by AttackMode.
    pub(crate) fn can_attack_monster(
        &self,
        attacker_sid: SessionId,
        map_index: i32,
        monster_id: u64,
    ) -> bool {
        let attacker = match self.players.get(&attacker_sid) {
            Some(p) => p,
            None => return false,
        };

        let monsters = match self.monsters.get(&map_index) {
            Some(ms) => ms,
            None => return false,
        };

        let monster = match monsters.iter().find(|m| m.id == monster_id && m.hp > 0) {
            Some(m) => m,
            None => return false,
        };

        // Wild monsters (no owner) are always valid targets regardless of
        // AttackMode; map-level NoFight/SafeZone rules are handled elsewhere.
        let owner_sid = match monster.owner_session_id {
            Some(sid) => sid,
            None => return true,
        };

        let owner = match self.players.get(&owner_sid) {
            Some(p) => p,
            // If the owner is missing from world state, fall back to treating
            // this as a wild monster.
            None => return true,
        };

        let mode = AttackMode::from_u8(attacker.attack_mode);

        // C#: if (attacker.AMode == AttackMode.Peace) return false; (for pets)
        if mode == AttackMode::Peace {
            return false;
        }

        // C#: if (Master == attacker) return attacker.AMode == AttackMode.All;
        if owner_sid == attacker_sid {
            return mode == AttackMode::All;
        }

        // Approximate the C# safe-zone rule for pets:
        // if (Master.Race == ObjectType.Player && (attacker.InSafeZone || InSafeZone)) return false;
        if let Some(map_info) = self.provider.get_map_info(map_index) {
            let attacker_in_safe =
                Self::point_in_safe_zone(map_info, attacker.x, attacker.y);
            let monster_in_safe =
                Self::point_in_safe_zone(map_info, monster.x, monster.y);
            if attacker_in_safe || monster_in_safe {
                return false;
            }
        }

        match mode {
            AttackMode::All => true,
            AttackMode::Group => {
                // C#: Group => Master.GroupMembers == null || !Master.GroupMembers.Contains(attacker)
                // Approximate via shared PartyId: disallow when both are in
                // the same party.
                if let (Some(a_pid), Some(o_pid)) = (attacker.party_id, owner.party_id) {
                    a_pid != o_pid
                } else {
                    true
                }
            }
            AttackMode::Guild => {
                // C#: Guild => master.MyGuild == null || master.MyGuild != attacker.MyGuild
                if owner.guild_name.is_empty() || attacker.guild_name.is_empty() {
                    true
                } else {
                    owner.guild_name != attacker.guild_name
                }
            }
            AttackMode::EnemyGuild => {
                // C#: EnemyGuild => master.MyGuild != null && attacker.MyGuild != null && master.MyGuild.IsEnemy(attacker.MyGuild)
                // Approximate as: both have non-empty, different guild names.
                if owner.guild_name.is_empty() || attacker.guild_name.is_empty() {
                    false
                } else {
                    owner.guild_name != attacker.guild_name
                }
            }
            AttackMode::RedBrown => {
                // C#: RedBrown => Master.PKPoints >= 200 || Envir.Time < Master.BrownTime
                owner.pk_points >= 200 || self.time_ms < owner.brown_time_ms
            }
            AttackMode::Peace => false, // already handled above
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
        }

        if !allow_pk_points || damage <= 0 || !dead {
            return;
        }

        // PK/BrownTime logic: mirror existing melee path.
        if let Some(t) = self.players.get_mut(&target_sid) {
            t.brown_time_ms = self.time_ms;
        }

        let mut murder = false;

        if let Some(map_info) = self.provider.get_map_info(map_index) {
            if !map_info.fight {
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
            self.apply_weapon_luck_curse(attacker_sid, events);
        }
    }

    /// When a player commits murder (gains PK points by killing a normal
    /// player), apply a chance for their weapon Luck to decrease, mirroring
    /// the C# logic:
    ///
    /// if (weapon != null && weapon.AddedStats[Stat.Luck] > (Settings.MaxLuck * -1)
    ///     && Envir.Random.Next(4) == 0) { weapon.AddedStats[Stat.Luck]--; }
    fn apply_weapon_luck_curse(
        &mut self,
        attacker_sid: SessionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let cfg = setup_config();
        let max_luck_cfg: i32 = cfg.items.max_luck.max(1).into();
        let min_luck = -max_luck_cfg;

        let mut rng = thread_rng();
        if rng.gen_range(0..4) != 0 {
            return;
        }

        {
            let attacker = match self.players.get_mut(&attacker_sid) {
                Some(p) => p,
                None => return,
            };

            // EquipmentSlot.Weapon is index 0 in the legacy C# enums.
            let weapon_slot = 0usize;
            let slot_opt = match attacker.equipment.slots.get_mut(weapon_slot) {
                Some(s) => s,
                None => return,
            };

            let weapon = match slot_opt.as_mut() {
                Some(w) => w,
                None => return,
            };

            let luck_stat_id = Stat::Luck as u8;
            let mut current_luck: i32 = 0;
            let mut luck_index: Option<usize> = None;
            for (idx, (sid, val)) in weapon.added_stats.entries.iter().enumerate() {
                if *sid == luck_stat_id {
                    current_luck = *val;
                    luck_index = Some(idx);
                    break;
                }
            }

            if current_luck <= min_luck {
                return;
            }

            let new_luck = current_luck.saturating_sub(1);
            match luck_index {
                Some(i) => weapon.added_stats.entries[i].1 = new_luck,
                None => weapon
                    .added_stats
                    .entries
                    .push((luck_stat_id, new_luck)),
            }
        }

        // Recalculate equipment stats so that the Luck change takes effect for
        // subsequent attacks, and notify the player via a system message.
        self.recalc_player_equipment_stats(attacker_sid);
        events.push(WorldEvent::PartySystemMessage {
            session_id: attacker_sid,
            message: "Your weapon has been cursed.".to_string(),
        });
    }

    #[allow(dead_code)]
    fn compute_physical_damage_base(player_level: u16) -> i32 {
        let lvl = player_level.max(1) as i32;
        let min_dc = 1 + lvl / 2;
        let max_dc = 2 + lvl;
        if max_dc <= min_dc {
            min_dc.max(1)
        } else {
            (min_dc + max_dc) / 2
        }
    }

    pub(super) fn handle_magic_command(
        &mut self,
        session_id: SessionId,
        spell: u8,
        direction: u8,
        target_id: u32,
        x: i32,
        y: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let now_ms = self.time_ms;
        let (player_map, player_x, player_y, has_magic) = match self.players.get(&session_id) {
            Some(p) => {
                let has_magic = p.magics.iter().any(|m| m.spell == spell);
                (p.map_index, p.x, p.y, has_magic)
            }
            None => return,
        };

        // Mirror C# CanCast gating which includes SpellTime (global magic delay)
        // plus ActionTime and Stun/Dazed/Paralysis/Frozen restrictions.
        if let Some(p) = self.players.get(&session_id) {
            if p.dead {
                return;
            }
            if p.next_action_time_ms != 0 && now_ms < p.next_action_time_ms {
                return;
            }
            if p.next_spell_time_ms != 0 && now_ms < p.next_spell_time_ms {
                tracing::debug!(
                    "handle_magic_command: session_id={} spell_id={} ignored (spell_time) now={} next_spell_time_ms={}",
                    session_id,
                    spell,
                    now_ms,
                    p.next_spell_time_ms
                );
                return;
            }
            let poisoned = p.current_poison_mask;
            if (poisoned & (crate::world::types::PoisonType::Stun as u16) != 0)
                || (poisoned & (crate::world::types::PoisonType::Dazed as u16) != 0)
                || (poisoned & (crate::world::types::PoisonType::Paralysis as u16) != 0)
                || (poisoned & (crate::world::types::PoisonType::Frozen as u16) != 0)
            {
                return;
            }
        }

        // If the player knows this magic, enforce a simple cooldown based on
        // the MagicInfo delay parameters and the per-magic UserMagic.cast_time
        // field. This mirrors the C# behaviour where repeated CMagic packets
        // while a spell is still on cooldown are ignored server-side.
        //
        // Note: we do NOT emit MagicCast here. In the original C# server,
        // S.MagicCast is only used for certain skills (e.g. ShoulderDash,
        // FlashDash, BackStep) that return early from Magic() and therefore do
        // not send S.Magic. For normal spells, the client receives S.Magic.
        if has_magic {
            // Mirror C# Magic(): after CanCast passes, arm SpellTime immediately
            // (and ActionTime for most spells) even if the cast later aborts
            // due to per-spell cooldown or insufficient MP.
            let mut spell_delay_ms: i64 = 1_800;
            if Spell::from_u8(spell) == Some(Spell::FlameField) {
                spell_delay_ms = 2_500;
            }

            if let Some(p) = self.players.get_mut(&session_id) {
                p.next_spell_time_ms = now_ms.saturating_add(spell_delay_ms);

                if Spell::from_u8(spell) != Some(Spell::ShoulderDash) {
                    let mut action_delay_ms: i64 = 600;
                    if (p.current_poison_mask
                        & (crate::world::types::PoisonType::Slow as u16))
                        != 0
                    {
                        action_delay_ms = action_delay_ms.saturating_mul(2);
                    }
                    p.next_action_time_ms = now_ms.saturating_add(action_delay_ms);
                }
            }

            if !self.check_and_update_magic_cooldown(session_id, spell) {
                tracing::debug!(
                    "handle_magic_command: session_id={} spell_id={} ignored (on cooldown)",
                    session_id,
                    spell,
                );
                return;
            }

            if let Some(p) = self.players.get(&session_id) {
                tracing::debug!(
                    "handle_magic_command: session_id={} job={:?} spell_id={} accepted",
                    session_id,
                    p.job,
                    spell,
                );
            }
        }

        if spell == Spell::FlamingSword as u8 {
            cast_flaming_sword(self, session_id, events);
            return;
        }

        if spell == Spell::Rage as u8 {
            cast_rage(self, session_id, events);
            return;
        }

        if spell == Spell::Blink as u8 {
            cast_blink(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::Teleport as u8 {
            cast_teleport(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::Fury as u8 {
            cast_fury(self, session_id, events);
            return;
        }

        if spell == Spell::ImmortalSkin as u8 {
            cast_immortal_skin(self, session_id, events);
            return;
        }

        if spell == Spell::ShoulderDash as u8 {
            cast_shoulder_dash(self, session_id, direction, events);
            return;
        }

        if spell == Spell::CounterAttack as u8 {
            cast_counter_attack(self, session_id, events);
            return;
        }

        if spell == Spell::Haste as u8 {
            cast_haste(self, session_id, events);
            return;
        }

        if spell == Spell::SwiftFeet as u8 {
            cast_swift_feet(self, session_id, events);
            return;
        }

        if spell == Spell::FireBang as u8 || spell == Spell::IceStorm as u8 {
            cast_fire_bang_ice_storm(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::FireWall as u8 {
            cast_fire_wall(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::ThunderStorm as u8 || spell == Spell::FlameField as u8 {
            cast_thunder_storm_flame_field(self, session_id, spell, direction, events);
            return;
        }

        if spell == Spell::StormEscape as u8 {
            cast_thunder_storm_flame_field(self, session_id, spell, direction, events);
            cast_storm_escape(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::Lightning as u8 {
            cast_lightning(self, session_id, spell, direction, events);
            return;
        }

        if spell == Spell::Blizzard as u8 {
            cast_blizzard(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::HellFire as u8 {
            cast_hell_fire(self, session_id, spell, direction, events);
            return;
        }

        if is_pure_magic_attack(spell) {
            // Route wizard single-target attack spells (FireBall, GreatFireBall,
            // ThunderBolt, SoulFireBall, etc.) through the unified attack
            // pipeline so that MP cost, damage, skill training and visual
            // effects are handled consistently. For these spells we honour the
            // client-provided target first, falling back to directional scan
            // when necessary.
            self.handle_attack_command(
                session_id,
                direction,
                spell,
                target_id,
                x,
                y,
                events,
            );
            return;
        }

        if spell == Spell::MagicShield as u8 {
            cast_magic_shield(self, session_id, events);
            return;
        }

        if spell == Spell::SoulShield as u8 {
            cast_soul_shield(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::BlessedArmour as u8 {
            cast_blessed_armour(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::Hiding as u8 {
            cast_hiding(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::MassHiding as u8 {
            cast_mass_hiding(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::EnergyShield as u8 {
            cast_energy_shield(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::UltimateEnhancer as u8 {
            cast_ultimate_enhancer(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::Healing as u8 {
            cast_healing(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::MassHealing as u8 {
            cast_mass_healing(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::Poisoning as u8 {
            cast_poisoning(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::Hallucination as u8 {
            cast_hallucination(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::SummonSkeleton as u8 {
            cast_summon_skeleton(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::SummonShinsu as u8 {
            cast_summon_shinsu(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::Reincarnation as u8 {
            cast_reincarnation(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::SummonHolyDeva as u8 {
            cast_summon_holy_deva(self, session_id, spell, direction, x, y, events);
            return;
        }

        // TODO: Handle other spells.
        // For now, just emit a visual effect to show something happened.
        events.push(WorldEvent::ObjectAttack {
            session_id,
            map_index: player_map,
            x: player_x,
            y: player_y,
            direction,
            spell,
            level: 0,
            attack_type: 0,
        });
    }

    

    pub(super) fn handle_attack_command(
        &mut self,
        session_id: SessionId,
        direction: u8,
        spell: u8,
        packet_target_id: u32,
        packet_target_x: i32,
        packet_target_y: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let (
            map_index,
            x,
            y,
            direction,
            effective_spell,
            level,
            fatal_level,
            attacker_stats,
            flaming_sword_trigger,
            slaying_toggled_on,
            moon_dark_spell,
            moon_dark_level,
        ) =
            match self.players.get_mut(&session_id) {
                Some(p) => {
                    if !Self::can_attack(p) {
                        return;
                    }

                    p.direction = direction;

                    // Check for FlamingSword buff logic BEFORE resolving spell.
                    let mut spell_to_use = spell;
                    let mut consumed_flaming_sword = false;
                    let mut slaying_toggled_on = false;

                    // Mirror the C# HumanObject.Attack behaviour for
                    // MoonLight/DarkBody via the assassin-specific helper: if
                    // the player is emerging from a hidden state with one of
                    // these buffs active, the very next physical attack
                    // receives a DamageBase bonus equal to magic.GetPower()
                    // for the corresponding spell. We approximate Hidden by
                    // the presence of the buff itself and remove the buffs
                    // immediately so the bonus is one-shot.
                    let (moon_dark_spell, moon_dark_level) =
                        resolve_moon_dark_opening(p);

                    // If it's a basic attack (spell 0) and we have FlamingSword buff
                    // active, treat this swing as FlamingSword and consume the buff.
                    if spell == 0 {
                        if let Some(idx) = p
                            .active_buffs
                            .iter()
                            .position(|b| b.buff_type == BuffType::FlamingSword)
                        {
                            p.active_buffs.remove(idx);
                            spell_to_use = Spell::FlamingSword as u8;
                            consumed_flaming_sword = true;
                        }
                    }

                    // Resolve the effective attack spell and its learned level.
                    let (mut effective_spell, mut level) =
                        resolve_attack_spell_and_level_for_player(p, spell_to_use);

                    // Mirror the C# HumanObject.Attack MP gating for certain
                    // warrior attack skills that are driven from the melee
                    // attack loop rather than the magic command path.
                    if level > 0 {
                        if let Some(spell_enum) = Spell::from_u8(effective_spell) {
                            use Spell as S;
                            match spell_enum {
                                // These skills consume MP per attack when used
                                // as part of the melee flow. If there is not
                                // enough MP, the swing still happens but
                                // falls back to a plain physical attack.
                                S::DoubleSlash | S::HalfMoon | S::CrossHalfMoon | S::TwinDrakeBlade => {
                                    if let Some(cost) = compute_magic_mana_cost(
                                        &self.provider,
                                        &p.stats.total,
                                        effective_spell,
                                        level,
                                    ) {
                                        if p.mp < cost {
                                            // Not enough MP: downgrade to
                                            // plain melee for this swing.
                                            effective_spell = 0;
                                            level = 0;
                                        } else {
                                            p.mp -= cost;
                                        }
                                    } else {
                                        // No MagicInfo entry; treat as plain
                                        // melee to avoid inconsistent state.
                                        effective_spell = 0;
                                        level = 0;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    // Handle Slaying activation/consumption and random charge
                    // via the warrior-specific helper, mirroring the C#
                    // HumanObject.Attack flow.
                    resolve_slaying_for_attack(
                        p,
                        &mut effective_spell,
                        &mut level,
                        &mut slaying_toggled_on,
                    );

                    let fatal_level = fatal_sword_level_for_player(p);
                    let attacker_stats = p.stats.total.clone();

                    (
                        p.map_index,
                        p.x,
                        p.y,
                        p.direction,
                        effective_spell,
                        level,
                        fatal_level,
                        attacker_stats,
                        consumed_flaming_sword,
                        slaying_toggled_on,
                        moon_dark_spell,
                        moon_dark_level,
                    )
                }
                None => {
                    return;
                }
            };

        if flaming_sword_trigger {
             events.push(WorldEvent::SpellToggle {
                session_id,
                spell_id: Spell::FlamingSword as u8,
                enabled: false,
            });
        }

        if slaying_toggled_on {
            events.push(WorldEvent::SpellToggle {
                session_id,
                spell_id: Spell::Slaying as u8,
                enabled: true,
            });
        }

        let is_half_moon = effective_spell == Spell::HalfMoon as u8;
        let is_cross_half_moon = effective_spell == Spell::CrossHalfMoon as u8;

        if is_half_moon {
            // Train HalfMoon when the attack is actually executed.
            self.level_up_magic_for_player(session_id, Spell::HalfMoon as u8, events);

            cast_half_moon(
                self,
                session_id,
                map_index,
                x,
                y,
                direction,
                level,
                fatal_level,
                &attacker_stats,
                events,
            );

            events.push(WorldEvent::UserLocation {
                session_id,
                map_index,
                x,
                y,
                direction,
            });

            events.push(WorldEvent::ObjectAttack {
                session_id,
                map_index,
                x,
                y,
                direction,
                spell: effective_spell,
                level,
                attack_type: 0,
            });

            return;
        }

        if is_cross_half_moon {
            // Train CrossHalfMoon when the attack is actually executed.
            self.level_up_magic_for_player(session_id, Spell::CrossHalfMoon as u8, events);

            cast_cross_half_moon(
                self,
                session_id,
                map_index,
                x,
                y,
                direction,
                level,
                fatal_level,
                &attacker_stats,
                events,
            );

            events.push(WorldEvent::UserLocation {
                session_id,
                map_index,
                x,
                y,
                direction,
            });

            events.push(WorldEvent::ObjectAttack {
                session_id,
                map_index,
                x,
                y,
                direction,
                spell: effective_spell,
                level,
                attack_type: 0,
            });

            return;
        }

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

        // Default melee target: one tile in front of the player.
        let mut target_x = x + dx;
        let mut target_y = y + dy;

        // First try to hit a player standing on the front tile before
        // looking for monsters. This keeps PVP path separate from the
        // monster-targeting logic below and mirrors C# HumanObject.Attack
        // semantics, where player targets are resolved first.
        if !is_pure_magic_attack(effective_spell) {
            let mut player_target: Option<SessionId> = None;
            for (&sid, p) in &self.players {
                if sid == session_id {
                    continue;
                }
                if p.map_index != map_index || p.x != target_x || p.y != target_y {
                    continue;
                }
                if self.can_attack_player(session_id, sid) {
                    player_target = Some(sid);
                    break;
                }
            }

            if let Some(target_session_id) = player_target {
                // Snapshot defender stats for damage calculation.
                let defender_stats = {
                    if let Some(t) = self.players.get(&target_session_id) {
                        let mut s = t.stats.total.clone();

                        // Apply red-poison armour reduction for players by scaling
                        // MinAC/MaxAC when the defender currently has Red poison
                        // active, approximating C# HumanObject.ArmourRate.
                        let red_mask = PoisonType::Red.as_u16();
                        if (t.current_poison_mask & red_mask) != 0 {
                            let percent: i32 = 90;
                            let min_ac = s.get(Stat::MinAC);
                            let max_ac = s.get(Stat::MaxAC);
                            let scaled_min = (min_ac as i64 * percent as i64 / 100) as i32;
                            let scaled_max = (max_ac as i64 * percent as i64 / 100) as i32;
                            s.set(Stat::MinAC, scaled_min);
                            s.set(Stat::MaxAC, scaled_max);
                        }

                        s
                    } else {
                        return;
                    }
                };

                let (hit, mut raw_damage, damage_type) =
                    compute_physical_melee_with_crit(&attacker_stats, &defender_stats);

                if hit && raw_damage > 0 {
                    // Apply MoonLight/DarkBody opening bonus if present.
                    if moon_dark_spell != 0 && moon_dark_level > 0 {
                        if let Some(info) = self.provider.get_magic_info(moon_dark_spell) {
                            let mut rng = thread_rng();
                            let bonus = magic_power(info, moon_dark_level, &mut rng);
                            if bonus > 0 {
                                raw_damage = raw_damage.saturating_add(bonus);
                            }
                        }
                    }

                    // Apply active attack spell scaling except for the
                    // Thrusting extended tile case (not used for adjacent
                    // player hits here).
                    if !is_thrusting_spell(effective_spell) {
                        raw_damage = apply_attack_spell_scaling(
                            &self.provider,
                            effective_spell,
                            level,
                            raw_damage,
                        );
                    }
                }

                // Apply passive FatalSword and undead tweaks; players are not
                // undead so we always pass false for undead.
                raw_damage = apply_fatal_sword_and_undead(
                    &self.provider,
                    fatal_level,
                    false,
                    raw_damage,
                );

                // Approximate multi-hit skills by doubling final damage.
                if effective_spell == Spell::DoubleSlash as u8
                    || effective_spell == Spell::TwinDrakeBlade as u8
                {
                    raw_damage = raw_damage.saturating_mul(2);
                }
                let damage_to_apply = if hit && raw_damage > 0 {
                    raw_damage
                } else {
                    0
                };

                self.apply_player_hit_from_player(
                    session_id,
                    target_session_id,
                    map_index,
                    damage_to_apply,
                    damage_type,
                    None,
                    None,
                    None,
                    true,
                    events,
                );

                // Emit the usual attack animation and position events for the
                // attacker, then stop; monster handling below is skipped.
                events.push(WorldEvent::UserLocation {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                });

                events.push(WorldEvent::ObjectAttack {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                    spell: effective_spell,
                    level,
                    attack_type: 0,
                });

                return;
            }
        }

        let target_info = match self.monsters.get(&map_index) {
            Some(monsters) => {
                if is_pure_magic_attack(effective_spell) {
                    // For pure magic attack spells (e.g. FireBall, SoulFireBall,
                    // ThunderBolt), first try to honour the explicit
                    // client-provided target (ID / location) and only fall
                    // back to a directional scan when no suitable monster is
                    // found. This mirrors the C# HumanObject.Fireball /
                    // ThunderBolt behaviour where Magic.Target is preferred
                    // over nearest-in-front.
                    let max_range: i32 = self
                        .provider
                        .get_magic_info(effective_spell)
                        .map(|info| info.range as i32)
                        .unwrap_or_else(|| {
                            // When there is no MagicInfo entry (e.g. an
                            // incomplete MagicInfoList in the DB), fall back
                            // to a reasonable default range so that pure
                            // magic spells like SoulFireBall can still reach
                            // distant monsters and produce ObjectMagic/
                            // damage events instead of silently failing.
                            if effective_spell == Spell::SoulFireBall as u8 {
                                9
                            } else {
                                6
                            }
                        })
                        .max(1);

                    let mut found: Option<(u64, i32, i32, i32)> = None;

                    // 1) Try explicit monster ID from the packet when present.
                    if packet_target_id != 0 {
                        if let Some(m) = monsters
                            .iter()
                            .find(|m| {
                                m.hp > 0
                                    && m.id == packet_target_id as u64
                                    && self.can_attack_monster(session_id, map_index, m.id)
                            })
                        {
                            let dx_t = m.x - x;
                            let dy_t = m.y - y;
                            if dx_t.abs().max(dy_t.abs()) <= max_range {
                                found = Some((m.id, m.monster_index, m.x, m.y));
                            }
                        }
                    }

                    // 2) If no ID match, try the explicit location from the
                    // packet to pick a monster standing exactly on that tile.
                    if found.is_none() && (packet_target_x != 0 || packet_target_y != 0) {
                        if let Some(m) = monsters
                            .iter()
                            .find(|m| {
                                m.hp > 0
                                    && m.x == packet_target_x
                                    && m.y == packet_target_y
                                    && self.can_attack_monster(session_id, map_index, m.id)
                            })
                        {
                            let dx_t = m.x - x;
                            let dy_t = m.y - y;
                            if dx_t.abs().max(dy_t.abs()) <= max_range {
                                found = Some((m.id, m.monster_index, m.x, m.y));
                            }
                        }
                    }

                    // 3) Fallback: scan forward along the attack direction up
                    // to the spell range and select the first monster
                    // encountered, preserving the original behaviour when no
                    // explicit target is usable.
                    if found.is_none() {
                        for step in 1..=max_range {
                            let tx = x + dx * step;
                            let ty = y + dy * step;
                            if let Some(m) = monsters
                                .iter()
                                .find(|m| {
                                    m.hp > 0
                                        && m.x == tx
                                        && m.y == ty
                                        && self.can_attack_monster(session_id, map_index, m.id)
                                })
                            {
                                found = Some((m.id, m.monster_index, tx, ty));
                                break;
                            }
                        }
                    }

                    if let Some((id, monster_index, fx, fy)) = found {
                        target_x = fx;
                        target_y = fy;
                        Some((id, monster_index))
                    } else {
                        None
                    }
                } else if is_thrusting_spell(effective_spell) {
                    // Thrusting extends melee range in a straight line while
                    // still using the physical melee model for damage.
                    let max_range: i32 = thrusting_max_range(&self.provider, level).max(1);

                    let mut found: Option<(u64, i32, i32, i32)> = None;
                    for step in 1..=max_range {
                        let tx = x + dx * step;
                        let ty = y + dy * step;
                        if let Some(m) = monsters
                            .iter()
                            .find(|m| {
                                m.hp > 0
                                    && m.x == tx
                                    && m.y == ty
                                    && self.can_attack_monster(session_id, map_index, m.id)
                            })
                        {
                            found = Some((m.id, m.monster_index, tx, ty));
                            break;
                        }
                    }

                    if let Some((id, monster_index, fx, fy)) = found {
                        target_x = fx;
                        target_y = fy;
                        Some((id, monster_index))
                    } else {
                        None
                    }
                } else {
                    monsters
                        .iter()
                        .find(|m| {
                            m.hp > 0
                                && m.x == target_x
                                && m.y == target_y
                                && self.can_attack_monster(session_id, map_index, m.id)
                        })
                        .map(|m| (m.id, m.monster_index))
                }
            }
            None => None,
        };
        if let Some((id, monster_index)) = target_info {
            // Update pet focus target when the player is using
            // FocusMasterTarget pet mode so that pets mirror the C# behaviour
            // of concentrating on the master's current target.
            self.maybe_update_pet_focus_target_for_player_on_monster(session_id, id);
            let mut dead = false;

            // For Thrusting we need to distinguish between a normal adjacent
            // melee hit (front tile) and the extended range tile used by the
            // C# HumanObject "Thrusting" label. In C#, only the extended
            // tile applies the Thrusting magic damage multiplier, while a
            // target directly in front uses plain physical damage.
            let mut use_thrusting_scaling = true;
            if is_thrusting_spell(effective_spell) {
                let front_x = x + dx;
                let front_y = y + dy;
                if target_x == front_x && target_y == front_y {
                    use_thrusting_scaling = false;
                }
            }

            // Train attack-type magic when a valid target is acquired and the
            // effective spell is non-zero (i.e. a learned skill rather than a
            // plain melee swing).
            if effective_spell != 0 {
                self.level_up_magic_for_player(session_id, effective_spell, events);
            }

            let (monster_exp, undead, max_hp, defender_stats, monster_drops): (
                u32,
                bool,
                i32,
                Stats,
                Vec<crate::world::drop::DropInfo>,
            ) = if let Some(info) = self.provider.get_monster_info(monster_index) {
                let max_hp = info.stats.get(Stat::HP).max(1);

                // Base defender stats from MonsterInfo plus any active
                // per-instance buff_stats on this specific monster instance,
                // mirroring C# MonsterObject.RefreshBuffs where Buff.Stats are
                // added on top of base stats.
                let mut defender_stats = info.stats.clone();
                if let Some(monsters) = self.monsters.get(&map_index) {
                    if let Some(m) = monsters.iter().find(|m| m.id == id) {
                        defender_stats.add(&m.buff_stats);

                        // Apply red-poison armour reduction for monsters by scaling
                        // MinAC/MaxAC when the defender currently has Red poison
                        // active, approximating C# MonsterObject.ArmourRate.
                        let red_mask = PoisonType::Red.as_u16();
                        if (m.current_poison_mask & red_mask) != 0 {
                            let percent: i32 = 50;
                            let min_ac = defender_stats.get(Stat::MinAC);
                            let max_ac = defender_stats.get(Stat::MaxAC);
                            let scaled_min = (min_ac as i64 * percent as i64 / 100) as i32;
                            let scaled_max = (max_ac as i64 * percent as i64 / 100) as i32;
                            defender_stats.set(Stat::MinAC, scaled_min);
                            defender_stats.set(Stat::MaxAC, scaled_max);
                        }
                    }
                }

                (
                    info.experience,
                    info.undead,
                    max_hp,
                    defender_stats,
                    info.drops.clone(),
                )
            } else {
                (0, false, 1, Stats::default(), Vec::new())
            };

            // Use the unified C#-style physical melee model (including
            // Accuracy/Agility, AC/DR and crit) for player -> monster hits for
            // most attacks. Pure magic spells such as FireBall and
            // SoulFireBall use their MagicInfo parameters directly instead of
            // relying on the melee helper for base damage.
            let use_pure_magic = is_pure_magic_attack(effective_spell);

            // For pure magic attacks, also emit an ObjectMagic-style world
            // event so that the connection layer can send SObjectMagic and
            // drive projectile animations and spell visuals on the client.
            if use_pure_magic {
                events.push(WorldEvent::ObjectMagic {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                    spell: effective_spell,
                    level,
                    target_id: id as u32,
                    target_x,
                    target_y,
                });

                // And notify the caster about the cast using a Magic event,
                // mirroring C# S.Magic. The legacy client uses this to set
                // User.Spell/User.Cast/TargetID/TargetPoint and then, when the
                // MirAction.Spell animation completes, spawn the correct
                // projectile and hit effects (e.g. FireBall explosion,
                // ThunderBolt lightning on the target).
                events.push(WorldEvent::Magic {
                    session_id,
                    spell_id: effective_spell,
                    target_id: id as u32,
                    x: target_x,
                    y: target_y,
                    cast: true,
                    level,
                    secondary_target_ids: Vec::new(),
                });
            }

            if use_pure_magic {
                // Apply MP cost for pure magic attack spells (FireBall,
                // SoulFireBall). If there is no MagicInfo entry, treat the
                // cost as zero rather than aborting the spell so that the
                // cast still animates and can deal damage using the fallback
                // path in compute_pure_magic_attack_damage.
                let cost = compute_magic_mana_cost(
                    &self.provider,
                    &attacker_stats,
                    effective_spell,
                    level,
                )
                .unwrap_or(0);

                if let Some(player) = self.players.get_mut(&session_id) {
                    if player.mp < cost {
                        return;
                    }
                    if cost > 0 {
                        player.mp -= cost;
                    }
                } else {
                    return;
                }
            }

            let (hit, mut raw_damage, damage_type) = if use_pure_magic {
                let dmg = compute_pure_magic_attack_damage(
                    &self.provider,
                    &attacker_stats,
                    effective_spell,
                    level,
                );
                if dmg > 0 {
                    (true, dmg, 0)
                } else {
                    (false, 0, 1)
                }
            } else {
                compute_physical_melee_with_crit(&attacker_stats, &defender_stats)
            };

            if hit && raw_damage > 0 {
                if use_pure_magic {
                    // ThunderBolt deals 1.5x damage to undead targets, mirroring
                    // the C# HumanObject.ThunderBolt implementation.
                    if effective_spell == Spell::ThunderBolt as u8 && undead {
                        let scaled = (raw_damage as f32 * 1.5) as i32;
                        raw_damage = scaled.max(1);
                    }

                    // FlameDisruptor deals 1.5x damage to non-undead targets,
                    // matching the C# HumanObject.FlameDisruptor behaviour
                    // where damage is scaled up when target.Undead == false.
                    if effective_spell == Spell::FlameDisruptor as u8 && !undead {
                        let scaled = (raw_damage as f32 * 1.5) as i32;
                        raw_damage = scaled.max(1);
                    }
                } else {
                    // Apply MoonLight / DarkBody opening strike bonus before
                    // any active attack spell scaling.
                    raw_damage = apply_moon_dark_bonus(
                        &self.provider,
                        moon_dark_spell,
                        moon_dark_level,
                        raw_damage,
                    );

                    // First apply the active attack spell (if any) using the
                    // MagicInfo parameters for that spell and the learned
                    // level. For Thrusting this multiplier should only be
                    // applied when the extended range tile is hit; an
                    // adjacent (front) target uses plain melee damage, as in
                    // the C# HumanObject.Attack/Thrusting flow.
                    if !is_thrusting_spell(effective_spell) || use_thrusting_scaling {
                        raw_damage = apply_attack_spell_scaling(
                            &self.provider,
                            effective_spell,
                            level,
                            raw_damage,
                        );
                    }
                }
            }

            // Then apply any passive FatalSword and undead-specific
            // tweaks via the shared skills helper.
            raw_damage = apply_fatal_sword_and_undead(
                &self.provider,
                fatal_level,
                undead,
                raw_damage,
            );

            // For TwinDrakeBlade, schedule a second delayed hit against the
            // same monster using the PendingMagicHit pipeline so that the
            // total damage is delivered as two separate strikes, mirroring the
            // C# HumanObject TwinDrakeBlade behaviour.
            if effective_spell == Spell::TwinDrakeBlade as u8 && hit && raw_damage > 0 {
                let delay_ms: i64 = 400;
                let base_time = self.time_ms.max(0);
                let due_time_ms = base_time.saturating_add(delay_ms);

                self.pending_magic_hits.push(PendingMagicHit {
                    due_time_ms,
                    attacker_session_id: session_id,
                    map_index,
                    target_monster_id: id,
                    monster_index,
                    spell_id: effective_spell,
                    damage: raw_damage,
                    damage_type,
                });
            }

            // Approximate other multi-hit skills such as DoubleSlash by doubling
            // the final damage, keeping a single hit event while preserving
            // overall DPS.
            if effective_spell == Spell::DoubleSlash as u8 {
                raw_damage = raw_damage.saturating_mul(2);
            }

            if use_pure_magic {
                // For pure magic attacks (FireBall/GreatFireBall/ThunderBolt/SoulFireBall),
                // schedule a delayed hit instead of applying damage
                // immediately so that the damage and visual hit effect
                // (projectile or lightning) stay in sync on the client.
                if hit && raw_damage > 0 {
                    let delay_ms: i64 = if let Some(spell_enum) = Spell::from_u8(effective_spell) {
                        use Spell as S;
                        match spell_enum {
                            // ThunderBolt uses a fixed 500ms delay in C#
                            S::ThunderBolt => 500,
                            // FireBall/GreatFireBall/SoulFireBall use
                            // MaxDistance * 50 + 500ms.
                            S::FireBall | S::GreatFireBall | S::SoulFireBall => {
                                let dx = target_x - x;
                                let dy = target_y - y;
                                let dist = dx.abs().max(dy.abs());
                                (dist as i64) * 50 + 500
                            }
                            _ => 500,
                        }
                    } else {
                        500
                    };

                    let base_time = self.time_ms.max(0);
                    let due_time_ms = base_time.saturating_add(delay_ms);

                    self.pending_magic_hits.push(PendingMagicHit {
                        due_time_ms,
                        attacker_session_id: session_id,
                        map_index,
                        target_monster_id: id,
                        monster_index,
                        spell_id: effective_spell,
                        damage: raw_damage,
                        damage_type,
                    });
                }

                // For pure magic we still emit the usual attacker
                // animation/location events, but skip immediate HP
                // changes and ObjectStruck; those will be handled when
                // the PendingMagicHit fires in World::update.
                events.push(WorldEvent::UserLocation {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                });

                events.push(WorldEvent::ObjectAttack {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                    spell: effective_spell,
                    level,
                    attack_type: 0,
                });

                return;
            }

            let mut strike_x = target_x;
            let mut strike_y = target_y;
            let mut strike_dir = direction;
            let mut damage_done: i32 = 0;
            let mut health_percent: u8 = 100;

            if let Some(monsters) = self.monsters.get_mut(&map_index) {
                if let Some(m) = monsters.iter_mut().find(|m| m.id == id) {
                    m.target_session_id = Some(session_id);
                    m.ai_state = MonsterAiState::Chase;

                    strike_x = m.x;
                    strike_y = m.y;
                    strike_dir = m.direction;

                    let old_hp = m.hp.max(0);
                    let mut new_hp = old_hp;

                    if hit && raw_damage > 0 {
                        damage_done = raw_damage;

                        if raw_damage >= m.hp {
                            m.hp = 0;
                            dead = true;
                        } else {
                            m.hp -= raw_damage;
                        }

                        new_hp = if dead { 0 } else { m.hp.max(0) };
                    }

                    if max_hp > 0 {
                        let pct = (new_hp as i64 * 100 / max_hp as i64)
                            .clamp(0, 100) as u8;
                        health_percent = pct;
                    } else {
                        health_percent = 0;
                    }
                }
            }

            // Emit an ObjectStruck-style event for both hits and misses so the
            // client can render Hit/Miss/Crit indicators. For misses we keep
            // damage at 0 and HP unchanged but pass through the damage_type
            // from the helper (1 = Miss).
            if hit {
                if damage_done > 0 {
                    events.push(WorldEvent::ObjectStruck {
                        attacker_id: session_id,
                        target_id: id,
                        map_index,
                        x: strike_x,
                        y: strike_y,
                        direction: strike_dir,
                        damage: damage_done,
                        damage_type,
                        health_percent,
                    });
                }
            } else {
                events.push(WorldEvent::ObjectStruck {
                    attacker_id: session_id,
                    target_id: id,
                    map_index,
                    x: strike_x,
                    y: strike_y,
                    direction: strike_dir,
                    damage: 0,
                    damage_type,
                    health_percent,
                });
            }

            if dead {
                self.mark_monster_dead(map_index, id);
                let _ = self.apply_quest_kill_for_player(session_id, monster_index);

                if let Some(info) = self.provider.get_monster_info(monster_index) {
                    tracing::trace!(
                        "[drop] monster_index={} name='{}' drop_path='{}' drops_len={}",
                        monster_index,
                        info.name,
                        info.drop_path,
                        monster_drops.len(),
                    );
                }

                if !monster_drops.is_empty() {
                    let item_offset = attacker_stats.get(Stat::ItemDropRatePercent);
                    let gold_offset = attacker_stats.get(Stat::GoldDropRatePercent);
                    let mut rng = thread_rng();
                    let mut total = crate::world::drop::DropRewardInfo {
                        items: Vec::new(),
                        gold: 0,
                    };

                    for d in &monster_drops {
                        // Mirror C# MonsterObject.Drop: quest-required drops
                        // (lines with trailing "Q" in the drop files) are not
                        // placed on the ground as normal map items when a
                        // monster dies. They are handled via quest / harvest
                        // flows instead. Here we skip such entries so that
                        // DeerMeat and similar quest items do not appear as
                        // ordinary drops.
                        if d.quest_required {
                            continue;
                        }

                        if let Some(r) = d.attempt_drop(
                            self.drop_rate,
                            item_offset,
                            gold_offset,
                            &mut rng,
                        ) {
                            total.gold = total.gold.saturating_add(r.gold);
                            if !r.items.is_empty() {
                                total.items.extend(r.items);
                            }
                        }
                    }

                    tracing::debug!(
                        "[drop-total] monster_index={} gold={} items_len={}",
                        monster_index,
                        total.gold,
                        total.items.len(),
                    );

                    if total.gold > 0 || !total.items.is_empty() {
                        // Set expire time for monster drops: default 5 minutes
                        // (300000 ms) after the current world time, mirroring
                        // the behaviour used for player-dropped items in
                        // WorldCommand::DropItem.
                        let item_timeout_ms: i64 = 300_000; // 5 minutes

                        if total.gold > 0 {
                            if let Some((drop_x, drop_y)) =
                                self.find_drop_location(map_index, strike_x, strike_y, 4)
                            {
                                let item_id = self.next_map_item_id;
                                self.next_map_item_id =
                                    self.next_map_item_id.wrapping_add(1);
                                let entry =
                                    self.map_items.entry(map_index).or_default();
                                entry.push(crate::world::map_item::MapItem {
                                    id: item_id,
                                    map_index,
                                    x: drop_x,
                                    y: drop_y,
                                    item_index: None,
                                    gold: total.gold,
                                    count: 0,
                                    item: None,
                                    expire_time_ms: self.time_ms + item_timeout_ms,
                                });

                                events.push(WorldEvent::GoldDropped {
                                    object_id: item_id,
                                    map_index,
                                    x: drop_x,
                                    y: drop_y,
                                    gold: total.gold,
                                });
                            }
                        }

                        for item_index in total.items {
                            if let Some((drop_x, drop_y)) =
                                self.find_drop_location(map_index, strike_x, strike_y, 4)
                            {
                                let item_id = self.next_map_item_id;
                                self.next_map_item_id =
                                    self.next_map_item_id.wrapping_add(1);
                                let entry =
                                    self.map_items.entry(map_index).or_default();
                                entry.push(crate::world::map_item::MapItem {
                                    id: item_id,
                                    map_index,
                                    x: drop_x,
                                    y: drop_y,
                                    item_index: Some(item_index),
                                    gold: 0,
                                    count: 1,
                                    item: None,
                                    expire_time_ms: self.time_ms + item_timeout_ms,
                                });

                                events.push(WorldEvent::ItemDropped {
                                    object_id: item_id,
                                    map_index,
                                    x: drop_x,
                                    y: drop_y,
                                    item_index,
                                    count: 1,
                                });
                            }
                        }
                    }
                }

                events.push(WorldEvent::MonsterDied {
                    object_id: id,
                    map_index,
                    x: strike_x,
                    y: strike_y,
                    direction: strike_dir,
                });

                if monster_exp > 0 {
                    // Apply experience gain (and any resulting level-ups)
                    // through the shared helper so that stats and
                    // rankings stay in sync with the new level/exp.
                    let _ = self.gain_experience_for_session(session_id, monster_exp, events);

                    events.push(WorldEvent::GainExperience {
                        session_id,
                        amount: monster_exp,
                    });
                }
            }
        }

        events.push(WorldEvent::UserLocation {
            session_id,
            map_index,
            x,
            y,
            direction,
        });

        events.push(WorldEvent::ObjectAttack {
            session_id,
            map_index,
            x,
            y,
            direction,
            spell: effective_spell,
            level,
            attack_type: 0,
        });
    }

}

