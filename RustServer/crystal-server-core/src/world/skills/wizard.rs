use crate::stats::{Stat, Stats};
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
    let (map_index, level, attacker_stats, player_x, player_y) = {
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
            player.x,
            player.y,
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

    events.push(WorldEvent::ObjectMagic {
        session_id,
        map_index,
        x: player_x,
        y: player_y,
        direction,
        spell,
        level,
        target_id: 0,
        target_x: center_x,
        target_y: center_y,
    });

    events.push(WorldEvent::Magic {
        session_id,
        spell_id: spell,
        target_id: 0,
        x: center_x,
        y: center_y,
        cast: true,
        level,
        secondary_target_ids: Vec::new(),
    });
}

pub fn cast_thunder_storm_flame_field<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    events: &mut Vec<WorldEvent>,
) {
    let (map_index, player_x, player_y, level, attacker_stats, _drop_rate) = {
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

/// Cast Blizzard for a wizard, mirroring the legacy C# behaviour where the
/// spell creates a 5x5 area of Spell.Blizzard SpellObjects that persist for
/// ~3 seconds and tick damage several times. We approximate this using the
/// existing FireWall-style map-spell pipeline plus PendingMagicHit so that
/// monster damage, drops and experience reuse the common runtime.
pub fn cast_blizzard<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    center_x: i32,
    center_y: i32,
    events: &mut Vec<WorldEvent>,
) {
    let (map_index, level, attacker_stats, player_x, player_y) = {
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
            player.x,
            player.y,
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

    // Map dimensions are stored as u16; convert to i32 so we can safely
    // compare against the i32 Blizzard coordinates.
    let width: i32 = map.width as i32;
    let height: i32 = map.height as i32;

    // Blizzard range: 5x5 square centred on the target location.
    let range = 2;
    let min_x = center_x - range;
    let max_x = center_x + range;
    let min_y = center_y - range;
    let max_y = center_y + range;

    let spell_id = Spell::Blizzard as u8;
    let mut cells: Vec<(i32, i32)> = Vec::new();

    for y in min_y..=max_y {
        if y < 0 || y >= height {
            continue;
        }

        for x in min_x..=max_x {
            if x < 0 || x >= width {
                continue;
            }

            // Avoid stacking multiple Blizzard instances on the same tile,
            // approximating the C# Cell.Objects Spell.Blizzard check.
            if world.cell_has_spell(map_index, x, y, spell_id) {
                continue;
            }

            cells.push((x, y));
        }
    }

    if cells.is_empty() {
        return;
    }

    let base_time = world.time_ms.max(0);
    // Approximate C# SpellObject lifetime: ExpireTime = now + 3000ms.
    let expire_time_ms = base_time.saturating_add(3_000);
    // Approximate SpellObject.TickSpeed = 440ms.
    let tick_speed_ms: i64 = 440;
    let next_tick_ms = base_time.saturating_add(tick_speed_ms);

    for (i, &(x, y)) in cells.iter().enumerate() {
        world.add_map_spell(map_index, x, y, spell_id);
        events.push(WorldEvent::MapSpellAdded {
            map_index,
            x,
            y,
            spell: spell_id,
            direction,
            // param mirrors SpellObject.Show: true only for the first tile.
            param: i == 0,
        });
    }

    // Schedule periodic damage ticks against monsters standing in any of the
    // Blizzard tiles by reusing the PendingMagicHit pipeline.
    let mut blizzard_cells = cells.clone();
    blizzard_cells.shrink_to_fit();

    world.fire_walls.push(crate::world::world::FireWallInstance {
        map_index,
        caster_session_id: session_id,
        value: damage_base,
        spell_id,
        cells: blizzard_cells,
        expire_time_ms,
        tick_speed_ms,
        next_tick_ms,
    });

    world.level_up_magic_for_player(session_id, spell, events);

    // Drive caster animation and local cooldown via ObjectMagic + Magic,
    // mirroring the FireBang/IceStorm pattern.
    events.push(WorldEvent::ObjectMagic {
        session_id,
        map_index,
        x: player_x,
        y: player_y,
        direction,
        spell,
        level,
        target_id: 0,
        target_x: center_x,
        target_y: center_y,
    });

    events.push(WorldEvent::Magic {
        session_id,
        spell_id: spell,
        target_id: 0,
        x: center_x,
        y: center_y,
        cast: true,
        level,
        secondary_target_ids: Vec::new(),
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
        spell_id: Spell::FireWall as u8,
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

/// Cast Blink for a wizard, mirroring the C# HumanObject Blink behaviour:
/// short-range teleport within the current map with a chance based on magic
/// level, obeying map NoTeleport, consuming MP (including any active
/// TemporalFlux penalty) and applying a 30s TemporalFlux buff on success.
pub fn cast_blink<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    _direction: u8,
    target_x: i32,
    target_y: i32,
    events: &mut Vec<WorldEvent>,
) {
    let (map_index, player_x, player_y, level) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => return,
        };

        // Respect map NoTeleport flag in the same way as the C# server:
        // if teleporting is forbidden on this map, abort the cast after
        // cooldown/MP checks without moving the player.
        if let Some(info) = world.provider.get_map_info(player.map_index) {
            if info.no_teleport {
                return;
            }
        }

        let level = magic.level;
        let cost =
            match compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level) {
                Some(c) => c,
                None => return,
            };

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        (player.map_index, player.x, player.y, level)
    };

    // Enforce the MagicInfo.Range limit using a Chebyshev distance check,
    // mirroring Functions.InRange(CurrentLocation, location, magic.Info.Range)
    // on the C# server.
    let info = match world.provider.get_magic_info(spell) {
        Some(i) => i,
        None => return,
    };
    let range: i32 = info.range as i32;
    let dx = (target_x - player_x).abs();
    let dy = (target_y - player_y).abs();
    if dx.max(dy) > range {
        return;
    }

    // Validate that the target cell is inside the map bounds and walkable.
    let map = match world.get_or_load_map(map_index) {
        Some(m) => m,
        None => return,
    };

    if target_x < 0 || target_y < 0 {
        return;
    }

    let ux = target_x as u16;
    let uy = target_y as u16;
    if ux >= map.width || uy >= map.height || !map.is_walkable(ux, uy) {
        return;
    }

    // Apply the Blink success chance: Envir.Random.Next(4) >= magic.Level + 1
    // results in failure on the C# server.
    let mut rng = thread_rng();
    let roll: u8 = rng.gen_range(0..4);
    if roll >= level.saturating_add(1) {
        return;
    }

    // Perform an in-map teleport by updating the player's coordinates and
    // occupancy, then emit a UserLocation event so the client updates the
    // local player position. This approximates the behaviour of
    // MapObject.Teleport for same-map moves.
    let (old_x, old_y, new_x, new_y, dir) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        if player.map_index != map_index {
            return;
        }

        let old_x = player.x;
        let old_y = player.y;
        player.x = target_x;
        player.y = target_y;

        (old_x, old_y, player.x, player.y, player.direction)
    };

    world.remove_player_from_occupancy(session_id, map_index, old_x, old_y);
    world.add_player_to_occupancy(session_id, map_index, new_x, new_y);

    events.push(WorldEvent::UserLocation {
        session_id,
        map_index,
        x: new_x,
        y: new_y,
        direction: dir,
    });

    // SpellEffect.Teleport visual for Blink (enum value 2 in C# SpellEffect).
    events.push(WorldEvent::ObjectEffect {
        session_id,
        effect: 2,
    });

    // Apply TemporalFlux for 30 seconds so subsequent Teleport/Blink/StormEscape
    // casts incur the TeleportManaPenaltyPercent, and train the magic level.
    world.level_up_magic_for_player(session_id, spell, events);

    let duration_ms: i64 = 30_000;
    let mut stats = Stats::default();
    stats.set(Stat::TeleportManaPenaltyPercent, 30);

    world.add_player_buff(
        session_id,
        BuffType::TemporalFlux,
        duration_ms,
        stats,
        Vec::new(),
        events,
    );
}

