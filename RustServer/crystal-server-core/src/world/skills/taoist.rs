use crate::stats::{Stat, Stats};
use crate::world::magic::{magic_damage, magic_power};
use crate::world::monster::MonsterAiState;
use crate::world::skills::compute_magic_mana_cost;
use crate::world::types::{BuffType, PetKind, PoisonType};
use crate::world::{Job, SessionId, World, WorldEvent, WorldProvider};
use crate::world::Spell;
use rand::{thread_rng, Rng};
use tracing::debug;

const ITEM_TYPE_AMULET: u8 = 8;
const SKELETON_AMULET_COUNT: u16 = 1;
const SHINSU_AMULET_COUNT: u16 = 5;
const HOLY_DEVA_AMULET_COUNT: u16 = 2;
const DEFAULT_AMULET_SHAPE: i16 = 0;
pub fn cast_healing<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    target_x: i32,
    target_y: i32,
    events: &mut Vec<WorldEvent>,
) {
    // Resolve caster and magic, check MP and compute heal value.
    let (map_index, caster_x, caster_y, level, heal_value) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => {
                debug!(
                    "cast_summon_shinsu: session_id={} no player found in world",
                    session_id
                );
                return;
            }
        };

        // Look up the learned level of Healing for this player if present;
        // if there is no UserMagic entry, treat the level as 0 so that the
        // spell can still be used (with base power) rather than aborting.
        let level = player
            .magics
            .iter()
            .find(|m| m.spell == spell)
            .map(|m| m.level)
            .unwrap_or(0);

        // If there is no MagicInfo entry for Healing, treat the MP cost as
        // zero rather than aborting so that the spell still executes and
        // emits PlayerHealed/visual events.
        let cost = compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level)
            .unwrap_or(0);

        if player.mp < cost {
            debug!(
                "cast_summon_shinsu: session_id={} mp={} < cost={}",
                session_id,
                player.mp,
                cost
            );
            return;
        }

        player.mp -= cost;

        // C# Healing uses magic.GetDamage(GetAttackPower(SC) * 2) + Level.
        // We approximate GetAttackPower(SC) using the same luck-biased model
        // as other magic helpers, then double it and add the caster level.
        let stats = &player.stats.total;
        let min_sc = stats.get(Stat::MinSC);
        let max_sc = stats.get(Stat::MaxSC);
        let luck = stats.get(Stat::Luck);

        const MAX_LUCK_FOR_MAGIC: i32 = 10;

        let mut rng = thread_rng();
        let base_sc = {
            let min = min_sc.max(0);
            let max = max_sc.max(min);

            if luck > 0 {
                if luck > rng.gen_range(0..MAX_LUCK_FOR_MAGIC) {
                    max
                } else {
                    min
                }
            } else if luck < 0 {
                if luck < -rng.gen_range(0..MAX_LUCK_FOR_MAGIC) {
                    min
                } else {
                    max
                }
            } else if max <= min {
                min
            } else {
                rng.gen_range(min..=max)
            }
        };

        let damage_base = base_sc.saturating_mul(2);

        let mut heal_value = if let Some(info) = world.provider.get_magic_info(spell) {
            let mut rng2 = thread_rng();
            let v = magic_damage(info, level, damage_base, &mut rng2);
            v.max(0)
        } else {
            // Fallback when there is no MagicInfo entry: use the sampled
            // SC-based damage_base directly so Healing still restores HP.
            damage_base.max(0)
        };

        heal_value = heal_value.saturating_add(player.level as i32);

        (player.map_index, player.x, player.y, level, heal_value)
    };

    if heal_value <= 0 {
        return;
    }

    // Choose a target player at the clicked location if present; otherwise,
    // fall back to healing the caster. This approximates the C# behaviour
    // where Healing can be used on friendly targets.
    let mut target_session = session_id;

    for (sid, p) in &world.players {
        if p.map_index != map_index || p.dead || p.hp <= 0 {
            continue;
        }
        if p.x == target_x && p.y == target_y {
            target_session = *sid;
            break;
        }
    }

    let player = match world.players.get_mut(&target_session) {
        Some(p) => p,
        None => return,
    };

    let max_hp = player.stats.total.get(Stat::HP).max(1);
    if player.hp >= max_hp {
        return;
    }

    let old_hp = player.hp;
    let new_hp = (old_hp + heal_value).min(max_hp);
    if new_hp <= old_hp {
        return;
    }

    let amount = new_hp - old_hp;
    player.hp = new_hp;

    events.push(WorldEvent::PlayerHealed {
        session_id: player.session_id,
        map_index,
        x: player.x,
        y: player.y,
        amount,
        new_hp,
    });

    world.level_up_magic_for_player(session_id, Spell::Healing as u8, events);

    // Emit a generic ObjectAttack event so that the client can play the
    // Healing animation and start icon cooldown, mirroring other spells.
    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x: caster_x,
        y: caster_y,
        direction,
        spell,
        level,
        attack_type: 0,
    });
}

