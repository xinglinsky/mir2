use rand::thread_rng;

use crate::combat::compute_physical_melee_with_crit;
use crate::stats::{Stat, Stats};
use crate::world::monster::MonsterAiState;
use crate::world::player::PlayerState;
use crate::world::provider::WorldProvider;
use crate::world::skills::{
    apply_attack_spell_scaling,
    apply_fatal_sword_and_undead,
    compute_pure_magic_attack_damage,
    fatal_sword_level_for_player,
    is_pure_magic_attack,
    resolve_attack_spell_and_level_for_player,
};
use crate::world::skills::warrior::{
    compute_cross_half_moon_targets,
    compute_half_moon_targets,
    is_thrusting_spell,
    thrusting_max_range,
};
use crate::world::Spell;

use super::{SessionId, World, WorldEvent};

impl<P: WorldProvider> World<P> {
    fn can_attack(_player: &PlayerState) -> bool {
        true
    }

    #[allow(dead_code)]
    fn compute_physical_damage_base(player_level: u16) -> i32 {
        let lvl = player_level.max(1) as i32;
        let min_dc = 1 + lvl / 2;
        let max_dc = 2 + lvl;
        if max_dc <= min_dc {
            min_dc.max(1)
        } else {
            (min_dc + max_dc) / 2
        }
    }

