use crate::stats::Stat;
use crate::world::types::{BuffProperty, BuffType, PoisonType, DamageType};
use crate::world::Spell;
use crate::world::provider::WorldProvider;

use super::{SessionId, World, WorldEvent};

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

            // After processing expirations for this player, recompute whether
            // they should be logically Hidden based on remaining buffs. This
            // mirrors the C# MapObject.RemoveBuff / HumanObject.ProcessBuffs
            // behaviour where Hiding/MoonLight/DarkBody/ClearRing control the
            // Hidden flag.
            let old_hidden = player.hidden;
            let new_hidden = player.active_buffs.iter().any(|b| {
                matches!(
                    b.buff_type,
                    BuffType::Hiding | BuffType::MoonLight | BuffType::DarkBody | BuffType::ClearRing
                )
            });

            // Debug: whenever a player is or becomes hidden due to buffs,
            // log their state so we can diagnose unexpected permanent
            // invisibility in SafeZones.
            if new_hidden || old_hidden {
                let buff_types: Vec<u8> = player
                    .active_buffs
                    .iter()
                    .map(|b| b.buff_type as u8)
                    .collect();
                tracing::debug!(
                    "process_player_buffs: session_id={} map={} pos=({}, {}) in_safe_zone={} old_hidden={} new_hidden={} buff_types={:?}",
                    session_id,
                    player.map_index,
                    player.x,
                    player.y,
                    in_safe_zone,
                    old_hidden,
                    new_hidden,
                    buff_types,
                );
            }

            if new_hidden != old_hidden {
                player.hidden = new_hidden;

                events.push(WorldEvent::ObjectHidden {
                    object_id: session_id,
                    map_index: player.map_index,
                    x: player.x,
                    y: player.y,
                    hidden: new_hidden,
                });
            }
        }
    }

    /// Cancel any in-progress Taoist Reincarnation attempts whose
    /// ReincarnationExpireTime has passed, mirroring the legacy C#
    /// HumanObject timer logic which resets ReincarnationReady /
    /// ActiveReincarnation and notifies the caster that the attempt failed.
    pub fn process_reincarnation(&mut self, now_ms: i64, events: &mut Vec<WorldEvent>) {
        use crate::world::SessionId;

        let mut hosts: Vec<SessionId> = Vec::new();

        for (&sid, p) in &self.players {
            if !p.reincarnation_ready {
                continue;
            }
            if p.reincarnation_target_session_id.is_none() {
                continue;
            }
            if p.reincarnation_expire_time_ms > 0
                && now_ms >= p.reincarnation_expire_time_ms
            {
                hosts.push(sid);
            }
        }

        for host_sid in hosts {
            if self.cancel_reincarnation_for_session(host_sid).is_some() {
                events.push(WorldEvent::PartySystemMessage {
                    session_id: host_sid,
                    message: "Reincarnation failed.".to_string(),
                });
                events.push(WorldEvent::ReincarnationCancelled {
                    session_id: host_sid,
                });
            }
        }
    }

    /// Process per-player poisons, approximating the core behaviour of
    /// C# HumanObject.ProcessPoison for damage-over-time poisons. This
    /// currently handles Green/Bleeding DOT ticks and maintains a
    /// bitmask of active poison types and emits Poisoned/ObjectPoisoned
    /// style events for network synchronisation.
    pub fn process_player_poison(&mut self, now_ms: i64, events: &mut Vec<WorldEvent>) {
        // Collect PvP-attributable poison hits so we can route them through
        // the unified PK/Brown logic after releasing per-player borrows.
        let mut pending_poison_hits: Vec<(SessionId, SessionId, i32, i32)> = Vec::new();
        // Duration/tick semantics mirror the C# Poison struct where Time
        // counts ticks and TickTime/TickSpeed control the next tick.
        for player in self.players.values_mut() {
            if player.dead {
                continue;
            }

            let old_mask = player.current_poison_mask;

            if !player.poisons.is_empty() {
                let mut idx = player.poisons.len();
                while idx > 0 {
                    idx -= 1;

                    let mut remove = false;
                    {
                        let poison = &mut player.poisons[idx];

                        if now_ms > poison.tick_time_ms {
                            poison.time = poison.time.saturating_add(1);

                            let step = poison.tick_speed_ms.max(0);
                            poison.tick_time_ms = if step > 0 {
                                now_ms.saturating_add(step)
                            } else {
                                now_ms
                            };

                            if poison.time >= poison.duration {
                                remove = true;
                            }

                            match poison.poison_type {
                                PoisonType::Green | PoisonType::Bleeding => {
                                    let dmg = poison.value.max(0);
                                    if dmg > 0 {
                                        if let Some(att_sid) = poison.owner_session_id {
                                            // Route PvP poison damage through unified PK logic.
                                            if att_sid != player.session_id {
                                                pending_poison_hits.push((
                                                    att_sid,
                                                    player.session_id,
                                                    dmg,
                                                    player.map_index,
                                                ));
                                                // Delay regen even when damage is routed through PK path.
                                                const REGEN_DELAY_MS: i64 = 10_000;
                                                player.next_regen_time_ms =
                                                    now_ms.saturating_add(REGEN_DELAY_MS);
                                                // Skip direct HP change; handled in PK path.
                                                continue;
                                            }
                                        }

                                        let max_hp = player.stats.total.get(Stat::HP).max(1);
                                        let old_hp = player.hp.max(0).min(max_hp);

                                        let new_hp = old_hp.saturating_sub(dmg);
                                        player.hp = new_hp;
                                        if player.hp <= 0 {
                                            player.dead = true;
                                        }

                                        // Mirror C# RegenTime = Envir.Time + RegenDelay
                                        // so that poison damage delays natural regen.
                                        const REGEN_DELAY_MS: i64 = 10_000;
                                        player.next_regen_time_ms =
                                            now_ms.saturating_add(REGEN_DELAY_MS);
                                    }
                                }
                                PoisonType::Red => {
                                    // Red poison reduces both HP and MP
                                    let dmg = poison.value.max(0);
                                    if dmg > 0 {
                                        if let Some(att_sid) = poison.owner_session_id {
                                            if att_sid != player.session_id {
                                                pending_poison_hits.push((
                                                    att_sid,
                                                    player.session_id,
                                                    dmg,
                                                    player.map_index,
                                                ));
                                                // Regen delay still applies.
                                                const REGEN_DELAY_MS: i64 = 10_000;
                                                player.next_regen_time_ms =
                                                    now_ms.saturating_add(REGEN_DELAY_MS);
                                                // Apply MP reduction locally (PK path only handles HP).
                                                let max_mp = player.stats.total.get(Stat::MP).max(0);
                                                let old_mp = player.mp.max(0).min(max_mp);
                                                let new_mp = old_mp.saturating_sub(dmg);
                                                player.mp = new_mp;
                                                continue;
                                            }
                                        }

                                        // HP damage
                                        let max_hp = player.stats.total.get(Stat::HP).max(1);
                                        let old_hp = player.hp.max(0).min(max_hp);

                                        let new_hp = old_hp.saturating_sub(dmg);
                                        player.hp = new_hp;
                                        if player.hp <= 0 {
                                            player.dead = true;
                                        }

                                        // MP damage
                                        let max_mp = player.stats.total.get(Stat::MP).max(0);
                                        let old_mp = player.mp.max(0).min(max_mp);
                                        let new_mp = old_mp.saturating_sub(dmg);
                                        player.mp = new_mp;

                                        // Regen delay
                                        const REGEN_DELAY_MS: i64 = 10_000;
                                        player.next_regen_time_ms =
                                            now_ms.saturating_add(REGEN_DELAY_MS);
                                    }
                                }
                                PoisonType::DelayedExplosion => {
                                    // Explosion triggers at the end of duration
                                    if poison.time >= poison.duration {
                                        let dmg = poison.value.max(0) * 2; // Double damage for explosion
                                        if dmg > 0 {
                                            if let Some(att_sid) = poison.owner_session_id {
                                                if att_sid != player.session_id {
                                                    pending_poison_hits.push((
                                                        att_sid,
                                                        player.session_id,
                                                        dmg,
                                                        player.map_index,
                                                    ));
                                                    const REGEN_DELAY_MS: i64 = 10_000;
                                                    player.next_regen_time_ms =
                                                        now_ms.saturating_add(REGEN_DELAY_MS);
                                                    // Skip direct HP change; handled in PK path.
                                                    // Clear poison after explosion.
                                                    remove = true;
                                                    continue;
                                                }
                                            }

                                            let max_hp = player.stats.total.get(Stat::HP).max(1);
                                            let old_hp = player.hp.max(0).min(max_hp);

                                            let new_hp = old_hp.saturating_sub(dmg);
                                            player.hp = new_hp;
                                            if player.hp <= 0 {
                                                player.dead = true;
                                            }
                                        }
                                    }
                                }
                                PoisonType::Slow => {
                                    // Slow poison reduces movement speed
                                    // This would need to be handled by the movement system
                                    // For now, we just track the poison state
                                }
                                PoisonType::Frozen => {
                                    // Frozen prevents movement completely
                                    // This would need to be handled by the movement system
                                    // For now, we just track the poison state
                                }
                                PoisonType::Stun => {
                                    // Stun prevents all actions
                                    // This would need to be handled by the action system
                                    // For now, we just track the poison state
                                }
                                PoisonType::Paralysis | PoisonType::LRParalysis => {
                                    // Paralysis prevents movement but allows other actions
                                    // This would need to be handled by the movement system
                                    // For now, we just track the poison state
                                }
                                PoisonType::Blindness => {
                                    // Blindness reduces accuracy and visibility
                                    // This would need to be handled by the combat system
                                    // For now, we just track the poison state
                                }
                                PoisonType::Dazed => {
                                    // Dazed causes confusion in movement
                                    // This would need to be handled by the movement system
                                    // For now, we just track the poison state
                                }
                                _ => {}
                            }
                        }
                    }

                    if remove {
                        player.poisons.remove(idx);
                    }
                }
            }

            let mut mask: u16 = 0;
            for poison in &player.poisons {
                mask |= poison.poison_type.as_u16();
            }

            player.current_poison_mask = mask;

            if mask != old_mask {
                events.push(WorldEvent::Poisoned {
                    session_id: player.session_id,
                    poison: mask,
                });
            }
        }

        // Apply deferred PvP poison hits so PK/Brown/death logic mirrors C#.
        for (att_sid, tgt_sid, dmg, map_index) in pending_poison_hits {
            self.apply_player_hit_from_player(
                att_sid,
                tgt_sid,
                map_index,
                dmg,
                DamageType::Physical.as_u8(),
                None,
                None,
                None,
                true, // allow_pk_points
                events,
            );
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
                        show_healing_effect: false,
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

    /// Attempt to revive a player using a Revival ring when their HP reaches
    /// zero, approximating the C# PlayerObject.Die revival-ring logic. This
    /// checks the ring slots for items whose ItemInfo.unique has the
    /// SpecialItemMode.Revival (0x0010) flag and CurrentDura >= 1000, enforces
    /// a 5-minute cooldown via PlayerState.last_revival_time_ms, restores HP
    /// to max, reduces ring durability by 1000 and emits PlayerHealed and a
    /// system message if successful.
    pub fn try_revive_with_revival_ring(
        &mut self,
        session_id: crate::world::SessionId,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        const SPECIAL_REVIVAL: i16 = 0x0010;
        const RING_L: usize = 7;
        const RING_R: usize = 8;
        const DURABILITY_COST: u16 = 1000;
        const REVIVAL_COOLDOWN_MS: i64 = 300_000;

        let now = self.time_ms;

        let map_index: i32;
        let x: i32;
        let y: i32;
        let mut healed: i32 = 0;
        let mut new_hp: i32 = 0;
        let mut revived = false;

        {
            let player = match self.players.get_mut(&session_id) {
                Some(p) => p,
                None => return false,
            };

            // Only attempt revival when the player is actually at or below
            // zero HP and not already alive.
            if player.hp > 0 {
                return false;
            }

            if player.last_revival_time_ms != 0 && now <= player.last_revival_time_ms {
                return false;
            }

            map_index = player.map_index;
            x = player.x;
            y = player.y;

            for slot in RING_L..=RING_R {
                let slot_opt = match player.equipment.slots.get_mut(slot) {
                    Some(s) => s,
                    None => continue,
                };

                let ring = match slot_opt.as_mut() {
                    Some(r) => r,
                    None => continue,
                };

                let info = match self.provider.get_item_info(ring.item_index) {
                    Some(i) => i,
                    None => continue,
                };

                if (info.unique & SPECIAL_REVIVAL) == 0 {
                    continue;
                }

                if ring.current_dura < DURABILITY_COST {
                    continue;
                }

                let max_hp = player.stats.total.get(Stat::HP).max(1);
                let old_hp = player.hp.max(0);

                if max_hp <= old_hp {
                    continue;
                }

                player.hp = max_hp;
                player.dead = false;
                ring.current_dura = ring.current_dura.saturating_sub(DURABILITY_COST);
                player.last_revival_time_ms = now.saturating_add(REVIVAL_COOLDOWN_MS);

                healed = max_hp.saturating_sub(old_hp);
                new_hp = max_hp;
                revived = true;
                break;
            }
        }

        if !revived {
            return false;
        }

        // Recalculate equipment stats so the durability change is reflected in
        // derived stats, then emit a heal + system message.
        self.recalc_player_equipment_stats(session_id);

        if healed > 0 {
            events.push(WorldEvent::PlayerHealed {
                session_id,
                map_index,
                x,
                y,
                amount: healed,
                new_hp,
                show_healing_effect: false,
            });
        }

        events.push(WorldEvent::PartySystemMessage {
            session_id,
            message: "You have been given a second chance at life".to_string(),
        });

        true
    }
}
