//! Damage calculation and application
//! 
//! This module handles core damage calculation logic:
//! - Hit checks (Accuracy vs Agility, MagicResist)
//! - Armour calculation (AC/MAC)
//! - Damage reduction (DamageReductionPercent)
//! - Critical hits
//! - EnergyShield, MagicShield, ElementalBarrier
//! - HPDrainRatePercent
//! - BrownTime, LevelOffset

use rand::{thread_rng, Rng};
use crate::stats::Stat;
use crate::world::types::{BuffType, DamageType, PoisonType, SpellEffect};
use crate::world::{SessionId, World, WorldEvent, WorldProvider};
use crate::world::configs::setup_config;

/// Heal a player by a certain amount
pub fn heal_player<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    heal_amount: i32,
    events: &mut Vec<WorldEvent>,
) {
    if let Some(player) = world.players.get_mut(&session_id) {
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
pub fn apply_damage_to_player<P: WorldProvider>(
    world: &mut World<P>,
    attacker_session_id: Option<SessionId>,
    target_session_id: SessionId,
    damage: i32,
    damage_type: DamageType,
    map_index: i32,
    events: &mut Vec<WorldEvent>,
) -> (i32, bool) {
    apply_damage_to_player_internal(
        world,
        attacker_session_id,
        target_session_id,
        damage,
        damage_type,
        map_index,
        events,
        false, // is_reflected flag to prevent infinite loops
    )
}

/// Internal damage application mirroring C# HumanObject.Attacked/Struck,
/// with a reflection guard to avoid infinite loops.
pub fn apply_damage_to_player_internal<P: WorldProvider>(
    world: &mut World<P>,
    attacker_session_id: Option<SessionId>,
    target_session_id: SessionId,
    damage: i32,
    damage_type: DamageType,
    map_index: i32,
    events: &mut Vec<WorldEvent>,
    is_reflected: bool,
) -> (i32, bool) {
    let now_ms = world.time_ms;

    // Extract basic defender state first.
    let (mut armour, regen_delay_ms, dir, pos_x, pos_y, _hp, max_hp, poison_mask) =
        match world.players.get(&target_session_id) {
            Some(p) => {
                if p.dead || p.hp <= 0 {
                    return (0, false);
                }
                (
                    0i32,
                    10_000i64,
                    p.direction,
                    p.x,
                    p.y,
                    p.hp,
                    p.stats.total.get(Stat::HP).max(1),
                    p.current_poison_mask,
                )
            }
            None => return (0, false),
        };

    let defender_stats = match world.players.get(&target_session_id) {
        Some(p) => p.stats.total.clone(),
        None => return (0, false),
    };
    let (att_crit_rate, att_crit_dmg) = if let Some(att_sid) = attacker_session_id {
        if let Some(att) = world.players.get(&att_sid) {
            (
                att.stats.total.get(Stat::CriticalRate).max(0),
                att.stats.total.get(Stat::CriticalDamage).max(0),
            )
        } else {
            (0, 0)
        }
    } else {
        (0, 0)
    };

    // Base armour selection mirrors C# GetArmour by defence type: Physical -> AC, Magical/elemental/poison -> MAC.
    // C# GetArmour also performs hit checks:
    // - ACAgility: Agility vs Accuracy check
    // - MACAgility: MagicResist check + Agility vs Accuracy check
    // - MAC: MagicResist check
    // In Rust, we handle hit checks in compute_physical_melee_with_crit for physical attacks.
    // For magical/elemental/poison damage, we need to check MagicResist here.
    let mut rng = thread_rng();
    
    // Check MagicResist for magical/elemental/poison damage (mirroring C# MAC/MACAgility)
    // C# MACAgility: checks MagicResist first, then Agility vs Accuracy
    // C# MAC: checks MagicResist only
    // C#: if (Envir.Random.Next(Settings.MagicResistWeight) < Stats[Stat.MagicResist]) { hit = false; }
    // Note: For MACAgility, C# also checks Agility vs Accuracy, but we don't have DefenceType here.
    // The Agility vs Accuracy check is handled in compute_physical_melee_with_crit for physical attacks.
    // For magical damage, we only check MagicResist here (matching C# MAC behavior).
    if matches!(damage_type, DamageType::Magical | DamageType::Poison) || damage_type.is_elemental() {
        let cfg = setup_config();
        let magic_resist_weight = cfg.items.magic_resist_weight.max(1) as i32;
        let magic_resist = defender_stats.get(Stat::MagicResist).max(0);
        if magic_resist > 0 && rng.gen_range(0..magic_resist_weight) < magic_resist {
            // Miss due to MagicResist
            if let Some(p) = world.players.get_mut(&target_session_id) {
                p.next_regen_time_ms = now_ms.saturating_add(regen_delay_ms);
            }
            let health_percent = if let Some(p) = world.players.get(&target_session_id) {
                ((p.hp as i64 * 100) / max_hp as i64).clamp(0, 100) as u8
            } else {
                0
            };
            events.push(WorldEvent::ObjectStruck {
                attacker_id: attacker_session_id.unwrap_or(target_session_id),
                target_id: target_session_id as u64,
                map_index,
                x: pos_x,
                y: pos_y,
                direction: dir,
                damage: 0,
                damage_type: 1, // Miss indicator
                health_percent,
                show_struck: true,
            });
            return (0, false);
        }
    }
    
    match damage_type {
        DamageType::Physical => {
            let min_ac = defender_stats.get(Stat::MinAC).max(0);
            let max_ac = defender_stats.get(Stat::MaxAC).max(min_ac);
            armour = if max_ac <= min_ac {
                min_ac
            } else {
                rng.gen_range(min_ac..=max_ac)
            };
        }
        DamageType::Magical | DamageType::Poison => {
            let min_mac = defender_stats.get(Stat::MinMAC).max(0);
            let max_mac = defender_stats.get(Stat::MaxMAC).max(min_mac);
            armour = if max_mac <= min_mac {
                min_mac
            } else {
                rng.gen_range(min_mac..=max_mac)
            };
        }
        _ if damage_type.is_elemental() => {
            let min_mac = defender_stats.get(Stat::MinMAC).max(0);
            let max_mac = defender_stats.get(Stat::MaxMAC).max(min_mac);
            armour = if max_mac <= min_mac {
                min_mac
            } else {
                rng.gen_range(min_mac..=max_mac)
            };
        }
        _ => {}
    }

    // Apply poison-derived ArmourRate/DamageRate approximations.
    let mut armour_rate: f32 = 1.0;
    let mut damage_rate: f32 = 1.0;
    if (poison_mask & PoisonType::Red.as_u16()) != 0 {
        armour_rate -= 0.10; // HumanObject red poison: -10% AC/MAC
    }
    if (poison_mask & PoisonType::Stun.as_u16()) != 0 {
        damage_rate += 0.20; // HumanObject stun poison: +20% damage taken
    }

    // C#: armour = (int)Math.Max(int.MinValue, (Math.Min(int.MaxValue, (decimal)(armour * ArmourRate))));
    // C#: damage = (int)Math.Max(int.MinValue, (Math.Min(int.MaxValue, (decimal)(damage * DamageRate))));
    // Rust: Use saturating operations to prevent overflow/underflow
    armour = ((armour as f32) * armour_rate) as i32;
    armour = armour.max(i32::MIN).min(i32::MAX);
    let mut damage = ((damage as f32) * damage_rate) as i32;
    damage = damage.max(i32::MIN).min(i32::MAX);

    // Apply AttackBonus for physical attacks (mirroring C#: damage += attacker.Stats[Stat.AttackBonus])
    // C# order: damageWeapon -> AttackBonus -> Reflect check
    // This is done after damageWeapon in C#, but since we don't have damageWeapon flag,
    // we apply it for all physical attacks.
    if damage_type == DamageType::Physical {
        if let Some(att_sid) = attacker_session_id {
            if let Some(attacker) = world.players.get(&att_sid) {
                let attack_bonus = attacker.stats.total.get(Stat::AttackBonus);
                if attack_bonus > 0 {
                    damage = damage.saturating_add(attack_bonus);
                }
            }
        }
    }

    // Stat-based reflect (Reflect%) - C# checks this BEFORE DamageReductionPercent
    // C# order: AttackBonus -> Reflect check -> DamageReductionPercent
    if !is_reflected {
        let reflect_chance = defender_stats.get(Stat::Reflect).max(0);
        if reflect_chance > 0 && rng.gen_range(0..100) < reflect_chance {
            if let Some(att_sid) = attacker_session_id {
                // Mirror C# PlayerObject.Attacked reflect behaviour:
                // `attacker.Attacked(this, damage, type, false)`.
                // Route via PvP helper so HP/death events are consistent,
                // but do NOT award PK points for reflect damage.
                world.apply_player_hit_from_player(
                    target_session_id,
                    att_sid,
                    map_index,
                    damage.max(0),
                    damage_type.as_u8(),
                    None,
                    None,
                    None,
                    false, // allow_pk_points
                    events,
                );
            }
            events.push(WorldEvent::ObjectEffect {
                session_id: target_session_id,
                effect: SpellEffect::Reflect as u8,
            });
            return (0, false);
        }
    }

    // Apply generic damage reduction (after Reflect check).
    // C#: if (Stats[Stat.DamageReductionPercent] > 0) { damage -= (damage * Stats[Stat.DamageReductionPercent]) / 100; }
    let dr_percent = defender_stats.get(Stat::DamageReductionPercent);
    if dr_percent != 0 {
        damage -= (damage * dr_percent) / 100;
    }

    // C#: if (armour >= damage) { BroadcastDamageIndicator(DamageType.Miss); return 0; }
    // This check happens after DamageReductionPercent but before Hidden removal
    if armour >= damage {
        // Miss-style outcome.
        if let Some(p) = world.players.get_mut(&target_session_id) {
            p.next_regen_time_ms = now_ms.saturating_add(regen_delay_ms);
        }
        let health_percent = if let Some(p) = world.players.get(&target_session_id) {
            ((p.hp as i64 * 100) / max_hp as i64).clamp(0, 100) as u8
        } else {
            0
        };
        let show_struck = if let Some(p) = world.players.get_mut(&target_session_id) {
            if now_ms >= p.next_struck_time_ms {
                p.next_struck_time_ms = now_ms.saturating_add(500);
                true
            } else {
                false
            }
        } else {
            true
        };
        events.push(WorldEvent::ObjectStruck {
            attacker_id: attacker_session_id.unwrap_or(target_session_id),
            target_id: target_session_id as u64,
            map_index,
            x: pos_x,
            y: pos_y,
            direction: dir,
            damage: 0,
            damage_type: 1, // Miss indicator
            health_percent,
            show_struck,
        });
        return (0, false);
    }

    // Hidden removal (MoonLight/DarkBody) - C# does this after armour check
    if let Some(p) = world.players.get_mut(&target_session_id) {
        if p.hidden {
            p.active_buffs
                .retain(|b| b.buff_type != BuffType::MoonLight && b.buff_type != BuffType::DarkBody);
            p.hidden = false;
        }
    }

    // EnergyShield heal proc.
    // C# order: Hidden -> EnergyShield -> Critical
    // C#: if (Stats[Stat.EnergyShieldPercent] > 0)
    //      if (Envir.Random.Next(100) < Stats[Stat.EnergyShieldPercent])
    //          if (HP + (Stats[Stat.EnergyShieldHPGain]) >= Stats[Stat.HP])
    //              SetHP(Stats[Stat.HP]);
    //          else
    //              ChangeHP(Stats[Stat.EnergyShieldHPGain]);
    let energy_shield_percent = defender_stats.get(Stat::EnergyShieldPercent).max(0);
    if energy_shield_percent > 0 && rng.gen_range(0..100) < energy_shield_percent {
        let gain = defender_stats.get(Stat::EnergyShieldHPGain).max(0);
        if gain > 0 {
            if let Some(p) = world.players.get_mut(&target_session_id) {
                let max = max_hp;
                // C# logic: if (HP + EnergyShieldHPGain >= MaxHP) SetHP(MaxHP) else ChangeHP(EnergyShieldHPGain)
                // This is equivalent to: hp = min(hp + gain, max_hp)
                let new_hp = (p.hp + gain).min(max);
                if new_hp > p.hp {
                    p.hp = new_hp;
                }
            }
        }
    }

    // Critical hit: apply after EnergyShield, before MagicShield/ElementalBarrier.
    // C# order: Hidden -> EnergyShield -> Critical -> MagicShield/ElementalBarrier
    let mut critical_hit = false;
    if att_crit_rate > 0 && att_crit_dmg > 0 {
        let cfg = setup_config();
        let rate_weight: i32 = cfg.items.critical_rate_weight as i32;
        let dmg_weight: i32 = cfg.items.critical_damage_weight as i32;
        let threshold = att_crit_rate
            .saturating_mul(rate_weight)
            .clamp(0, 100);
        if threshold > 0 && rng.gen_range(0..100) < threshold {
            critical_hit = true;
            // C#: damage = Math.Min(int.MaxValue, damage + (int)Math.Floor(damage * (((double)attacker.Stats[Stat.CriticalDamage] / (double)Settings.CriticalDamageWeight) * 10)));
            let factor = (att_crit_dmg as f64 / dmg_weight.max(1) as f64) * 10.0;
            let bonus = ((damage as f64) * factor).floor() as i32;
            // C# uses Math.Min(int.MaxValue, ...) to prevent overflow
            // Rust uses saturating_add which provides equivalent protection
            damage = damage.saturating_add(bonus).min(i32::MAX);
        }
    }


    // MagicShield / ElementalBarrier duration reduction based on (damage - armour).
    // C#: var duration = (int)Math.Min(int.MaxValue, magicShield.ExpireTime - ((damage - armour) * 60));
    //      AddBuff(BuffType.MagicShield, this, duration, null);
    // C# AddBuff semantics: if buff exists and StackType is ResetDuration, ExpireTime = duration (absolute time)
    // So: new_ExpireTime = old_ExpireTime - ((damage - armour) * 60)
    // Note: C# uses Math.Min(int.MaxValue, ...) to prevent overflow
    let absorbed = (damage - armour).max(0) as i64;
    if absorbed > 0 {
        let reduce_ms = absorbed.saturating_mul(60);
        if let Some(p) = world.players.get_mut(&target_session_id) {
            p.active_buffs.retain_mut(|b| {
                if b.buff_type == BuffType::MagicShield || b.buff_type == BuffType::ElementalBarrier {
                    // C#: magicShield.ExpireTime is absolute time, subtract (damage - armour) * 60
                    // C#: Math.Min(int.MaxValue, ...) prevents overflow
                    // In Rust, expire_time_ms is also absolute time, use saturating_sub to prevent underflow
                    let new_expire_time = b.expire_time_ms.saturating_sub(reduce_ms);
                    // C#: Math.Min(int.MaxValue, ...) ensures result doesn't exceed int.MaxValue
                    // In Rust, we check if buff expires immediately
                    if new_expire_time <= world.time_ms {
                        // Buff expires immediately
                        return false;
                    }
                    // C#: AddBuff sets ExpireTime = duration (which is already absolute time)
                    // So we just update expire_time_ms directly
                    b.expire_time_ms = new_expire_time;
                    true
                } else {
                    true
                }
            });
        }
    }

    // HPDrain (life steal) accumulation on attacker.
    // C#: if (attacker.Stats[Stat.HPDrainRatePercent] > 0 && damageWeapon)
    // C#: attacker.HpDrain += Math.Max(0, ((float)(damage - armour) / 100) * attacker.Stats[Stat.HPDrainRatePercent]);
    // C#: if (attacker.HpDrain > 2) { int HpGain = (int)Math.Floor(attacker.HpDrain); attacker.ChangeHP(HpGain); attacker.HpDrain -= HpGain; }
    // Since we don't have damageWeapon flag, apply only for physical attacks
    if damage_type == DamageType::Physical {
        if let Some(att_sid) = attacker_session_id {
            if let Some(attacker) = world.players.get_mut(&att_sid) {
                let rate = attacker.stats.total.get(Stat::HPDrainRatePercent).max(0);
                if rate > 0 {
                    // C#: Math.Max(0, ((float)(damage - armour) / 100) * attacker.Stats[Stat.HPDrainRatePercent])
                    let gain = ((absorbed as f32) / 100.0) * rate as f32;
                    attacker.hp_drain += gain.max(0.0);
                    // C#: if (attacker.HpDrain > 2) { int HpGain = (int)Math.Floor(attacker.HpDrain); ... }
                    if attacker.hp_drain > 2.0 {
                        let hp_gain = attacker.hp_drain.floor() as i32;
                        if hp_gain > 0 {
                            let max_hp_att = attacker.stats.total.get(Stat::HP).max(1);
                            attacker.hp = (attacker.hp + hp_gain).min(max_hp_att);
                            attacker.hp_drain -= hp_gain as f32;
                        }
                    }
                }
                
                // Note: C# server does not have ManaSteal weapon effect.
                // MPEater is a skill (Spell.MPEater) that drains MP, not a weapon effect.
                // MPEater is already implemented in commands.rs as a skill-based effect.
            }
        }
    }

    // MentorDamageRatePercent bonus (mirroring C# HumanObject.Attacked)
    // TODO: Requires mentor info in PlayerState (Info.Mentor, Info.IsMentor)
    // and mentee lookup. Currently skipped as mentor system not fully implemented.
    // C# logic:
    // if (attacker.Info.Mentor != 0 && attacker.Info.IsMentor)
    // {
    //     if (attacker.HasBuff(BuffType.Mentor, out _))
    //     {
    //         CharacterInfo mentee = Envir.GetCharacterInfo(attacker.Info.Mentor);
    //         PlayerObject player = Envir.GetPlayer(mentee.Name);
    //         if (player != null && player.CurrentMap == attacker.CurrentMap && 
    //             Functions.InRange(player.CurrentLocation, attacker.CurrentLocation, Globals.DataRange) && 
    //             !player.Dead)
    //         {
    //             if (GroupMembers != null && GroupMembers.Contains(player))
    //                 damage += (int)Math.Round((double)(damage * attacker.Stats[Stat.MentorDamageRatePercent]) / 100);
    //         }
    //     }
    // }

    // Clear LRParalysis poison on hit.
    if let Some(p) = world.players.get_mut(&target_session_id) {
        let before = p.poisons.len();
        p.poisons
            .retain(|po| po.poison_type != PoisonType::LRParalysis);
        if p.poisons.len() != before {
            // Rebuild poison mask.
            let mut mask: u16 = 0;
            for po in &p.poisons {
                mask |= po.poison_type.as_u16();
            }
            p.current_poison_mask = mask;
            p.operate_time_ms = 0; // C# sets OperateTime = 0 when LRParalysis is removed
        }
    }

    // Apply regen/log timers and clear states.
    // C# order: LastHitter = attacker; LastHitTime = Envir.Time + 10000; RegenTime; LogTime
    if let Some(p) = world.players.get_mut(&target_session_id) {
        p.last_hit_time_ms = (now_ms as i64).saturating_add(10_000);
        p.last_hitter_session_id = attacker_session_id;
        p.reincarnation_ready = false;
        p.reincarnation_target_session_id = None;
        p.active_blizzard = false;
        p.active_reincarnation = false;
        p.next_regen_time_ms = (now_ms as i64).saturating_add(regen_delay_ms as i64);
        p.log_time_ms = (now_ms as i64).saturating_add(10_000); // C# LogDelay = 10000
    }

    // BrownTime setting for attacker (mirroring C# HumanObject.Attacked)
    // C#: if (Envir.Time > BrownTime && PKPoints < 200 && !AtWar(attacker))
    //      attacker.BrownTime = Envir.Time + Settings.Minute;
    if let Some(att_sid) = attacker_session_id {
        if let Some(target) = world.players.get(&target_session_id) {
            if world.time_ms > target.brown_time_ms && target.pk_points < 200 {
                // C#: if (Envir.Time > BrownTime && PKPoints < 200 && !AtWar(attacker))
                //      attacker.BrownTime = Envir.Time + Settings.Minute;
                // AtWar checks if attacker and target are in warring guilds
                // TODO: Implement AtWar check when guild war system is available
                // For now, we skip the AtWar check and always set BrownTime if conditions are met
                if let Some(attacker) = world.players.get_mut(&att_sid) {
                    attacker.brown_time_ms = world.time_ms.saturating_add(60_000); // Settings.Minute = 60000
                }
            }
        }
    }

    // Calculate LevelOffset for ApplyNegativeEffects
    // C#: ushort LevelOffset = (byte)(Level > attacker.Level ? 0 : Math.Min(10, attacker.Level - Level));
    let level_offset = if let (Some(target), Some(att_sid)) = (
        world.players.get(&target_session_id),
        attacker_session_id,
    ) {
        if let Some(attacker) = world.players.get(&att_sid) {
            if target.level > attacker.level {
                0
            } else {
                (attacker.level.saturating_sub(target.level) as u16).min(10)
            }
        } else {
            0
        }
    } else {
        0
    };

    // Apply negative effects (paralysis, freezing, poison attack)
    // C#: ApplyNegativeEffects(attacker, type, LevelOffset);
    use crate::world::combat::effects;
    effects::apply_negative_effects_with_level_offset(
        world,
        attacker_session_id,
        target_session_id,
        damage_type,
        level_offset,
    );

    // Gather element for attacker (meditation/concentration)
    // C# order: HPDrainRatePercent -> GatherElement -> MentorDamageRatePercent
    // Positioned here to match C# HumanObject.Attacked flow
    if let Some(att_sid) = attacker_session_id {
        use crate::world::combat::buffs;
        buffs::gather_element(world, att_sid, events);
    }

    // Equipment durability loss mirrors DamageDura.
    // C#: DamageDura(); ActiveBlizzard = false; ActiveReincarnation = false;
    // Positioned before CounterAttackCast to match C# HumanObject.Attacked flow
    // Note: In C#, DamageDura is called before HP change, but it uses the damage value
    // which is already calculated. We apply it after HP change for simplicity,
    // but the effect is the same since durability loss is based on damage_taken.
    // ActiveBlizzard and ActiveReincarnation are already cleared in the regen/log timers section above.

    // CounterAttackCast (if defender has CounterAttack buff)
    // C#: CounterAttackCast(GetMagic(Spell.CounterAttack), LastHitter);
    // Positioned after GatherElement and DamageDura to match C# HumanObject.Attacked flow
    if let Some(att_sid) = attacker_session_id {
        use crate::world::combat::buffs;
        buffs::try_counter_attack(world, target_session_id, att_sid, map_index, events);
    }

    // Calculate damage taken before applying HP change
    // C# order: CounterAttackCast -> Enqueue(S.Struck) -> Broadcast(ObjectStruck) -> BroadcastDamageIndicator -> ChangeHP
    let damage_taken = (damage - armour).max(0);
    
    // C#: Enqueue(new S.Struck { AttackerID = attacker.ObjectID });
    // This is sent to the attacker to notify them of the hit
    // In Rust, we send a separate Struck event to the attacker
    // Note: ObjectStruck is broadcast to everyone, Struck is sent only to attacker
    // C# sends Struck with target's ObjectID as AttackerID (confusing naming)
    // Only send Struck if damage was actually taken (hit was successful)
    if let Some(att_sid) = attacker_session_id {
        if att_sid != target_session_id && damage_taken > 0 {
            // Only send Struck to attacker if they are different from target and hit was successful
            // (self-damage doesn't need Struck notification, misses don't need it either)
            events.push(WorldEvent::Struck {
                attacker_session_id: att_sid,
                target_id: target_session_id as u64,
            });
        }
    }
    
    // C#: Broadcast(new S.ObjectStruck { ... });
    // Send ObjectStruck event before HP change to match C# order
    let health_percent_before = if let Some(player) = world.players.get(&target_session_id) {
        ((player.hp as i64 * 100) / max_hp as i64).clamp(0, 100) as u8
    } else {
        0
    };
    events.push(WorldEvent::ObjectStruck {
        attacker_id: attacker_session_id.unwrap_or(target_session_id),
        target_id: target_session_id as u64,
        map_index,
        x: pos_x,
        y: pos_y,
        direction: dir,
        damage: damage_taken,
        damage_type: damage_type.as_u8(),
        health_percent: health_percent_before, // Health before damage
        show_struck: true,
    });
    if critical_hit {
        events.push(WorldEvent::ObjectEffect {
            session_id: target_session_id,
            effect: SpellEffect::Critical.as_u8(),
        });
    }

    // C#: BroadcastDamageIndicator(DamageType.Hit, armour - damage);
    // Damage indicator is included in ObjectStruck event in Rust
    // The damage_type field already indicates Hit/Miss/Critical

    // Apply HP change.
    // C#: ChangeHP(armour - damage);
    let mut dead = false;
    if let Some(p) = world.players.get_mut(&target_session_id) {
        if damage_taken >= p.hp {
            p.hp = 0;
            p.dead = true;
            dead = true;
        } else {
            p.hp = p.hp.saturating_sub(damage_taken);
        }
    }

    // Apply elemental damage after physical damage (if attacker has element damage)
    // Elemental damage is applied separately and uses MAC for armor calculation
    // Only apply if physical damage was successfully applied (damage_taken > 0)
    if let Some(att_sid) = attacker_session_id {
        if damage_taken > 0 {
            // Calculate elemental damage from attacker's equipment
            let elemental_damages = world.calculate_elemental_damage(att_sid);
            if !elemental_damages.is_empty() {
                // Apply elemental damage to target
                // This applies element damage with proper resistance and MAC reduction
                let _elemental_damage_applied = world.apply_elemental_damage(
                    Some(att_sid),
                    target_session_id,
                    &elemental_damages,
                    map_index,
                    events,
                );
            }
        }
    }

    // Equipment durability loss mirrors DamageDura.
    // C#: DamageDura() is called before CounterAttackCast, but it uses the damage value
    // which is already calculated. In Rust, we apply it after HP change for simplicity,
    // but the effect is the same since durability loss is based on damage_taken,
    // which is calculated before HP change. The order doesn't affect functionality.
    if damage_taken > 0 {
        apply_equipment_durability_loss(world, target_session_id, damage_taken, events);
    }

    if dead {
        if world.try_revive_with_revival_ring(target_session_id, events) {
            return (damage_taken, false);
        }
        world.remove_all_pets_for_session(target_session_id);
        world.clear_player_buffs_on_death(target_session_id, events);
        world.apply_player_death_drops(target_session_id, map_index, events);
        
        // Call default NPC Die page after player death
        // C#: CallDefaultNPC(DefaultNPCType.Die)
        // C# generates key as: "Die" (no parameters)
        // Then wraps it as: string.Format("[@_{0}]", key) -> "[@_Die]"
        events.push(WorldEvent::PlayerDied {
            session_id: target_session_id,
        });
    }

    // Counterattack / reflect buffs.
    // Pass original damage_type for proper reflect damage type
    use crate::world::combat::buffs;
    buffs::handle_combat_buffs_and_counters(
        world,
        attacker_session_id,
        target_session_id,
        damage_taken,
        damage_type, // Pass original damage type for reflect
        map_index,
        events,
        is_reflected,
    );

    // BrownTime logic: if target is not red and attacker exists, arm BrownTime for attacker.
    if let Some(att_sid) = attacker_session_id {
        // Skip BrownTime arming when the map is a fight/war zone (matches C# AtWar guard).
        let in_fight_zone = world
            .provider
            .get_map_info(map_index)
            .map(|mi| mi.fight)
            .unwrap_or(false);

        if !in_fight_zone {
            let tgt_state = world
                .players
                .get(&target_session_id)
                .map(|t| (t.pk_points, t.brown_time_ms));
            if let (Some(att), Some((t_pk, t_brown))) = (world.players.get_mut(&att_sid), tgt_state) {
                if t_pk < 200 && world.time_ms > t_brown {
                    att.brown_time_ms = world.time_ms.saturating_add(60_000);
                }
            }
        }
    }

    (damage_taken, dead)
}

/// Apply equipment durability loss when taking damage
/// Mirrors C# HumanObject.DamageDura / DamageWeapon
pub fn apply_equipment_durability_loss<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    _damage_taken: i32,
    _events: &mut Vec<WorldEvent>,
) {
    // Mirror C# HumanObject.DamageDura / DamageWeapon:
    // - Skip entirely if any equipped item has SpecialItemMode.NoDuraLoss
    // - Non-weapon slots: reduce by Random(1)+1 (i.e., 1..=2) minus Strong, min 1
    // - Weapon slot: reduce by Random(4)+1 (i.e., 1..=5) minus Strong, min 1
    use rand::Rng;

    const WEAPON_SLOT: usize = 0;
    const SPECIAL_MODE_NO_DURA_LOSS: i16 = 0x0400;

    if let Some(player) = world.players.get_mut(&session_id) {
        // Check for NoDuraLoss special mode on any equipped item.
        let has_no_dura_loss = player
            .equipment
            .slots
            .iter()
            .filter_map(|slot| slot.as_ref())
            .any(|item| {
                world.provider
                    .get_item_info(item.item_index)
                    .map(|info| (info.unique & SPECIAL_MODE_NO_DURA_LOSS) != 0)
                    .unwrap_or(false)
            });

        if has_no_dura_loss {
            return;
        }

        let mut rng = thread_rng();
        let mut equipment_changed = false;

        for (idx, slot_opt) in player.equipment.slots.iter_mut().enumerate() {
            let item = match slot_opt {
                Some(i) => i,
                None => continue,
            };

            if item.current_dura == 0 {
                continue;
            }

            let info = match world.provider.get_item_info(item.item_index) {
                Some(info) => info,
                None => continue,
            };

            if info.item_type == 8 {
                // Amulet durability is not reduced by DamageDura/DamageWeapon.
                continue;
            }

            let strong_total: i32 = {
                let base: i32 = info
                    .stats
                    .entries
                    .iter()
                    .filter(|(stat_id, _)| *stat_id == Stat::Strong as u8)
                    .map(|(_, val)| *val)
                    .sum();
                let added: i32 = item
                    .added_stats
                    .entries
                    .iter()
                    .filter(|(stat_id, _)| *stat_id == Stat::Strong as u8)
                    .map(|(_, val)| *val)
                    .sum();
                base + added
            };

            let base_loss: i32 = if idx == WEAPON_SLOT {
                rng.gen_range(1..=5)
            } else {
                rng.gen_range(1..=2)
            };

            let loss = (base_loss - strong_total).max(1) as u16;
            let new_dura = item.current_dura.saturating_sub(loss);
            if new_dura != item.current_dura {
                item.current_dura = new_dura;
                equipment_changed = true;
            }
        }

        if equipment_changed {
            world.recalc_player_equipment_stats(session_id);
        }
    }
}

/// Compute base physical damage for a given player level
/// This is a utility function for damage calculations
#[allow(dead_code)]
pub fn compute_physical_damage_base(player_level: u16) -> i32 {
    let lvl = player_level.max(1) as i32;
    let min_dc = 1 + lvl / 2;
    let max_dc = 2 + lvl;
    if max_dc <= min_dc {
        min_dc.max(1)
    } else {
        (min_dc + max_dc) / 2
    }
}