pub fn cast_hallucination<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    target_x: i32,
    target_y: i32,
    events: &mut Vec<WorldEvent>,
) {
    let mut rng = thread_rng();
    let now_ms = world.time_ms.max(0);

    let (map_index, caster_x, caster_y, player_level, magic_level) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => return,
        };

        let level = magic.level;
        let cost = compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level)
            .unwrap_or(0);

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        (
            player.map_index,
            player.x,
            player.y,
            i32::from(player.level),
            i32::from(level),
        )
    };

    let mut target_id: Option<u64> = None;
    let mut target_mx: i32 = 0;
    let mut target_my: i32 = 0;
    let mut target_monster_index: i32 = 0;

    if let Some(monsters) = world.monsters.get(&map_index) {
        for m in monsters {
            if m.hp <= 0 {
                continue;
            }
            if m.x == target_x && m.y == target_y {
                target_id = Some(m.id);
                target_mx = m.x;
                target_my = m.y;
                target_monster_index = m.monster_index;
                break;
            }
        }
    }

    let target_id = match target_id {
        Some(id) => id,
        None => return,
    };

    if !world.can_attack_monster(session_id, map_index, target_id) {
        return;
    }

    let dx = target_mx - caster_x;
    let dy = target_my - caster_y;
    if dx.abs().max(dy.abs()) > 7 {
        return;
    }

    let monster_level: i32 = world
        .provider
        .get_monster_info(target_monster_index)
        .map(|info| i32::from(info.level))
        .unwrap_or(0);

    let max_roll = (player_level + 20 + magic_level.saturating_mul(5)).max(1);
    if rng.gen_range(0..max_roll) <= monster_level + 10 {
        return;
    }

    let hallucination_until_ms = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let mut amulet_slot: Option<usize> = None;

        for (idx, slot) in player.equipment.slots.iter().enumerate() {
            let item = match slot.as_ref() {
                Some(i) => i,
                None => continue,
            };

            let info = match world.provider.get_item_info(item.item_index) {
                Some(i) => i,
                None => continue,
            };

            if info.item_type != ITEM_TYPE_AMULET {
                continue;
            }

            if info.shape != DEFAULT_AMULET_SHAPE {
                continue;
            }

            if item.count as u32 >= 1 {
                amulet_slot = Some(idx);
                break;
            }
        }

        let amulet_slot = match amulet_slot {
            Some(idx) => idx,
            None => return,
        };

        if let Some(slot) = player.equipment.slots.get_mut(amulet_slot) {
            if let Some(item) = slot.as_mut() {
                if item.count > 1 {
                    item.count = item.count.saturating_sub(1);
                } else {
                    *slot = None;
                }
            }
        }

        world.level_up_magic_for_player(session_id, Spell::Hallucination as u8, events);

        let extra_secs = rng.gen_range(0..20) + 10;
        now_ms.saturating_add(i64::from(extra_secs).saturating_mul(1_000))
    };

    if let Some(monsters) = world.monsters.get_mut(&map_index) {
        if let Some(m) = monsters.iter_mut().find(|m| m.id == target_id) {
            m.hallucination_time_ms = hallucination_until_ms;
            m.target_session_id = None;
            m.ai_state = MonsterAiState::Idle;
        }
    }

    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x: caster_x,
        y: caster_y,
        direction,
        spell,
        level: magic_level as u8,
        attack_type: 0,
    });
}

