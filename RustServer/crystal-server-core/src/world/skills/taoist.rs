use crate::stats::{Stat, Stats};
use crate::world::magic::magic_damage;
use crate::world::monster::MonsterAiState;
use crate::world::skills::compute_magic_mana_cost;
use crate::world::types::{BuffType, PetKind, PetMode, PoisonType};
use crate::world::{Job, SessionId, World, WorldEvent, WorldProvider};
use crate::world::Spell;
use rand::{thread_rng, Rng};
use tracing::debug;

const ITEM_TYPE_AMULET: u8 = 8;
const SKELETON_AMULET_COUNT: u16 = 1;
const SHINSU_AMULET_COUNT: u16 = 5;
const HOLY_DEVA_AMULET_COUNT: u16 = 2;
const DEFAULT_AMULET_SHAPE: i16 = 0;
const REINCARNATION_AMULET_SHAPE: i16 = 3;
const POISON_SHAPE_GREEN: i16 = 1;
const POISON_SHAPE_RED: i16 = 2;

pub fn cast_hiding<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    _x: i32,
    _y: i32,
    events: &mut Vec<WorldEvent>,
) {
    debug!(
        "cast_hiding: session_id={} spell={} dir={} starting",
        session_id,
        spell,
        direction
    );

    let (map_index, caster_x, caster_y, level, duration_ms) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => {
                debug!(
                    "cast_hiding: session_id={} no player found in world",
                    session_id
                );
                return;
            }
        };

        debug!(
            "cast_hiding: session_id={} mp_before={} map={} pos=({}, {})",
            session_id,
            player.mp,
            player.map_index,
            player.x,
            player.y
        );

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => {
                debug!(
                    "cast_hiding: session_id={} has no UserMagic entry for spell {}",
                    session_id,
                    spell
                );
                return;
            }
        };

        let level = magic.level;
        let cost = compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level)
            .unwrap_or(0);

        debug!(
            "cast_hiding: session_id={} magic_level={} mp_before={} cost_mp={}",
            session_id,
            level,
            player.mp,
            cost
        );

        if player.mp < cost {
            debug!(
                "cast_hiding: session_id={} mp={} < cost={} (insufficient MP)",
                session_id,
                player.mp,
                cost
            );
            return;
        }

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
            None => {
                debug!(
                    "cast_hiding: session_id={} has no suitable amulet (ItemType.Amulet, Shape={})",
                    session_id,
                    DEFAULT_AMULET_SHAPE,
                );
                return;
            }
        };

        player.mp -= cost;

        if let Some(slot) = player.equipment.slots.get_mut(amulet_slot) {
            if let Some(item) = slot.as_mut() {
                if item.count > 1 {
                    item.count = item.count.saturating_sub(1);
                } else {
                    *slot = None;
                }
            }
        }

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

        let mut value: i64 = base_sc as i64 + (level as i64 + 1) * 5;
        if value <= 0 {
            value = 1;
        }
        let duration_ms = value.saturating_mul(1_000);

        debug!(
            "cast_hiding: session_id={} base_sc={} level={} duration_ms={}",
            session_id,
            base_sc,
            level,
            duration_ms
        );

        (
            player.map_index,
            player.x,
            player.y,
            level,
            duration_ms,
        )
    };

    debug!(
        "cast_hiding: session_id={} applying Hiding buff on map={} at=({}, {}) duration_ms={}",
        session_id,
        map_index,
        caster_x,
        caster_y,
        duration_ms
    );

    world.add_player_buff(
        session_id,
        BuffType::Hiding,
        duration_ms,
        Stats::default(),
        Vec::new(),
        events,
    );

    world.level_up_magic_for_player(session_id, Spell::Hiding as u8, events);

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

