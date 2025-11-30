use crate::stats::{Stat, Stats};
use crate::world::monster::MonsterAiState;
use crate::world::skills::{
    compute_magic_mana_cost, compute_pure_magic_attack_damage,
};
use crate::world::magic::magic_power_with_base;
use crate::world::types::BuffType;
use crate::world::world::PendingMagicHit;
use crate::world::{SessionId, Spell, World, WorldEvent, WorldProvider};
use rand::{thread_rng, Rng};

/// Wizard-specific helpers for casting AoE spells such as FireBang,
/// IceStorm, ThunderStorm and FlameField. These operate on the World
/// state but keep the spell-specific logic grouped under the skills
/// module, similar to the warrior helpers.
pub fn cast_fire_bang_ice_storm<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    center_x: i32,
    center_y: i32,
    events: &mut Vec<WorldEvent>,
) {
    let (map_index, level, attacker_stats) = {
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

        (
            player.map_index,
            level,
            player.stats.total.clone(),
        )
    };

    // Calculate base magic damage for this cast.
    let damage_base =
        compute_pure_magic_attack_damage(&world.provider, &attacker_stats, spell, level);

    // FireBang/IceStorm range: 3x3 around the clicked point (range=1).
    let range = 1;
    let min_x = center_x - range;
    let max_x = center_x + range;
    let min_y = center_y - range;
    let max_y = center_y + range;

    let mut trained = false;

    // Route monster damage through the unified PendingMagicHit pipeline so
    // that HP, experience and drops are handled consistently with other
    // spells (FireWall, single-target magic, etc.), while still matching the
    // C# Map.FireBang/IceStorm 3x3 AoE and IsAttackTarget rules.
    if damage_base > 0 {
        let base_time = world.time_ms.max(0);

        if let Some(monsters) = world.monsters.get(&map_index) {
            for m in monsters.iter() {
                if m.hp <= 0 {
                    continue;
                }

                if m.x < min_x || m.x > max_x || m.y < min_y || m.y > max_y {
                    continue;
                }

                // Respect pet/AttackMode rules via can_attack_monster,
                // approximating target.IsAttackTarget(player).
                if !world.can_attack_monster(session_id, map_index, m.id) {
                    continue;
                }

                let damage = damage_base;
                if damage <= 0 {
                    continue;
                }

                trained = true;

                let due_time_ms = base_time;
                world.pending_magic_hits.push(PendingMagicHit {
                    due_time_ms,
                    attacker_session_id: session_id,
                    map_index,
                    target_monster_id: m.id,
                    monster_index: m.monster_index,
                    spell_id: spell,
                    damage,
                    damage_type: 0,
                });
            }
        }
    }

    // Damage players within the 3x3 AoE, mirroring the C# Map.FireBang/IceStorm
    // behaviour where both monsters and players that satisfy IsAttackTarget are
    // hit using DefenceType.MAC. We approximate MAC/DR using the same logic as
    // FireWall's player damage and route HP/death/PK via the unified
    // apply_player_hit_from_player helper.
    if damage_base > 0 {
        let mut player_targets: Vec<SessionId> = Vec::new();

        for (&sid, p) in &world.players {
            if p.map_index != map_index || p.dead || p.hp <= 0 {
                continue;
            }

            if p.x < min_x || p.x > max_x || p.y < min_y || p.y > max_y {
                continue;
            }

            if !world.can_attack_player(session_id, sid) {
                continue;
            }

            player_targets.push(sid);
        }

        for sid in player_targets {
            let dmg = {
                let p = match world.players.get(&sid) {
                    Some(p) => p,
                    None => continue,
                };

                let stats = &p.stats.total;

                let min_mac = stats.get(Stat::MinMAC).max(0);
                let max_mac = stats.get(Stat::MaxMAC).max(min_mac);
                let mut dmg = damage_base;

                if max_mac > 0 {
                    let mut rng = thread_rng();
                    let armour = rng.gen_range(min_mac..=max_mac);
                    dmg = dmg.saturating_sub(armour);
                }

                if dmg <= 0 {
                    0
                } else {
                    let dr_percent = stats.get(Stat::DamageReductionPercent);
                    if dr_percent > 0 {
                        let clamped = dr_percent.clamp(0, 95);
                        dmg = dmg.saturating_mul(100 - clamped) / 100;
                    }
                    dmg
                }
            };

            if dmg <= 0 {
                continue;
            }

            trained = true;

            world.apply_player_hit_from_player(
                session_id,
                sid,
                map_index,
                dmg,
                0,
                None,
                None,
                None,
                true,
                events,
            );
        }
    }

    if trained {
        world.level_up_magic_for_player(session_id, spell, events);
    }

    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x: center_x,
        y: center_y,
        direction,
        spell,
        level,
        attack_type: 0,
    });
}