pub fn cast_soul_shield<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    _x: i32,
    _y: i32,
    events: &mut Vec<WorldEvent>,
) {
    debug!(
        "cast_summon_shinsu: session_id={} spell={} dir={} starting",
        session_id,
        spell,
        direction
    );
    let (map_index, caster_x, caster_y, level, duration_ms, stats) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => {
                debug!(
                    "cast_summon_shinsu: session_id={} has no UserMagic entry for spell {}",
                    session_id,
                    spell
                );
                return;
            }
        };

        let level = magic.level;
        debug!(
            "cast_summon_shinsu: session_id={} magic_level={} mp_before={}",
            session_id,
            level,
            player.mp
        );
        let cost = match compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level)
        {
            Some(c) => c,
            None => {
                debug!(
                    "cast_summon_shinsu: session_id={} no MagicInfo / cost for spell {} level {}",
                    session_id,
                    spell,
                    level
                );
                return;
            }
        };

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        let mut duration_sec: i64 = 0;
        if let Some(info) = world.provider.get_magic_info(spell) {
            let mut rng = thread_rng();
            let power = magic_power(info, level, &mut rng).max(1);
            duration_sec = power as i64;
        }
        if duration_sec <= 0 {
            duration_sec = 60;
        }
        let duration_ms = duration_sec.saturating_mul(1_000);

        let mut stats = Stats::default();
        let bonus = (player.level as i32 / 7).saturating_add(4);
        stats.set(Stat::MaxMAC, bonus);

        (
            player.map_index,
            player.x,
            player.y,
            level,
            duration_ms,
            stats,
        )
    };

    world.add_player_buff(
        session_id,
        BuffType::SoulShield,
        duration_ms,
        stats,
        Vec::new(),
        events,
    );

    world.level_up_magic_for_player(session_id, Spell::SoulShield as u8, events);

    // Emit a generic ObjectAttack so the client can play the SoulShield
    // animation and start the skill icon cooldown.
    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x: caster_x,
        y: caster_y,
        direction,
        spell,
        level,
        attack_type: 0,
    });
}

pub fn cast_blessed_armour<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    _x: i32,
    _y: i32,
    events: &mut Vec<WorldEvent>,
) {
    let (map_index, caster_x, caster_y, level, duration_ms, stats) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => return,
        };

        let level = magic.level;
        let cost = match compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level)
        {
            Some(c) => c,
            None => return,
        };

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        let mut duration_sec: i64 = 0;
        if let Some(info) = world.provider.get_magic_info(spell) {
            let mut rng = thread_rng();
            let power = magic_power(info, level, &mut rng).max(1);
            duration_sec = power as i64;
        }
        if duration_sec <= 0 {
            duration_sec = 60;
        }
        let duration_ms = duration_sec.saturating_mul(1_000);

        let mut stats = Stats::default();
        let bonus = (player.level as i32 / 7).saturating_add(4);
        stats.set(Stat::MaxAC, bonus);

        (
            player.map_index,
            player.x,
            player.y,
            level,
            duration_ms,
            stats,
        )
    };

    world.add_player_buff(
        session_id,
        BuffType::BlessedArmour,
        duration_ms,
        stats,
        Vec::new(),
        events,
    );

    world.level_up_magic_for_player(session_id, Spell::BlessedArmour as u8, events);

    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x: caster_x,
        y: caster_y,
        direction,
        spell,
        level,
        attack_type: 0,
    });
}

