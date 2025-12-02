use crate::stats::{Stat, Stats};
use crate::world::monster::MonsterAiState;
use crate::world::player::PlayerState;
use crate::world::provider::WorldProvider;
use crate::world::skills::compute_magic_mana_cost;
use crate::world::types::BuffType;
use crate::world::{SessionId, Spell, World, WorldEvent};
use crate::combat::compute_physical_melee_with_crit;
use rand::{thread_rng, Rng};

const SPELL_HALF_MOON: u8 = crate::world::Spell::HalfMoon as u8;
const SPELL_THRUSTING: u8 = crate::world::Spell::Thrusting as u8;
const SPELL_CROSS_HALF_MOON: u8 = crate::world::Spell::CrossHalfMoon as u8;

pub fn is_warrior_aoe_skill(spell: u8) -> bool {
    spell == SPELL_HALF_MOON || spell == SPELL_CROSS_HALF_MOON
}

/// Return true if this spell is the Thrusting skill, which extends melee
/// attack range in a straight line.
pub fn is_thrusting_spell(spell: u8) -> bool {
    spell == SPELL_THRUSTING
}

/// Compute the list of target coordinates for a HalfMoon attack based on
/// the attacker's position and direction.
///
/// This mirrors the C# HumanObject HalfMoon label: starting from the
/// direction immediately to the left of the facing direction, walk 4
/// directions around the player and hit the tiles one step away, skipping
/// the tile directly in front (which is handled by the primary melee
/// attack).
///
/// Returns a vector of (x, y, is_primary_target) tuples; the primary target
/// is the tile directly in front of the attacker.
pub fn compute_half_moon_targets(x: i32, y: i32, direction: u8) -> Vec<(i32, i32, bool)> {
    let mut targets = Vec::new();

    let front_offset = match direction {
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
    let front_x = x + front_offset.0;
    let front_y = y + front_offset.1;

    // Primary target: one step directly in front.
    targets.push((front_x, front_y, true));

    // Secondary targets: previous direction, facing direction, next, and
    // next-next, skipping the already-handled Front tile. This produces up
    // to three additional tiles around the arc.
    let mut dir = (direction + 7) % 8; // previous dir

    let dir_offset = |d: u8| -> (i32, i32) {
        match d {
            0 => (0, -1),
            1 => (1, -1),
            2 => (1, 0),
            3 => (1, 1),
            4 => (0, 1),
            5 => (-1, 1),
            6 => (-1, 0),
            7 => (-1, -1),
            _ => (0, 0),
        }
    };

    for _ in 0..4 {
        let (dx, dy) = dir_offset(dir);
        let tx = x + dx;
        let ty = y + dy;

        if tx != front_x || ty != front_y {
            targets.push((tx, ty, false));
        }

        dir = (dir + 1) % 8;
    }

    targets
}

/// Calculate damage for a HalfMoon secondary target.
/// Usually it's a percentage of the primary damage.
pub fn compute_half_moon_secondary_damage(primary_damage: i32, level: u8) -> i32 {
    // C# logic often scales AOE damage by level.
    // Example: Base 10% + Level * 10%? 
    // Or just flat divide.
    // Assuming 30% base + level scaling for now or flat ratio.
    // Legacy C# often does: damage / (some_factor).
    
    // Let's use a safe default: 30% of primary damage for secondary targets
    // plus a small bonus per level.
    let percent = 30 + (level as i32 * 5); // Lv0=30%, Lv3=45%
    (primary_damage * percent / 100).max(1)
}

/// Compute the final HalfMoon damage for a single target, given the base
/// physical melee damage, skill level and whether this is the primary target
/// in the arc. This applies the HalfMoon MagicInfo scaling and then, for
/// secondary targets, the additional AoE reduction ratio.
pub fn compute_half_moon_damage_for_target<P: WorldProvider>(
    provider: &P,
    base_physical_damage: i32,
    level: u8,
    _is_primary: bool,
) -> i32 {
    if base_physical_damage <= 0 {
        return 0;
    }

    // Apply the HalfMoon MagicInfo scaling in the same way as C#
    // HumanObject, using the shared magic_damage path via
    // apply_attack_spell_scaling. All HalfMoon targets use the same
    // damage model; we do not apply any additional reduction for
    // secondary tiles.
    let dmg = super::apply_attack_spell_scaling(
        provider,
        SPELL_HALF_MOON,
        level,
        base_physical_damage,
    );
    dmg
}

pub fn compute_cross_half_moon_targets(
    x: i32,
    y: i32,
    direction: u8,
) -> Vec<(i32, i32, bool)> {
    let mut targets = Vec::new();

    let front_offset = match direction {
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
    let front_x = x + front_offset.0;
    let front_y = y + front_offset.1;

    // Primary tile: directly in front of the attacker.
    targets.push((front_x, front_y, true));

    // CrossHalfMoon matches the C# label: starting from the facing
    // direction, walk all 8 directions around the player and hit the
    // adjacent tiles, skipping the Front tile which is already handled by
    // the primary attack. This effectively covers all surrounding tiles
    // except the one directly in front.
    let mut dir = direction;

    let dir_offset = |d: u8| -> (i32, i32) {
        match d {
            0 => (0, -1),
            1 => (1, -1),
            2 => (1, 0),
            3 => (1, 1),
            4 => (0, 1),
            5 => (-1, 1),
            6 => (-1, 0),
            7 => (-1, -1),
            _ => (0, 0),
        }
    };

    for _ in 0..8 {
        let (dx, dy) = dir_offset(dir);
        let tx = x + dx;
        let ty = y + dy;

        if tx != front_x || ty != front_y {
            targets.push((tx, ty, false));
        }

        dir = (dir + 1) % 8;
    }

    targets
}

pub fn compute_cross_half_moon_damage_for_target<P: WorldProvider>(
    provider: &P,
    base_physical_damage: i32,
    level: u8,
    _is_primary: bool,
) -> i32 {
    if base_physical_damage <= 0 {
        return 0;
    }

    // CrossHalfMoon uses its own MagicInfo multipliers in C#. Here we reuse
    // the shared magic_damage path so that all tiles share the same
    // damage model without extra secondary reductions.
    let dmg = super::apply_attack_spell_scaling(
        provider,
        SPELL_CROSS_HALF_MOON,
        level,
        base_physical_damage,
    );
    dmg
}

/// Determine the effective maximum range (in tiles) for Thrusting. This uses
/// the MagicInfo.range value when available and falls back to a small
/// reasonable default when not.
pub fn thrusting_max_range<P: WorldProvider>(provider: &P, _level: u8) -> i32 {
    let base = provider
        .get_magic_info(SPELL_THRUSTING)
        .map(|info| info.range as i32)
        .unwrap_or(2)
        .max(1);

    // If we ever want to scale Thrusting range by level, we can do so here.
    // For now we keep it at the static MagicInfo range.
    base
}

/// Resolve the Slaying (刺杀剑术) charge/consume behaviour for a single
/// melee attack. This mirrors the logic embedded in combat.rs, updating the
/// effective spell and level as well as the player's slaying_charged flag
/// and whether a SpellToggle event should be emitted.
pub fn resolve_slaying_for_attack(
    player: &mut PlayerState,
    effective_spell: &mut u8,
    level: &mut u8,
    slaying_toggled_on: &mut bool,
) {
    // If the player explicitly attempts to use Slaying, require an existing
    // charge; otherwise fall back to a plain melee swing.
    if *level > 0 && *effective_spell == Spell::Slaying as u8 {
        if !player.slaying_charged {
            *effective_spell = 0;
            *level = 0;
        } else {
            // Consume the one-shot Slaying charge for this swing but do not
            // emit a SpellToggle(false) event. The original C# implementation
            // only sends SpellToggle when Slaying becomes available, not when
            // it is spent, so the icon may remain lit until the next refresh.
            player.slaying_charged = false;
        }
    }

    // If Slaying is not currently charged, roll for a new charge using the
    // same pattern as the C# server:
    //   if (magic != null && Random.Next(12) <= magic.Level)
    //       Slaying = true;
    if !player.slaying_charged {
        if let Some(magic) = player
            .magics
            .iter()
            .find(|m| m.spell == Spell::Slaying as u8)
        {
            let mut rng = thread_rng();
            let roll: i32 = rng.gen_range(0..12);
            if roll <= magic.level as i32 {
                player.slaying_charged = true;
                *slaying_toggled_on = true;
            }
        }
    }
}

/// Cast HalfMoon for a warrior, applying physical damage,
/// FatalSword/undead modifiers, experience and drops to all monsters hit
/// by the arc. This mirrors the logic previously embedded in combat.rs.
pub fn cast_half_moon<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    map_index: i32,
    x: i32,
    y: i32,
    direction: u8,
    level: u8,
    fatal_level: Option<u8>,
    attacker_stats: &Stats,
    events: &mut Vec<WorldEvent>,
) {
    let targets = compute_half_moon_targets(x, y, direction);

    for (tx, ty, is_primary) in targets {
        let target_info = match world.monsters.get(&map_index) {
            Some(monsters) => monsters
                .iter()
                .find(|m| m.hp > 0 && m.x == tx && m.y == ty)
                .map(|m| (m.id, m.monster_index)),
            None => None,
        };

        if let Some((id, monster_index)) = target_info {
            let (monster_exp, undead, max_hp, defender_stats, monster_drops): (
                u32,
                bool,
                i32,
                Stats,
                Vec<crate::world::drop::DropInfo>,
            ) = if let Some(info) = world.provider.get_monster_info(monster_index) {
                let max_hp = info.stats.get(Stat::HP).max(1);
                (
                    info.experience,
                    info.undead,
                    max_hp,
                    info.stats.clone(),
                    info.drops.clone(),
                )
            } else {
                (0, false, 1, Stats::default(), Vec::new())
            };

            let (hit, mut raw_damage, damage_type) =
                compute_physical_melee_with_crit(attacker_stats, &defender_stats);

            if hit && raw_damage > 0 {
                raw_damage = compute_half_moon_damage_for_target(
                    &world.provider,
                    raw_damage,
                    level,
                    is_primary,
                );

                raw_damage = super::apply_fatal_sword_and_undead(
                    &world.provider,
                    fatal_level,
                    undead,
                    raw_damage,
                );
            }

            let mut strike_x = tx;
            let mut strike_y = ty;
            let mut strike_dir = direction;
            let mut damage_done: i32 = 0;
            let mut health_percent: u8 = 100;
            let mut dead = false;

            if let Some(monsters) = world.monsters.get_mut(&map_index) {
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
                world.mark_monster_dead(map_index, id);

                if let Some(info) = world.provider.get_monster_info(monster_index) {
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
                        if d.quest_required {
                            continue;
                        }

                        if let Some(r) = d.attempt_drop(
                            world.drop_rate,
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
                        let entry = world.map_items.entry(map_index).or_default();
                        let item_timeout_ms: i64 = 300_000;

                        if total.gold > 0 {
                            let item_id = world.next_map_item_id;
                            world.next_map_item_id =
                                world.next_map_item_id.wrapping_add(1);
                            entry.push(crate::world::map_item::MapItem {
                                id: item_id,
                                map_index,
                                x: strike_x,
                                y: strike_y,
                                item_index: None,
                                gold: total.gold,
                                count: 0,
                                item: None,
                                expire_time_ms: world.time_ms + item_timeout_ms,
                            });

                            events.push(WorldEvent::GoldDropped {
                                object_id: item_id,
                                map_index,
                                x: strike_x,
                                y: strike_y,
                                gold: total.gold,
                            });
                        }

                        for item_index in total.items {
                            let item_id = world.next_map_item_id;
                            world.next_map_item_id =
                                world.next_map_item_id.wrapping_add(1);
                            entry.push(crate::world::map_item::MapItem {
                                id: item_id,
                                map_index,
                                x: strike_x,
                                y: strike_y,
                                item_index: Some(item_index),
                                gold: 0,
                                count: 1,
                                item: None,
                                expire_time_ms: world.time_ms + item_timeout_ms,
                            });

                            events.push(WorldEvent::ItemDropped {
                                object_id: item_id,
                                map_index,
                                x: strike_x,
                                y: strike_y,
                                item_index,
                                count: 1,
                            });

                            if let Some(info) = world.provider.get_item_info(item_index) {
                                if info.global_drop_notify {
                                    let base_name = info.friendly_name();
                                    let monster_name = world
                                        .provider
                                        .get_monster_info(monster_index)
                                        .map(|mi| mi.name.clone())
                                        .unwrap_or_else(|| "Monster".to_string());
                                    let text = format!(
                                        "{} has dropped {}.",
                                        monster_name,
                                        base_name
                                    );
                                    for (&sid, _) in world.players.iter() {
                                        events.push(WorldEvent::PartySystemMessage {
                                            session_id: sid,
                                            message: text.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }

                if monster_exp > 0 {
                    // Route experience gain through the shared helper so
                    // that level-ups and ranking updates are applied
                    // consistently for warrior skills.
                    let _ = world.gain_experience_for_session(session_id, monster_exp, events);

                    events.push(WorldEvent::GainExperience {
                        session_id,
                        amount: monster_exp,
                    });
                }
            }
        }
    }
}

/// Cast CrossHalfMoon for a warrior in the same fashion as the original
/// combat.rs implementation, including drops and experience.
pub fn cast_cross_half_moon<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    map_index: i32,
    x: i32,
    y: i32,
    direction: u8,
    level: u8,
    fatal_level: Option<u8>,
    attacker_stats: &Stats,
    events: &mut Vec<WorldEvent>,
) {
    let targets = compute_cross_half_moon_targets(x, y, direction);

    for (tx, ty, is_primary) in targets {
        let target_info = match world.monsters.get(&map_index) {
            Some(monsters) => monsters
                .iter()
                .find(|m| m.hp > 0 && m.x == tx && m.y == ty)
                .map(|m| (m.id, m.monster_index)),
            None => None,
        };

        if let Some((id, monster_index)) = target_info {
            let (monster_exp, undead, max_hp, defender_stats, monster_drops): (
                u32,
                bool,
                i32,
                Stats,
                Vec<crate::world::drop::DropInfo>,
            ) = if let Some(info) = world.provider.get_monster_info(monster_index) {
                let max_hp = info.stats.get(Stat::HP).max(1);
                (
                    info.experience,
                    info.undead,
                    max_hp,
                    info.stats.clone(),
                    info.drops.clone(),
                )
            } else {
                (0, false, 1, Stats::default(), Vec::new())
            };

            let (hit, mut raw_damage, damage_type) =
                compute_physical_melee_with_crit(attacker_stats, &defender_stats);

            if hit && raw_damage > 0 {
                raw_damage = compute_cross_half_moon_damage_for_target(
                    &world.provider,
                    raw_damage,
                    level,
                    is_primary,
                );

                raw_damage = super::apply_fatal_sword_and_undead(
                    &world.provider,
                    fatal_level,
                    undead,
                    raw_damage,
                );
            }

            let mut strike_x = tx;
            let mut strike_y = ty;
            let mut strike_dir = direction;
            let mut damage_done: i32 = 0;
            let mut health_percent: u8 = 100;
            let mut dead = false;

            if let Some(monsters) = world.monsters.get_mut(&map_index) {
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
                world.mark_monster_dead(map_index, id);

                if let Some(info) = world.provider.get_monster_info(monster_index) {
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
                        if d.quest_required {
                            continue;
                        }

                        if let Some(r) = d.attempt_drop(
                            world.drop_rate,
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
                        let entry = world.map_items.entry(map_index).or_default();
                        let item_timeout_ms: i64 = 300_000;

                        if total.gold > 0 {
                            let item_id = world.next_map_item_id;
                            world.next_map_item_id =
                                world.next_map_item_id.wrapping_add(1);
                            entry.push(crate::world::map_item::MapItem {
                                id: item_id,
                                map_index,
                                x: strike_x,
                                y: strike_y,
                                item_index: None,
                                gold: total.gold,
                                count: 0,
                                item: None,
                                expire_time_ms: world.time_ms + item_timeout_ms,
                            });

                            events.push(WorldEvent::GoldDropped {
                                object_id: item_id,
                                map_index,
                                x: strike_x,
                                y: strike_y,
                                gold: total.gold,
                            });
                        }

                        for item_index in total.items {
                            let item_id = world.next_map_item_id;
                            world.next_map_item_id =
                                world.next_map_item_id.wrapping_add(1);
                            entry.push(crate::world::map_item::MapItem {
                                id: item_id,
                                map_index,
                                x: strike_x,
                                y: strike_y,
                                item_index: Some(item_index),
                                gold: 0,
                                count: 1,
                                item: None,
                                expire_time_ms: world.time_ms + item_timeout_ms,
                            });

                            events.push(WorldEvent::ItemDropped {
                                object_id: item_id,
                                map_index,
                                x: strike_x,
                                y: strike_y,
                                item_index,
                                count: 1,
                            });
                        }
                    }
                }

                if monster_exp > 0 {
                    if let Some(p) = world.players.get_mut(&session_id) {
                        p.experience = p
                            .experience
                            .saturating_add(monster_exp as i64);
                    }

                    events.push(WorldEvent::GainExperience {
                        session_id,
                        amount: monster_exp,
                    });
                }
            }
        }
    }
}

/// Cast FlamingSword for a warrior, applying the temporary weapon buff and
/// training the magic if successful. This mirrors the logic previously
/// in combat.rs, but is kept here with other warrior-specific helpers.
pub fn cast_flaming_sword<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    events: &mut Vec<WorldEvent>,
) {
    let spell_id = Spell::FlamingSword as u8;
    let duration_ms = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell_id) {
            Some(m) => m,
            None => return,
        };

        let level = magic.level;
        let cost = match compute_magic_mana_cost(&world.provider, &player.stats.total, spell_id, level)
        {
            Some(c) => c,
            None => return,
        };

        if player.mp < cost {
            return;
        }

        // 如果已有 FlamingSword Buff，则不允许叠加，行为与 C#
        // HumanObject.FlamingSword 语义一致。
        if player
            .active_buffs
            .iter()
            .any(|b| b.buff_type == BuffType::FlamingSword)
        {
            return;
        }

        player.mp -= cost;

        // 保持原有 10 秒持续时间的近似实现；具体的“一次性触发并
        // 消耗”仍由 combat.rs 中的攻击逻辑驱动。
        10_000_i64
    };

    // 通过统一的 World::add_player_buff 接口挂载 FlamingSword Buff，
    // 这样其生命周期、死亡/下线清理等均由世界 Buff 系统管理。
    world.add_player_buff(
        session_id,
        BuffType::FlamingSword,
        duration_ms,
        Stats::default(),
        Vec::new(),
        events,
    );

    events.push(WorldEvent::SpellToggle {
        session_id,
        spell_id,
        enabled: true,
    });

    world.level_up_magic_for_player(session_id, spell_id, events);
}

/// Cast Rage, granting a temporary DC buff via a player buff entry.
pub fn cast_rage<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    events: &mut Vec<WorldEvent>,
) {
    let spell_id = Spell::Rage as u8;
    let (duration_ms, stats) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell_id) {
            Some(m) => m,
            None => return,
        };

        let level = magic.level;
        let cost = match compute_magic_mana_cost(&world.provider, &player.stats.total, spell_id, level)
        {
            Some(c) => c,
            None => return,
        };

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        let duration_sec = 18 + 6 * level as i64;
        let duration_ms = duration_sec.saturating_mul(1_000);

        let max_dc = player.stats.total.get(Stat::MaxDC) as f32;
        let multiplier = 0.12 + 0.03 * level as f32;
        let add_value = (max_dc * multiplier).round() as i32;

        let mut stats = Stats::default();
        stats.set(Stat::MaxDC, add_value);
        stats.set(Stat::MinDC, add_value);

        (duration_ms, stats)
    };

    world.add_player_buff(
        session_id,
        BuffType::Rage,
        duration_ms,
        stats,
        Vec::new(),
        events,
    );
}

/// Cast Fury, granting a temporary flat AttackSpeed buff via a player buff
/// entry. This mirrors the C# HumanObject FurySpell implementation:
///   duration = 60s + level * 10s, AttackSpeed = 4.
pub fn cast_fury<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    events: &mut Vec<WorldEvent>,
) {
    let spell_id = Spell::Fury as u8;
    let (duration_ms, stats) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell_id) {
            Some(m) => m,
            None => return,
        };

        let level = magic.level;
        let cost = match compute_magic_mana_cost(&world.provider, &player.stats.total, spell_id, level)
        {
            Some(c) => c,
            None => return,
        };

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        let duration_sec = 60_i64.saturating_add(10_i64.saturating_mul(level as i64));
        let duration_ms = duration_sec.saturating_mul(1_000);

        let mut stats = Stats::default();
        stats.set(Stat::AttackSpeed, 4);

        (duration_ms, stats)
    };

    world.add_player_buff(
        session_id,
        BuffType::Fury,
        duration_ms,
        stats,
        Vec::new(),
        events,
    );

    world.level_up_magic_for_player(session_id, spell_id, events);
}