pub fn cast_thunder_storm_flame_field<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    events: &mut Vec<WorldEvent>,
) {
    let (map_index, player_x, player_y, level, attacker_stats, drop_rate) = {
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

        (
            player.map_index,
            player.x,
            player.y,
            level,
            player.stats.total.clone(),
            world.drop_rate,
        )
    };

    let damage_base =
        compute_pure_magic_attack_damage(&world.provider, &attacker_stats, spell, level);

    // ThunderStorm/FlameField range: 5x5 around the caster (range=2).
    let range = 2;
    let min_x = player_x - range;
    let max_x = player_x + range;
    let min_y = player_y - range;
    let max_y = player_y + range;

    let mut trained = false;

    // As with FireBang/IceStorm, route monster damage through
    // PendingMagicHit so that death, experience and drops are handled by the
    // shared monster-runtime pipeline. Apply the C# ThunderStorm special
    // rule (1/10 damage to non-undead) per monster before scheduling the hit.
    if damage_base > 0 {
        let base_time = world.time_ms.max(0);

        if let Some(monsters) = world.monsters.get(&map_index) {
            for m in monsters.iter() {
                if m.hp <= 0 {
                    continue;
                }

                if m.x < min_x || m.x > max_x || m.y < min_y || m.y > max_y {
                    continue;
                }

                if !world.can_attack_monster(session_id, map_index, m.id) {
                    continue;
                }

                let mut damage = damage_base;

                if spell == Spell::ThunderStorm as u8 {
                    let is_undead = if let Some(info) = world
                        .provider
                        .get_monster_info(m.monster_index)
                    {
                        info.undead
                    } else {
                        false
                    };

                    if !is_undead {
                        damage = damage / 10;
                    }
                }

                if damage <= 0 {
                    continue;
                }

                trained = true;

                let due_time_ms = base_time;
                world.pending_magic_hits.push(PendingMagicHit {
                    due_time_ms,
                    attacker_session_id: session_id,
                    map_index,
                    target_monster_id: m.id,
                    monster_index: m.monster_index,
                    spell_id: spell,
                    damage,
                    damage_type: 0,
                });
            }
        }
    }

    if trained {
        world.level_up_magic_for_player(session_id, spell, events);
    }

    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x: player_x,
        y: player_y,
        direction,
        spell,
        level,
        attack_type: 0,
    });
}