pub fn cast_storm_escape<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    _direction: u8,
    target_x: i32,
    target_y: i32,
    events: &mut Vec<WorldEvent>,
) {
    let (map_index, level) = {
        let player = match world.players.get(&session_id) {
            Some(p) => p,
            None => return,
        };

        if let Some(info) = world.provider.get_map_info(player.map_index) {
            if info.no_teleport {
                events.push(WorldEvent::PartySystemMessage {
                    session_id,
                    message: "You cannot teleport on this map".to_string(),
                });
                return;
            }
        }

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => return,
        };

        (player.map_index, magic.level)
    };

    let map = match world.get_or_load_map(map_index) {
        Some(m) => m,
        None => return,
    };

    if target_x < 0 || target_y < 0 {
        return;
    }

    let ux = target_x as u16;
    let uy = target_y as u16;
    if ux >= map.width || uy >= map.height || !map.is_walkable(ux, uy) {
        return;
    }

    let mut rng = thread_rng();
    let roll: u8 = rng.gen_range(0..4);
    if roll >= level.saturating_add(1) {
        return;
    }

    let (old_x, old_y, new_x, new_y, dir) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        if player.map_index != map_index {
            return;
        }

        let old_x = player.x;
        let old_y = player.y;
        player.x = target_x;
        player.y = target_y;

        (old_x, old_y, player.x, player.y, player.direction)
    };

    world.remove_player_from_occupancy(session_id, map_index, old_x, old_y);
    world.add_player_to_occupancy(session_id, map_index, new_x, new_y);

    events.push(WorldEvent::UserLocation {
        session_id,
        map_index,
        x: new_x,
        y: new_y,
        direction: dir,
    });

    world.level_up_magic_for_player(session_id, spell, events);

    let duration_ms: i64 = 30_000;
    let mut stats = Stats::default();
    stats.set(Stat::TeleportManaPenaltyPercent, 30);

    world.add_player_buff(
        session_id,
        BuffType::TemporalFlux,
        duration_ms,
        stats,
        Vec::new(),
        events,
    );

    events.push(WorldEvent::ObjectEffect {
        session_id,
        effect: 29,
    });
}