/// Cast ImmortalSkin, trading DC for AC via a timed buff.
pub fn cast_immortal_skin<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    events: &mut Vec<WorldEvent>,
) {
    let spell_id = Spell::ImmortalSkin as u8;
    let (duration_ms, stats) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell_id) {
            Some(m) => m,
            None => return,
        };

        let level = magic.level;
        let cost = match compute_magic_mana_cost(&world.provider, &player.stats.total, spell_id, level)
        {
            Some(c) => c,
            None => return,
        };

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        let duration_sec = 60 + level as i64;
        let duration_ms = duration_sec.saturating_mul(1_000);

        let max_dc = player.stats.total.get(Stat::MaxDC) as f32;
        let max_ac = player.stats.total.get(Stat::MaxAC) as f32;

        let dc_loss = (max_dc * (0.05 + 0.01 * level as f32)).round() as i32;
        let ac_gain = (max_ac * (0.10 + 0.07 * level as f32)).round() as i32;

        let mut stats = Stats::default();
        stats.set(Stat::MaxDC, -dc_loss);
        stats.set(Stat::MaxAC, ac_gain);

        (duration_ms, stats)
    };

    world.add_player_buff(
        session_id,
        BuffType::ImmortalSkin,
        duration_ms,
        stats,
        Vec::new(),
        events,
    );
}

/// Cast CounterAttack, applying a temporary AC/MAC buff.
pub fn cast_counter_attack<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    events: &mut Vec<WorldEvent>,
) {
    let spell_id = Spell::CounterAttack as u8;
    let (duration_ms, stats) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell_id) {
            Some(m) => m,
            None => return,
        };

        let level = magic.level;
        let cost = match compute_magic_mana_cost(&world.provider, &player.stats.total, spell_id, level)
        {
            Some(c) => c,
            None => return,
        };

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        let duration_sec = 7_i64;
        let duration_ms = duration_sec.saturating_mul(1_000);

        let bonus = 11 + level as i32 * 3;
        let mut stats = Stats::default();
        stats.set(Stat::MinAC, bonus);
        stats.set(Stat::MaxAC, bonus);
        stats.set(Stat::MinMAC, bonus);
        stats.set(Stat::MaxMAC, bonus);

        (duration_ms, stats)
    };

    world.add_player_buff(
        session_id,
        BuffType::CounterAttack,
        duration_ms,
        stats,
        Vec::new(),
        events,
    );
}