pub fn cast_lightning<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    events: &mut Vec<WorldEvent>,
) {
    let (map_index, player_x, player_y, level, attacker_stats) = {
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

        (
            player.map_index,
            player.x,
            player.y,
            level,
            player.stats.total.clone(),
        )
    };

    let damage_base =
        compute_pure_magic_attack_damage(&world.provider, &attacker_stats, spell, level);

    if damage_base <= 0 {
        return;
    }

    // Drive the Lightning spell animation and effects on the C# client by
    // mirroring the original server's S.Magic + S.ObjectMagic pattern: send
    // a Magic event to the caster and an ObjectMagic event to nearby
    // viewers. Lightning itself does not rely on a specific target point for
    // visuals, only the caster's location and facing direction.
    events.push(WorldEvent::ObjectMagic {
        session_id,
        map_index,
        x: player_x,
        y: player_y,
        direction,
        spell,
        level,
        target_id: 0,
        target_x: player_x,
        target_y: player_y,
    });

    events.push(WorldEvent::Magic {
        session_id,
        spell_id: spell,
        target_id: 0,
        x: player_x,
        y: player_y,
        cast: true,
        level,
        secondary_target_ids: Vec::new(),
    });

    // Map Lightning range: up to 6 tiles in a straight line from the caster,
    // using the current facing direction. This mirrors the C# Map.Lightning
    // logic which walks 6 steps via Functions.PointMove.
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

    let map = match world.get_or_load_map(map_index) {
        Some(m) => m,
        None => return,
    };
    let width = map.width;
    let height = map.height;

    let mut trained = false;
    let mut monster_hits: Vec<(u64, i32)> = Vec::new();
    let mut player_hits: Vec<SessionId> = Vec::new();

    let mut cx = player_x;
    let mut cy = player_y;

    // Walk up to 6 tiles along the direction, attempting to hit the first
    // Monster or Player in each tile that satisfies IsAttackTarget.
    for _ in 0..6 {
        cx = cx.saturating_add(dx);
        cy = cy.saturating_add(dy);

        if cx < 0 || cy < 0 {
            continue;
        }

        let ux = cx as u16;
        let uy = cy as u16;
        if ux >= width || uy >= height {
            continue;
        }

        let mut hit_any = false;

        if let Some(monsters) = world.monsters.get(&map_index) {
            if let Some(m) = monsters
                .iter()
                .find(|m| {
                    m.hp > 0
                        && m.x == cx
                        && m.y == cy
                        && world.can_attack_monster(session_id, map_index, m.id)
                })
            {
                monster_hits.push((m.id, m.monster_index));
                hit_any = true;
            }
        }

        if hit_any {
            continue;
        }

        let mut target_player: Option<SessionId> = None;
        for (&sid, p) in &world.players {
            if p.map_index != map_index || p.dead || p.hp <= 0 {
                continue;
            }

            if p.x != cx || p.y != cy {
                continue;
            }

            if !world.can_attack_player(session_id, sid) {
                continue;
            }

            target_player = Some(sid);
            break;
        }

        if let Some(sid) = target_player {
            player_hits.push(sid);
        }
    }

    let base_time = world.time_ms.max(0);
    // C# HumanObject.Lightning schedules a DelayedAction at
    // Envir.Time + 500; mirror that here so that damage, death and
    // drops are applied ~500ms after the cast, keeping them in sync
    // with the spell animation on the client.
    let delay_ms: i64 = 500;
    let due_time_ms = base_time.saturating_add(delay_ms);

    for (monster_id, monster_index) in monster_hits {
        trained = true;
        world.pending_magic_hits.push(PendingMagicHit {
            due_time_ms,
            attacker_session_id: session_id,
            map_index,
            target_monster_id: monster_id,
            monster_index,
            spell_id: spell,
            damage: damage_base,
            damage_type: 0,
        });
    }

    for sid in player_hits {
        let dmg = {
            let p = match world.players.get(&sid) {
                Some(p) => p,
                None => continue,
            };

            let stats = &p.stats.total;

            let min_mac = stats.get(Stat::MinMAC).max(0);
            let max_mac = stats.get(Stat::MaxMAC).max(min_mac);
            let mut dmg = damage_base;

            if max_mac > 0 {
                let mut rng = thread_rng();
                let armour = rng.gen_range(min_mac..=max_mac);
                dmg = dmg.saturating_sub(armour);
            }

            if dmg <= 0 {
                0
            } else {
                let dr_percent = stats.get(Stat::DamageReductionPercent);
                if dr_percent > 0 {
                    let clamped = dr_percent.clamp(0, 95);
                    dmg = dmg.saturating_mul(100 - clamped) / 100;
                }
                dmg
            }
        };

        if dmg <= 0 {
            continue;
        }

        trained = true;

        world.apply_player_hit_from_player(
            session_id,
            sid,
            map_index,
            dmg,
            0,
            None,
            None,
            None,
            true,
            events,
        );
    }

    if trained {
        world.level_up_magic_for_player(session_id, spell, events);
    }

    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x: player_x,
        y: player_y,
        direction,
        spell,
        level,
        attack_type: 0,
    });
}