pub fn cast_poisoning<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    target_x: i32,
    target_y: i32,
    events: &mut Vec<WorldEvent>,
) {
    let now_ms = world.time_ms.max(0);

    let (map_index, caster_x, caster_y, level, power) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => return,
        };

        let level = magic.level;
        let cost = compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level)
            .unwrap_or(0);

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        // Mirror `magic.GetDamage(GetAttackPower(MinSC, MaxSC))` by sampling a
        // base SC value and passing it through the generic magic_damage helper.
        let stats = &player.stats.total;
        let min_sc = stats.get(Stat::MinSC);
        let max_sc = stats.get(Stat::MaxSC);
        let luck = stats.get(Stat::Luck);

        const MAX_LUCK_FOR_MAGIC: i32 = 10;

        let mut rng = thread_rng();
        let base_sc = {
            let min = min_sc.max(0);
            let max = max_sc.max(min);

            if luck > 0 {
                if luck > rng.gen_range(0..MAX_LUCK_FOR_MAGIC) {
                    max
                } else {
                    min
                }
            } else if luck < 0 {
                if luck < -rng.gen_range(0..MAX_LUCK_FOR_MAGIC) {
                    min
                } else {
                    max
                }
            } else if max <= min {
                min
            } else {
                rng.gen_range(min..=max)
            }
        };

        let power = if let Some(info) = world.provider.get_magic_info(spell) {
            let mut rng2 = thread_rng();
            let v = magic_damage(info, level, base_sc, &mut rng2);
            v.max(0)
        } else {
            base_sc.max(0)
        };

        (player.map_index, player.x, player.y, level, power)
    };

    // Find a target monster at the clicked location, mirroring the targeting
    // approach used by Hallucination.
    let mut target_id: Option<u64> = None;
    let mut target_monster_index: i32 = 0;

    if let Some(monsters) = world.monsters.get(&map_index) {
        for m in monsters {
            if m.hp <= 0 {
                continue;
            }
            if m.x == target_x && m.y == target_y {
                target_id = Some(m.id);
                target_monster_index = m.monster_index;
                break;
            }
        }
    }

    let target_id = match target_id {
        Some(id) => id,
        None => return,
    };

    if !world.can_attack_monster(session_id, map_index, target_id) {
        return;
    }

    // For now, always use Green poison; Red poison support via different
    // poison items can be added later.
    let poison_type = PoisonType::Green;

    // Duration in ticks: (power * 2) + ((Level + 1) * 7), matching
    // HumanObject.Process(DelayedAction) for Spell.Poisoning.
    let mut duration: i64 = (power.saturating_mul(2) as i64)
        .saturating_add((i32::from(level) + 1) as i64 * 7);
    if duration <= 0 {
        duration = 1;
    }

    // Tick speed: 2000ms per tick.
    let tick_speed_ms: i64 = 2_000;

    // Per-tick damage: value / 15 + magic.Level + 1 + rand(PoisonAttack).
    let poison_value = {
        let player = match world.players.get(&session_id) {
            Some(p) => p,
            None => return,
        };
        let stats = &player.stats.total;
        let poison_attack = stats.get(Stat::PoisonAttack).max(0);
        let mut rng = thread_rng();
        let bonus = if poison_attack > 0 {
            rng.gen_range(0..poison_attack.max(0))
        } else {
            0
        };

        let base = power / 15;
        let mut v = base
            .saturating_add(i32::from(level) + 1)
            .saturating_add(bonus);
        if v <= 0 {
            v = 1;
        }
        v
    };

    if !world.apply_poison_to_monster_from_player(
        session_id,
        map_index,
        target_id,
        poison_type,
        poison_value,
        duration,
        tick_speed_ms,
    ) {
        return;
    }

    world.level_up_magic_for_player(session_id, Spell::Poisoning as u8, events);

    // Emit a generic ObjectAttack so the client can play the Poisoning cast
    // animation and start icon cooldown.
    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x: caster_x,
        y: caster_y,
        direction,
        spell,
        level,
        attack_type: 0,
    });
}

pub fn cast_ultimate_enhancer<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    _x: i32,
    _y: i32,
    events: &mut Vec<WorldEvent>,
) {
    let (map_index, caster_x, caster_y, level, duration_ms, stats) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => return,
        };

        let level = magic.level;
        let cost = match compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level)
        {
            Some(c) => c,
            None => return,
        };

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        let stats_total = &player.stats.total;
        let min_sc = stats_total.get(Stat::MinSC);
        let max_sc = stats_total.get(Stat::MaxSC);
        let luck = stats_total.get(Stat::Luck);

        const MAX_LUCK_FOR_MAGIC: i32 = 10;

        let mut rng = thread_rng();
        let base_sc = {
            let min = min_sc.max(0);
            let max = max_sc.max(min);

            if luck > 0 {
                if luck > rng.gen_range(0..MAX_LUCK_FOR_MAGIC) {
                    max
                } else {
                    min
                }
            } else if luck < 0 {
                if luck < -rng.gen_range(0..MAX_LUCK_FOR_MAGIC) {
                    min
                } else {
                    max
                }
            } else if max <= min {
                min
            } else {
                rng.gen_range(min..=max)
            }
        };

        let mut duration_sec: i64 = base_sc.saturating_mul(4) as i64 + (level as i64 + 1) * 50;
        if duration_sec <= 0 {
            duration_sec = 60;
        }
        let duration_ms = duration_sec.saturating_mul(1_000);

        let max_sc = stats_total.get(Stat::MaxSC);
        let mut value = if max_sc >= 5 {
            (max_sc / 5).min(8)
        } else {
            1
        };
        if value <= 0 {
            value = 1;
        }

        let mut stats = Stats::default();
        match player.job {
            Job::Warrior | Job::Assassin => stats.set(Stat::MaxDC, value),
            Job::Wizard | Job::Archer => stats.set(Stat::MaxMC, value),
            Job::Taoist => stats.set(Stat::MaxSC, value),
        }

        (
            player.map_index,
            player.x,
            player.y,
            level,
            duration_ms,
            stats,
        )
    };

    world.add_player_buff(
        session_id,
        BuffType::UltimateEnhancer,
        duration_ms,
        stats,
        Vec::new(),
        events,
    );

    world.level_up_magic_for_player(session_id, Spell::UltimateEnhancer as u8, events);

    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x: caster_x,
        y: caster_y,
        direction,
        spell,
        level,
        attack_type: 0,
    });
}

