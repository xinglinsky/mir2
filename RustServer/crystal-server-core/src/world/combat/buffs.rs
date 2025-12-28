//! Combat buffs and effects
//! 
//! This module handles combat-related buff processing:
//! - CounterAttack (反击)
//! - ThornReflect (反伤)
//! - LifeSteal (吸血)
//! - GatherElement (元素收集)

use rand::{thread_rng, Rng};
use crate::stats::Stat;
use crate::world::types::{BuffType, DamageType, Spell};
use crate::world::{SessionId, World, WorldEvent, WorldProvider};
use crate::world::magic::magic_damage;

/// Check if a player has a specific buff
pub fn player_has_buff<P: WorldProvider>(
    world: &World<P>,
    session_id: SessionId,
    buff_type: BuffType,
) -> bool {
    if let Some(player) = world.players.get(&session_id) {
        player.active_buffs.iter().any(|buff| buff.buff_type == buff_type)
    } else {
        false
    }
}

/// Remove a specific buff from a player
pub fn remove_player_buff<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    buff_type: BuffType,
) {
    if let Some(player) = world.players.get_mut(&session_id) {
        player.active_buffs.retain(|buff| buff.buff_type != buff_type);
    }
}

/// Perform counter attack from defender to attacker
/// Mirrors C# HumanObject.CounterAttackCast
pub fn perform_counter_attack<P: WorldProvider>(
    world: &mut World<P>,
    defender_sid: SessionId,
    attacker_sid: SessionId,
    map_index: i32,
    events: &mut Vec<WorldEvent>,
) {
    // Get defender's magic info and stats
    let (defender_stats, defender_accuracy, attacker_x, attacker_y, spell_level) = match (
        world.players.get(&defender_sid),
        world.players.get(&attacker_sid),
    ) {
        (Some(defender), Some(attacker)) => {
            // Get CounterAttack spell level from buff values
            let level = defender.active_buffs.iter()
                .find(|b| b.buff_type == BuffType::CounterAttack)
                .and_then(|b| b.values.get(0).copied())
                .unwrap_or(0) as u8;
            
            (
                defender.stats.total.clone(),
                defender.stats.total.get(Stat::Accuracy),
                attacker.x,
                attacker.y,
                level,
            )
        }
        _ => return,
    };

    // C#: int damageBase = GetAttackPower(Stats[Stat.MinDC], Stats[Stat.MaxDC]);
    let min_dc = defender_stats.get(Stat::MinDC).max(0);
    let max_dc = defender_stats.get(Stat::MaxDC).max(min_dc);
    let mut damage_base = if max_dc == min_dc {
        min_dc
    } else {
        let mut rng = thread_rng();
        rng.gen_range(min_dc..=max_dc)
    };

    // C#: if (Envir.Random.Next(0, 100) <= Stats[Stat.Accuracy])
    //      damageBase += damageBase; // double damage on hit
    let mut rng = thread_rng();
    if rng.gen_range(0..100) <= defender_accuracy {
        damage_base += damage_base; // Double damage on accuracy hit
    }

    // C#: int damageFinal = magic.GetDamage(damageBase);
    // GetDamage = (DamageBase + GetPower()) * GetMultiplier()
    let spell_id = Spell::CounterAttack as u8;
    let damage_final = if let Some(magic_info) = world.provider.get_magic_info(spell_id) {
        magic_damage(magic_info, spell_level, damage_base, &mut rng)
    } else {
        // Fallback if magic info not found
        damage_base
    };

    // Calculate direction: reverse of attacker's direction
    // C#: MirDirection dir = Functions.ReverseDirection(target.Direction);
    let defender_dir = world.players.get(&defender_sid)
        .map(|p| p.direction)
        .unwrap_or(0);
    // Reverse direction: (dir + 4) % 8
    let counter_dir = (defender_dir + 4) % 8;

    // Apply counter attack damage using DefenceType.AC (physical)
    // C#: DelayedAction action = new DelayedAction(DelayedType.Damage, AttackTime, target, damageFinal, DefenceType.AC, true);
    let _ = world.apply_damage_to_player(
        Some(defender_sid),
        attacker_sid,
        damage_final,
        crate::world::types::DamageType::Physical,
        map_index,
        events,
    );

    // Send counter attack animation
    // C#: Enqueue(new S.ObjectMagic { ObjectID = ObjectID, Direction = Direction, Location = CurrentLocation, Spell = Spell.CounterAttack, TargetID = target.ObjectID, Target = target.CurrentLocation, Cast = true, Level = GetMagic(Spell.CounterAttack).Level, SelfBroadcast = true });
    events.push(WorldEvent::ObjectMagic {
        session_id: defender_sid,
        map_index,
        direction: counter_dir,
        x: 0, // Will be set from player position
        y: 0,
        spell: spell_id,
        level: spell_level,
        target_id: attacker_sid as u32,
        target_x: attacker_x,
        target_y: attacker_y,
    });
}