/// Cast the wizard Teleport spell: consume MP (including any active
/// TeleportManaPenaltyPercent), respect map.Info.NoTeleport and, on success,
/// request that the connection perform a bind-based teleport using the
/// C# MagicTeleport algorithm. The actual random destination selection and
/// Teleport movement are handled in the connection layer.
pub fn cast_teleport<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    _direction: u8,
    _target_x: i32,
    _target_y: i32,
    events: &mut Vec<WorldEvent>,
) {
    let (map_index, level) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => return,
        };

        if let Some(info) = world.provider.get_map_info(player.map_index) {
            if info.no_teleport {
                events.push(WorldEvent::PartySystemMessage {
                    session_id,
                    message: "You cannot teleport on this map".to_string(),
                });
                return;
            }
        }

        let level = magic.level;
        let cost =
            match compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level) {
                Some(c) => c,
                None => return,
            };

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        (player.map_index, level)
    };

    let _ = map_index; // map_index is kept for potential future use.

    events.push(WorldEvent::TeleportToBindRequested {
        session_id,
        spell_id: spell,
        level,
    });
}

/// Cast Vampirism skill - deals magic damage and accumulates healing
/// C#: HumanObject.Vampirism - DelayedAction deals damage, then accumulates VampAmount
pub fn cast_vampirism<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    x: i32,
    y: i32,
    events: &mut Vec<WorldEvent>,
) {
    use crate::world::magic::magic_damage;
    use rand::Rng;

    let spell_id = Spell::Vampirism as u8;

    // First, perform all mutable operations and collect data
    let (map_index, magic_level, attacker_stats) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let now_ms = world.time_ms;
        if player.dead
            || (player.next_action_time_ms != 0 && now_ms < player.next_action_time_ms)
        {
            return;
        }

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
        player.direction = direction;
        player.next_action_time_ms = now_ms.saturating_add(600); // C# has 600ms action delay

        (
            player.map_index,
            level,
            player.stats.total.clone(),
        )
    };

    // Now find targets (after releasing mutable borrow)
    let map_index_for_target = map_index;
    
    // First, collect monster candidates
    let monster_candidates: Vec<u64> = {
        if let Some(monsters) = world.monsters.get(&map_index_for_target) {
            monsters.iter()
                .filter(|m| m.hp > 0 && m.x == x && m.y == y)
                .map(|m| m.id)
                .collect()
        } else {
            Vec::new()
        }
    };
    
    // Check if can attack monsters
    let target_monster_id = monster_candidates.into_iter()
        .find(|&monster_id| world.can_attack_monster(session_id, map_index_for_target, monster_id));
    
    // Collect player candidates
    let player_candidates: Vec<SessionId> = if target_monster_id.is_none() {
        world.players.iter()
            .filter(|(target_sid, target_player)| {
                **target_sid != session_id
                    && target_player.map_index == map_index_for_target
                    && target_player.x == x
                    && target_player.y == y
                    && !target_player.dead
                    && target_player.hp > 0
            })
            .map(|(target_sid, _)| *target_sid)
            .collect()
    } else {
        Vec::new()
    };
    
    // Check if can attack players
    let target_player_id = player_candidates.into_iter()
        .find(|&target_sid| world.can_attack_player(session_id, target_sid));

    // Schedule delayed damage application for monsters
    if let Some(monster_id) = target_monster_id {
        let delay_ms: i64 = 500;
        let due_time_ms = world.time_ms.saturating_add(delay_ms);

        world.pending_magic_hits.push(PendingMagicHit {
            due_time_ms,
            attacker_session_id: session_id,
            map_index,
            target_monster_id: monster_id,
            monster_index: -4, // Special marker for Vampirism
            spell_id,
            damage: 0, // Will be calculated in delayed action
            damage_type: crate::world::types::DamageType::Magical as u8,
        });
    } else if let Some(target_sid) = target_player_id {
        // For player targets, calculate and apply damage immediately
        let magic_info = match world.provider.get_magic_info(spell_id) {
            Some(info) => info,
            None => {
                world.record_magic_cast_time(session_id, spell_id);
                events.push(WorldEvent::MagicCast {
                    session_id,
                    spell_id,
                });
                return;
            }
        };

        let min_mc = attacker_stats.get(Stat::MinMC).max(0);
        let max_mc = attacker_stats.get(Stat::MaxMC).max(min_mc);
        let mut rng = thread_rng();
        let attack_power = if max_mc > min_mc {
            rng.gen_range(min_mc..=max_mc)
        } else {
            min_mc
        };

        let damage = magic_damage(&magic_info, magic_level, attack_power, &mut rng);

        let (damage_taken, _) = crate::world::combat::damage::apply_damage_to_player(
            world,
            Some(session_id),
            target_sid,
            damage,
            crate::world::types::DamageType::Magical,
            map_index,
            events,
        );
        
        if damage_taken > 0 {
            // Accumulate vamp amount
            if let Some(player) = world.players.get_mut(&session_id) {
                if player.vamp_amount == 0 {
                    player.vamp_time_ms = world.time_ms.saturating_add(1000);
                }
                let vamp_gain = ((damage_taken as f32) * (magic_level as f32 + 1.0) * 0.25) as u16;
                player.vamp_amount = player.vamp_amount.saturating_add(vamp_gain);
            }
            
            world.level_up_magic_for_player(session_id, spell_id, events);
        }
    } else {
        // No valid target
        world.record_magic_cast_time(session_id, spell_id);
        events.push(WorldEvent::MagicCast {
            session_id,
            spell_id,
        });
        return;
    }

    world.record_magic_cast_time(session_id, spell_id);
    events.push(WorldEvent::MagicCast {
        session_id,
        spell_id,
    });
}