pub fn cast_energy_shield<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    _x: i32,
    _y: i32,
    events: &mut Vec<WorldEvent>,
) {
    let (map_index, caster_x, caster_y, level, duration_ms, stats) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => return,
        };

        let level = magic.level;
        let cost = match compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level)
        {
            Some(c) => c,
            None => return,
        };

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        let stats_total = &player.stats.total;
        let min_sc = stats_total.get(Stat::MinSC);
        let max_sc = stats_total.get(Stat::MaxSC);
        let luck = stats_total.get(Stat::Luck);

        const MAX_LUCK_FOR_MAGIC: i32 = 10;

        let mut rng = thread_rng();
        let base_sc = {
            let min = min_sc.max(0);
            let max = max_sc.max(min);

            if luck > 0 {
                if luck > rng.gen_range(0..MAX_LUCK_FOR_MAGIC) {
                    max
                } else {
                    min
                }
            } else if luck < 0 {
                if luck < -rng.gen_range(0..MAX_LUCK_FOR_MAGIC) {
                    min
                } else {
                    max
                }
            } else if max <= min {
                min
            } else {
                rng.gen_range(min..=max)
            }
        };

        let power = if let Some(info) = world.provider.get_magic_info(spell) {
            let mut rng2 = thread_rng();
            let v = magic_damage(info, level, base_sc, &mut rng2);
            v.max(0)
        } else {
            0
        };

        let mut duration_sec: i64 = 30 + 50 * level as i64;
        if duration_sec <= 0 {
            duration_sec = 60;
        }
        let duration_ms = duration_sec.saturating_mul(1_000);

        let mut stats = Stats::default();
        let mut chance = 10 - (luck / 3 + level as i32 + 1);
        if chance < 2 {
            chance = 2;
        }

        let percent = ((1.0f32 / chance as f32) * 100.0).round() as i32;
        stats.set(Stat::EnergyShieldPercent, percent);
        stats.set(Stat::EnergyShieldHPGain, power);

        (
            player.map_index,
            player.x,
            player.y,
            level,
            duration_ms,
            stats,
        )
    };

    world.add_player_buff(
        session_id,
        BuffType::EnergyShield,
        duration_ms,
        stats,
        Vec::new(),
        events,
    );

    world.level_up_magic_for_player(session_id, Spell::EnergyShield as u8, events);

    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x: caster_x,
        y: caster_y,
        direction,
        spell,
        level,
        attack_type: 0,
    });
}

