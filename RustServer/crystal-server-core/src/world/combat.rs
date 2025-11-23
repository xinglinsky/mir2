use rand::thread_rng;

use crate::combat::compute_physical_melee_with_crit;
use crate::stats::{Stat, Stats};
use crate::world::magic::magic_damage;
use crate::world::player::PlayerState;
use crate::world::provider::WorldProvider;

use super::{SessionId, World, WorldEvent};

const SPELL_FATAL_SWORD: u8 = crate::world::Spell::FatalSword as u8;

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

    fn resolve_attack_spell_and_level(player: &PlayerState, requested_spell: u8) -> (u8, u8) {
        if requested_spell == 0 {
            return (0, 0);
        }

        if let Some(magic) = player.magics.iter().find(|m| m.spell == requested_spell) {
            (requested_spell, magic.level)
        } else {
            (0, 0)
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
                        Self::resolve_attack_spell_and_level(p, spell);
                    let fatal_level = p
                        .magics
                        .iter()
                        .find(|m| m.spell == SPELL_FATAL_SWORD)
                        .map(|m| m.level);
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
        let target_x = x + dx;
        let target_y = y + dy;

        let target_info = match self.monsters.get(&map_index) {
            Some(monsters) => monsters
                .iter()
                .find(|m| m.x == target_x && m.y == target_y)
                .map(|m| (m.id, m.monster_index)),
            None => None,
        };
        if let Some((id, monster_index)) = target_info {
            let mut dead = false;

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
            // Accuracy/Agility, AC/DR and crit) for player -> monster hits.
            let (hit, mut raw_damage, damage_type) =
                compute_physical_melee_with_crit(&attacker_stats, &defender_stats);

            // Apply FatalSword as an additional scalar on top of the physical
            // hit if present. This approximates C# UserMagic.GetDamage where
            // the magic modifies the base physical damage.
            if hit && raw_damage > 0 {
                if let Some(fatal_level) = fatal_level {
                    if let Some(info) = self.provider.get_magic_info(SPELL_FATAL_SWORD) {
                        let mut rng = thread_rng();
                        let boosted = magic_damage(info, fatal_level, raw_damage, &mut rng);
                        if boosted > 0 {
                            raw_damage = boosted;
                        }
                    }
                }

                if undead {
                    let holy_bonus: i32 = 0;
                    raw_damage = raw_damage.saturating_add(holy_bonus);
                }
            }

            let mut strike_x = target_x;
            let mut strike_y = target_y;
            let mut strike_dir = direction;
            let mut damage_done: i32 = 0;
            let mut health_percent: u8 = 100;

            if let Some(monsters) = self.monsters.get_mut(&map_index) {
                if let Some(m) = monsters.iter_mut().find(|m| m.id == id) {
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
                            });

                            events.push(WorldEvent::ItemDropped {
                                object_id: item_id,
                                map_index,
                                x: strike_x,
                                y: strike_y,
                                item_index,
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
}

