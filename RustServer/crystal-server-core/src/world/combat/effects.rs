//! Negative effects application
//! 
//! This module handles negative effects:
//! - Freezing (冰冻)
//! - PoisonAttack (毒攻击)
//! - Paralysis (麻痹)
//! - Weapon luck/curse effects

use rand::{thread_rng, Rng};
use crate::stats::Stat;
use crate::world::types::{DamageType, PoisonType, Spell};
use crate::world::{SessionId, World, WorldEvent, WorldProvider};
use crate::world::configs::setup_config;

/// Apply negative effects to target (wrapper function with default level_offset = 0)
pub fn apply_negative_effects<P: WorldProvider>(
    world: &mut World<P>,
    attacker_session_id: Option<SessionId>,
    target_session_id: SessionId,
    damage_type: DamageType,
) {
    // Default level_offset = 0 for backward compatibility
    apply_negative_effects_with_level_offset(
        world,
        attacker_session_id,
        target_session_id,
        damage_type,
        0,
    );
}

/// Apply negative effects to target with level offset
/// Mirrors C# HumanObject.ApplyNegativeEffects
pub fn apply_negative_effects_with_level_offset<P: WorldProvider>(
        world: &mut World<P>,
        attacker_session_id: Option<SessionId>,
        target_session_id: SessionId,
        damage_type: DamageType,
        level_offset: u16,
    ) {
        use rand::Rng;
        
        let mut rng = thread_rng();
        
        // C#: Skip for MAC damage types (DefenceType.MAC and DefenceType.MACAgility)
        // In Rust, we check if damage_type is Magical (which uses MAC)
        if damage_type == DamageType::Magical {
            return;
        }
        
        if let Some(attacker) = attacker_session_id.and_then(|sid| world.players.get(&sid)) {
            let attacker_stats = attacker.stats.total.clone();
            
            // Paralysis - C#: SpecialMode.Paralize check, 1/15 chance
            // C#: if (attacker.SpecialMode.HasFlag(SpecialItemMode.Paralize) && type != DefenceType.MAC && type != DefenceType.MACAgility && 1 == Envir.Random.Next(1, 15))
            // Check if attacker has Paralize flag in any equipped item (typically weapon)
            // SpecialItemMode.Paralize = 0x0001
            const PARALIZE_FLAG: i16 = 0x0001;
            let has_paralize = {
                let attacker_ref = world.players.get(&attacker_session_id.unwrap());
                if let Some(att) = attacker_ref {
                    // Check all equipped items for Paralize flag
                    att.equipment.slots.iter().any(|slot_opt| {
                        if let Some(item) = slot_opt {
                            if let Some(item_info) = world.provider.get_item_info(item.item_index) {
                                (item_info.unique & PARALIZE_FLAG) != 0
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    })
                } else {
                    false
                }
            };
            
            if has_paralize {
                // C#: 1 == Envir.Random.Next(1, 15) means 1/14 chance (range is 1..15, exclusive)
                // Rust: gen_range(1..15) returns 1..14, so we check if result == 1
                if rng.gen_range(1..15) == 1 {
                    // C#: ApplyPoison(new Poison { PType = PoisonType.Paralysis, Duration = 5, TickSpeed = 1000 }, attacker)
                    world.apply_poison_to_player_from_monster(
                        0, // No monster ID for player attacks
                        0, // Map index not needed for poison application
                        target_session_id,
                        PoisonType::Paralysis,
                        0,
                        5,
                        1000,
                        false, // no_resist
                        false, // ignore_defence
                    );
                }
            }
            
            // Freezing - C#: ApplyNegativeEffects uses LevelOffset to reduce chance
            // C#: if ((attacker.Stats[Stat.Freezing] > 0) && (Settings.PvpCanFreeze || Race != ObjectType.Player) && type != DefenceType.MAC && type != DefenceType.MACAgility)
            // C#: if ((Envir.Random.Next(Settings.FreezingAttackWeight) < attacker.Stats[Stat.Freezing]) && (Envir.Random.Next(levelOffset) == 0))
            let freezing = attacker_stats.get(Stat::Freezing).max(0);
            if freezing > 0 {
                let cfg = setup_config();
                // Check PvpCanFreeze setting or target is not a player
                // C#: (Settings.PvpCanFreeze || Race != ObjectType.Player)
                // In Rust, this function is only called for player attacks on players (target_session_id is SessionId)
                // So we check PvpCanFreeze setting: if false, players cannot freeze other players
                let can_freeze = cfg.items.pvp_can_freeze;
                
                if can_freeze {
                    // C#: Envir.Random.Next(Settings.FreezingAttackWeight) < attacker.Stats[Stat.Freezing]
                    // C#: Envir.Random.Next(levelOffset) == 0
                    // Note: C# uses Next(levelOffset) which returns 0..levelOffset, so == 0 means success when levelOffset > 0
                    // If levelOffset == 0, Next(0) throws, so we need to handle that
                    let level_offset_check = if level_offset == 0 {
                        true // Always succeed when levelOffset is 0
                    } else {
                        rng.gen_range(0..level_offset) == 0
                    };
                    
                    if rng.gen_range(0..cfg.items.freezing_attack_weight as u32) < freezing as u32 
                        && level_offset_check {
                        // C#: Duration = Math.Min(10, (3 + Envir.Random.Next(attacker.Stats[Stat.Freezing])))
                        let duration = (3 + rng.gen_range(0..freezing)).min(10);
                        // C#: ApplyPoison(new Poison { PType = PoisonType.Slow, Duration = ..., TickSpeed = 1000 }, attacker)
                        world.apply_poison_to_player_from_monster(
                            0, // No monster ID for player attacks
                            0, // Map index not needed for poison application
                            target_session_id,
                            PoisonType::Slow, // C# uses PoisonType.Slow, not Frozen
                            0,
                            duration as i64,
                            1000,
                            false, // no_resist
                            false, // ignore_defence
                        );
                    }
                }
            }
            
            // PoisonAttack - C#: ApplyNegativeEffects uses LevelOffset to reduce chance
            // C#: if (attacker.Stats[Stat.PoisonAttack] > 0 && type != DefenceType.MAC && type != DefenceType.MACAgility)
            // C#: if ((Envir.Random.Next(Settings.PoisonAttackWeight) < attacker.Stats[Stat.PoisonAttack]) && (Envir.Random.Next(levelOffset) == 0))
            let poison_attack = attacker_stats.get(Stat::PoisonAttack).max(0);
            if poison_attack > 0 {
                let cfg = setup_config();
                // C#: Envir.Random.Next(levelOffset) == 0
                let level_offset_check = if level_offset == 0 {
                    true // Always succeed when levelOffset is 0
                } else {
                    rng.gen_range(0..level_offset) == 0
                };
                
                if rng.gen_range(0..cfg.items.poison_attack_weight as u32) < poison_attack as u32 
                    && level_offset_check {
                    // C#: Value = Math.Min(10, 3 + Envir.Random.Next(attacker.Stats[Stat.PoisonAttack]))
                    let value = (3 + rng.gen_range(0..poison_attack)).min(10);
                    // C#: ApplyPoison(new Poison { PType = PoisonType.Green, Duration = 5, TickSpeed = 1000, Value = ... }, attacker)
                    world.apply_poison_to_player_from_monster(
                        0, // No monster ID for player attacks
                        0, // Map index not needed for poison application
                        target_session_id,
                        PoisonType::Green,
                        value,
                        5,
                        1000,
                        false, // no_resist
                        false, // ignore_defence
                    );
                }
            }
        }
}

/// Apply weapon luck/curse effect after attack
/// Mirrors C# HumanObject.ApplyWeaponLuckCurse
pub fn apply_weapon_luck_curse<P: WorldProvider>(
        world: &mut World<P>,
        attacker_sid: SessionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let cfg = setup_config();
        let max_luck_cfg: i32 = cfg.items.max_luck.max(1).into();
        let min_luck = -max_luck_cfg;

        let mut rng = thread_rng();
        // C#: if (Envir.Random.Next(4) != 0) return;
        if rng.gen_range(0..4) != 0 {
            return;
        }

        {
            let attacker = match world.players.get_mut(&attacker_sid) {
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
        world.recalc_player_equipment_stats(attacker_sid);
        events.push(WorldEvent::PartySystemMessage {
            session_id: attacker_sid,
            message: "Your weapon has been cursed.".to_string(),
        });
}
