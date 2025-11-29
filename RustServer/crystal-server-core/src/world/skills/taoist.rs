use crate::stats::{Stat, Stats};
use crate::world::magic::{magic_damage, magic_power};
use crate::world::skills::compute_magic_mana_cost;
use crate::world::types::{BuffType, PetKind};
use crate::world::{Job, SessionId, World, WorldEvent, WorldProvider};
use crate::world::Spell;
use rand::{thread_rng, Rng};

const ITEM_TYPE_AMULET: u8 = 8;
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
            None => return,
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

pub fn cast_soul_shield<P: WorldProvider>(
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
    _world: &mut World<P>,
    _session_id: SessionId,
    _spell: u8,
    _direction: u8,
    _x: i32,
    _y: i32,
    _events: &mut Vec<WorldEvent>,
) {
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

pub fn cast_summon_shinsu<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    _x: i32,
    _y: i32,
    events: &mut Vec<WorldEvent>,
) {
    // First try to recall an existing Shinsu pet for this player, mirroring
    // the C# behaviour where a second cast recalls instead of summoning a
    // new instance.
    if world.recall_pet_for_player(session_id, PetKind::TaoistShinsu, events) {
        return;
    }

    let (map_index, caster_x, caster_y, level, amulet_slot) = {
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

        let mut amulet_slot: Option<usize> = None;

        for (idx, slot) in player.inventory.slots.iter().enumerate() {
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
            None => return,
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

    if world
        .spawn_pet_for_player(session_id, PetKind::TaoistShinsu)
        .is_none()
    {
        return;
    }

    if let Some(player) = world.players.get_mut(&session_id) {
        if let Some(slot) = player.inventory.slots.get_mut(amulet_slot) {
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
    // As with Shinsu, recast behaves as a recall for an existing HolyDeva
    // pet when present.
    if world.recall_pet_for_player(session_id, PetKind::TaoistHolyDeva, events) {
        return;
    }

    let (map_index, caster_x, caster_y, level, amulet_slot) = {
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

        let mut amulet_slot: Option<usize> = None;

        for (idx, slot) in player.inventory.slots.iter().enumerate() {
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
            None => return,
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

    if world
        .spawn_pet_for_player(session_id, PetKind::TaoistHolyDeva)
        .is_none()
    {
        return;
    }

    if let Some(player) = world.players.get_mut(&session_id) {
        if let Some(slot) = player.inventory.slots.get_mut(amulet_slot) {
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