/// Handle combat buffs and counters (LifeSteal, ThornReflect, CounterAttack)
/// Mirrors C# HumanObject.Attacked buff processing
pub fn handle_combat_buffs_and_counters<P: WorldProvider>(
    world: &mut World<P>,
    attacker_session_id: Option<SessionId>,
    target_session_id: SessionId,
    damage: i32,
    original_damage_type: DamageType,
    map_index: i32,
    events: &mut Vec<WorldEvent>,
    is_reflected: bool,
) {
    if let Some(attacker_sid) = attacker_session_id {
        // Apply LifeSteal吸血 if the attacker has the buff
        if player_has_buff(world, attacker_sid, BuffType::LifeSteal) {
            // Get LifeSteal percentage from buff values
            // values[0] typically contains the percentage (10-30% depending on spell level)
            let lifesteal_percent = if let Some(player) = world.players.get(&attacker_sid) {
                if let Some(lifesteal_buff) = player.active_buffs.iter()
                    .find(|b| b.buff_type == BuffType::LifeSteal) {
                    // Read percentage from buff values, default to 15% if not set
                    lifesteal_buff.values.get(0).copied().unwrap_or(15) as i32
                } else {
                    15 // Default fallback
                }
            } else {
                15 // Default fallback
            };
            
            let heal_amount = (damage * lifesteal_percent) / 100;
            
            if heal_amount > 0 {
                // heal_player is a private method, we'll need to access it through World
                // For now, we'll use a workaround by directly modifying player HP
                if let Some(player) = world.players.get_mut(&attacker_sid) {
                    let max_hp = player.stats.total.get(Stat::HP).max(1);
                    player.hp = (player.hp + heal_amount).min(max_hp);
                }
            }
        }

        // Apply ThornReflect反伤 if the defender has the buff
        if !is_reflected && player_has_buff(world, target_session_id, BuffType::ThornReflect) {
            // Get ThornReflect percentage from buff values
            // values[0] typically contains the percentage (10-30% depending on spell level)
            let reflect_percent = if let Some(player) = world.players.get(&target_session_id) {
                if let Some(reflect_buff) = player.active_buffs.iter()
                    .find(|b| b.buff_type == BuffType::ThornReflect) {
                    // Read percentage from buff values, default to 20% if not set
                    reflect_buff.values.get(0).copied().unwrap_or(20) as i32
                } else {
                    20 // Default fallback
                }
            } else {
                20 // Default fallback
            };
            
            let reflect_damage = (damage * reflect_percent) / 100;
            
            if reflect_damage > 0 {
                // Apply reflected damage back to attacker using original damage type
                // This ensures magic damage reflects as magic, physical as physical
                world.apply_player_hit_from_player(
                    target_session_id,   // defender becomes attacker
                    attacker_sid,        // original attacker takes reflect
                    map_index,
                    reflect_damage,
                    original_damage_type.as_u8(), // Use original damage type instead of fixed Physical
                    None,
                    None,
                    None,
                    true, // allow_pk_points
                    events,
                );
            }
        }
        
        // Apply CounterAttack if the defender has the buff
        // Mirroring C# CounterAttackCast: if (Envir.Random.Next(10) > magic.Level + 6) return;
        // This means: trigger if random(0-9) <= (level + 6)
        if player_has_buff(world, target_session_id, BuffType::CounterAttack) {
            // Get the CounterAttack buff to check its level
            if let Some(player) = world.players.get(&target_session_id) {
                if let Some(counter_buff) = player.active_buffs.iter()
                    .find(|b| b.buff_type == BuffType::CounterAttack) {
                    let spell_level = counter_buff.values.get(0).copied().unwrap_or(0) as u8;
                    
                    // C# logic: Envir.Random.Next(10) > magic.Level + 6 means return (don't trigger)
                    // So we trigger if: random(0-9) <= (level + 6)
                    let mut rng = thread_rng();
                    let roll = rng.gen_range(0..10);
                    if roll <= (spell_level + 6) {
                        // Check distance: must be within 1 cell (C#: Functions.InRange(CurrentLocation, target.CurrentLocation, 1))
                        let (defender_x, defender_y, attacker_x, attacker_y) = match (
                            world.players.get(&target_session_id),
                            world.players.get(&attacker_sid),
                        ) {
                            (Some(def), Some(att)) => (def.x, def.y, att.x, att.y),
                            _ => return,
                        };
                        
                        let dx = (defender_x - attacker_x).abs();
                        let dy = (defender_y - attacker_y).abs();
                        let distance = dx.max(dy); // Chebyshev distance (8-directional)
                        
                        if distance <= 1 {
                            // Perform counter attack
                            perform_counter_attack(
                                world,
                                target_session_id,
                                attacker_sid,
                                map_index,
                                events,
                            );
                            
                            // Remove the CounterAttack buff after use
                            remove_player_buff(world, target_session_id, BuffType::CounterAttack);
                        }
                    }
                }
            }
        }
    }
}