pub fn cast_mass_hiding<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    center_x: i32,
    center_y: i32,
    events: &mut Vec<WorldEvent>,
) {
    debug!(
        "cast_mass_hiding: session_id={} spell={} dir={} center=({}, {}) starting",
        session_id,
        spell,
        direction,
        center_x,
        center_y
    );

    let (map_index, caster_x, caster_y, level, duration_ms) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => {
                debug!(
                    "cast_mass_hiding: session_id={} no player found in world",
                    session_id
                );
                return;
            }
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => {
                debug!(
                    "cast_mass_hiding: session_id={} has no UserMagic entry for spell {}",
                    session_id,
                    spell
                );
                return;
            }
        };

        let level = magic.level;
        let cost = compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level)
            .unwrap_or(0);

        debug!(
            "cast_mass_hiding: session_id={} magic_level={} mp_before={} cost_mp={}",
            session_id,
            level,
            player.mp,
            cost
        );

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
            None => {
                debug!(
                    "cast_mass_hiding: session_id={} has no suitable amulet (ItemType.Amulet, Shape={})",
                    session_id,
                    DEFAULT_AMULET_SHAPE,
                );
                return;
            }
        };

        if player.mp < cost {
            debug!(
                "cast_mass_hiding: session_id={} mp={} < cost={} (insufficient MP)",
                session_id,
                player.mp,
                cost
            );
            return;
        }

        player.mp -= cost;

        if let Some(slot) = player.equipment.slots.get_mut(amulet_slot) {
            if let Some(item) = slot.as_mut() {
                if item.count > 1 {
                    item.count = item.count.saturating_sub(1);
                } else {
                    *slot = None;
                }
            }
        }

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

        let mut value: i64 = (base_sc as i64 / 2).saturating_add((level as i64 + 1) * 2);
        if value <= 0 {
            value = 1;
        }
        let duration_ms = value.saturating_mul(1_000);

        debug!(
            "cast_mass_hiding: session_id={} base_sc={} level={} duration_ms={}",
            session_id,
            base_sc,
            level,
            duration_ms
        );

        (
            player.map_index,
            player.x,
            player.y,
            level,
            duration_ms,
        )
    };

    const RADIUS: i32 = 2;

    let mut any_applied = false;
    let mut applied_count: usize = 0;

    let player_ids: Vec<SessionId> = world.players.keys().cloned().collect();
    for sid in player_ids {
        if let Some(p) = world.players.get(&sid) {
            if p.map_index != map_index || p.dead {
                continue;
            }

            let dx = p.x - center_x;
            let dy = p.y - center_y;
            if dx.abs().max(dy.abs()) > RADIUS {
                continue;
            }

            let friendly = if sid == session_id {
                true
            } else {
                let caster_party = world
                    .players
                    .get(&session_id)
                    .and_then(|c| c.party_id);
                let target_party = p.party_id;
                caster_party.is_some() && caster_party == target_party
            };

            if !friendly {
                continue;
            }
        } else {
            continue;
        }

        world.add_player_buff(
            sid,
            BuffType::Hiding,
            duration_ms,
            Stats::default(),
            Vec::new(),
            events,
        );
        any_applied = true;
        applied_count = applied_count.saturating_add(1);
    }

    debug!(
        "cast_mass_hiding: session_id={} map={} center=({}, {}) duration_ms={} applied_count={} any_applied={}",
        session_id,
        map_index,
        center_x,
        center_y,
        duration_ms,
        applied_count,
        any_applied
    );

    if any_applied {
        world.level_up_magic_for_player(session_id, Spell::MassHiding as u8, events);
    }

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

    // Choose a friendly target at the clicked location if present. This
    // follows the spirit of C# MapObject.IsFriendlyTarget: Taoist Healing can
    // be cast on self, party/guild members and their pets.

    // 1) Try friendly players at the clicked location.
    let mut healed_any = false;

    let caster_party_id;
    let caster_guild_name;
    {
        let caster = match world.players.get(&session_id) {
            Some(p) => p,
            None => return,
        };
        caster_party_id = caster.party_id;
        caster_guild_name = caster.guild_name.clone();
    }

    for (&sid, p) in &mut world.players {
        if p.map_index != map_index || p.dead || p.hp <= 0 {
            continue;
        }
        if p.x != target_x || p.y != target_y {
            continue;
        }

        let is_self = sid == session_id;
        let same_party = match (caster_party_id, p.party_id) {
            (Some(a), Some(b)) if a == b => true,
            _ => false,
        };
        let same_guild = !caster_guild_name.is_empty()
            && !p.guild_name.is_empty()
            && p.guild_name.eq_ignore_ascii_case(&caster_guild_name);

        if !is_self && !same_party && !same_guild {
            continue;
        }

        let max_hp = p.stats.total.get(Stat::HP).max(1);
        if p.hp >= max_hp {
            continue;
        }

        let old_hp = p.hp;
        let new_hp = (old_hp + heal_value).min(max_hp);
        if new_hp <= old_hp {
            continue;
        }

        let amount = new_hp - old_hp;
        p.hp = new_hp;

        events.push(WorldEvent::PlayerHealed {
            session_id: p.session_id,
            map_index,
            x: p.x,
            y: p.y,
            amount,
            new_hp,
            show_healing_effect: true,
        });

        healed_any = true;
        break;
    }

    // 2) If no player was healed, try friendly pets (monsters with is_pet)
    // at the clicked location. We treat pets as friendly when their owner is
    // the caster or a member of the same party/guild.
    if !healed_any {
        if let Some(monsters) = world.monsters.get_mut(&map_index) {
            for m in monsters.iter_mut() {
                if !m.is_pet || m.hp <= 0 {
                    continue;
                }
                if m.x != target_x || m.y != target_y {
                    continue;
                }

                let Some(owner_sid) = m.owner_session_id else {
                    continue;
                };

                let Some(owner) = world.players.get(&owner_sid) else {
                    continue;
                };

                let is_self_owner = owner_sid == session_id;
                let same_party_owner = match (caster_party_id, owner.party_id) {
                    (Some(a), Some(b)) if a == b => true,
                    _ => false,
                };
                let same_guild_owner = !caster_guild_name.is_empty()
                    && !owner.guild_name.is_empty()
                    && owner
                        .guild_name
                        .eq_ignore_ascii_case(&caster_guild_name);

                if !is_self_owner && !same_party_owner && !same_guild_owner {
                    continue;
                }

                let max_hp = if let Some(info) = world.provider.get_monster_info(m.monster_index)
                {
                    info.stats.get(Stat::HP).max(1)
                } else {
                    1
                };

                if m.hp >= max_hp {
                    continue;
                }

                let old_hp = m.hp;
                let new_hp = (old_hp + heal_value).min(max_hp);
                if new_hp <= old_hp {
                    continue;
                }

                let amount = new_hp - old_hp;
                m.hp = new_hp;

                let percent: u8 = ((new_hp as i64 * 100 / max_hp as i64)
                    .clamp(0, 100)) as u8;

                events.push(WorldEvent::MonsterHealed {
                    monster_id: m.id,
                    map_index,
                    x: m.x,
                    y: m.y,
                    amount,
                    new_hp,
                    health_percent: percent,
                    show_healing_effect: true,
                });

                healed_any = true;
                break;
            }
        }
    }

    if !healed_any {
        // If we found no valid friendly target at the clicked tile, fall back
        // to healing the caster themselves when they are below max HP.
        if let Some(p) = world.players.get_mut(&session_id) {
            if p.map_index == map_index && !p.dead {
                let max_hp = p.stats.total.get(Stat::HP).max(1);
                if p.hp < max_hp {
                    let old_hp = p.hp;
                    let new_hp = (old_hp + heal_value).min(max_hp);
                    if new_hp > old_hp {
                        let amount = new_hp - old_hp;
                        p.hp = new_hp;
                        events.push(WorldEvent::PlayerHealed {
                            session_id: p.session_id,
                            map_index,
                            x: p.x,
                            y: p.y,
                            amount,
                            new_hp,
                            show_healing_effect: true,
                        });
                        healed_any = true;
                    }
                }
            }
        }
    }

    if healed_any {
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
    center_x: i32,
    center_y: i32,
    events: &mut Vec<WorldEvent>,
) {
    debug!(
        "cast_soul_shield: session_id={} spell={} dir={} starting",
        session_id,
        spell,
        direction
    );

    let (
        map_index,
        caster_x,
        caster_y,
        level,
        duration_ms,
        caster_party_id,
        caster_guild_name,
    ) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell) {
            Some(m) => m,
            None => {
                debug!(
                    "cast_soul_shield: session_id={} has no UserMagic entry for spell {}",
                    session_id,
                    spell
                );
                return;
            }
        };

        let level = magic.level;
        debug!(
            "cast_soul_shield: session_id={} magic_level={} mp_before={}",
            session_id,
            level,
            player.mp
        );

        let cost = match compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level)
        {
            Some(c) => c,
            None => {
                debug!(
                    "cast_soul_shield: session_id={} no MagicInfo / cost for spell {} level {}",
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

        // Find a generic Taoist amulet (ItemType.Amulet, Shape=0) and
        // consume one, mirroring C# GetAmulet(1)/ConsumeItem for
        // SoulShield/BlessedArmour.
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
            None => {
                debug!(
                    "cast_soul_shield: session_id={} has no suitable amulet (ItemType.Amulet, Shape={})",
                    session_id,
                    DEFAULT_AMULET_SHAPE,
                );
                return;
            }
        };

        player.mp -= cost;

        if let Some(slot) = player.equipment.slots.get_mut(amulet_slot) {
            if let Some(item) = slot.as_mut() {
                if item.count > 1 {
                    item.count = item.count.saturating_sub(1);
                } else {
                    *slot = None;
                }
            }
        }

        // Compute SC-based power similarly to GetAttackPower(MinSC,MaxSC)
        // and derive the duration in seconds as
        //   value = power * 4 + (level + 1) * 50
        // mirroring HumanObject.SoulShield and Map.SoulShield handling.
        let stats_total = &player.stats.total;
        let min_sc = stats_total.get(Stat::MinSC);
        let max_sc = stats_total.get(Stat::MaxSC);
        let luck = stats_total.get(Stat::Luck);

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

        let mut value_sec: i64 = (damage_base as i64)
            .saturating_mul(4)
            .saturating_add((level as i64 + 1).saturating_mul(50));
        if value_sec <= 0 {
            value_sec = 1;
        }
        let duration_ms = value_sec.saturating_mul(1_000);

        let caster_party_id = player.party_id;
        let caster_guild_name = player.guild_name.clone();

        (
            player.map_index,
            player.x,
            player.y,
            level,
            duration_ms,
            caster_party_id,
            caster_guild_name,
        )
    };

    // 7x7 area around the clicked location (center_x, center_y).
    let range = 3;
    let min_x = center_x - range;
    let max_x = center_x + range;
    let min_y = center_y - range;
    let max_y = center_y + range;

    let mut trained = false;

    // Collect candidate friendly player sessions first.
    let mut targets: Vec<SessionId> = Vec::new();
    for (&sid, p) in &world.players {
        if p.map_index != map_index || p.dead || p.hp <= 0 {
            continue;
        }
        if p.x < min_x || p.x > max_x || p.y < min_y || p.y > max_y {
            continue;
        }

        let is_self = sid == session_id;
        let same_party = match (caster_party_id, p.party_id) {
            (Some(a), Some(b)) if a == b => true,
            _ => false,
        };
        let same_guild = !caster_guild_name.is_empty()
            && !p.guild_name.is_empty()
            && p.guild_name.eq_ignore_ascii_case(&caster_guild_name);

        if !is_self && !same_party && !same_guild {
            continue;
        }

        targets.push(sid);
    }

    for sid in targets {
        let player = match world.players.get_mut(&sid) {
            Some(p) => p,
            None => continue,
        };

        let bonus = (player.level as i32 / 7).saturating_add(4);
        if bonus <= 0 {
            continue;
        }

        let mut stats = Stats::default();
        stats.set(Stat::MaxMAC, bonus);

        world.add_player_buff(
            sid,
            BuffType::SoulShield,
            duration_ms,
            stats,
            Vec::new(),
            events,
        );

        trained = true;
    }

    // Also buff friendly pets (monsters with is_pet=true whose owner is
    // the caster or a friendly player in the same party/guild) that
    // stand inside the same 7x7 area. Each pet uses its
    // MonsterInfo.Level to compute the bonus in the same way as
    // players (level / 7 + 4), mirroring the C# behaviour where
    // SoulShield can affect heroes/pets.
    let mut pet_targets: Vec<(u64, i32)> = Vec::new();
    {
        if let Some(monsters) = world.monsters.get(&map_index) {
            for m in monsters.iter() {
                if !m.is_pet || m.hp <= 0 {
                    continue;
                }
                if m.x < min_x || m.x > max_x || m.y < min_y || m.y > max_y {
                    continue;
                }

                // Resolve the pet's owner to determine if it is friendly
                // (self, same party or same guild as the caster).
                let Some(owner_sid) = m.owner_session_id else {
                    continue;
                };

                let Some(owner) = world.players.get(&owner_sid) else {
                    continue;
                };

                let is_self_owner = owner_sid == session_id;
                let same_party_owner = match (caster_party_id, owner.party_id) {
                    (Some(a), Some(b)) if a == b => true,
                    _ => false,
                };
                let same_guild_owner = !caster_guild_name.is_empty()
                    && !owner.guild_name.is_empty()
                    && owner
                        .guild_name
                        .eq_ignore_ascii_case(&caster_guild_name);

                if !is_self_owner && !same_party_owner && !same_guild_owner {
                    continue;
                }

                let bonus = if let Some(info) = world.provider.get_monster_info(m.monster_index) {
                    (info.level as i32 / 7).saturating_add(4)
                } else {
                    0
                };

                if bonus <= 0 {
                    continue;
                }

                pet_targets.push((m.id, bonus));
            }
        }
    }

    for (monster_id, bonus) in pet_targets {
        let mut stats = Stats::default();
        stats.set(Stat::MaxMAC, bonus);

        world.add_monster_buff(map_index, monster_id, BuffType::SoulShield, duration_ms, stats);

        trained = true;
    }

    if trained {
        world.level_up_magic_for_player(session_id, Spell::SoulShield as u8, events);
    }

    // Drive SoulShield visuals in the same way as wizard AoE spells by
    // emitting both ObjectMagic (for nearby viewers) and Magic (for the
    // caster) in addition to a generic ObjectAttack.
    events.push(WorldEvent::ObjectMagic {
        session_id,
        map_index,
        x: caster_x,
        y: caster_y,
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

    // Emit a generic ObjectAttack so the client can still play the
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
    center_x: i32,
    center_y: i32,
    events: &mut Vec<WorldEvent>,
) {
    let (
        map_index,
        caster_x,
        caster_y,
        level,
        duration_ms,
        caster_party_id,
        caster_guild_name,
    ) = {
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

        // Same amulet requirement as SoulShield: generic Taoist amulet.
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
            None => {
                debug!(
                    "cast_blessed_armour: session_id={} has no suitable amulet (ItemType.Amulet, Shape={})",
                    session_id,
                    DEFAULT_AMULET_SHAPE,
                );
                return;
            }
        };

        player.mp -= cost;

        if let Some(slot) = player.equipment.slots.get_mut(amulet_slot) {
            if let Some(item) = slot.as_mut() {
                if item.count > 1 {
                    item.count = item.count.saturating_sub(1);
                } else {
                    *slot = None;
                }
            }
        }

        let stats_total = &player.stats.total;
        let min_sc = stats_total.get(Stat::MinSC);
        let max_sc = stats_total.get(Stat::MaxSC);
        let luck = stats_total.get(Stat::Luck);

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

        let mut value_sec: i64 = (damage_base as i64)
            .saturating_mul(4)
            .saturating_add((level as i64 + 1).saturating_mul(50));
        if value_sec <= 0 {
            value_sec = 1;
        }
        let duration_ms = value_sec.saturating_mul(1_000);

        let caster_party_id = player.party_id;
        let caster_guild_name = player.guild_name.clone();

        (
            player.map_index,
            player.x,
            player.y,
            level,
            duration_ms,
            caster_party_id,
            caster_guild_name,
        )
    };

    let range = 3;
    let min_x = center_x - range;
    let max_x = center_x + range;
    let min_y = center_y - range;
    let max_y = center_y + range;

    let mut trained = false;

    let mut targets: Vec<SessionId> = Vec::new();
    for (&sid, p) in &world.players {
        if p.map_index != map_index || p.dead || p.hp <= 0 {
            continue;
        }
        if p.x < min_x || p.x > max_x || p.y < min_y || p.y > max_y {
            continue;
        }

        let is_self = sid == session_id;
        let same_party = match (caster_party_id, p.party_id) {
            (Some(a), Some(b)) if a == b => true,
            _ => false,
        };
        let same_guild = !caster_guild_name.is_empty()
            && !p.guild_name.is_empty()
            && p.guild_name.eq_ignore_ascii_case(&caster_guild_name);

        if !is_self && !same_party && !same_guild {
            continue;
        }

        targets.push(sid);
    }

    for sid in targets {
        let player = match world.players.get_mut(&sid) {
            Some(p) => p,
            None => continue,
        };

        let bonus = (player.level as i32 / 7).saturating_add(4);
        if bonus <= 0 {
            continue;
        }

        let mut stats = Stats::default();
        stats.set(Stat::MaxAC, bonus);

        world.add_player_buff(
            sid,
            BuffType::BlessedArmour,
            duration_ms,
            stats,
            Vec::new(),
            events,
        );

        trained = true;
    }

    // Extend BlessedArmour to also affect friendly pets within the same
    // 7x7 area. Pets are considered friendly when their owner is the
    // caster or a player in the same party/guild. The AC bonus is based
    // on MonsterInfo.Level, mirroring the C# friendly target semantics
    // for SoulShield/BlessedArmour.
    let mut pet_targets: Vec<(u64, i32)> = Vec::new();
    {
        if let Some(monsters) = world.monsters.get(&map_index) {
            for m in monsters.iter() {
                if !m.is_pet || m.hp <= 0 {
                    continue;
                }
                if m.x < min_x || m.x > max_x || m.y < min_y || m.y > max_y {
                    continue;
                }

                // Resolve the pet's owner and check if it is friendly to
                // the caster (self, same party or same guild).
                let Some(owner_sid) = m.owner_session_id else {
                    continue;
                };

                let Some(owner) = world.players.get(&owner_sid) else {
                    continue;
                };

                let is_self_owner = owner_sid == session_id;
                let same_party_owner = match (caster_party_id, owner.party_id) {
                    (Some(a), Some(b)) if a == b => true,
                    _ => false,
                };
                let same_guild_owner = !caster_guild_name.is_empty()
                    && !owner.guild_name.is_empty()
                    && owner
                        .guild_name
                        .eq_ignore_ascii_case(&caster_guild_name);

                if !is_self_owner && !same_party_owner && !same_guild_owner {
                    continue;
                }

                let bonus = if let Some(info) = world.provider.get_monster_info(m.monster_index) {
                    (info.level as i32 / 7).saturating_add(4)
                } else {
                    0
                };

                if bonus <= 0 {
                    continue;
                }

                pet_targets.push((m.id, bonus));
            }
        }
    }

    for (monster_id, bonus) in pet_targets {
        let mut stats = Stats::default();
        stats.set(Stat::MaxAC, bonus);

        world.add_monster_buff(map_index, monster_id, BuffType::BlessedArmour, duration_ms, stats);

        trained = true;
    }

    if trained {
        world.level_up_magic_for_player(session_id, Spell::BlessedArmour as u8, events);
    }

    // BlessedArmour uses the same visual pattern as SoulShield: send an
    // ObjectMagic event for viewers and a Magic event for the caster so
    // the client plays the correct spell animation in addition to the
    // generic ObjectAttack swing.
    events.push(WorldEvent::ObjectMagic {
        session_id,
        map_index,
        x: caster_x,
        y: caster_y,
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
    let _now_ms = world.time_ms.max(0);

    let (map_index, caster_x, caster_y, level, power, poison_slot, poison_type) = {
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

        // Find an equipped poison amulet (Shape 1 = Green, Shape 2 = Red),
        // mirroring C# HumanObject.GetPoison(count:1, shape:0) which scans the
        // equipment array for ItemType.Amulet with Shape 1 or 2.
        let mut poison_slot: Option<usize> = None;
        let mut poison_type: Option<PoisonType> = None;

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

            let ptype = match info.shape {
                POISON_SHAPE_GREEN => PoisonType::Green,
                POISON_SHAPE_RED => PoisonType::Red,
                _ => continue,
            };

            if item.count as u32 >= 1 {
                poison_slot = Some(idx);
                poison_type = Some(ptype);
                break;
            }
        }

        let poison_slot = match poison_slot {
            Some(idx) => idx,
            None => return,
        };

        let poison_type = match poison_type {
            Some(pt) => pt,
            None => PoisonType::Green,
        };

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

        (
            player.map_index,
            player.x,
            player.y,
            level,
            power,
            poison_slot,
            poison_type,
        )
    };

    // Find a target monster at the clicked location, mirroring the targeting
    // approach used by Hallucination.
    let mut target_id: Option<u64> = None;

    if let Some(monsters) = world.monsters.get(&map_index) {
        for m in monsters {
            if m.hp <= 0 {
                continue;
            }
            if m.x == target_x && m.y == target_y {
                target_id = Some(m.id);
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

    // Duration in ticks: (power * 2) + ((Level + 1) * 7), matching
    // HumanObject.Process(DelayedAction) for Spell.Poisoning.
    let mut duration: i64 = (power.saturating_mul(2) as i64)
        .saturating_add((i32::from(level) + 1) as i64 * 7);
    if duration <= 0 {
        duration = 1;
    }

    // Tick speed: 2000ms per tick.
    let tick_speed_ms: i64 = 2_000;

    // Per-tick damage for Green poison: value / 15 + magic.Level + 1 +
    // rand(PoisonAttack). Red poison in the C# server does not apply DOT
    // damage; it instead reduces armour while active. We approximate this by
    // using a non-damaging poison (value = 0) for Red.
    let poison_value = if poison_type == PoisonType::Green {
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
    } else {
        0
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

    // Consume one poison amulet from the selected equipment slot on
    // successful application, mirroring C# HumanObject.Poisoning which calls
    // ConsumeItem(GetPoison(...), 1).
    if let Some(player) = world.players.get_mut(&session_id) {
        if let Some(slot) = player.equipment.slots.get_mut(poison_slot) {
            if let Some(item) = slot.as_mut() {
                if item.count > 1 {
                    item.count = item.count.saturating_sub(1);
                } else {
                    *slot = None;
                }
            }
        }
    }

    // If the caster is in FocusMasterTarget pet mode, update their pets'
    // focus target to this monster so that pet AI can concentrate on the
    // same target, mirroring the C# PetMode.FocusMasterTarget behaviour.
    if let Some(p) = world.players.get(&session_id) {
        if PetMode::from_u8(p.pet_mode) == PetMode::FocusMasterTarget {
            world.set_player_pet_focus_target_monster(session_id, Some(target_id));
        }
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
            show_healing_effect: true,
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

pub fn cast_reincarnation<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    spell: u8,
    direction: u8,
    target_x: i32,
    target_y: i32,
    events: &mut Vec<WorldEvent>,
) {
    let now_ms = world.time_ms;

    let (map_index, caster_x, caster_y, level) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        if let Some(info) = world.provider.get_map_info(player.map_index) {
            if info.no_reincarnation {
                return;
            }
        }

        let level = player
            .magics
            .iter()
            .find(|m| m.spell == spell)
            .map(|m| m.level)
            .unwrap_or(0);

        let cost = compute_magic_mana_cost(&world.provider, &player.stats.total, spell, level)
            .unwrap_or(0);

        if player.mp < cost {
            return;
        }

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

            if info.shape != REINCARNATION_AMULET_SHAPE {
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

        player.mp -= cost;

        if let Some(slot) = player.equipment.slots.get_mut(amulet_slot) {
            if let Some(item) = slot.as_mut() {
                if item.count > 1 {
                    item.count = item.count.saturating_sub(1);
                } else {
                    *slot = None;
                }
            }
        }

        (
            player.map_index,
            player.x,
            player.y,
            level,
        )
    };

    let mut target_session_id: Option<SessionId> = None;
    for (&sid, p) in &world.players {
        if sid == session_id {
            continue;
        }
        if p.map_index != map_index || !p.dead || p.hp > 0 {
            continue;
        }
        if p.x != target_x || p.y != target_y {
            continue;
        }

        let dx = p.x - caster_x;
        let dy = p.y - caster_y;
        if dx.abs().max(dy.abs()) > 7 {
            continue;
        }

        target_session_id = Some(sid);
        break;
    }

    let target_session_id = match target_session_id {
        Some(sid) => sid,
        None => return,
    };

    {
        let target = match world.players.get(&target_session_id) {
            Some(p) => p,
            None => return,
        };

        if !target.dead || target.hp > 0 {
            return;
        }

        if target.reincarnation_host_session_id.is_some() {
            return;
        }
    }

    let _ = world.cancel_reincarnation_for_session(session_id);
    let _ = world.cancel_reincarnation_for_session(target_session_id);

    let mut rng = thread_rng();
    let lvl_i32 = i32::from(level);
    let threshold = (1 + lvl_i32).saturating_mul(10);
    let roll = rng.gen_range(0..30);
    if roll > threshold {
        events.push(WorldEvent::PartySystemMessage {
            session_id,
            message: "Reincarnation attempt failed.".to_string(),
        });
        return;
    }

    if let Some(host) = world.players.get_mut(&session_id) {
        host.reincarnation_ready = true;
        host.reincarnation_target_session_id = Some(target_session_id);
        host.reincarnation_expire_time_ms = now_ms.saturating_add(6_000);
    }

    if let Some(target) = world.players.get_mut(&target_session_id) {
        target.reincarnation_host_session_id = Some(session_id);
        target.reincarnation_expire_time_ms = now_ms.saturating_add(6_000);
    }

    world.level_up_magic_for_player(session_id, Spell::Reincarnation as u8, events);

    events.push(WorldEvent::ReincarnationRequested {
        host_session_id: session_id,
        target_session_id,
    });

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
