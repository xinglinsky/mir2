use crate::stats::Stat;
use crate::world::types::{BuffProperty, BuffType};
use crate::world::Spell;
use crate::world::provider::WorldProvider;

use super::{World, WorldEvent};

impl<P: WorldProvider> World<P> {
    pub fn process_player_buffs(&mut self, now_ms: i64, events: &mut Vec<WorldEvent>) {
        for (&session_id, player) in &mut self.players {
            let mut removed_indices: Vec<usize> = Vec::new();
            let mut stats_changed = false;

            // Determine whether the player is currently standing inside any
            // configured SafeZone for their map. This mirrors the C#
            // MapObject.InSafeZone check used by BuffProperty.PauseInSafeZone
            // semantics.
            let in_safe_zone = self
                .provider
                .get_map_info(player.map_index)
                .map(|info| Self::point_in_safe_zone(info, player.x, player.y))
                .unwrap_or(false);

            for (idx, buff) in player.active_buffs.iter_mut().enumerate() {
                // PauseInSafeZone: if the BuffInfo has this property, toggle
                // the paused flag when the player is in/out of a SafeZone and
                // maintain pause_remaining_ms so that remaining duration is
                // preserved while paused.
                if let Some(info) = self.provider.get_buff_info(buff.buff_type) {
                    if info.has_property(BuffProperty::PauseInSafeZone) {
                        if in_safe_zone && !buff.paused {
                            // Transition: running -> paused.
                            if !buff.infinite {
                                let remaining = buff.expire_time_ms.saturating_sub(now_ms);
                                buff.pause_remaining_ms = remaining.max(0);
                            } else {
                                buff.pause_remaining_ms = 0;
                            }
                            buff.paused = true;
                            events.push(WorldEvent::PauseBuff {
                                session_id,
                                buff_type: buff.buff_type.as_u8(),
                                paused: true,
                            });
                        } else if !in_safe_zone && buff.paused {
                            // Transition: paused -> running.
                            buff.paused = false;
                            if !buff.infinite {
                                let remaining = if buff.pause_remaining_ms > 0 {
                                    buff.pause_remaining_ms
                                } else {
                                    buff.expire_time_ms.saturating_sub(now_ms).max(0)
                                };
                                buff.expire_time_ms = now_ms.saturating_add(remaining);
                            }
                            buff.pause_remaining_ms = 0;
                            events.push(WorldEvent::PauseBuff {
                                session_id,
                                buff_type: buff.buff_type.as_u8(),
                                paused: false,
                            });
                        }
                    }
                }

                // Expiration: only advance and expire buffs that are not
                // infinite and not currently paused.
                if !buff.infinite && !buff.paused && buff.expire_time_ms <= now_ms {
                    removed_indices.push(idx);
                }
            }

            if removed_indices.is_empty() {
                continue;
            }

            // Sort descending to remove from back.
            removed_indices.sort_unstable_by(|a, b| b.cmp(a));

            for idx in removed_indices {
                let buff = player.active_buffs.remove(idx);
                stats_changed = true;

                if buff.buff_type == BuffType::FlamingSword {
                    events.push(WorldEvent::SpellToggle {
                        session_id,
                        spell_id: Spell::FlamingSword as u8,
                        enabled: false,
                    });
                } else {
                    events.push(WorldEvent::RemoveBuff {
                        session_id,
                        buff_type: buff.buff_type as u8,
                    });
                }
            }

            if stats_changed {
                player.stats.buffs.clear();
                for b in &player.active_buffs {
                    player.stats.buffs.add(&b.stats);
                }
                player.stats.recalc_if_dirty_for_job(player.job);
            }
        }
    }

    pub fn process_player_regen(&mut self, now_ms: i64, events: &mut Vec<WorldEvent>) {
        const REGEN_INTERVAL_MS: i64 = 10_000; // C# HumanObject.RegenDelay
        const HEALTH_REGEN_WEIGHT: i32 = 10; // Settings.HealthRegenWeight
        const MANA_REGEN_WEIGHT: i32 = 10; // Settings.ManaRegenWeight

        for player in self.players.values_mut() {
            if player.dead {
                continue;
            }

            let max_hp = player.stats.total.get(Stat::HP).max(0);
            let max_mp = player.stats.total.get(Stat::MP).max(0);

            if max_hp <= 0 && max_mp <= 0 {
                continue;
            }

            // Mirror CanRegen/regen delay semantics using next_regen_time_ms.
            if player.next_regen_time_ms != 0 && now_ms < player.next_regen_time_ms {
                continue;
            }
            player.next_regen_time_ms = now_ms.saturating_add(REGEN_INTERVAL_MS);

            let mut hp_regen: i32 = 0;
            let mut mp_regen: i32 = 0;

            if player.hp < max_hp {
                // Base: 3% of Max HP + 1, mirroring `(Stats[HP] * 0.03F) + 1`.
                let base = ((max_hp as f32 * 0.03_f32) as i32).saturating_add(1);
                let recovery = player.stats.total.get(Stat::HealthRecovery).max(0);
                let bonus = if HEALTH_REGEN_WEIGHT > 0 {
                    (base as i64 * recovery as i64 / HEALTH_REGEN_WEIGHT as i64) as i32
                } else {
                    0
                };
                hp_regen = base.saturating_add(bonus);
            }

            if player.mp < max_mp {
                // Base: 3% of Max MP + 1, mirroring `(Stats[MP] * 0.03F) + 1`.
                let base = ((max_mp as f32 * 0.03_f32) as i32).saturating_add(1);
                let recovery = player.stats.total.get(Stat::SpellRecovery).max(0);
                let bonus = if MANA_REGEN_WEIGHT > 0 {
                    (base as i64 * recovery as i64 / MANA_REGEN_WEIGHT as i64) as i32
                } else {
                    0
                };
                mp_regen = base.saturating_add(bonus);
            }

            if hp_regen > 0 && max_hp > 0 {
                let old_hp = player.hp;
                let new_hp = old_hp.saturating_add(hp_regen).min(max_hp);
                let amount = new_hp.saturating_sub(old_hp);
                if amount > 0 {
                    player.hp = new_hp;
                    events.push(WorldEvent::PlayerHealed {
                        session_id: player.session_id,
                        map_index: player.map_index,
                        x: player.x,
                        y: player.y,
                        amount,
                        new_hp,
                    });
                }
            }

            if mp_regen > 0 && max_mp > 0 {
                let old_mp = player.mp;
                let new_mp = old_mp.saturating_add(mp_regen).min(max_mp);
                if new_mp > old_mp {
                    player.mp = new_mp;
                }
            }
        }
    }

    pub fn process_player_pk(&mut self, now_ms: i64) {
        const PK_DECAY_INTERVAL_MS: i64 = 12_000;

        for player in self.players.values_mut() {
            if player.pk_points > 0 {
                if player.next_pk_decay_ms == 0 || now_ms >= player.next_pk_decay_ms {
                    player.pk_points = player.pk_points.saturating_sub(1);
                    player.next_pk_decay_ms = now_ms.saturating_add(PK_DECAY_INTERVAL_MS);
                }
            } else {
                player.next_pk_decay_ms = 0;
            }
        }
    }
}