pub fn cast_hell_fire<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    events: &mut Vec<WorldEvent>,
) {
    let (map_index, player_x, player_y, level, attacker_stats) = {
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

        (
            player.map_index,
            player.x,
            player.y,
            level,
            player.stats.total.clone(),
        )
    };

    let damage_base =
        compute_pure_magic_attack_damage(&world.provider, &attacker_stats, spell, level);

    if damage_base <= 0 {
        return;
    }

    let map = match world.get_or_load_map(map_index) {
        Some(m) => m,
        None => return,
    };
    let width = map.width;
    let height = map.height;

    // Primary direction plus side directions at level 3, mirroring the C#
    // HellFire behaviour where level 3 also emits rays at Direction+1 and
    // Direction-1.
    let mut dirs: Vec<u8> = vec![direction];
    if level == 3 {
        dirs.push(direction.wrapping_add(1) & 7);
        dirs.push(direction.wrapping_add(7) & 7);
    }

    let base_time = world.time_ms.max(0);
    // C# HumanObject.HellFire also uses a fixed 500ms delay between
    // cast and Map.HellFire damage application. Use the same delay
    // here so that line damage and visual effects remain aligned.
    let delay_ms: i64 = 500;
    let due_time_ms = base_time.saturating_add(delay_ms);

    let mut trained = false;

    for dir in dirs {
        let (dx, dy) = match dir {
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

        let mut cx = player_x;
        let mut cy = player_y;

        // Up to 4 tiles along each ray, matching the count parameter used by
        // the C# HumanObject.HellFire/Map.HellFire path.
        for _ in 0..4 {
            cx = cx.saturating_add(dx);
            cy = cy.saturating_add(dy);

            if cx < 0 || cy < 0 {
                break;
            }

            let ux = cx as u16;
            let uy = cy as u16;
            if ux >= width || uy >= height {
                break;
            }

            let mut hit_any = false;

            if let Some(monsters) = world.monsters.get(&map_index) {
                if let Some(m) = monsters
                    .iter()
                    .find(|m| {
                        m.hp > 0
                            && m.x == cx
                            && m.y == cy
                            && world.can_attack_monster(session_id, map_index, m.id)
                    })
                {
                    trained = true;
                    world.pending_magic_hits.push(PendingMagicHit {
                        due_time_ms,
                        attacker_session_id: session_id,
                        map_index,
                        target_monster_id: m.id,
                        monster_index: m.monster_index,
                        spell_id: spell,
                        damage: damage_base,
                        damage_type: 0,
                    });
                    hit_any = true;
                }
            }

            if hit_any {
                continue;
            }

            let mut target_player: Option<SessionId> = None;
            for (&sid, p) in &world.players {
                if p.map_index != map_index || p.dead || p.hp <= 0 {
                    continue;
                }

                if p.x != cx || p.y != cy {
                    continue;
                }

                if !world.can_attack_player(session_id, sid) {
                    continue;
                }

                target_player = Some(sid);
                break;
            }

            if let Some(sid) = target_player {
                let dmg = {
                    let p = match world.players.get(&sid) {
                        Some(p) => p,
                        None => continue,
                    };

                    let stats = &p.stats.total;

                    let min_mac = stats.get(Stat::MinMAC).max(0);
                    let max_mac = stats.get(Stat::MaxMAC).max(min_mac);
                    let mut dmg = damage_base;

                    if max_mac > 0 {
                        let mut rng = thread_rng();
                        let armour = rng.gen_range(min_mac..=max_mac);
                        dmg = dmg.saturating_sub(armour);
                    }

                    if dmg <= 0 {
                        0
                    } else {
                        let dr_percent = stats.get(Stat::DamageReductionPercent);
                        if dr_percent > 0 {
                            let clamped = dr_percent.clamp(0, 95);
                            dmg = dmg.saturating_mul(100 - clamped) / 100;
                        }
                        dmg
                    }
                };

                if dmg <= 0 {
                    continue;
                }

                trained = true;

                world.apply_player_hit_from_player(
                    session_id,
                    sid,
                    map_index,
                    dmg,
                    0,
                    None,
                    None,
                    None,
                    true,
                    events,
                );
            }
        }
    }

    if trained {
        world.level_up_magic_for_player(session_id, spell, events);
    }

    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x: player_x,
        y: player_y,
        direction,
        spell,
        level,
        attack_type: 0,
    });
}