/// Gather element for the attacker (meditation/concentration mana regen)
/// Mirrors C# HumanObject.GatherElement:
/// - Checks for Meditation spell
/// - If Concentration buff active and not interrupted, adds (Concentration.Level + 1) to chance
/// - Roll >= (8 - meditationLvl - concentrateChance) triggers ObtainElement and LevelMagic
pub fn gather_element<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    events: &mut Vec<WorldEvent>,
) {
    use rand::Rng;
    
    let mut rng = thread_rng();
    
    let (meditation_level, concentration_chance) = if let Some(player) = world.players.get(&session_id) {
        // Check if player has Meditation spell
        let meditation_level_opt = player.magics.iter()
            .find(|m| m.spell == Spell::Meditation as u8)
            .map(|m| m.level);
        
        let meditation_level = match meditation_level_opt {
            Some(level) => level,
            None => return, // C#: if (magic == null) return;
        };
        
        let mut concentrate_chance = 0;
        
        // Check for Concentration buff and if it's not interrupted
        // C#: if (HasBuff(BuffType.Concentration, out Buff concentration) && !concentration.Get<bool>("Interrupted"))
        // In Rust, we store Interrupted state in buff.values[0] (0 = false, 1 = true)
        // and InterruptTime in buff.values[1] (low 32 bits) and buff.values[2] (high 32 bits) as i64
        // Note: InterruptTime reset is handled in the buff update loop (not here)
        if let Some(conc_buff) = player.active_buffs.iter().find(|b| b.buff_type == BuffType::Concentration) {
            // Check if Concentration is interrupted
            // C#: !concentration.Get<bool>("Interrupted")
            let is_interrupted = conc_buff.values.get(0).copied().unwrap_or(0) != 0;
            
            // If interrupted, check if InterruptTime has passed
            // C#: if (buff.Get<bool>("Interrupted") && buff.Get<long>("InterruptTime") <= Envir.Time)
            // The reset is handled in the buff update loop, but we check here to determine if we should add concentrate_chance
            if is_interrupted {
                let interrupt_time_low = conc_buff.values.get(1).copied().unwrap_or(0) as u32;
                let interrupt_time_high = conc_buff.values.get(2).copied().unwrap_or(0) as u32;
                let interrupt_time_ms = ((interrupt_time_high as u64) << 32) | (interrupt_time_low as u64);
                let interrupt_time_ms = interrupt_time_ms as i64;
                
                // If InterruptTime has not passed yet, Concentration is still interrupted
                // Note: The reset of Interrupted flag is handled in the buff update loop
                if interrupt_time_ms > world.time_ms {
                    // Still interrupted, don't add concentrate_chance
                    return;
                }
                // If InterruptTime has passed, treat as not interrupted (reset will be handled in buff update loop)
            }
            
            // Concentration is active and not interrupted, add concentrate_chance
            if let Some(conc_magic) = player.magics.iter().find(|m| m.spell == Spell::Concentration as u8) {
                concentrate_chance = conc_magic.level + 1; // C#: concentrateChance = magic.Level + 1
            }
        }
        
        (meditation_level, concentrate_chance)
    } else {
        return;
    };
    
    // C#: if (meditationLvl >= 0) { int rnd = Envir.Random.Next(10); if (rnd >= (8 - meditationLvl - concentrateChance)) { ... } }
    // meditation_level is u8, so it's always >= 0, but we keep the check for clarity
    let roll = rng.gen_range(0..10);
    let threshold = 8i32.saturating_sub(meditation_level as i32).saturating_sub(concentration_chance as i32);
    if roll as i32 >= threshold {
        // C#: ObtainElement(false); LevelMagic(GetMagic(Spell.Meditation));
        // ObtainElement is a complex system that manages elemental orbs:
        // - Tracks ElementsLevel and HasElemental state
        // - Uses Settings.OrbsExpList for level thresholds
        // - Uses Settings.OrbsDmgList and Settings.OrbsDefList for orb power
        // - Sends S.SetElemental packet to update client
        // TODO: Implement ObtainElement when elemental orb system is available
        // This requires:
        // 1. ElementsLevel and HasElemental fields in PlayerState
        // 2. OrbsExpList, OrbsDmgList, OrbsDefList in Settings/Config
        // 3. S.SetElemental packet support
        // For now, just level up meditation (the skill leveling part works)
        let _ = world.level_up_magic_for_player(session_id, Spell::Meditation as u8, events);
    }
}