/// Cast Repulsion skill - pushes back targets within 1-tile radius
/// C#: HumanObject.Repulsion - pushes targets away from caster
pub fn cast_repulsion<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    x: i32,
    y: i32,
    events: &mut Vec<WorldEvent>,
) {
    use crate::world::magic::magic_damage;
    use crate::world::skills::direction_from_point;
    use rand::Rng;

    let spell_id = Spell::Repulsion as u8;

    let (map_index, caster_x, caster_y, caster_level, magic_level) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let now_ms = world.time_ms;
        if player.dead
            || (player.next_action_time_ms != 0 && now_ms < player.next_action_time_ms)
        {
            return;
        }

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
        player.direction = direction;
        player.next_action_time_ms = now_ms.saturating_add(600); // C# has 600ms action delay

        (
            player.map_index,
            player.x,
            player.y,
            player.level,
            level,
        )
    };

    let map = match world.get_or_load_map(map_index) {
        Some(m) => m,
        None => {
            world.record_magic_cast_time(session_id, spell_id);
            events.push(WorldEvent::MagicCast {
                session_id,
                spell_id,
            });
            return;
        }
    };

    let mut result = false;
    let mut rng = thread_rng();

    // C#: for (int d = 0; d <= 1; d++) - check radius 0 and 1
    for d in 0..=1 {
        for dy in -d..=d {
            let ty = caster_y + dy;
            if ty < 0 || ty >= map.height as i32 {
                continue;
            }

            // C#: x += Math.Abs(y - CurrentLocation.Y) == d ? 1 : d * 2
            // This creates a diamond pattern: at d=0, check center; at d=1, check surrounding 8 tiles
            let x_step = if d == 0 { 1 } else if dy.abs() == d { 1 } else { d * 2 };
            let mut dx_current = -d;
            while dx_current <= d {
                let tx = caster_x + dx_current;
                if tx < 0 || tx >= map.width as i32 {
                    dx_current += x_step;
                    continue;
                }

                // Skip caster's own position
                if tx == caster_x && ty == caster_y {
                    dx_current += x_step;
                    continue;
                }

                // Collect targets at this location
                let mut player_targets: Vec<SessionId> = Vec::new();
                let mut monster_targets: Vec<(u64, i32, u16)> = Vec::new(); // (id, monster_index, level)

                // Check players
                for (target_sid, target_player) in world.players.iter() {
                    if target_sid == &session_id {
                        continue; // Skip self
                    }
                    if target_player.map_index != map_index || target_player.x != tx || target_player.y != ty {
                        continue;
                    }
                    if target_player.dead || target_player.hp <= 0 {
                        continue;
                    }
                    // C#: if (!ob.IsAttackTarget(this) || ob.Level >= Level) continue;
                    if !world.can_attack_player(session_id, *target_sid) {
                        continue;
                    }
                    if target_player.level >= caster_level {
                        continue;
                    }
                    player_targets.push(*target_sid);
                }

                // Check monsters
                if let Some(monsters) = world.monsters.get(&map_index) {
                    for monster in monsters.iter() {
                        if monster.hp <= 0 || monster.x != tx || monster.y != ty {
                            continue;
                        }
                        let monster_info = match world.provider.get_monster_info(monster.monster_index) {
                            Some(info) => info,
                            None => continue,
                        };
                        // C#: if (!ob.IsAttackTarget(this) || ob.Level >= Level) continue;
                        if monster_info.level >= caster_level {
                            continue;
                        }
                        monster_targets.push((monster.id, monster.monster_index, monster_info.level));
                    }
                }

                // Process player targets
                for target_sid in player_targets {
                    let target_level = {
                        world.players.get(&target_sid).map(|p| p.level).unwrap_or(0)
                    };

                    // C#: if (Envir.Random.Next(20) >= 6 + magic.Level * 3 + Level - ob.Level) continue;
                    let check = rng.gen_range(0..20);
                    let threshold = 6 + magic_level as i32 * 3 + caster_level as i32 - target_level as i32;
                    if check >= threshold {
                        dx_current += x_step;
                        continue;
                    }

                    // C#: int distance = 1 + Math.Max(0, magic.Level - 1) + Envir.Random.Next(2);
                    let distance = 1 + (magic_level as i32 - 1).max(0) + rng.gen_range(0..2);

                    // C#: MirDirection dir = Functions.DirectionFromPoint(CurrentLocation, ob.CurrentLocation);
                    let push_dir = direction_from_point(caster_x, caster_y, tx, ty);

                    // Push player
                    let (push_dx, push_dy) = match push_dir {
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

                    let mut push_x = tx;
                    let mut push_y = ty;
                    let mut pushed_distance = 0;

                    // Push step by step
                    for _step in 0..distance {
                        let next_x = push_x + push_dx;
                        let next_y = push_y + push_dy;

                        if next_x < 0 || next_y < 0 || next_x >= map.width as i32 || next_y >= map.height as i32 {
                            break;
                        }

                        let ux = next_x as u16;
                        let uy = next_y as u16;
                        if !map.is_walkable(ux, uy) {
                            break;
                        }

                        if let Some(info) = world.provider.get_map_info(map_index) {
                            if World::<P>::point_in_safe_zone(info, next_x, next_y) {
                                break;
                            }
                        }

                        if world.is_cell_blocked(map_index, next_x, next_y) {
                            break;
                        }

                        push_x = next_x;
                        push_y = next_y;
                        pushed_distance += 1;
                    }

                    if pushed_distance > 0 {
                        let (old_x, old_y) = {
                            world.players.get(&target_sid).map(|p| (p.x, p.y)).unwrap_or((tx, ty))
                        };

                        {
                            let target = match world.players.get_mut(&target_sid) {
                                Some(p) => p,
                                None => {
                                    dx_current += x_step;
                                    continue;
                                },
                            };
                            target.x = push_x;
                            target.y = push_y;
                            target.direction = push_dir;
                        }

                        world.remove_player_from_occupancy(target_sid, map_index, old_x, old_y);
                        world.add_player_to_occupancy(target_sid, map_index, push_x, push_y);

                        events.push(WorldEvent::PlayerPushed {
                            session_id: target_sid,
                            map_index,
                            x: push_x,
                            y: push_y,
                            direction: push_dir,
                        });

                        // C#: ob.Attacked(this, magic.GetDamage(0), DefenceType.None, false);
                        let magic_info = match world.provider.get_magic_info(spell_id) {
                            Some(info) => info,
                            None => {
                                dx_current += x_step;
                                continue;
                            },
                        };
                        let damage = magic_damage(&magic_info, magic_level, 0, &mut rng);

                        let (damage_taken, _) = crate::world::combat::damage::apply_damage_to_player(
                            world,
                            Some(session_id),
                            target_sid,
                            damage,
                            crate::world::types::DamageType::Physical, // C#: DefenceType.None, but we use Physical
                            map_index,
                            events,
                        );

                        if damage_taken > 0 {
                            result = true;
                        }
                    }
                }

                // Process monster targets
                for (monster_id, monster_index, monster_level) in monster_targets {
                    // C#: if (Envir.Random.Next(20) >= 6 + magic.Level * 3 + Level - ob.Level) continue;
                    let check = rng.gen_range(0..20);
                    let threshold = 6 + magic_level as i32 * 3 + caster_level as i32 - monster_level as i32;
                    if check >= threshold {
                        dx_current += x_step;
                        continue;
                    }

                    // C#: int distance = 1 + Math.Max(0, magic.Level - 1) + Envir.Random.Next(2);
                    let distance = 1 + (magic_level as i32 - 1).max(0) + rng.gen_range(0..2);

                    // C#: MirDirection dir = Functions.DirectionFromPoint(CurrentLocation, ob.CurrentLocation);
                    let push_dir = direction_from_point(caster_x, caster_y, tx, ty);

                    // Push monster
                    let (push_dx, push_dy) = match push_dir {
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

                    let mut push_x = tx;
                    let mut push_y = ty;
                    let mut pushed_distance = 0;

                    // Push step by step
                    for _step in 0..distance {
                        let next_x = push_x + push_dx;
                        let next_y = push_y + push_dy;

                        if next_x < 0 || next_y < 0 || next_x >= map.width as i32 || next_y >= map.height as i32 {
                            break;
                        }

                        let ux = next_x as u16;
                        let uy = next_y as u16;
                        if !map.is_walkable(ux, uy) {
                            break;
                        }

                        if let Some(info) = world.provider.get_map_info(map_index) {
                            if World::<P>::point_in_safe_zone(info, next_x, next_y) {
                                break;
                            }
                        }

                        if world.is_cell_blocked(map_index, next_x, next_y) {
                            break;
                        }

                        push_x = next_x;
                        push_y = next_y;
                        pushed_distance += 1;
                    }

                    if pushed_distance > 0 {
                        let (old_x, old_y) = {
                            if let Some(monsters) = world.monsters.get(&map_index) {
                                monsters.iter().find(|m| m.id == monster_id).map(|m| (m.x, m.y)).unwrap_or((tx, ty))
                            } else {
                                (tx, ty)
                            }
                        };

                        {
                            let monsters = match world.monsters.get_mut(&map_index) {
                                Some(ms) => ms,
                                None => {
                                    dx_current += x_step;
                                    continue;
                                },
                            };
                            if let Some(m) = monsters.iter_mut().find(|m| m.id == monster_id) {
                                m.x = push_x;
                                m.y = push_y;
                                m.direction = push_dir;
                            } else {
                                dx_current += x_step;
                                continue;
                            }
                        }

                        world.remove_monster_from_occupancy(monster_id, map_index, old_x, old_y);
                        world.add_monster_to_occupancy(monster_id, map_index, push_x, push_y);

                        events.push(WorldEvent::ObjectPushed {
                            object_id: monster_id,
                            map_index,
                            x: push_x,
                            y: push_y,
                            direction: push_dir,
                        });

                        result = true;
                    }
                }
                dx_current += x_step;
            }
        }
    }

    world.record_magic_cast_time(session_id, spell_id);
    events.push(WorldEvent::MagicCast {
        session_id,
        spell_id,
    });

    // C#: if (result) LevelMagic(magic);
    if result {
        world.level_up_magic_for_player(session_id, spell_id, events);
    }
}

/// Cast MagicBooster skill - adds MC buff and ManaPenaltyPercent debuff
/// C#: HumanObject.MagicBooster - DelayedAction adds buff with MinMC/MaxMC and ManaPenaltyPercent
pub fn cast_magic_booster<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    x: i32,
    y: i32,
    events: &mut Vec<WorldEvent>,
) {
    let spell_id = Spell::MagicBooster as u8;
    
    let (magic_level, bonus) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let now_ms = world.time_ms;
        if player.dead
            || (player.next_action_time_ms != 0 && now_ms < player.next_action_time_ms)
        {
            return;
        }

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

        // C#: int bonus = 6 + magic.Level * 6;
        let bonus = 6 + level as i32 * 6;

        (level, bonus)
    };

    // C#: ActionList.Add(new DelayedAction(DelayedType.Magic, Envir.Time + 500, magic, bonus));
    // Schedule delayed buff application
    let delay_ms: i64 = 500;
    let due_time_ms = world.time_ms.saturating_add(delay_ms);

    // Store the delayed buff info - we'll process it in World::update
    // Use a special marker in PendingMagicHit to indicate MagicBooster buff
    world.pending_magic_hits.push(PendingMagicHit {
        due_time_ms,
        attacker_session_id: session_id,
        map_index: 0, // Not used for buffs
        target_monster_id: 0, // Special marker for MagicBooster
        monster_index: -2, // Special marker for MagicBooster (different from SlashingBurst's -1)
        spell_id,
        damage: bonus, // Store bonus value in damage field
        damage_type: 0, // Not used
    });

    world.record_magic_cast_time(session_id, spell_id);
    events.push(WorldEvent::MagicCast {
        session_id,
        spell_id,
    });
}