pub fn cast_fire_wall<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    target_x: i32,
    target_y: i32,
    events: &mut Vec<WorldEvent>,
) {
    let (map_index, player_x, player_y, level, value) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => return,
        };

        let level = magic.level;
        let stats = player.stats.total.clone();
        let cost = match compute_magic_mana_cost(&world.provider, &stats, spell, level)
        {
            Some(c) => c,
            None => return,
        };

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        // Approximate C# FireWall damage using the same pure magic helper that
        // other wizard attack spells use: DamageBase from MC and MagicInfo,
        // then use that as the per-tick Value for the wall.
        let dmg = compute_pure_magic_attack_damage(&world.provider, &stats, spell, level)
            .max(0);

        (player.map_index, player.x, player.y, level, dmg)
    };

    if value <= 0 {
        return;
    }

    // Determine the FireWall tiles: centre + four cardinal neighbours around
    // the clicked location, mirroring the C# Map.FireWall placement which
    // uses Functions.PointMove with Up/Right/Down/Left.
    let map = match world.get_or_load_map(map_index) {
        Some(m) => m,
        None => return,
    };

    let fw_spell_id = Spell::FireWall as u8;

    let candidates = [
        (target_x, target_y),                // centre
        (target_x, target_y - 1),            // up
        (target_x + 1, target_y),            // right
        (target_x, target_y + 1),            // down
        (target_x - 1, target_y),            // left
    ];

    let mut cells: Vec<(i32, i32)> = Vec::new();

    for (x, y) in candidates {
        if x < 0 || y < 0 {
            continue;
        }

        let ux = x as u16;
        let uy = y as u16;

        if ux >= map.width || uy >= map.height {
            continue;
        }

        if !map.is_walkable(ux, uy) {
            continue;
        }

        // Avoid stacking multiple FireWall instances on the same tile,
        // approximating the C# Cell.Objects Spell.FireWall check.
        if world.cell_has_spell(map_index, x, y, fw_spell_id) {
            continue;
        }

        cells.push((x, y));
    }

    if cells.is_empty() {
        return;
    }

    // Lifetime and tick timing: ExpireTime = now + (10 + value/2) * 1000,
    // TickSpeed = 2000ms, matching the C# SpellObject fields.
    let base_time = world.time_ms.max(0);
    let duration_secs: i64 = (10_i64)
        .saturating_add((value as i64) / 2)
        .max(1);
    let expire_time_ms = base_time.saturating_add(duration_secs.saturating_mul(1_000));

    tracing::debug!(
        "[wizard] FireWall cast: value={} duration_secs={} expire_time_ms={} (base_time={})",
        value,
        duration_secs,
        expire_time_ms,
        base_time,
    );
    let tick_speed_ms: i64 = 2_000;
    let next_tick_ms = base_time.saturating_add(tick_speed_ms);

    for &(x, y) in &cells {
        world.add_map_spell(map_index, x, y, fw_spell_id);
        events.push(WorldEvent::MapSpellAdded {
            map_index,
            x,
            y,
            spell: fw_spell_id,
            direction,
            param: false,
        });
    }

    world.fire_walls.push(crate::world::world::FireWallInstance {
        map_index,
        caster_session_id: session_id,
        value,
        cells: cells.clone(),
        expire_time_ms,
        tick_speed_ms,
        next_tick_ms,
    });

    world.level_up_magic_for_player(session_id, Spell::FireWall as u8, events);

    events.push(WorldEvent::ObjectMagic {
        session_id,
        map_index,
        x: player_x,
        y: player_y,
        direction,
        spell,
        level,
        target_id: 0,
        target_x,
        target_y,
    });
}

