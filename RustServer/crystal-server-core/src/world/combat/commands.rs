//! Attack command handlers
//! 
//! This module handles player-initiated attack commands:
//! - Range attacks (bows, crossbows)
//! - Melee attacks (swords, daggers, etc.)
//! - Magic attacks (spells)

use rand::{thread_rng, Rng};
use crate::combat::compute_physical_melee_with_crit;
use crate::world::magic::magic_power;
use crate::stats::{Stat, Stats};
use crate::world::monster::MonsterAiState;
use crate::world::provider::WorldProvider;
use crate::world::types::{BuffType, PetMode, PoisonType};
use crate::world::{PendingMagicHit, SessionId, World, WorldEvent, Spell};
use crate::world::combat::can_attack;
use crate::world::skills::{
    apply_attack_spell_scaling,
    apply_fatal_sword_and_undead,
    compute_magic_mana_cost,
    compute_pure_magic_attack_damage,
    fatal_sword_level_for_player,
    mp_eater_level_for_player,
    hemorrhage_level_for_player,
    is_pure_magic_attack,
    resolve_attack_spell_and_level_for_player,
};
use crate::world::skills::assassin::{apply_moon_dark_bonus, resolve_moon_dark_opening};
use crate::world::skills::warrior::{
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
use crate::world::skills::warrior::cast_counter_attack;
use crate::world::skills::assassin::{cast_swift_feet, cast_haste};

impl<P: WorldProvider> World<P> {
    /// Handle a range attack command from a player
    pub(crate) fn handle_range_attack_command(
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
                if !can_attack(p) {
                    return;
                }
                p.direction = direction;
                p.log_time_ms = self.time_ms.saturating_add(10_000);
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

    /// When a player in FocusMasterTarget pet mode acquires a valid monster
    /// target (via melee or attack-command driven magic), update their pets'
    /// focus target so that pet AI can mirror C# PetMode.FocusMasterTarget
    /// behaviour and only attack that monster.
    pub(crate) fn maybe_update_pet_focus_target_for_player_on_monster(
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

    /// Handle a magic command from a player
    pub(crate) fn handle_magic_command(
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

        if spell == Spell::LionRoar as u8 {
            use crate::world::skills::warrior::cast_lion_roar;
            cast_lion_roar(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::BladeAvalanche as u8 {
            use crate::world::skills::warrior::cast_blade_avalanche;
            cast_blade_avalanche(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::ProtectionField as u8 {
            use crate::world::skills::warrior::cast_protection_field;
            cast_protection_field(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::Repulsion as u8 {
            use crate::world::skills::wizard::cast_repulsion;
            cast_repulsion(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::SlashingBurst as u8 {
            use crate::world::skills::warrior::cast_slashing_burst;
            cast_slashing_burst(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::MagicBooster as u8 {
            use crate::world::skills::wizard::cast_magic_booster;
            cast_magic_booster(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::TurnUndead as u8 {
            use crate::world::skills::wizard::cast_turn_undead;
            cast_turn_undead(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::Vampirism as u8 {
            use crate::world::skills::wizard::cast_vampirism;
            cast_vampirism(self, session_id, spell, direction, x, y, events);
            return;
        }

        // TODO: Handle other spells that are not yet implemented:
        //   (Note: Entrapment is handled in attack flow via resolve_attack_spell_and_level_for_player)
        // - Wizard: EnergyRepulsor, ElectricShock,
        //   Mirroring, MeteorStrike, IceThrust
        //   (Note: FrostCrunch is handled in attack flow via is_pure_magic_attack)
        // - Taoist: Revelation, EnergyRepulsor, TrapHexagon, Purification, Curse, Plague, PoisonCloud
        // - Assassin: (most assassin spells are already implemented)
        // - Archer: (archer spells not yet implemented)
        //
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

    /// Handle an attack command from a player
    pub(crate) fn handle_attack_command(
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
            fatal_sword_active,
            mp_eater_level,
            mp_eater_active,
            hemorrhage_level,
            hemorrhage_active,
            attacker_stats,
            flaming_sword_trigger,
            slaying_toggled_on,
            moon_dark_spell,
            moon_dark_level,
        ) =
            match self.players.get_mut(&session_id) {
                Some(p) => {
                    if !can_attack(p) {
                        return;
                    }

                    p.direction = direction;
                    p.log_time_ms = self.time_ms.saturating_add(10_000);

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

                    // C# FatalSword activation: 10% chance per attack if not already active
                    // C#: if (!FatalSword && Envir.Random.Next(10) == 0) FatalSword = true;
                    let fatal_level = fatal_sword_level_for_player(p);
                    if fatal_level.is_some() && !p.fatal_sword {
                        let mut rng = thread_rng();
                        if rng.gen_range(0..10) == 0 {
                            p.fatal_sword = true;
                        }
                    }

                    // C# MPEater: Increment counter per attack, activate when >= 100
                    // C#: int baseCount = 1 + Stats[Stat.Accuracy] / 2;
                    // C#: int maxCount = baseCount + magic.Level * 5;
                    // C#: MPEaterCount += Envir.Random.Next(baseCount, maxCount);
                    // C#: if (!MPEater && 100 <= MPEaterCount) MPEater = true;
                    let mp_eater_level = mp_eater_level_for_player(p);
                    if let Some(level) = mp_eater_level {
                        if !p.mp_eater {
                            let mut rng = thread_rng();
                            let accuracy = p.stats.total.get(Stat::Accuracy).max(0);
                            let base_count = 1 + accuracy / 2;
                            let max_count = base_count + (level as i32 * 5);
                            let increment = rng.gen_range(base_count..=max_count);
                            p.mp_eater_count += increment;
                            if p.mp_eater_count >= 100 {
                                p.mp_eater = true;
                            }
                        }
                    }

                    let fatal_sword_active = p.fatal_sword;
                    let mp_eater_active = p.mp_eater;
                    let hemorrhage_level = hemorrhage_level_for_player(p);
                    let hemorrhage_active = p.hemorrhage;
                    let attacker_stats = p.stats.total.clone();

                    (
                        p.map_index,
                        p.x,
                        p.y,
                        p.direction,
                        effective_spell,
                        level,
                        fatal_level,
                        fatal_sword_active,
                        mp_eater_level,
                        mp_eater_active,
                        hemorrhage_level,
                        hemorrhage_active,
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

                // Apply FatalSword damage enhancement only if active
                // C#: if (FatalSword) damageBase = magic.GetDamage(damageBase);
                if fatal_sword_active {
                    if let Some(fatal_lvl) = fatal_level {
                        raw_damage = apply_fatal_sword_and_undead(
                            &self.provider,
                            Some(fatal_lvl),
                            false,
                            raw_damage,
                        );
                    }
                }

                // Apply MPEater damage enhancement if active
                // C#: if (MPEater) damageFinal = magic.GetDamage(damageBase);
                if mp_eater_active {
                    if let Some(mp_eater_lvl) = mp_eater_level {
                        if let Some(info) = self.provider.get_magic_info(Spell::MPEater as u8) {
                            use crate::world::magic::magic_damage;
                            let mut rng = thread_rng();
                            let boosted = magic_damage(info, mp_eater_lvl, raw_damage, &mut rng);
                            if boosted > 0 {
                                raw_damage = boosted;
                            }
                        }
                    }
                }

                // Apply Hemorrhage damage enhancement if active
                // C#: if (Hemorrhage) damageFinal = magic.GetDamage(damageBase);
                if hemorrhage_active {
                    if let Some(hemorrhage_lvl) = hemorrhage_level {
                        if let Some(info) = self.provider.get_magic_info(Spell::Hemorrhage as u8) {
                            use crate::world::magic::magic_damage;
                            let mut rng = thread_rng();
                            let boosted = magic_damage(info, hemorrhage_lvl, raw_damage, &mut rng);
                            if boosted > 0 {
                                raw_damage = boosted;
                            }
                        }
                    }
                }

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

                // C# FatalSword: Send effect and reset state after successful hit
                // C#: if (FatalSword) { Broadcast(ObjectEffect with SpellEffect.FatalSword); FatalSword = false; LevelMagic(magic); }
                if fatal_sword_active && damage_to_apply > 0 {
                    if let Some(p) = self.players.get_mut(&session_id) {
                        p.fatal_sword = false;
                        
                        // Send FatalSword effect to target
                        events.push(WorldEvent::ObjectEffect {
                            session_id: target_session_id,
                            effect: crate::world::types::SpellEffect::FatalSword.as_u8(),
                        });
                        
                        // Level up FatalSword skill (mirroring C# LevelMagic)
                        // C#: LevelMagic(magic) is called after successful FatalSword hit
                        let _ = self.level_up_magic_for_player(session_id, Spell::FatalSword as u8, events);
                    }
                }

                // C# MPEater: Send effect, drain MP from target, restore MP to attacker, reset state
                // C#: if (MPEater) { Broadcast(ObjectEffect with SpellEffect.MPEater); ChangeMP(addMp); target.ChangeMP(-addMp); MPEaterCount = 0; MPEater = false; }
                if mp_eater_active && damage_to_apply > 0 {
                    // Calculate MP gain first (need attacker stats)
                    let mp_gain = if let Some(attacker) = self.players.get(&session_id) {
                        if let Some(mp_eater_lvl) = mp_eater_level {
                            let accuracy = attacker.stats.total.get(Stat::Accuracy).max(0);
                            // C#: int addMp = 5 * (magic.Level + Stats[Stat.Accuracy] / 4);
                            Some(5 * (mp_eater_lvl as i32 + accuracy / 4))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some(mp_gain_val) = mp_gain {
                        // Send MPEater effect to target
                        events.push(WorldEvent::ObjectEffect {
                            session_id: target_session_id,
                            effect: crate::world::types::SpellEffect::MPEater.as_u8(),
                        });
                        
                        // Drain MP from target
                        if let Some(target) = self.players.get_mut(&target_session_id) {
                            let max_mp_target = target.stats.total.get(Stat::MP).max(0);
                            target.mp = (target.mp - mp_gain_val).max(0).min(max_mp_target);
                        }
                        
                        // Restore MP to attacker and reset state
                        if let Some(attacker) = self.players.get_mut(&session_id) {
                            let max_mp_attacker = attacker.stats.total.get(Stat::MP).max(0);
                            attacker.mp = (attacker.mp + mp_gain_val).min(max_mp_attacker);
                            
                            // Reset MPEater state
                            attacker.mp_eater = false;
                            attacker.mp_eater_count = 0;
                            
                            // Level up MPEater skill (mirroring C# LevelMagic)
                            // C#: LevelMagic(magic) is called when MPEater triggers
                            let _ = self.level_up_magic_for_player(session_id, Spell::MPEater as u8, events);
                        }
                    }
                }

                // C# Hemorrhage: Send effect, apply Bleeding poison, reset state
                // C#: if (Hemorrhage) { Broadcast(ObjectEffect with SpellEffect.Hemorrhage); ApplyPoison(Bleeding); HemorrhageAttackCount = 0; Hemorrhage = false; }
                if hemorrhage_active && damage_to_apply > 0 {
                    if let Some(attacker) = self.players.get(&session_id) {
                        if let Some(hemorrhage_lvl) = hemorrhage_level {
                            // Send Hemorrhage effect to target
                            events.push(WorldEvent::ObjectEffect {
                                session_id: target_session_id,
                                effect: crate::world::types::SpellEffect::Hemorrhage.as_u8(),
                            });
                            
                            // C#: long calcDuration = magic.Level * 2 + Stats[Stat.Luck] / 6;
                            let luck = attacker.stats.total.get(Stat::Luck).max(0);
                            let duration = (hemorrhage_lvl as i64 * 2 + luck as i64 / 6).max(1);
                            
                            // C#: Value = Stats[Stat.MaxDC] + 1
                            let max_dc = attacker.stats.total.get(Stat::MaxDC).max(0);
                            let poison_value = max_dc + 1;
                            
                            // Apply Bleeding poison to target
                            // C#: ob.ApplyPoison(new Poison { PType = PoisonType.Bleeding, Duration = ..., TickSpeed = 1000, Value = ... }, this)
                            // Use apply_poison_to_player_from_monster with monster_id = 0 for player-to-player poison
                            self.apply_poison_to_player_from_monster(
                                0, // No monster ID for player attacks
                                map_index,
                                target_session_id,
                                PoisonType::Bleeding,
                                poison_value,
                                duration,
                                1000, // tick_speed
                                false, // no_resist
                                false, // ignore_defence
                            );
                            
                            // Reset target's operate time
                            // C#: ob.OperateTime = 0;
                            if let Some(target) = self.players.get_mut(&target_session_id) {
                                target.operate_time_ms = 0;
                            }
                            
                            // Reset Hemorrhage state
                            if let Some(attacker_mut) = self.players.get_mut(&session_id) {
                                attacker_mut.hemorrhage = false;
                                attacker_mut.hemorrhage_attack_count = 0;
                                
                                // Level up Hemorrhage skill (mirroring C# LevelMagic)
                                // C#: LevelMagic(magic) is called when Hemorrhage triggers
                                let _ = self.level_up_magic_for_player(session_id, Spell::Hemorrhage as u8, events);
                            }
                        }
                    }
                }

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

            // Store monster master info before applying damage (needed for BrownTime logic)
            let monster_master_sid = if let Some(monsters) = self.monsters.get(&map_index) {
                monsters.iter()
                    .find(|m| m.id == id)
                    .and_then(|m| m.owner_session_id)
            } else {
                None
            };

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

            // C# MonsterObject.Attacked: BrownTime and pet target setting
            // These happen after damage is applied but before ObjectStruck
            if hit && damage_done > 0 {
                // BrownTime logic: if monster has a player master and conditions are met
                // C#: if (Master != null && Master != attacker && Master.Race == ObjectType.Player && 
                //      Envir.Time > Master.BrownTime && Master.PKPoints < 200 && !((PlayerObject)Master).AtWar(attacker))
                //      attacker.BrownTime = Envir.Time + Settings.Minute;
                if let Some(master_sid) = monster_master_sid {
                    if master_sid != session_id {
                        if let Some(master) = self.players.get(&master_sid) {
                            // Check if master is a player (always true in our case)
                            // Check BrownTime and PKPoints conditions
                            // C#: if (Envir.Time > BrownTime && PKPoints < 200 && !AtWar(attacker))
                            //      attacker.BrownTime = Envir.Time + Settings.Minute;
                            // AtWar checks if attacker and target are in warring guilds
                            // TODO: Implement AtWar check when guild war system is available
                            // For now, we skip the AtWar check and always set BrownTime if conditions are met
                            if self.time_ms > master.brown_time_ms && master.pk_points < 200 {
                                if let Some(attacker) = self.players.get_mut(&session_id) {
                                    attacker.brown_time_ms = self.time_ms.saturating_add(60_000); // Settings.Minute = 60000
                                }
                            }
                        }
                    }
                }

                // Pet target setting: if attacker's pets can attack target and have no target, set target
                // C#: for (int i = 0; i < attacker.Pets.Count; i++) {
                //      MonsterObject ob = attacker.Pets[i];
                //      if (IsAttackTarget(ob) && (ob.Target == null)) ob.Target = this;
                // }
                // TODO: Implement pet target setting when pet system is fully integrated
                // This requires checking if pets can attack the monster and if they have no target
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
                        show_struck: true,
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
                    show_struck: true,
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