/// Try to perform a counter attack if defender has CounterAttack buff
/// This is a simplified version used in some contexts
pub fn try_counter_attack<P: WorldProvider>(
    world: &mut World<P>,
    defender_sid: SessionId,
    attacker_sid: SessionId,
    map_index: i32,
    events: &mut Vec<WorldEvent>,
) {
    use rand::Rng;
    
    let mut rng = thread_rng();
    
    if let (Some(defender), Some(attacker)) = (
        world.players.get(&defender_sid),
        world.players.get(&attacker_sid),
    ) {
        // Check if defender has CounterAttack buff active
        let has_counter_attack = defender.active_buffs.iter()
            .any(|b| b.buff_type == BuffType::CounterAttack);
        
        if !has_counter_attack {
            return;
        }
        
        // Check if defender has CounterAttack magic
        let counter_magic = defender.magics.iter()
            .find(|m| m.spell == Spell::CounterAttack as u8);
        
        let magic_level = counter_magic.map(|m| m.level).unwrap_or(0);
        
        // Calculate damage
        let min_dc = defender.stats.total.get(Stat::MinDC).max(0);
        let max_dc = defender.stats.total.get(Stat::MaxDC).max(min_dc);
        let damage_base = rng.gen_range(min_dc..=max_dc);
        
        // Critical hit chance based on Accuracy
        let accuracy = defender.stats.total.get(Stat::Accuracy).max(0);
        let damage_final = if rng.gen_range(0..100) <= accuracy {
            damage_base * 2 // Double damage for crit
        } else {
            damage_base
        };
        
        // Check range (must be adjacent)
        let distance = ((defender.x as i32 - attacker.x as i32).abs()
            + (defender.y as i32 - attacker.y as i32).abs()) as u32;
        
        if distance > 1 {
            return;
        }
        
        // Success chance: 10 - (random) > magic.Level + 6
        if rng.gen_range(0..10) > magic_level + 6 {
            return;
        }
        
        // Extract needed data before mutable borrow
        let defender_pos = (defender.x, defender.y, defender.direction);
        let attacker_pos = (attacker.x, attacker.y);
        
        // Perform counter attack
        // Note: apply_damage_to_player_internal is private, use apply_damage_to_player instead
        let _ = world.apply_damage_to_player(
            Some(defender_sid), // defender is now the attacker
            attacker_sid, // attacker is now the target
            damage_final,
            crate::world::types::DamageType::Physical,
            map_index,
            events,
        );
        
        // Broadcast counter attack effect
        events.push(WorldEvent::ObjectMagic {
            session_id: defender_sid,
            map_index,
            x: defender_pos.0,
            y: defender_pos.1,
            direction: defender_pos.2,
            spell: Spell::CounterAttack as u8,
            level: magic_level,
            target_id: attacker_sid as u32,
            target_x: attacker_pos.0,
            target_y: attacker_pos.1,
        });
        
        // Level up the magic
        let _ = world.level_up_magic_for_player(defender_sid, Spell::CounterAttack as u8, events);
        
        // Remove CounterAttack buff
        remove_player_buff(world, defender_sid, BuffType::CounterAttack);
    }
}