pub fn cast_mass_healing<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    center_x: i32,
    center_y: i32,
    events: &mut Vec<WorldEvent>,
) {
    // Resolve caster, magic level and MP cost, and compute the base heal
    // value using the Taoist's SC stats and the spell's MagicInfo.
    let (map_index, caster_x, caster_y, level, heal_value) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        // Look up the learned level of MassHealing if present; when there is
        // no UserMagic entry we treat the level as 0 so Taoist characters can
        // still use the base version of the spell even if their magic list is
        // incomplete.
        let level = player
            .magics
            .iter()
            .find(|m| m.spell == spell)
            .map(|m| m.level)
            .unwrap_or(0);

        // As with single-target Healing, fall back to zero MP cost when there
        // is no MagicInfo entry so MassHealing still works even with a partial
        // MagicInfoList.
        let cost = compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level)
            .unwrap_or(0);

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        // Sample a base magic power from SC, mirroring the C# pattern
        // magic.GetDamage(GetAttackPower(MinSC, MaxSC)). We approximate
        // GetAttackPower using the same luck-biased sampling model as the
        // magic attack helper.
        let stats = &player.stats.total;
        let min_sc = stats.get(Stat::MinSC);
        let max_sc = stats.get(Stat::MaxSC);
        let luck = stats.get(Stat::Luck);

        const MAX_LUCK_FOR_MAGIC: i32 = 10;

        let mut rng = thread_rng();
        let damage_base = {
            let min = min_sc.max(0);
            let max = max_sc.max(min);

            if luck > 0 {
                if luck > rng.gen_range(0..MAX_LUCK_FOR_MAGIC) {
                    max
                } else {
                    min
                }
            } else if luck < 0 {
                if luck < -rng.gen_range(0..MAX_LUCK_FOR_MAGIC) {
                    min
                } else {
                    max
                }
            } else if max <= min {
                min
            } else {
                rng.gen_range(min..=max)
            }
        };

        let heal_value = if let Some(info) = world.provider.get_magic_info(spell) {
            let mut rng2 = thread_rng();
            let v = magic_damage(info, level, damage_base, &mut rng2);
            v.max(0)
        } else {
            // Fallback when there is no MagicInfo entry: use the sampled
            // SC-based damage_base directly so MassHealing still restores HP.
            damage_base.max(0)
        };

        (player.map_index, player.x, player.y, level, heal_value)
    };

    if heal_value <= 0 {
        return;
    }

    let range = 1;
    let min_x = center_x - range;
    let max_x = center_x + range;
    let min_y = center_y - range;
    let max_y = center_y + range;

    let mut trained = false;

    // Collect candidate player sessions first to avoid borrowing issues.
    let mut targets = Vec::new();
    for (sid, p) in &world.players {
        if p.map_index != map_index || p.dead || p.hp <= 0 {
            continue;
        }
        if p.x < min_x || p.x > max_x || p.y < min_y || p.y > max_y {
            continue;
        }

        targets.push(*sid);
    }

    for sid in targets {
        let player = match world.players.get_mut(&sid) {
            Some(p) => p,
            None => continue,
        };

        let max_hp = player.stats.total.get(Stat::HP).max(1);
        if player.hp >= max_hp {
            continue;
        }

        let old_hp = player.hp;
        let new_hp = (old_hp + heal_value).min(max_hp);
        if new_hp <= old_hp {
            continue;
        }

        let amount = new_hp - old_hp;
        player.hp = new_hp;

        events.push(WorldEvent::PlayerHealed {
            session_id: player.session_id,
            map_index,
            x: player.x,
            y: player.y,
            amount,
            new_hp,
        });
        trained = true;
    }

    if trained {
        world.level_up_magic_for_player(session_id, Spell::MassHealing as u8, events);
    }

    // Emit a generic ObjectAttack event so that the client can play the
    // MassHealing animation and start icon cooldown.
    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x: caster_x,
        y: caster_y,
        direction,
        spell,
        level,
        attack_type: 0,
    });
}

pub fn cast_poison_cloud<P: WorldProvider>(
    _world: &mut World<P>,
    _session_id: SessionId,
    _spell: u8,
    _direction: u8,
    _x: i32,
    _y: i32,
    _events: &mut Vec<WorldEvent>,
) {
}

pub fn cast_summon_skeleton<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    _x: i32,
    _y: i32,
    events: &mut Vec<WorldEvent>,
) {
    debug!(
        "cast_summon_skeleton: session_id={} spell={} dir={} starting",
        session_id,
        spell,
        direction
    );

    // First try to recall an existing skeleton pet for this player.
    if world.recall_pet_for_player(session_id, PetKind::TaoistSkeleton, events) {
        debug!(
            "cast_summon_skeleton: session_id={} recalled existing pet and returning",
            session_id
        );
        return;
    }

    let (map_index, caster_x, caster_y, level, amulet_slot) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => {
                debug!(
                    "cast_summon_skeleton: session_id={} no player found in world",
                    session_id
                );
                return;
            }
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => {
                debug!(
                    "cast_summon_skeleton: session_id={} has no UserMagic entry for spell {}",
                    session_id,
                    spell
                );
                return;
            }
        };

        let level = magic.level;
        debug!(
            "cast_summon_skeleton: session_id={} magic_level={} mp_before={}",
            session_id,
            level,
            player.mp
        );
        let cost = match compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level)
        {
            Some(c) => c,
            None => {
                debug!(
                    "cast_summon_skeleton: session_id={} no MagicInfo / cost for spell {} level {}",
                    session_id,
                    spell,
                    level
                );
                return;
            }
        };

        if player.mp < cost {
            debug!(
                "cast_summon_skeleton: session_id={} mp={} < cost={}",
                session_id,
                player.mp,
                cost
            );
            return;
        }

        let mut amulet_slot: Option<usize> = None;

        // Mirror C# HumanObject.GetAmulet(1): search equipped amulets.
        for (idx, slot) in player.equipment.slots.iter().enumerate() {
            let item = match slot.as_ref() {
                Some(i) => i,
                None => continue,
            };

            let info = match world.provider.get_item_info(item.item_index) {
                Some(i) => i,
                None => continue,
            };

            if info.item_type != ITEM_TYPE_AMULET {
                continue;
            }

            if info.shape != DEFAULT_AMULET_SHAPE {
                continue;
            }

            if item.count as u32 >= SKELETON_AMULET_COUNT as u32 {
                amulet_slot = Some(idx);
                break;
            }
        }

        let amulet_slot = match amulet_slot {
            Some(idx) => idx,
            None => {
                debug!(
                    "cast_summon_skeleton: session_id={} has no suitable Skeleton amulet",
                    session_id
                );
                return;
            }
        };

        if player.mp < cost {
            debug!(
                "cast_summon_skeleton: session_id={} mp={} < cost={} (second check)",
                session_id,
                player.mp,
                cost
            );
            return;
        }

        player.mp -= cost;

        (
            player.map_index,
            player.x,
            player.y,
            level,
            amulet_slot,
        )
    };

    let spawned = world.spawn_pet_for_player(session_id, PetKind::TaoistSkeleton);
    debug!(
        "cast_summon_skeleton: session_id={} spawn_pet_for_player result={:?}",
        session_id,
        spawned
    );
    if spawned.is_none() {
        return;
    }

    if let Some(player) = world.players.get_mut(&session_id) {
        if let Some(slot) = player.equipment.slots.get_mut(amulet_slot) {
            if let Some(item) = slot.as_mut() {
                if item.count > SKELETON_AMULET_COUNT {
                    item.count = item.count.saturating_sub(SKELETON_AMULET_COUNT);
                } else {
                    *slot = None;
                }
            }
        }
    }

    world.level_up_magic_for_player(session_id, Spell::SummonSkeleton as u8, events);

    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x: caster_x,
        y: caster_y,
        direction,
        spell,
        level,
        attack_type: 0,
    });
}

