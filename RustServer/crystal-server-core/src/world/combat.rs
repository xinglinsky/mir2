use rand::thread_rng;

use crate::stats::Stat;
use crate::world::magic::magic_damage;
use crate::world::player::PlayerState;
use crate::world::provider::WorldProvider;

use super::{SessionId, World, WorldEvent};

const SPELL_FATAL_SWORD: u8 = crate::world::Spell::FatalSword as u8;

impl<P: WorldProvider> World<P> {
    fn can_attack(_player: &PlayerState) -> bool {
        true
    }

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
        let (map_index, x, y, direction, effective_spell, level, fatal_level, player_level) =
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
                    let player_level = p.level;

                    (
                        p.map_index,
                        p.x,
                        p.y,
                        p.direction,
                        effective_spell,
                        level,
                        fatal_level,
                        player_level,
                    )
                }
                None => {
                    return;
                }
            };

        let mut damage_base: i32 = Self::compute_physical_damage_base(player_level);
        let mut damage_final: i32 = damage_base;

        if let Some(fatal_level) = fatal_level {
            if let Some(info) = self.provider.get_magic_info(SPELL_FATAL_SWORD) {
                let mut rng = thread_rng();
                damage_base = magic_damage(info, fatal_level, damage_base, &mut rng);
                damage_final = damage_base;
            }
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

            let (undead, max_hp) = self
                .provider
                .get_monster_info(monster_index)
                .map(|info| {
                    let max_hp = info.stats.get(Stat::HP).max(1);
                    (info.undead, max_hp)
                })
                .unwrap_or((false, 1));

            if undead {
                let holy_bonus: i32 = 0;
                damage_base = damage_base.saturating_add(holy_bonus);
                damage_final = damage_base;
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

                    if damage_final > 0 {
                        damage_done = damage_final;

                        if damage_final >= m.hp {
                            m.hp = 0;
                            dead = true;
                        } else {
                            m.hp -= damage_final;
                        }

                        let remaining_hp = if dead { 0 } else { m.hp.max(0) };
                        if max_hp > 0 {
                            let pct = (remaining_hp as i64 * 100 / max_hp as i64)
                                .clamp(0, 100) as u8;
                            health_percent = pct;
                        } else {
                            health_percent = 0;
                        }
                    }
                }
            }

            if damage_done > 0 {
                events.push(WorldEvent::ObjectStruck {
                    attacker_id: session_id,
                    target_id: id,
                    map_index,
                    x: strike_x,
                    y: strike_y,
                    direction: strike_dir,
                    damage: damage_done,
                    damage_type: 0,
                    health_percent,
                });
            }

            if dead {
                self.mark_monster_dead(map_index, id);
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