    pub(super) fn handle_magic_command(
        &mut self,
        session_id: SessionId,
        spell: u8,
        direction: u8,
        _target_id: u32,
        _x: i32,
        _y: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let (player_map, player_x, player_y) = match self.players.get(&session_id) {
            Some(p) => (p.map_index, p.x, p.y),
            None => return,
        };

        if spell == Spell::FlamingSword as u8 {
            self.handle_flaming_sword_spell(session_id, events);
            return;
        }

        if spell == Spell::Rage as u8 {
            self.handle_rage_spell(session_id, events);
            return;
        }

        if spell == Spell::ImmortalSkin as u8 {
            self.handle_immortal_skin_spell(session_id, events);
            return;
        }

        if spell == Spell::CounterAttack as u8 {
            self.handle_counter_attack_spell(session_id, events);
            return;
        }

        // TODO: Handle other spells.
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

    fn handle_flaming_sword_spell(
        &mut self,
        session_id: SessionId,
        _events: &mut Vec<WorldEvent>,
    ) {
        if let Some(player) = self.players.get_mut(&session_id) {
            // Toggle Flaming Sword state.
            // In C#, this is:
            // FlamingSword = true;
            // FlamingSwordTime = Envir.Time + 10000;
            // Enqueue(new S.SpellToggle { ObjectID = ObjectID, Spell = Spell.FlamingSword, CanUse = true });
            // ChangeMP(-cost);

            // For now, just print a debug message. We need to add Buff/SpellToggle support to WorldEvent
            // and PlayerState to fully implement this.
            println!("[combat] FlamingSword toggle requested for session {}", session_id);
            
            // TODO: Deduct MP
            // TODO: Update PlayerState with FlamingSword active + expiry time
            // TODO: Emit SpellToggle event
        }
    }

    fn handle_rage_spell(
        &mut self,
        session_id: SessionId,
        _events: &mut Vec<WorldEvent>,
    ) {
        if let Some(_player) = self.players.get_mut(&session_id) {
            // In C# this applies a long-duration Rage buff (attack-oriented).
            // The full behaviour depends on Buffs and MagicInfo; here we only
            // record that the spell was invoked so that the build succeeds.
            println!("[combat] Rage spell requested for session {}", session_id);
            // TODO: Implement Rage buff and MP cost using MagicInfo and Buff events.
        }
    }

    fn handle_immortal_skin_spell(
        &mut self,
        session_id: SessionId,
        _events: &mut Vec<WorldEvent>,
    ) {
        if let Some(_player) = self.players.get_mut(&session_id) {
            // In C# this applies a temporary ImmortalSkin defence buff via
            // AddBuff(BuffType.ImmortalSkin, ...). For now we just log.
            println!(
                "[combat] ImmortalSkin spell requested for session {}",
                session_id
            );
            // TODO: Implement ImmortalSkin buff as a defensive AC/MAC increase.
        }
    }

    fn handle_counter_attack_spell(
        &mut self,
        session_id: SessionId,
        _events: &mut Vec<WorldEvent>,
    ) {
        if let Some(_player) = self.players.get_mut(&session_id) {
            // In C# this toggles CounterAttack state and adds a short buff
            // window during which incoming hits can be reflected. Here we
            // only log the request.
            println!(
                "[combat] CounterAttack spell requested for session {}",
                session_id
            );
            // TODO: Implement CounterAttack window and reactive damage logic.
        }
    }

    pub(super) fn handle_attack_command(
        &mut self,
        session_id: SessionId,
        direction: u8,
        spell: u8,
        events: &mut Vec<WorldEvent>,
    ) {
        let (map_index, x, y, direction, effective_spell, level, fatal_level, attacker_stats) =
            match self.players.get_mut(&session_id) {
                Some(p) => {
                    if !Self::can_attack(p) {
                        return;
                    }

                    p.direction = direction;
                    let (effective_spell, level) =
                        resolve_attack_spell_and_level_for_player(p, spell);
                    let fatal_level = fatal_sword_level_for_player(p);
                    let attacker_stats = p.stats.total.clone();

                    (
                        p.map_index,
                        p.x,
                        p.y,
                        p.direction,
                        effective_spell,
                        level,
                        fatal_level,
                        attacker_stats,
                    )
                }
                None => {
                    return;
                }
            };

        let is_half_moon = effective_spell == Spell::HalfMoon as u8;
        let is_cross_half_moon = effective_spell == Spell::CrossHalfMoon as u8;

        if is_half_moon {
            self.handle_half_moon_attack(
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
            self.handle_cross_half_moon_attack(
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

        let target_info = match self.monsters.get(&map_index) {
            Some(monsters) => {
                if is_pure_magic_attack(effective_spell) {
                    // For pure magic attack spells (e.g. FireBall, SoulFireBall)
                    // scan forward along the attack direction up to the spell
                    // range and select the first monster encountered.
                    let max_range: i32 = self
                        .provider
                        .get_magic_info(effective_spell)
                        .map(|info| info.range as i32)
                        .unwrap_or(1)
                        .max(1);

                    let mut found: Option<(u64, i32, i32, i32)> = None;
                    for step in 1..=max_range {
                        let tx = x + dx * step;
                        let ty = y + dy * step;
                        if let Some(m) = monsters.iter().find(|m| m.x == tx && m.y == ty) {
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
                } else if is_thrusting_spell(effective_spell) {
                    // Thrusting extends melee range in a straight line while
                    // still using the physical melee model for damage.
                    let max_range: i32 = thrusting_max_range(&self.provider, level).max(1);

                    let mut found: Option<(u64, i32, i32, i32)> = None;
                    for step in 1..=max_range {
                        let tx = x + dx * step;
                        let ty = y + dy * step;
                        if let Some(m) = monsters.iter().find(|m| m.x == tx && m.y == ty) {
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
                        .find(|m| m.x == target_x && m.y == target_y)
                        .map(|m| (m.id, m.monster_index))
                }
            }
            None => None,
        };
        if let Some((id, monster_index)) = target_info {
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

            let (monster_exp, undead, max_hp, defender_stats, monster_drops): (
                u32,
                bool,
                i32,
                Stats,
                Vec<crate::world::drop::DropInfo>,
            ) = if let Some(info) = self.provider.get_monster_info(monster_index) {
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

            // Use the unified C#-style physical melee model (including
            // Accuracy/Agility, AC/DR and crit) for player -> monster hits for
            // most attacks. Pure magic spells such as FireBall and
            // SoulFireBall use their MagicInfo parameters directly instead of
            // relying on the melee helper for base damage.
            let use_pure_magic = is_pure_magic_attack(effective_spell);

            let (hit, mut raw_damage, damage_type) = if use_pure_magic {
                let dmg = compute_pure_magic_attack_damage(
                    &self.provider,
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
                if !use_pure_magic {
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

                // Then apply any passive FatalSword and undead-specific
                // tweaks via the shared skills helper.
                raw_damage = apply_fatal_sword_and_undead(
                    &self.provider,
                    fatal_level,
                    undead,
                    raw_damage,
                );

                // Finally, approximate multi-hit skills such as DoubleSlash
                // and TwinDrakeBlade by doubling the final damage. In the C#
                // HumanObject implementation these skills schedule two
                // DelayedAction damage entries with the same magic-scaled
                // damage value; here we aggregate them into a single hit with
                // twice the damage to keep the world-event model simple while
                // preserving total DPS.
                if effective_spell == Spell::DoubleSlash as u8
                    || effective_spell == Spell::TwinDrakeBlade as u8
                {
                    raw_damage = raw_damage.saturating_mul(2);
                }
            }

            let mut strike_x = target_x;
            let mut strike_y = target_y;
            let mut strike_dir = direction;
            let mut damage_done: i32 = 0;
            let mut health_percent: u8 = 100;

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
                self.mark_monster_dead(map_index, id);

                if let Some(info) = self.provider.get_monster_info(monster_index) {
                    println!(
                        "[drop-debug] monster_index={} name='{}' drop_path='{}' drops_len={}",
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

                    println!(
                        "[drop-total] monster_index={} gold={} items_len={}",
                        monster_index,
                        total.gold,
                        total.items.len(),
                    );

                    if total.gold > 0 || !total.items.is_empty() {
                        let entry = self.map_items.entry(map_index).or_default();

                        // Set expire time for monster drops: default 5 minutes
                        // (300000 ms) after the current world time, mirroring
                        // the behaviour used for player-dropped items in
                        // WorldCommand::DropItem.
                        let item_timeout_ms: i64 = 300_000; // 5 minutes

                        if total.gold > 0 {
                            let item_id = self.next_map_item_id;
                            self.next_map_item_id = self.next_map_item_id.wrapping_add(1);
                            entry.push(crate::world::map_item::MapItem {
                                id: item_id,
                                map_index,
                                x: strike_x,
                                y: strike_y,
                                item_index: None,
                                gold: total.gold,
                                count: 0,
                                item: None,
                                expire_time_ms: self.time_ms + item_timeout_ms,
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
                            let item_id = self.next_map_item_id;
                            self.next_map_item_id = self.next_map_item_id.wrapping_add(1);
                            entry.push(crate::world::map_item::MapItem {
                                id: item_id,
                                map_index,
                                x: strike_x,
                                y: strike_y,
                                item_index: Some(item_index),
                                gold: 0,
                                count: 1,
                                item: None,
                                expire_time_ms: self.time_ms + item_timeout_ms,
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

                events.push(WorldEvent::MonsterDied {
                    object_id: id,
                    map_index,
                    x: strike_x,
                    y: strike_y,
                    direction: strike_dir,
                });

                if monster_exp > 0 {
                    if let Some(p) = self.players.get_mut(&session_id) {
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

    fn handle_half_moon_attack(
        &mut self,
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
            let target_info = match self.monsters.get(&map_index) {
                Some(monsters) => monsters
                    .iter()
                    .find(|m| m.x == tx && m.y == ty)
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
                ) = if let Some(info) = self.provider.get_monster_info(monster_index) {
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

                let (hit, mut raw_damage, mut damage_type) =
                    compute_physical_melee_with_crit(attacker_stats, &defender_stats);

                if hit && raw_damage > 0 {
                    raw_damage = crate::world::skills::warrior::compute_half_moon_damage_for_target(
                        &self.provider,
                        raw_damage,
                        level,
                        is_primary,
                    );

                    raw_damage = apply_fatal_sword_and_undead(
                        &self.provider,
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
                    self.mark_monster_dead(map_index, id);

                    if let Some(info) = self.provider.get_monster_info(monster_index) {
                        println!(
                            "[drop-debug] monster_index={} name='{}' drop_path='{}' drops_len={}",
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

                        println!(
                            "[drop-total] monster_index={} gold={} items_len={}",
                            monster_index,
                            total.gold,
                            total.items.len(),
                        );

                        if total.gold > 0 || !total.items.is_empty() {
                            let entry = self.map_items.entry(map_index).or_default();
                            let item_timeout_ms: i64 = 300_000;

                            if total.gold > 0 {
                                let item_id = self.next_map_item_id;
                                self.next_map_item_id =
                                    self.next_map_item_id.wrapping_add(1);
                                entry.push(crate::world::map_item::MapItem {
                                    id: item_id,
                                    map_index,
                                    x: strike_x,
                                    y: strike_y,
                                    item_index: None,
                                    gold: total.gold,
                                    count: 0,
                                    item: None,
                                    expire_time_ms: self.time_ms + item_timeout_ms,
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
                                let item_id = self.next_map_item_id;
                                self.next_map_item_id =
                                    self.next_map_item_id.wrapping_add(1);
                                entry.push(crate::world::map_item::MapItem {
                                    id: item_id,
                                    map_index,
                                    x: strike_x,
                                    y: strike_y,
                                    item_index: Some(item_index),
                                    gold: 0,
                                    count: 1,
                                    item: None,
                                    expire_time_ms: self.time_ms + item_timeout_ms,
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
                        if let Some(p) = self.players.get_mut(&session_id) {
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

    fn handle_cross_half_moon_attack(
        &mut self,
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
            let target_info = match self.monsters.get(&map_index) {
                Some(monsters) => monsters
                    .iter()
                    .find(|m| m.x == tx && m.y == ty)
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
                ) = if let Some(info) = self.provider.get_monster_info(monster_index) {
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

                let (hit, mut raw_damage, mut damage_type) =
                    compute_physical_melee_with_crit(attacker_stats, &defender_stats);

                if hit && raw_damage > 0 {
                    raw_damage = crate::world::skills::warrior::compute_cross_half_moon_damage_for_target(
                        &self.provider,
                        raw_damage,
                        level,
                        is_primary,
                    );

                    raw_damage = apply_fatal_sword_and_undead(
                        &self.provider,
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
                    self.mark_monster_dead(map_index, id);

                    if let Some(info) = self.provider.get_monster_info(monster_index) {
                        println!(
                            "[drop-debug] monster_index={} name='{}' drop_path='{}' drops_len={}",
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

                        println!(
                            "[drop-total] monster_index={} gold={} items_len={}",
                            monster_index,
                            total.gold,
                            total.items.len(),
                        );

                        if total.gold > 0 || !total.items.is_empty() {
                            let entry = self.map_items.entry(map_index).or_default();
                            let item_timeout_ms: i64 = 300_000;

                            if total.gold > 0 {
                                let item_id = self.next_map_item_id;
                                self.next_map_item_id =
                                    self.next_map_item_id.wrapping_add(1);
                                entry.push(crate::world::map_item::MapItem {
                                    id: item_id,
                                    map_index,
                                    x: strike_x,
                                    y: strike_y,
                                    item_index: None,
                                    gold: total.gold,
                                    count: 0,
                                    item: None,
                                    expire_time_ms: self.time_ms + item_timeout_ms,
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
                                let item_id = self.next_map_item_id;
                                self.next_map_item_id =
                                    self.next_map_item_id.wrapping_add(1);
                                entry.push(crate::world::map_item::MapItem {
                                    id: item_id,
                                    map_index,
                                    x: strike_x,
                                    y: strike_y,
                                    item_index: Some(item_index),
                                    gold: 0,
                                    count: 1,
                                    item: None,
                                    expire_time_ms: self.time_ms + item_timeout_ms,
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
                        if let Some(p) = self.players.get_mut(&session_id) {
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
}