pub fn cast_summon_shinsu<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    _x: i32,
    _y: i32,
    events: &mut Vec<WorldEvent>,
) {
    debug!(
        "cast_summon_shinsu: session_id={} spell={} dir={} starting",
        session_id,
        spell,
        direction
    );

    // First try to recall an existing Shinsu pet for this player, mirroring
    // the C# behaviour where a second cast recalls instead of summoning a
    // new instance.
    if world.recall_pet_for_player(session_id, PetKind::TaoistShinsu, events) {
        debug!(
            "cast_summon_shinsu: session_id={} recalled existing pet and returning",
            session_id
        );
        return;
    }

    let (map_index, caster_x, caster_y, level, amulet_slot) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => {
                debug!(
                    "cast_summon_shinsu: session_id={} no player found in world",
                    session_id
                );
                return;
            }
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => {
                debug!(
                    "cast_summon_shinsu: session_id={} has no UserMagic entry for spell {}",
                    session_id,
                    spell
                );
                return;
            }
        };

        let level = magic.level;
        debug!(
            "cast_summon_shinsu: session_id={} magic_level={} mp_before={}",
            session_id,
            level,
            player.mp
        );
        let cost = match compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level)
        {
            Some(c) => c,
            None => {
                debug!(
                    "cast_summon_shinsu: session_id={} no MagicInfo / cost for spell {} level {}",
                    session_id,
                    spell,
                    level
                );
                return;
            }
        };

        if player.mp < cost {
            debug!(
                "cast_summon_shinsu: session_id={} mp={} < cost={}",
                session_id,
                player.mp,
                cost
            );
            return;
        }

        let mut amulet_slot: Option<usize> = None;

        // Mirror C# HumanObject.GetAmulet: search equipped amulets rather
        // than loose items in the bag. The original server checks
        // Info.Equipment and matches on ItemType.Amulet plus Shape/count.
        for (idx, slot) in player.equipment.slots.iter().enumerate() {
            let item = match slot.as_ref() {
                Some(i) => i,
                None => continue,
            };

            let info = match world.provider.get_item_info(item.item_index) {
                Some(i) => i,
                None => continue,
            };

            if info.item_type != ITEM_TYPE_AMULET {
                continue;
            }

            if info.shape != DEFAULT_AMULET_SHAPE {
                continue;
            }

            if item.count as u32 >= SHINSU_AMULET_COUNT as u32 {
                amulet_slot = Some(idx);
                break;
            }
        }

        let amulet_slot = match amulet_slot {
            Some(idx) => idx,
            None => {
                debug!(
                    "cast_summon_shinsu: session_id={} has no suitable Shinsu amulet",
                    session_id
                );
                return;
            }
        };

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        (
            player.map_index,
            player.x,
            player.y,
            level,
            amulet_slot,
        )
    };

    let spawned = world.spawn_pet_for_player(session_id, PetKind::TaoistShinsu);
    debug!(
        "cast_summon_shinsu: session_id={} spawn_pet_for_player result={:?}",
        session_id,
        spawned
    );
    if spawned.is_none() {
        return;
    }

    if let Some(player) = world.players.get_mut(&session_id) {
        if let Some(slot) = player.equipment.slots.get_mut(amulet_slot) {
            if let Some(item) = slot.as_mut() {
                if item.count > SHINSU_AMULET_COUNT {
                    item.count = item.count.saturating_sub(SHINSU_AMULET_COUNT);
                } else {
                    *slot = None;
                }
            }
        }
    }

    world.level_up_magic_for_player(session_id, Spell::SummonShinsu as u8, events);

    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x: caster_x,
        y: caster_y,
        direction,
        spell,
        level,
        attack_type: 0,
    });
}