/// Cast MagicShield for a wizard, applying a temporary damage reduction buff
/// and training the magic on success. This mirrors the original
/// handle_magic_shield_spell logic from combat.rs.
pub fn cast_magic_shield<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    events: &mut Vec<WorldEvent>,
) {
    let spell_id = Spell::MagicShield as u8;
    let (duration_ms, stats) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        tracing::debug!(
            "MagicShield: cast_magic_shield enter session_id={} mp={} buff_count={}",
            session_id,
            player.mp,
            player.active_buffs.len(),
        );

        // Do not stack MagicShield: if already active, ignore the cast
        // without consuming MP, mirroring the C# behaviour.
        if player
            .active_buffs
            .iter()
            .any(|b| b.buff_type == BuffType::MagicShield)
        {
            tracing::debug!(
                "MagicShield: cast aborted, buff already active for session_id={}",
                session_id
            );
            return;
        }

        let magic = match player.magics.iter().find(|m| m.spell == spell_id) {
            Some(m) => m,
            None => {
                tracing::debug!(
                    "MagicShield: cast aborted, player has no magic entry for spell_id={} session_id={}",
                    spell_id,
                    session_id
                );
                return;
            }
        };

        let level = magic.level;
        // 对 MagicShield 而言，如果 MirDB 里暂时缺少 MagicInfo，我们不直接
        // 中断施法，而是把消耗视为 0，这样技能仍然可以生效并挂上 Buff，行为
        // 与 Healing/MassHealing 的容错方式一致。
        let cost = compute_magic_mana_cost(&world.provider, &player.stats.total, spell_id, level)
            .unwrap_or(0);

        tracing::debug!(
            "MagicShield: computed mana cost session_id={} level={} mp_before={} cost={}",
            session_id,
            level,
            player.mp,
            cost
        );

        if player.mp < cost {
            tracing::debug!(
                "MagicShield: cast aborted, insufficient MP session_id={} mp={} cost={}",
                session_id,
                player.mp,
                cost
            );
            return;
        }

        player.mp -= cost;

        tracing::debug!(
            "MagicShield: MP deducted session_id={} mp_after={} cost={}",
            session_id,
            player.mp,
            cost
        );

        // Approximate C# MagicShield duration by mirroring
        //   magic.GetPower(GetAttackPower(MinMC, MaxMC) + 15)
        // and then using that value as a number of seconds.
        let min_mc = player.stats.total.get(Stat::MinMC).max(0);
        let max_mc = player.stats.total.get(Stat::MaxMC).max(min_mc);

        let mut rng = thread_rng();
        let attack_roll = if max_mc > min_mc {
            rng.gen_range(min_mc..=max_mc)
        } else {
            min_mc
        };

        let base_power = attack_roll.saturating_add(15);

        let mut duration_sec: i64 = 0;
        if let Some(info) = world.provider.get_magic_info(spell_id) {
            let power = magic_power_with_base(info, level, base_power, &mut rng).max(1);
            duration_sec = power as i64;
        }
        // 正常情况下，根据 MagicInfo 和 GetAttackPower 计算出的 GetPower
        // 应该始终为正值；这里仅在极端情况下（例如 MirDB 缺少 MagicInfo）
        // 做一个保底，避免持续时间为 0 或负数。
        if duration_sec <= 0 {
            duration_sec = 60;
        }

        let duration_ms = duration_sec.saturating_mul(1_000);

        let mut stats = Stats::default();
        let dr = (level as i32 + 2) * 10;
        stats.set(Stat::DamageReductionPercent, dr);

        (duration_ms, stats)
    };

    world.add_player_buff(
        session_id,
        BuffType::MagicShield,
        duration_ms,
        stats,
        Vec::new(),
        events,
    );

    world.level_up_magic_for_player(session_id, spell_id, events);
}