/// Cast TurnUndead skill - kills undead monsters with a chance based on level difference
/// C#: HumanObject.TurnUndead - checks if target is undead monster, then DelayedAction kills it
pub fn cast_turn_undead<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    x: i32,
    y: i32,
    events: &mut Vec<WorldEvent>,
) {
    use rand::Rng;

    let spell_id = Spell::TurnUndead as u8;

    let (map_index, caster_x, caster_y, caster_level, magic_level, target_monster_id) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let now_ms = world.time_ms;
        if player.dead
            || (player.next_action_time_ms != 0 && now_ms < player.next_action_time_ms)
        {
            return;
        }

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
        player.direction = direction;
        player.next_action_time_ms = now_ms.saturating_add(600); // C# has 600ms action delay

        // Find target monster (from packet or directional scan)
        let target_id = if let Some(monsters) = world.monsters.get(&player.map_index) {
            // Try to find monster at target location or in direction
            let mut found: Option<u64> = None;
            
            // Check monsters at (x, y) first
            for m in monsters.iter() {
                if m.hp > 0 && m.x == x && m.y == y {
                    found = Some(m.id);
                    break;
                }
            }
            
            // If not found, scan in direction
            if found.is_none() {
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
                
                let max_range: i32 = 9; // C#: Range = 9
                for dist in 1..=max_range {
                    let tx = player.x + dx * dist;
                    let ty = player.y + dy * dist;
                    
                    for m in monsters.iter() {
                        if m.hp > 0 && m.x == tx && m.y == ty {
                            found = Some(m.id);
                            break;
                        }
                    }
                    if found.is_some() {
                        break;
                    }
                }
            }
            
            found
        } else {
            None
        };

        (
            player.map_index,
            player.x,
            player.y,
            player.level,
            level,
            target_id,
        )
    };

    // Check if target is valid undead monster
    let (monster_id, monster_index, monster_level) = {
        if let Some(target_id) = target_monster_id {
            if let Some(monsters) = world.monsters.get(&map_index) {
                if let Some(m) = monsters.iter().find(|m| m.id == target_id && m.hp > 0) {
                    let monster_info = match world.provider.get_monster_info(m.monster_index) {
                        Some(info) => info,
                        None => return,
                    };
                    
                    // C#: target.Undead && target.IsAttackTarget(this)
                    if !monster_info.undead {
                        world.record_magic_cast_time(session_id, spell_id);
                        events.push(WorldEvent::MagicCast {
                            session_id,
                            spell_id,
                        });
                        return;
                    }
                    
                    if !world.can_attack_monster(session_id, map_index, target_id) {
                        world.record_magic_cast_time(session_id, spell_id);
                        events.push(WorldEvent::MagicCast {
                            session_id,
                            spell_id,
                        });
                        return;
                    }
                    
                    (target_id, m.monster_index, monster_info.level)
                } else {
                    world.record_magic_cast_time(session_id, spell_id);
                    events.push(WorldEvent::MagicCast {
                        session_id,
                        spell_id,
                    });
                    return;
                }
            } else {
                world.record_magic_cast_time(session_id, spell_id);
                events.push(WorldEvent::MagicCast {
                    session_id,
                    spell_id,
                });
                return;
            }
        } else {
            world.record_magic_cast_time(session_id, spell_id);
            events.push(WorldEvent::MagicCast {
                session_id,
                spell_id,
            });
            return;
        }
    };

    // C#: if (Envir.Random.Next(2) + Level - 1 <= target.Level)
    let mut rng = rand::thread_rng();
    if rng.gen_range(0..2) + caster_level as i32 - 1 <= monster_level as i32 {
        // Set monster target to caster (make it attack the caster)
        if let Some(monsters) = world.monsters.get_mut(&map_index) {
            if let Some(m) = monsters.iter_mut().find(|m| m.id == monster_id) {
                m.target_session_id = Some(session_id);
            }
        }
        world.record_magic_cast_time(session_id, spell_id);
        events.push(WorldEvent::MagicCast {
            session_id,
            spell_id,
        });
        return;
    }

    // C#: int dif = Level - target.Level + 15;
    // C#: if (Envir.Random.Next(100) >= (magic.Level + 1 << 3) + dif)
    let dif = caster_level as i32 - monster_level as i32 + 15;
    let threshold = ((magic_level as i32 + 1) << 3) + dif;
    if rng.gen_range(0..100) >= threshold {
        // Set monster target to caster
        if let Some(monsters) = world.monsters.get_mut(&map_index) {
            if let Some(m) = monsters.iter_mut().find(|m| m.id == monster_id) {
                m.target_session_id = Some(session_id);
            }
        }
        world.record_magic_cast_time(session_id, spell_id);
        events.push(WorldEvent::MagicCast {
            session_id,
            spell_id,
        });
        return;
    }

    // C#: DelayedAction action = new DelayedAction(DelayedType.Magic, Envir.Time + 500, magic, target);
    // Schedule delayed kill
    let delay_ms: i64 = 500;
    let due_time_ms = world.time_ms.saturating_add(delay_ms);

    // Use PendingMagicHit with special marker for TurnUndead kill
    world.pending_magic_hits.push(PendingMagicHit {
        due_time_ms,
        attacker_session_id: session_id,
        map_index,
        target_monster_id: monster_id,
        monster_index: -3, // Special marker for TurnUndead kill
        spell_id,
        damage: 0, // Not used for instant kill
        damage_type: 0, // Not used
    });

    world.record_magic_cast_time(session_id, spell_id);
    events.push(WorldEvent::MagicCast {
        session_id,
        spell_id,
    });
}