pub fn cast_summon_holy_deva<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    _x: i32,
    _y: i32,
    events: &mut Vec<WorldEvent>,
) {
    debug!(
        "cast_summon_holy_deva: session_id={} spell={} dir={} starting",
        session_id,
        spell,
        direction
    );

    // As with Shinsu, recast behaves as a recall for an existing HolyDeva
    // pet when present.
    if world.recall_pet_for_player(session_id, PetKind::TaoistHolyDeva, events) {
        debug!(
            "cast_summon_holy_deva: session_id={} recalled existing pet and returning",
            session_id
        );
        return;
    }

    let (map_index, caster_x, caster_y, level, amulet_slot) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => {
                debug!(
                    "cast_summon_holy_deva: session_id={} no player found in world",
                    session_id
                );
                return;
            }
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => {
                debug!(
                    "cast_summon_holy_deva: session_id={} has no UserMagic entry for spell {}",
                    session_id,
                    spell
                );
                return;
            }
        };

        let level = magic.level;
        debug!(
            "cast_summon_holy_deva: session_id={} magic_level={} mp_before={}",
            session_id,
            level,
            player.mp
        );
        let cost = match compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level)
        {
            Some(c) => c,
            None => {
                debug!(
                    "cast_summon_holy_deva: session_id={} no MagicInfo / cost for spell {} level {}",
                    session_id,
                    spell,
                    level
                );
                return;
            }
        };

        if player.mp < cost {
            debug!(
                "cast_summon_holy_deva: session_id={} mp={} < cost={}",
                session_id,
                player.mp,
                cost
            );
            return;
        }

        let mut amulet_slot: Option<usize> = None;

        // As with Shinsu, mirror C# HumanObject.GetAmulet by searching the
        // equipped amulet slots rather than inventory for HolyDeva.
        for (idx, slot) in player.equipment.slots.iter().enumerate() {
            let item = match slot.as_ref() {
                Some(i) => i,
                None => continue,
            };

            let info = match world.provider.get_item_info(item.item_index) {
                Some(i) => i,
                None => continue,
            };

            if info.item_type != ITEM_TYPE_AMULET {
                continue;
            }

            if info.shape != DEFAULT_AMULET_SHAPE {
                continue;
            }

            if item.count as u32 >= HOLY_DEVA_AMULET_COUNT as u32 {
                amulet_slot = Some(idx);
                break;
            }
        }

        let amulet_slot = match amulet_slot {
            Some(idx) => idx,
            None => {
                debug!(
                    "cast_summon_holy_deva: session_id={} has no suitable HolyDeva amulet",
                    session_id
                );
                return;
            }
        };

        if player.mp < cost {
            debug!(
                "cast_summon_holy_deva: session_id={} mp={} < cost={} (second check)",
                session_id,
                player.mp,
                cost
            );
            return;
        }

        player.mp -= cost;

        (
            player.map_index,
            player.x,
            player.y,
            level,
            amulet_slot,
        )
    };

    let spawned = world.spawn_pet_for_player(session_id, PetKind::TaoistHolyDeva);
    debug!(
        "cast_summon_holy_deva: session_id={} spawn_pet_for_player result={:?}",
        session_id,
        spawned
    );
    if spawned.is_none() {
        return;
    }

    if let Some(player) = world.players.get_mut(&session_id) {
        if let Some(slot) = player.equipment.slots.get_mut(amulet_slot) {
            if let Some(item) = slot.as_mut() {
                if item.count > HOLY_DEVA_AMULET_COUNT {
                    item.count = item.count.saturating_sub(HOLY_DEVA_AMULET_COUNT);
                } else {
                    *slot = None;
                }
            }
        }
    }

    world.level_up_magic_for_player(session_id, Spell::SummonHolyDeva as u8, events);

    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x: caster_x,
        y: caster_y,
        direction,
        spell,
        level,
        attack_type: 0,
    });
}
