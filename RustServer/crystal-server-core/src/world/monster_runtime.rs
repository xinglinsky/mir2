use rand::Rng;
use rand::thread_rng;
use tracing::{trace, debug};
use std::fs;
use crate::combat::compute_physical_melee_with_crit;
use crate::stats::{Stat, Stats};
use crate::world::map::{self, RespawnInfo};
use crate::world::monster::{MonsterAiState, MonsterInstance};
use crate::world::provider::WorldProvider;
use crate::world::types::AttackMode;
use crate::world::PoisonType;
use crate::world::configs::setup_config;
use crate::world::Spell;

use super::{World, WorldEvent, PendingMagicHit, SessionId};

#[derive(Clone, Debug)]
pub struct RoutePoint {
    pub x: i32,
    pub y: i32,
    pub delay_ms: i64,
}

/// Runtime state for a single RespawnInfo entry on a map. This mirrors a subset
/// of the C# MapRespawn fields so that we can later drive timed respawns from
/// World::update without re-reading map metadata.
#[derive(Clone, Debug)]
pub struct RespawnRuntime {
    pub info: RespawnInfo,
    pub map_index: i32,
    /// Current number of monsters spawned for this respawn point. This will be
    /// kept in sync with the World.monsters list once death/despawn logic is
    /// implemented.
    pub current_count: u16,
    /// Next time (in ms since Unix epoch) when this respawn should attempt to
    /// spawn more monsters. This will be derived from RespawnInfo.delay and
    /// random_delay once World::update starts driving spawns.
    pub next_spawn_time_ms: i64,
    pub next_spawn_tick: u64,
    /// Simple error counter mirroring the C# MapRespawn.ErrorCount field; for
    /// now it is unused but reserved for future spawn failure backoff.
    pub error_count: u8,
    pub route_points: Option<Vec<RoutePoint>>,
}

impl<P: WorldProvider> World<P> {
    pub(super) fn spawn_monsters_for_map(&mut self, map_index: i32, map: &map::Map) {
        if self.monsters.contains_key(&map_index) {
            return;
        }

        let mut instances = Vec::new();

        // Initialise respawn runtime state for this map if we have not already
        // done so. For now we simply record the RespawnInfo entries together
        // with their initial Count; World::update will later evolve this into
        // a timed respawn system driven by delay/random_delay/respawn_ticks.
        if !self.respawns.contains_key(&map_index) {
            let mut runtimes = Vec::new();
            for respawn in &map.info.respawns {
                if respawn.monster_index <= 0 {
                    continue;
                }

                // Ensure the monster definition exists in the DB; if not, skip.
                if self
                    .provider
                    .get_monster_info(respawn.monster_index)
                    .is_none()
                {
                    continue;
                }

                runtimes.push(RespawnRuntime {
                    info: respawn.clone(),
                    map_index,
                    current_count: respawn.count,
                    next_spawn_time_ms: 0,
                    next_spawn_tick: 0,
                    error_count: 0,
                    route_points: self.load_route_points(&respawn.route_path),
                });
            }
            self.respawns.insert(map_index, runtimes);
        }

        for respawn in &map.info.respawns {
            if respawn.monster_index <= 0 {
                continue;
            }

            // Ensure the monster definition exists in the DB; if not, skip.
            if self
                .provider
                .get_monster_info(respawn.monster_index)
                .is_none()
            {
                continue;
            }

            instances.extend(self.create_monsters_from_respawn(map, respawn));
        }

        for m in &instances {
            self.add_monster_to_occupancy(m.id, m.map_index, m.x, m.y);
        }

        self.monsters.insert(map_index, instances);
    }

    fn load_route_points(&self, route_path: &str) -> Option<Vec<RoutePoint>> {
        if route_path.is_empty() {
            return None;
        }

        let file_name = format!("{route_path}.txt");
        let full_path = self.config.routes_path.join(file_name);

        let contents = fs::read_to_string(&full_path).ok()?;
        let mut points = Vec::new();

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() < 2 {
                continue;
            }

            let Ok(x) = parts[0].parse::<i32>() else { continue };
            let Ok(y) = parts[1].parse::<i32>() else { continue };
            let delay_ms = if parts.len() >= 3 {
                parts[2].parse::<i64>().unwrap_or(0)
            } else {
                0
            };

            points.push(RoutePoint { x, y, delay_ms });
        }

        if points.is_empty() {
            None
        } else {
            Some(points)
        }
    }

    pub fn monsters_for_map(&self, map_index: i32) -> Vec<MonsterInstance> {
        self.monsters
            .get(&map_index)
            .cloned()
            .unwrap_or_default()
    }

    /// Query monsters on a given map within a rectangular view range around the
    /// provided centre, returning cloned MonsterInstance values. This mirrors
    /// the visibility logic currently used by the connection layer.
    pub fn monsters_in_view_for_map(
        &self,
        map_index: i32,
        centre_x: i32,
        centre_y: i32,
        range: i32,
    ) -> Vec<MonsterInstance> {
        self.monsters
            .get(&map_index)
            .into_iter()
            .flat_map(|monsters| monsters.iter())
            .filter(|m| {
                (m.x - centre_x).abs() <= range && (m.y - centre_y).abs() <= range
            })
            .cloned()
            .collect()
    }

    /// World update tick. This will eventually drive respawns, monster AI,
    /// buffs and other timed systems.
    #[allow(unused_assignments)]
    pub fn update(&mut self, now_ms: i64) -> Vec<WorldEvent> {
        self.time_ms = now_ms;
        self.update_respawn_tick_counter(now_ms);

        let mut events: Vec<WorldEvent> = Vec::new();

        // Process expired map items: remove items that have exceeded their
        // expire_time_ms, mirroring C# ItemObject.Process() behavior.
        self.process_map_items(now_ms, &mut events);

        // Process player buffs: check for expiration
        self.process_player_buffs(now_ms, &mut events);

        // Natural HP/MP regeneration for players, mirroring the core of C#
        // HumanObject.ProcessRegen (RegenDelay, 3% max HP/MP and recovery
        // stats). This runs independently of SafeZone healing.
        self.process_player_regen(now_ms, &mut events);

        // Process player poisons after regen, approximating the C# ordering
        // of ProcessRegen followed by ProcessPoison on HumanObject.
        self.process_player_poison(now_ms, &mut events);

        self.process_player_pk(now_ms);

        // SafeZoneHealing: periodically heal players standing inside any
        // configured SafeZone when Settings.SafeZoneHealing is enabled. This
        // mirrors the effect of C# Healing spell objects placed by
        // Map.CreateSafeZone but applies healing directly to player HP.
        if self.config.safe_zone_healing {
            const SAFEZONE_HEAL_INTERVAL_MS: i64 = 2_000;
            let due = self.safezone_heal_last_ms == 0
                || now_ms.saturating_sub(self.safezone_heal_last_ms) >= SAFEZONE_HEAL_INTERVAL_MS;
            if due {
                self.process_safezone_healing(&mut events);
                self.safezone_heal_last_ms = now_ms;
            }
        }

        self.process_fire_walls(now_ms, &mut events);

        self.process_monster_buffs(now_ms);

        let mut jobs: Vec<(i32, usize, u16)> = Vec::new();
        let mut total_spawned: u32 = 0;
        let mut total_jobs: u32 = 0;

        let map_indices: Vec<i32> = self.respawns.keys().cloned().collect();

        for map_index in &map_indices {
            if let Some(runtimes) = self.respawns.get_mut(map_index) {
                for (idx, rt) in runtimes.iter_mut().enumerate() {
                    let base_count = rt.info.count as u32;
                    let multiplier = self.spawn_multiplier.max(1) as u32;
                    let target_total =
                        base_count.saturating_mul(multiplier).min(u16::MAX as u32) as u16;

                    if rt.info.respawn_ticks != 0 {
                        let current_tick = self.respawn_tick_counter;

                        if rt.next_spawn_tick == 0 {
                            let delta = rt.info.respawn_ticks as u64;
                            rt.next_spawn_tick = current_tick.wrapping_add(delta);
                            continue;
                        }

                        if current_tick < rt.next_spawn_tick {
                            continue;
                        }

                        if rt.current_count < target_total {
                            let needed = target_total - rt.current_count;
                            if needed > 0 {
                                jobs.push((*map_index, idx, needed));
                            }
                        }

                        let delta = rt.info.respawn_ticks as u64;
                        rt.next_spawn_tick = current_tick.wrapping_add(delta);
                    } else {
                        if rt.next_spawn_time_ms == 0 {
                            let delay_ms = Self::compute_respawn_delay_ms(&rt.info);
                            rt.next_spawn_time_ms = now_ms.saturating_add(delay_ms);
                            continue;
                        }

                        if now_ms < rt.next_spawn_time_ms {
                            continue;
                        }

                        if rt.current_count < target_total {
                            let needed = target_total - rt.current_count;
                            if needed > 0 {
                                jobs.push((*map_index, idx, needed));
                            }
                        }

                        let delay_ms = Self::compute_respawn_delay_ms(&rt.info);
                        rt.next_spawn_time_ms = now_ms.saturating_add(delay_ms);
                    }
                }
            }
        }

        for (map_index, rt_index, needed) in jobs {
            let map = match self.get_or_load_map(map_index) {
                Some(m) => m,
                None => continue,
            };

            let base_info = match self
                .respawns
                .get(&map_index)
                .and_then(|v| v.get(rt_index))
            {
                Some(rt) => &rt.info,
                None => continue,
            };

            let mut tmp = base_info.clone();
            tmp.count = needed;
            let spawned = self.create_monsters_from_respawn(&map, &tmp);

            let actual = spawned.len() as u16;
            if actual == 0 {
                if let Some(runtimes) = self.respawns.get_mut(&map_index) {
                    if let Some(rt) = runtimes.get_mut(rt_index) {
                        if rt.info.respawn_ticks == 0 {
                            let minute_ms = 60_000_i64;
                            rt.next_spawn_time_ms =
                                self.time_ms.saturating_add(minute_ms);
                        }
                        if rt.error_count < u8::MAX {
                            rt.error_count = rt.error_count.saturating_add(1);
                        }
                    }
                }
                continue;
            }

            for m in &spawned {
                self.add_monster_to_occupancy(m.id, m.map_index, m.x, m.y);
            }

            let monsters_on_map = self.monsters.entry(map_index).or_default();
            monsters_on_map.extend(spawned);

            if let Some(runtimes) = self.respawns.get_mut(&map_index) {
                if let Some(rt) = runtimes.get_mut(rt_index) {
                    rt.current_count = rt.current_count.saturating_add(actual);
                    rt.error_count = 0;
                }
            }
            if actual > 0 {
                total_spawned = total_spawned.saturating_add(actual as u32);
                total_jobs = total_jobs.saturating_add(1);
            }
        }

        if total_spawned > 0 {
            debug!(
                "respawn_update: t={} tick={} multiplier={} base_min={} jobs={} spawned={}",
                self.time_ms,
                self.respawn_tick_counter,
                self.spawn_multiplier,
                self.respawn_base_spawn_rate_minutes,
                total_jobs,
                total_spawned,
            );
        }

        self.process_monster_ai(now_ms, &mut events);
        self.process_guard_ai(now_ms, &mut events);

        // Process monster poisons before resolving any pending magic hits so
        // that poison damage is applied through the same pipeline as other
        // magic-based attacks (drops/experience attribution, etc.).
        self.process_monster_poison(now_ms, &mut events);

        self.process_pending_magic_hits(now_ms, &mut events);

        // After processing combat and AI for this tick, remove any monsters
        // whose corpses have expired. This mirrors C# MonsterObject.Dead /
        // DeadTime behaviour: monsters remain in the world as corpses for a
        // short period (so the client can render them) before being removed
        // from the map and respawn counts updated.
        self.cleanup_dead_monsters(now_ms);

        events
    }

    fn process_fire_walls(&mut self, now_ms: i64, events: &mut Vec<WorldEvent>) {
        if self.fire_walls.is_empty() {
            return;
        }

        let instances = std::mem::take(&mut self.fire_walls);
        let mut remaining = Vec::new();

        for mut fw in instances {
            let spell_id = fw.spell_id;
            // C# SpellObject.Process has special handling for FireWall:
            // - If Caster becomes null, or
            // - If CurrentMap != Caster.CurrentMap
            // then the spell object is removed immediately, even before
            // ExpireTime. We approximate this by checking whether the caster
            // player still exists on the same map.
            let caster_ok = match self.players.get(&fw.caster_session_id) {
                Some(p) if p.map_index == fw.map_index => true,
                _ => false,
            };

            if !caster_ok {
                tracing::debug!(
                    "[firewall] removed because caster invalid: map_index={} caster_session_id={} value={} now_ms={} expire_time_ms={} cell_count={}",
                    fw.map_index,
                    fw.caster_session_id,
                    fw.value,
                    now_ms,
                    fw.expire_time_ms,
                    fw.cells.len(),
                );
                for &(x, y) in &fw.cells {
                    self.remove_map_spell(fw.map_index, x, y, spell_id);
                    events.push(WorldEvent::MapSpellRemoved {
                        map_index: fw.map_index,
                        x,
                        y,
                        spell: spell_id,
                    });
                }
                continue;
            }

            if now_ms > fw.expire_time_ms {
                tracing::debug!(
                    "[firewall] removed because expired: map_index={} caster_session_id={} value={} now_ms={} expire_time_ms={} cell_count={}",
                    fw.map_index,
                    fw.caster_session_id,
                    fw.value,
                    now_ms,
                    fw.expire_time_ms,
                    fw.cells.len(),
                );
                for &(x, y) in &fw.cells {
                    self.remove_map_spell(fw.map_index, x, y, Spell::FireWall as u8);
                    events.push(WorldEvent::MapSpellRemoved {
                        map_index: fw.map_index,
                        x,
                        y,
                        spell: Spell::FireWall as u8,
                    });
                }
                continue;
            }

            if now_ms >= fw.next_tick_ms {
                let step = fw.tick_speed_ms.max(0);
                if step > 0 {
                    fw.next_tick_ms = now_ms.saturating_add(step);
                } else {
                    fw.next_tick_ms = now_ms;
                }

                let map_index = fw.map_index;
                let caster_session_id = fw.caster_session_id;
                let value = fw.value;

                if value > 0 {
                    // Damage monsters standing on any FireWall tile by
                    // scheduling PendingMagicHit entries, reusing the common
                    // monster damage + drops/experience pipeline.
                    for &(cx, cy) in &fw.cells {
                        if let Some(monsters) = self.monsters.get(&map_index) {
                            for m in monsters.iter() {
                                if m.hp <= 0 {
                                    continue;
                                }
                                if m.x != cx || m.y != cy {
                                    continue;
                                }
                                if !self.can_attack_monster(caster_session_id, map_index, m.id) {
                                    continue;
                                }

                                let due_time_ms = now_ms;
                                self.pending_magic_hits.push(PendingMagicHit {
                                    due_time_ms,
                                    attacker_session_id: caster_session_id,
                                    map_index,
                                    target_monster_id: m.id,
                                    monster_index: m.monster_index,
                                    spell_id,
                                    damage: value,
                                    damage_type: 0,
                                });
                            }
                        }
                    }

                    // Damage players standing on any FireWall tile, using the
                    // same PK and safe-zone rules as C#
                    // PlayerObject.IsAttackTarget(HumanObject attacker) via
                    // can_attack_player, and approximating DefenceType.MAC by
                    // sampling MinMAC/MaxMAC and applying
                    // DamageReductionPercent. We then route the actual
                    // HP/death + PK logic through the unified
                    // apply_player_hit_from_player helper so that FireWall
                    // kills behave like other player-vs-player attacks.
                    let mut player_targets: Vec<SessionId> = Vec::new();
                    for (&sid, p) in &self.players {
                        if p.map_index != map_index || p.dead || p.hp <= 0 {
                            continue;
                        }

                        if !fw.cells.contains(&(p.x, p.y)) {
                            continue;
                        }

                        if !self.can_attack_player(caster_session_id, sid) {
                            continue;
                        }

                        player_targets.push(sid);
                    }

                    for sid in player_targets {
                        // Compute FireWall damage against this player's
                        // MAC/DR without mutating state; the unified helper
                        // will handle HP/death and PK side-effects.
                        let dmg = {
                            let p = match self.players.get(&sid) {
                                Some(p) => p,
                                None => continue,
                            };

                            let stats = &p.stats.total;

                            // Sample a MAC-based armour value similar to how
                            // physical armour is sampled, but using
                            // MinMAC/MaxMAC instead of MinAC/MaxAC.
                            let min_mac = stats.get(Stat::MinMAC).max(0);
                            let max_mac = stats.get(Stat::MaxMAC).max(min_mac);
                            let mut dmg = value;

                            if max_mac > 0 {
                                let mut rng = thread_rng();
                                let armour = rng.gen_range(min_mac..=max_mac);
                                dmg = dmg.saturating_sub(armour);
                            }

                            if dmg <= 0 {
                                0
                            } else {
                                // Apply generic damage reduction percent on
                                // the defender, mirroring physical damage
                                // handling.
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

                        self.apply_player_hit_from_player(
                            caster_session_id,
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

            remaining.push(fw);
        }

        self.fire_walls = remaining;
    }

    fn process_monster_poison(&mut self, now_ms: i64, events: &mut Vec<WorldEvent>) {
        for monsters in self.monsters.values_mut() {
            for monster in monsters.iter_mut() {
                if monster.hp <= 0 {
                    continue;
                }

                let old_mask = monster.current_poison_mask;

                if !monster.poisons.is_empty() {
                    let mut idx = monster.poisons.len();
                    while idx > 0 {
                        idx -= 1;

                        let mut remove = false;
                        let (ptype, value, owner_sid) = {
                            let poison = &mut monster.poisons[idx];

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

                                (
                                    poison.poison_type,
                                    poison.value.max(0),
                                    poison.owner_session_id,
                                )
                            } else {
                                // No tick this update; skip damage and keep the
                                // current poison entry.
                                (
                                    poison.poison_type,
                                    0,
                                    poison.owner_session_id,
                                )
                            }
                        };

                        if remove {
                            monster.poisons.remove(idx);
                        }

                        // For Green/Bleeding poisons applied by players, route the
                        // damage through the existing PendingMagicHit pipeline so
                        // that drops and experience attribution behave like other
                        // magic-based attacks.
                        if (ptype == PoisonType::Green || ptype == PoisonType::Bleeding)
                            && value > 0
                        {
                            if let Some(attacker_sid) = owner_sid {
                                self.pending_magic_hits.push(PendingMagicHit {
                                    due_time_ms: now_ms,
                                    attacker_session_id: attacker_sid,
                                    map_index: monster.map_index,
                                    target_monster_id: monster.id,
                                    monster_index: monster.monster_index,
                                    spell_id: Spell::Poisoning as u8,
                                    damage: value,
                                    damage_type: 0,
                                });
                            }
                        }
                    }
                }

                let mut mask: u16 = 0;
                for poison in &monster.poisons {
                    mask |= poison.poison_type.as_u16();
                }
                monster.current_poison_mask = mask;

                if mask != old_mask {
                    events.push(WorldEvent::ObjectPoisoned {
                        object_id: monster.id,
                        map_index: monster.map_index,
                        x: monster.x,
                        y: monster.y,
                        poison: mask,
                    });
                }
            }
        }
    }

    fn process_pending_magic_hits(&mut self, now_ms: i64, events: &mut Vec<WorldEvent>) {
        if self.pending_magic_hits.is_empty() {
            return;
        }

        let mut remaining: Vec<PendingMagicHit> = Vec::new();
        let hits = std::mem::take(&mut self.pending_magic_hits);

        for hit in hits {
            if hit.due_time_ms > now_ms {
                remaining.push(hit);
                continue;
            }

            self.apply_pending_magic_hit(hit, events);
        }

        self.pending_magic_hits = remaining;
    }

    fn apply_pending_magic_hit(&mut self, hit: PendingMagicHit, events: &mut Vec<WorldEvent>) {
        let PendingMagicHit {
            due_time_ms: _,
            attacker_session_id,
            map_index,
            target_monster_id,
            monster_index,
            spell_id,
            damage,
            damage_type,
        } = hit;

        if damage <= 0 {
            return;
        }

        // Look up monster definition for max HP, undead flag and drops/experience.
        let (monster_exp, max_hp, monster_drops) = if let Some(info) =
            self.provider.get_monster_info(monster_index)
        {
            (
                info.experience,
                info.stats.get(Stat::HP).max(1),
                info.drops.clone(),
            )
        } else {
            (0, 1, Vec::new())
        };

        let strike_x;
        let strike_y;
        let strike_dir: u8;
        let mut damage_done: i32 = 0;
        let health_percent: u8;
        let mut dead = false;

        if let Some(monsters) = self.monsters.get_mut(&map_index) {
            if let Some(m) = monsters.iter_mut().find(|m| m.id == target_monster_id) {
                // If the monster is already dead or has zero HP, there is
                // nothing left to apply.
                if m.hp <= 0 {
                    return;
                }

                m.target_session_id = Some(attacker_session_id);
                m.ai_state = MonsterAiState::Chase;

                strike_x = m.x;
                strike_y = m.y;
                strike_dir = m.direction;

                let old_hp = m.hp.max(0);
                let mut new_hp = old_hp;

                if damage > 0 {
                    damage_done = damage;

                    if damage >= m.hp {
                        m.hp = 0;
                        dead = true;
                    } else {
                        m.hp -= damage;
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
            } else {
                // Target monster no longer exists on this map.
                return;
            }
        } else {
            // Map has no monsters; nothing to do.
            return;
        }

        if damage_done <= 0 {
            return;
        }

        if spell_id == Spell::TwinDrakeBlade as u8 {
            if let Some(info) = self.provider.get_monster_info(monster_index) {
                let target_resist = info.stats.get(Stat::PoisonResist).max(0);
                let cfg = setup_config();
                let weight = cfg.items.poison_attack_weight.max(1) as i32;
                let mut rng = thread_rng();
                let roll = rng.gen_range(0..weight.max(1));
                if roll >= target_resist {
                    if let Some(player) = self.players.get(&attacker_session_id) {
                        if let Some(magic) = player
                            .magics
                            .iter()
                            .find(|m| m.spell == Spell::TwinDrakeBlade as u8)
                        {
                            let level = magic.level as i32;
                            let check = rng.gen_range(0..20);
                            if check <= level + 1 {
                                let duration_secs: i64 = 2_i64.saturating_add(level as i64);
                                let duration_ms = duration_secs.saturating_mul(1_000);
                                let applied = self.apply_poison_to_monster_from_player(
                                    attacker_session_id,
                                    map_index,
                                    target_monster_id,
                                    PoisonType::Stun,
                                    1,
                                    duration_ms,
                                    1_000,
                                );

                                if applied {
                                    const TWIN_DRAKE_BLADE_EFFECT: u8 = 5;
                                    events.push(WorldEvent::ObjectEffect {
                                        session_id: attacker_session_id,
                                        effect: TWIN_DRAKE_BLADE_EFFECT,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        events.push(WorldEvent::ObjectStruck {
            attacker_id: attacker_session_id,
            target_id: target_monster_id,
            map_index,
            x: strike_x,
            y: strike_y,
            direction: strike_dir,
            damage: damage_done,
            damage_type,
            health_percent,
        });

        if !dead {
            return;
        }

        // Monster death: update respawn counts, drops and experience.
        self.mark_monster_dead(map_index, target_monster_id);

        if !monster_drops.is_empty() {
            let (item_offset, gold_offset) = if let Some(p) = self.players.get(&attacker_session_id)
            {
                (
                    p.stats.total.get(Stat::ItemDropRatePercent),
                    p.stats.total.get(Stat::GoldDropRatePercent),
                )
            } else {
                (0, 0)
            };

            let mut rng = thread_rng();
            let mut total = crate::world::drop::DropRewardInfo {
                items: Vec::new(),
                gold: 0,
            };

            for d in &monster_drops {
                // Mirror C# MonsterObject.Drop quest-required behaviour:
                // skip quest-only drops when placing map items.
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

            if let Some(info) = self.provider.get_monster_info(monster_index) {
                trace!(
                    "[drop] monster_index={} name='{}' drop_path='{}' drops_len={}",
                    monster_index,
                    info.name,
                    info.drop_path,
                    monster_drops.len(),
                );
            }

            debug!(
                "[drop-total-pending-magic] monster_index={} gold={} items_len={}",
                monster_index,
                total.gold,
                total.items.len(),
            );

            if total.gold > 0 || !total.items.is_empty() {
                let item_timeout_ms: i64 = 300_000; // 5 minutes

                if total.gold > 0 {
                    if let Some((drop_x, drop_y)) =
                        self.find_drop_location(map_index, strike_x, strike_y, 4)
                    {
                        let item_id = self.next_map_item_id;
                        self.next_map_item_id = self.next_map_item_id.wrapping_add(1);
                        let entry = self.map_items.entry(map_index).or_default();
                        entry.push(crate::world::map_item::MapItem {
                            id: item_id,
                            map_index,
                            x: drop_x,
                            y: drop_y,
                            item_index: None,
                            gold: total.gold,
                            count: 0,
                            item: None,
                            expire_time_ms: self.time_ms + item_timeout_ms,
                        });

                        events.push(WorldEvent::GoldDropped {
                            object_id: item_id,
                            map_index,
                            x: drop_x,
                            y: drop_y,
                            gold: total.gold,
                        });
                    }
                }

                for item_index in total.items {
                    if let Some((drop_x, drop_y)) =
                        self.find_drop_location(map_index, strike_x, strike_y, 4)
                    {
                        let item_id = self.next_map_item_id;
                        self.next_map_item_id = self.next_map_item_id.wrapping_add(1);
                        let entry = self.map_items.entry(map_index).or_default();
                        entry.push(crate::world::map_item::MapItem {
                            id: item_id,
                            map_index,
                            x: drop_x,
                            y: drop_y,
                            item_index: Some(item_index),
                            gold: 0,
                            count: 1,
                            item: None,
                            expire_time_ms: self.time_ms + item_timeout_ms,
                        });

                        events.push(WorldEvent::ItemDropped {
                            object_id: item_id,
                            map_index,
                            x: drop_x,
                            y: drop_y,
                            item_index,
                            count: 1,
                        });

                        // Global drop notify for monster drops: if the item is
                        // flagged with global_drop_notify, broadcast a system
                        // message to all players indicating that this monster
                        // has dropped the item.
                        if let Some(info) = self.provider.get_item_info(item_index) {
                            if info.global_drop_notify {
                                let base_name = info.friendly_name();
                                let monster_name = self
                                    .provider
                                    .get_monster_info(monster_index)
                                    .map(|mi| mi.name.clone())
                                    .unwrap_or_else(|| "Monster".to_string());
                                let text = format!(
                                    "{} has dropped {}.",
                                    monster_name,
                                    base_name
                                );
                                for (&sid, _) in self.players.iter() {
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
        }

        if monster_exp > 0 {
            // Use the shared levelling helper so normal experience gain can
            // trigger level-ups and associated ranking updates.
            let _ = self.gain_experience_for_session(attacker_session_id, monster_exp, events);

            events.push(WorldEvent::GainExperience {
                session_id: attacker_session_id,
                amount: monster_exp,
            });
        }

        events.push(WorldEvent::MonsterDied {
            object_id: target_monster_id,
            map_index,
            x: strike_x,
            y: strike_y,
            direction: strike_dir,
        });
    }

    fn process_monster_buffs(&mut self, now_ms: i64) {
        for monsters in self.monsters.values_mut() {
            for monster in monsters.iter_mut() {
                // Skip any monsters that have already been reduced to 0 HP by
                // previous combat logic (player skills, guards, etc.). Dead
                // monsters remain in the monsters list so that the client can
                // continue to render their corpses, but they should not run
                // further AI.
                if monster.hp <= 0 {
                    continue;
                }
                if monster.buffs.is_empty() {
                    continue;
                }

                let mut removed_indices: Vec<usize> = Vec::new();

                for (idx, buff) in monster.buffs.iter_mut().enumerate() {
                    if buff.infinite {
                        continue;
                    }
                    if buff.flag_for_removal
                        || (buff.expire_time_ms > 0 && buff.expire_time_ms <= now_ms)
                    {
                        removed_indices.push(idx);
                    }
                }

                if removed_indices.is_empty() {
                    continue;
                }

                removed_indices.sort_unstable_by(|a, b| b.cmp(a));

                for idx in removed_indices {
                    monster.buffs.remove(idx);
                }

                monster.buff_stats.clear();
                for b in &monster.buffs {
                    monster.buff_stats.add(&b.stats);
                }
            }
        }
    }

    fn process_monster_ai(&mut self, now_ms: i64, events: &mut Vec<WorldEvent>) {
        let player_positions: Vec<(u32, i32, i32, i32, u8)> = self
            .players
            .iter()
            .filter(|(_, p)| !p.dead && p.hp > 0)
            .map(|(&sid, p)| (sid, p.map_index, p.x, p.y, p.direction))
            .collect();

        let mut pending_attacks: Vec<(u64, i32, u32, i32)> = Vec::new();
        let mut pending_pet_attacks: Vec<(u64, i32, u64, i32, u32, i32)> = Vec::new();

        let mut rng = thread_rng();

        // Temporarily take ownership of the monsters map to avoid borrowing
        // conflicts while iterating. This allows us to call methods on `self`
        // (like get_or_load_map, is_cell_blocked, occupancy updates) inside
        // the loop.
        let mut monsters_map = std::mem::take(&mut self.monsters);

        for (map_index, monsters) in monsters_map.iter_mut() {
            let map_index = *map_index;

            // Load the map for this index so that movement checks can respect
            // walkability and bounds, mirroring the player movement logic.
            let map = match self.get_or_load_map(map_index) {
                Some(m) => m,
                None => continue,
            };

            // Snapshot hostile (non-pet) monsters on this map so that pets can
            // select nearby targets without borrowing conflicts while we
            // iterate over monsters mutably.
            let hostile_monsters: Vec<(u64, i32, i32, i32)> = monsters
                .iter()
                .filter(|m| m.hp > 0 && !m.is_pet)
                .map(|m| (m.id, m.monster_index, m.x, m.y))
                .collect();

            for monster in monsters.iter_mut() {
                if monster.hp <= 0 {
                    monster.ai_state = MonsterAiState::Idle;
                    monster.target_session_id = None;
                    continue;
                }

                let (ai, view_range, move_speed, attack_speed) = {
                    let Some(info) = self.provider.get_monster_info(monster.monster_index) else {
                        monster.ai_state = MonsterAiState::Idle;
                        monster.target_session_id = None;
                        continue;
                    };
                    (
                        info.ai,
                        info.view_range as i32,
                        info.move_speed,
                        info.attack_speed,
                    )
                };

                // Pet-specific AI is delegated to pet_runtime to keep this
                // module focused on generic monster behaviour.
                if monster.is_pet {
                    self.process_pet_ai_for_monster(
                        now_ms,
                        map_index,
                        &map,
                        move_speed,
                        attack_speed,
                        monster,
                        &hostile_monsters,
                        &mut pending_pet_attacks,
                        events,
                    );
                    continue;
                }

                if matches!(ai, 6 | 57 | 58 | 102 | 103 | 104 | 105 | 113) {
                    monster.ai_state = MonsterAiState::Idle;
                    monster.target_session_id = None;
                    continue;
                }

                // Use the monster's configured view range to decide when to
                // start chasing nearby players.
                if view_range <= 0 {
                    monster.ai_state = MonsterAiState::Idle;
                    monster.target_session_id = None;
                    continue;
                }

                // -----------------------------
                // CheckAlone: periodically determine whether this monster has
                // any players nearby on its map. Mirrors C#
                // MonsterObject.CheckAlone, which uses AloneTime/AloneDelay and
                // Globals.DataRange * 2 to decide if the monster should
                // consider itself "alone" and potentially skip AI work when no
                // players are around.
                // -----------------------------
                const ALONE_DELAY_MS: i64 = 3_000; // C# AloneDelay
                const ALONE_RANGE: i32 = 32; // Globals.DataRange (16) * 2

                if now_ms >= monster.alone_time_ms {
                    monster.alone_time_ms = now_ms.saturating_add(ALONE_DELAY_MS);

                    let mut has_player_on_map = false;
                    let mut near_player = false;
                    for (_sid, p_map, px, py, _dir) in player_positions.iter().copied() {
                        if p_map != map_index {
                            continue;
                        }

                        has_player_on_map = true;
                        let dx = px - monster.x;
                        let dy = py - monster.y;
                        if dx.abs() <= ALONE_RANGE && dy.abs() <= ALONE_RANGE {
                            near_player = true;
                            break;
                        }
                    }

                    if !has_player_on_map {
                        monster.alone = true;
                    } else {
                        monster.alone = !near_player;
                    }
                }

                // In C#: ProcessAI only runs ProcessSearch/ProcessRoam/ProcessTarget
                // when !Alone or Settings.MonsterProcessWhenAlone. We currently
                // treat MonsterProcessWhenAlone as false, so skip AI when alone
                // to match default behaviour.
                if monster.alone {
                    continue;
                }

                // Mirror C# MonsterObject.HallucinationTime behaviour: when a
                // monster is under the Hallucination effect, it temporarily
                // stops acquiring or chasing targets. We approximate this by
                // clearing its current target and skipping the rest of the AI
                // processing for this tick while Envir.Time < HallucinationTime.
                if monster.hallucination_time_ms > 0
                    && now_ms < monster.hallucination_time_ms
                {
                    monster.ai_state = MonsterAiState::Idle;
                    monster.target_session_id = None;
                    continue;
                }

                // -----------------------------
                // ProcessSearch: periodically (SearchDelay) try to acquire or
                // refresh a target, mirroring C# MonsterObject.ProcessSearch.
                // -----------------------------
                if now_ms >= monster.search_time_ms {
                    monster.search_time_ms = now_ms.saturating_add(3_000);

                    if !matches!(ai, 1 | 2 | 3) {
                        let should_search = monster.target_session_id.is_none()
                            || rng.gen_range(0..3) == 0;

                        if should_search {
                            let mut best: Option<(i32, u32)> = None;
                            for (sid, p_map, px, py, _dir) in player_positions.iter().copied() {
                                if p_map != map_index {
                                    continue;
                                }

                                let dx = px - monster.x;
                                let dy = py - monster.y;
                                if dx.abs() > view_range || dy.abs() > view_range {
                                    continue;
                                }

                                let dist = dx.abs().max(dy.abs());
                                match best {
                                    None => best = Some((dist, sid)),
                                    Some((best_dist, _)) if dist < best_dist => {
                                        best = Some((dist, sid));
                                    }
                                    _ => {}
                                }
                            }

                            if let Some((_dist, sid)) = best {
                                monster.ai_state = MonsterAiState::Chase;
                                monster.target_session_id = Some(sid);
                            } else {
                                monster.ai_state = MonsterAiState::Idle;
                                monster.target_session_id = None;
                            }
                        }
                    }
                }

                // -----------------------------
                // ProcessRoam: when no target, occasionally perform a random
                // turn or step, mirroring C# MonsterObject.ProcessRoam.
                // -----------------------------
                if monster.target_session_id.is_none() {
                    if now_ms >= monster.roam_time_ms {
                        monster.roam_time_ms = now_ms.saturating_add(1_000);

                        // Only roam 1/10 of the time when the timer fires.
                        if rng.gen_range(0..10) == 0 {
                            let choice = rng.gen_range(0..3);
                            if choice == 0 {
                                // Turn to a random direction without moving.
                                monster.direction = rng.gen_range(0..8);
                            } else {
                                // Walk one step in the current direction, but
                                // only if the destination tile is walkable on
                                // the current map, mirroring player movement.
                                let (step_x, step_y) = match monster.direction {
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

                                if step_x != 0 || step_y != 0 {
                                    let new_x = monster.x.saturating_add(step_x);
                                    let new_y = monster.y.saturating_add(step_y);

                                    // Reject moves that go outside the map
                                    // bounds. Monster positions are expected
                                    // to remain within 0..width/height.
                                    if new_x < 0 || new_y < 0 {
                                        continue;
                                    }

                                    if monster.x < 0 || monster.y < 0 {
                                        continue;
                                    }

                                    let from_x = monster.x as u16;
                                    let from_y = monster.y as u16;
                                    let to_x = new_x as u16;
                                    let to_y = new_y as u16;

                                    if to_x >= map.width || to_y >= map.height {
                                        continue;
                                    }

                                    if !map.can_move(from_x, from_y, to_x, to_y) {
                                        continue;
                                    }

                                    if self.is_cell_blocked(map_index, new_x, new_y) {
                                        continue;
                                    }

                                    self.remove_monster_from_occupancy(
                                        monster.id,
                                        map_index,
                                        monster.x,
                                        monster.y,
                                    );
                                    self.add_monster_to_occupancy(
                                        monster.id,
                                        map_index,
                                        new_x,
                                        new_y,
                                    );

                                    monster.x = new_x;
                                    monster.y = new_y;

                                    let delay_ms =
                                        Self::compute_monster_move_delay_ms(move_speed);
                                    monster.next_move_time_ms =
                                        now_ms.saturating_add(delay_ms);

                                    events.push(WorldEvent::ObjectLocation {
                                        object_id: monster.id,
                                        map_index,
                                        x: monster.x,
                                        y: monster.y,
                                        direction: monster.direction,
                                    });
                                }
                            }
                        }
                    }

                    // No target to chase this tick.
                    continue;
                }

                // -----------------------------
                // ProcessTarget: chase and melee attack logic when a target is
                // present, mirroring C# MonsterObject.ProcessTarget and
                // InAttackRange/MoveTo behaviour.
                // -----------------------------
                let target_sid = match monster.target_session_id {
                    Some(sid) => sid,
                    None => continue,
                };

                // Find the latest position of the current target.
                let mut target_pos: Option<(i32, i32)> = None;
                for (sid, p_map, px, py, _dir) in player_positions.iter().copied() {
                    if sid == target_sid && p_map == map_index {
                        target_pos = Some((px, py));
                        break;
                    }
                }

                let (tx, ty) = match target_pos {
                    Some(pos) => pos,
                    None => {
                        monster.ai_state = MonsterAiState::Idle;
                        monster.target_session_id = None;
                        continue;
                    }
                };

                let dx_full = tx - monster.x;
                let dy_full = ty - monster.y;
                let dist_full = dx_full.abs().max(dy_full.abs());

                // If in melee range (adjacent tile, not same cell), schedule an
                // attack instead of moving. This mirrors C#
                // MonsterObject.InAttackRange which checks
                //   Target.CurrentLocation != CurrentLocation &&
                //   Functions.InRange(CurrentLocation, Target.CurrentLocation, 1)
                // where InRange uses a Chebyshev distance. Attacks are gated by
                // a separate attack cooldown derived from MonsterInfo.AttackSpeed.
                if dist_full == 1 {
                    if now_ms >= monster.next_attack_time_ms {
                        let sx = dx_full.clamp(-1, 1);
                        let sy = dy_full.clamp(-1, 1);
                        let dir = match (sx, sy) {
                            (0, -1) => 0,
                            (1, -1) => 1,
                            (1, 0) => 2,
                            (1, 1) => 3,
                            (0, 1) => 4,
                            (-1, 1) => 5,
                            (-1, 0) => 6,
                            (-1, -1) => 7,
                            _ => monster.direction,
                        };
                        monster.direction = dir;

                        events.push(WorldEvent::ObjectAttack {
                            session_id: monster.id as u32,
                            map_index,
                            x: monster.x,
                            y: monster.y,
                            direction: dir,
                            spell: 0,
                            level: 0,
                            attack_type: 0,
                        });

                        pending_attacks.push((
                            monster.id,
                            map_index,
                            target_sid,
                            monster.monster_index,
                        ));

                        let delay_ms = Self::compute_monster_attack_delay_ms(attack_speed);
                        monster.next_attack_time_ms = now_ms.saturating_add(delay_ms);
                    }

                    // When already in attack range we do not move this tick.
                    continue;
                }

                // Step at most one tile towards the target (8-directional),
                // respecting the per-monster movement cooldown.
                if monster.next_move_time_ms != 0 && now_ms < monster.next_move_time_ms {
                    continue;
                }

                // Compute the primary direction towards the target using the
                // same 8-way scheme as the C# server, then attempt to walk in
                // that direction. If blocked (by map tiles or other
                // blocking objects), try up to 7 alternative directions by
                // rotating clockwise or counter-clockwise, mirroring
                // MonsterObject.MoveTo/Walk behaviour.

                let base_step_x = dx_full.clamp(-1, 1);
                let base_step_y = dy_full.clamp(-1, 1);

                // If we cannot determine a primary step, skip movement.
                if base_step_x == 0 && base_step_y == 0 {
                    continue;
                }

                let base_dir: u8 = match (base_step_x, base_step_y) {
                    (0, -1) => 0,
                    (1, -1) => 1,
                    (1, 0) => 2,
                    (1, 1) => 3,
                    (0, 1) => 4,
                    (-1, 1) => 5,
                    (-1, 0) => 6,
                    (-1, -1) => 7,
                    _ => monster.direction,
                };

                let mut candidate_dirs: [u8; 8] = [0; 8];
                candidate_dirs[0] = base_dir;

                // Randomise whether we explore clockwise or counter-clockwise
                // first, as in the C# MoveTo implementation.
                let mut dir = base_dir;
                if rng.gen_range(0..2) == 0 {
                    for i in 1..8 {
                        dir = (dir + 1) & 7;
                        candidate_dirs[i] = dir;
                    }
                } else {
                    for i in 1..8 {
                        dir = dir.wrapping_sub(1) & 7;
                        candidate_dirs[i] = dir;
                    }
                }

                let mut moved = false;

                for &dir in &candidate_dirs {
                    let (step_x, step_y) = match dir {
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

                    if step_x == 0 && step_y == 0 {
                        continue;
                    }

                    let new_x = monster.x.saturating_add(step_x);
                    let new_y = monster.y.saturating_add(step_y);
                    if new_x < 0 || new_y < 0 {
                        continue;
                    }

                    if monster.x < 0 || monster.y < 0 {
                        continue;
                    }

                    let from_x = monster.x as u16;
                    let from_y = monster.y as u16;
                    let to_x = new_x as u16;
                    let to_y = new_y as u16;

                    if to_x >= map.width || to_y >= map.height {
                        continue;
                    }

                    if !map.can_move(from_x, from_y, to_x, to_y) {
                        continue;
                    }

                    if self.is_cell_blocked(map_index, new_x, new_y) {
                        continue;
                    }

                    self.remove_monster_from_occupancy(
                        monster.id,
                        map_index,
                        monster.x,
                        monster.y,
                    );
                    self.add_monster_to_occupancy(
                        monster.id,
                        map_index,
                        new_x,
                        new_y,
                    );

                    monster.x = new_x;
                    monster.y = new_y;
                    monster.direction = dir;

                    let delay_ms = Self::compute_monster_move_delay_ms(move_speed);
                    monster.next_move_time_ms = now_ms.saturating_add(delay_ms);

                    events.push(WorldEvent::ObjectLocation {
                        object_id: monster.id,
                        map_index,
                        x: monster.x,
                        y: monster.y,
                        direction: monster.direction,
                    });

                    moved = true;
                    break;
                }

                if !moved {
                    // All directions blocked this tick; monster stays put.
                }
            }
        }

        // Restore the monsters map to the world state.
        self.monsters = monsters_map;

        // Resolve pending monster attacks against players after we finish
        // iterating over the monsters map to avoid borrow conflicts.
        for (monster_id, map_index, target_sid, monster_index) in pending_attacks {
            let Some(player) = self.players.get_mut(&target_sid) else {
                continue;
            };

            if player.dead || player.hp <= 0 {
                continue;
            }

            let defender_stats: Stats = player.stats.total.clone();
            let max_hp = defender_stats.get(Stat::HP).max(1);
            if max_hp <= 0 {
                continue;
            }

            let Some(info) = self.provider.get_monster_info(monster_index) else {
                continue;
            };

            // Start from the static MonsterInfo stats and layer on any
            // per-instance buff_stats that are currently active on this
            // monster instance, mirroring C# MonsterObject.RefreshBuffs where
            // Buff.Stats are added on top of base stats.
            let mut attacker_stats: Stats = info.stats.clone();
            if let Some(monsters) = self.monsters.get(&map_index) {
                if let Some(m) = monsters.iter().find(|m| m.id == monster_id) {
                    attacker_stats.add(&m.buff_stats);
                }
            }
            let (hit, raw_damage, raw_damage_type) =
                compute_physical_melee_with_crit(&attacker_stats, &defender_stats);

            // Translate the helper's result into a concrete outcome.
            //
            // - When `hit` is false (Accuracy/Agility check failed) we emit a
            //   Miss (damage_type = 1, damage = 0) without changing the
            //   player's HP, mirroring C#'s BroadcastDamageIndicator(Miss).
            // - When `hit` is true but raw_damage <= 0 (e.g. very high AC/DR)
            //   we treat it as a fully absorbed hit and skip emitting any
            //   event.
            let (damage, damage_type, new_hp, dead) = if !hit {
                let old_hp = player.hp.max(0);
                (0, 1_u8, old_hp, false)
            } else {
                if raw_damage <= 0 {
                    continue;
                }

                let damage = raw_damage;
                let old_hp = player.hp.max(0);
                let new_hp = old_hp.saturating_sub(damage).max(0);
                if new_hp == old_hp {
                    continue;
                }

                player.hp = new_hp;
                let mut dead = false;
                if new_hp <= 0 {
                    player.dead = true;
                    dead = true;
                }

                (damage, raw_damage_type, new_hp, dead)
            };

            let health_percent = ((new_hp as i64 * 100) / max_hp as i64)
                .clamp(0, 100) as u8;

            events.push(WorldEvent::MonsterHitPlayer {
                attacker_monster_id: monster_id,
                session_id: target_sid,
                map_index,
                x: player.x,
                y: player.y,
                direction: player.direction,
                damage,
                damage_type,
                health_percent,
            });

            if dead {
                // 玩家被怪物击杀时，先尝试使用复活戒指复活，如果成功则不
                // 进入死亡掉落流程。复活失败时，按与 PVP 相同的规则处理：
                // 先清理该玩家的所有宠物/召唤物，再清理带 RemoveOnDeath
                // 属性的 Buff，并根据 PlayerObject.Die -> DeathDrop /
                // RedDeathDrop 规则计算装备/背包掉落。
                if !self.try_revive_with_revival_ring(target_sid, events) {
                    self.remove_all_pets_for_session(target_sid);
                    self.clear_player_buffs_on_death(target_sid, events);
                    self.apply_player_death_drops(target_sid, map_index, events);
                }
            }
        }

        // Resolve pending pet attacks against monsters after the main AI loop,
        // mirroring the core player->monster combat flow but attributing
        // experience and drops to the pet's owner.
        for (pet_id, map_index, target_id, target_monster_index, owner_sid, pet_monster_index) in
            pending_pet_attacks
        {
            // Fetch and validate the owner first so that we can later award
            // experience and use their drop modifiers.
            let owner_stats = match self.players.get(&owner_sid) {
                Some(p) if !p.dead && p.hp > 0 && p.map_index == map_index => {
                    p.stats.total.clone()
                }
                _ => continue,
            };

            // Attacker (pet) stats: MonsterInfo base stats plus any
            // per-instance buff_stats currently active on the pet.
            let mut attacker_stats: Stats = match self.provider.get_monster_info(pet_monster_index)
            {
                Some(info) => info.stats.clone(),
                None => continue,
            };
            if let Some(monsters) = self.monsters.get(&map_index) {
                if let Some(m) = monsters.iter().find(|m| m.id == pet_id) {
                    attacker_stats.add(&m.buff_stats);
                }
            }

            // Defender (target monster) stats: MonsterInfo base plus
            // per-instance buff_stats.
            let (monster_exp, _undead, max_hp, defender_stats, monster_drops): (
                u32,
                bool,
                i32,
                Stats,
                Vec<crate::world::drop::DropInfo>,
            ) = if let Some(info) = self.provider.get_monster_info(target_monster_index) {
                let max_hp = info.stats.get(Stat::HP).max(1);

                let mut defender_stats = info.stats.clone();
                if let Some(monsters) = self.monsters.get(&map_index) {
                    if let Some(m) = monsters.iter().find(|m| m.id == target_id) {
                        defender_stats.add(&m.buff_stats);
                    }
                }

                (
                    info.experience,
                    info.undead,
                    max_hp,
                    defender_stats,
                    info.drops.clone(),
                )
            } else {
                (0, false, 1, Stats::default(), Vec::new())
            };

            if max_hp <= 0 {
                continue;
            }

            let (hit, raw_damage, damage_type) =
                compute_physical_melee_with_crit(&attacker_stats, &defender_stats);

            if !hit || raw_damage <= 0 {
                // For now we do not emit miss / zero-damage indicators for
                // pet attacks to keep the model simple.
                continue;
            }

            let strike_x: i32;
            let strike_y: i32;
            let strike_dir: u8;
            let mut damage_done: i32 = 0;
            let health_percent: u8;
            let mut dead = false;

            if let Some(monsters) = self.monsters.get_mut(&map_index) {
                if let Some(m) = monsters.iter_mut().find(|m| m.id == target_id) {
                    strike_x = m.x;
                    strike_y = m.y;
                    strike_dir = m.direction;

                    let old_hp = m.hp.max(0);
                    let mut new_hp = old_hp;

                    if raw_damage > 0 {
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
                } else {
                    // Target no longer present on this map.
                    continue;
                }
            } else {
                continue;
            }

            if damage_done <= 0 {
                continue;
            }

            // Attribute the visual hit and HP-bar update to the pet's owner
            // rather than the pet's own monster id so that the connection
            // layer (which keys off attacker_id == session_id) will emit
            // SObjectStruck/SDamageIndicator/SObjectHealth exactly as if the
            // player had performed the attack directly. This mirrors the C#
            // behaviour where pet damage is effectively shown as coming from
            // the owning player for client-side UI purposes.
            events.push(WorldEvent::ObjectStruck {
                attacker_id: owner_sid,
                target_id,
                map_index,
                x: strike_x,
                y: strike_y,
                direction: strike_dir,
                damage: damage_done,
                damage_type,
                health_percent,
            });

            if dead {
                // Remove the monster from the world state and update respawn
                // counts before handling drops and experience.
                self.mark_monster_dead(map_index, target_id);

                if !monster_drops.is_empty() {
                    let item_offset = owner_stats.get(Stat::ItemDropRatePercent);
                    let gold_offset = owner_stats.get(Stat::GoldDropRatePercent);
                    let mut rng = thread_rng();
                    let mut total = crate::world::drop::DropRewardInfo {
                        items: Vec::new(),
                        gold: 0,
                    };

                    for d in &monster_drops {
                        // Mirror C# MonsterObject.Drop quest-required behaviour:
                        // skip quest-only drops when placing map items.
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

                    // Place gold and item drops on the ground near the
                    // monster's last position, mirroring the player->monster
                    // flow in World::handle_attack_command.
                    if total.gold > 0 || !total.items.is_empty() {
                        let item_timeout_ms: i64 = 300_000; // 5 minutes

                        if total.gold > 0 {
                            if let Some((drop_x, drop_y)) =
                                self.find_drop_location(map_index, strike_x, strike_y, 4)
                            {
                                let item_id = self.next_map_item_id;
                                self.next_map_item_id =
                                    self.next_map_item_id.wrapping_add(1);
                                let entry =
                                    self.map_items.entry(map_index).or_default();
                                entry.push(crate::world::map_item::MapItem {
                                    id: item_id,
                                    map_index,
                                    x: drop_x,
                                    y: drop_y,
                                    item_index: None,
                                    gold: total.gold,
                                    count: 0,
                                    item: None,
                                    expire_time_ms: self.time_ms + item_timeout_ms,
                                });

                                events.push(WorldEvent::GoldDropped {
                                    object_id: item_id,
                                    map_index,
                                    x: drop_x,
                                    y: drop_y,
                                    gold: total.gold,
                                });
                            }
                        }

                        for item_index in total.items {
                            if let Some((drop_x, drop_y)) =
                                self.find_drop_location(map_index, strike_x, strike_y, 4)
                            {
                                let item_id = self.next_map_item_id;
                                self.next_map_item_id =
                                    self.next_map_item_id.wrapping_add(1);
                                let entry =
                                    self.map_items.entry(map_index).or_default();
                                entry.push(crate::world::map_item::MapItem {
                                    id: item_id,
                                    map_index,
                                    x: drop_x,
                                    y: drop_y,
                                    item_index: Some(item_index),
                                    gold: 0,
                                    count: 1,
                                    item: None,
                                    expire_time_ms: self.time_ms + item_timeout_ms,
                                });

                                events.push(WorldEvent::ItemDropped {
                                    object_id: item_id,
                                    map_index,
                                    x: drop_x,
                                    y: drop_y,
                                    item_index,
                                    count: 1,
                                });

                                if let Some(info) = self.provider.get_item_info(item_index) {
                                    if info.global_drop_notify {
                                        let base_name = info.friendly_name();
                                        let monster_name = self
                                            .provider
                                            .get_monster_info(target_monster_index)
                                            .map(|mi| mi.name.clone())
                                            .unwrap_or_else(|| "Monster".to_string());
                                        let text = format!(
                                            "{} has dropped {}.",
                                            monster_name,
                                            base_name
                                        );
                                        for (&sid, _) in self.players.iter() {
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
                }

                if monster_exp > 0 {
                    // Attribute experience for pet kills via the same helper
                    // so levels and rankings remain consistent.
                    let _ = self.gain_experience_for_session(owner_sid, monster_exp, events);

                    events.push(WorldEvent::GainExperience {
                        session_id: owner_sid,
                        amount: monster_exp,
                    });
                }

                events.push(WorldEvent::MonsterDied {
                    object_id: target_id,
                    map_index,
                    x: strike_x,
                    y: strike_y,
                    direction: strike_dir,
                });
            }
        }
    }

    fn process_guard_ai(&mut self, now_ms: i64, events: &mut Vec<WorldEvent>) {
        const GUARD_AIS: [u8; 8] = [6, 57, 58, 102, 103, 104, 105, 113];
        let map_indices: Vec<i32> = self.monsters.keys().cloned().collect();
        let mut pending_kills: Vec<(i32, u64, i32, i32, u8)> = Vec::new();
        let player_positions: Vec<(u32, i32, i32, i32, u8, i32)> = self
            .players
            .iter()
            .filter(|(_, p)| !p.dead && p.hp > 0)
            .map(|(&sid, p)| (sid, p.map_index, p.x, p.y, p.direction, p.pk_points))
            .collect();
        let mut pending_guard_hits: Vec<(u64, i32, u32, i32)> = Vec::new();
        let mut pending_guard_moves: Vec<(u64, i32, i32, i32, i32, i32)> = Vec::new();

        for map_index in map_indices {
            // Load the map so that guard movement (both chase and patrol)
            // can respect bounds and walkability, mirroring the behaviour
            // used by the generic monster AI.
            let map = match self.get_or_load_map(map_index) {
                Some(m) => m,
                None => continue,
            };

            let Some(monsters) = self.monsters.get_mut(&map_index) else {
                continue;
            };

            let len = monsters.len();
            for i in 0..len {
                let (
                    attack_range,
                    attack_delay_ms,
                    guard_x,
                    guard_y,
                    guard_ai,
                    guard_monster_index,
                    guard_move_speed,
                ) = {
                    let m = &monsters[i];

                    if m.hp <= 0 {
                        continue;
                    }

                    let Some(info) = self.provider.get_monster_info(m.monster_index) else {
                        continue;
                    };

                    if !GUARD_AIS.contains(&info.ai) {
                        continue;
                    }

                    let range: i32 = match info.ai {
                        // IceGuard uses a fixed AttackRange of 8 in C#.
                        102 => 8,
                        // KingGuard uses a fixed AttackRange of 10.
                        105 => 10,
                        _ => info.view_range as i32,
                    }
                    .max(1);

                    let delay_ms = Self::compute_monster_attack_delay_ms(info.attack_speed);

                    (range, delay_ms, m.x, m.y, info.ai, m.monster_index, info.move_speed)
                };

                if now_ms < monsters[i].next_attack_time_ms {
                    continue;
                }

                // Select the best monster target within range (for instant kills).
                let mut best_monster: Option<(usize, i32)> = None;

                for j in 0..len {
                    if j == i {
                        continue;
                    }

                    let t = &monsters[j];
                    if t.hp <= 0 {
                        continue;
                    }

                    let Some(t_info) = self.provider.get_monster_info(t.monster_index) else {
                        continue;
                    };

                    if GUARD_AIS.contains(&t_info.ai) {
                        continue;
                    }

                    // Mirror C# MonsterObject.IsAttackTarget(MonsterObject
                    // attacker) rules for guards and TaoGuards when deciding
                    // which monsters may be attacked:
                    //
                    // - Deer/Hen/Tree types (AI 1/2/3) are never targeted.
                    // - Guard (AI 6/113) only attack wild monsters or pets
                    //   whose master is red (PKPoints >= 200).
                    // - TaoGuard (AI 58) attack wild monsters and pets whose
                    //   master is not in Peace attack mode.
                    if matches!(t_info.ai, 1 | 2 | 3) {
                        continue;
                    }

                    if matches!(guard_ai, 6 | 113) {
                        let mut allowed = false;
                        if let Some(owner_sid) = t.owner_session_id {
                            if let Some(owner) = self.players.get(&owner_sid) {
                                if owner.pk_points >= 200 {
                                    allowed = true;
                                }
                            }
                        } else {
                            // Wild monster (no master) is always valid.
                            allowed = true;
                        }

                        if !allowed {
                            continue;
                        }
                    } else if guard_ai == 58 {
                        let mut allowed = false;
                        if let Some(owner_sid) = t.owner_session_id {
                            if let Some(owner) = self.players.get(&owner_sid) {
                                let mode = AttackMode::from_u8(owner.attack_mode);
                                if mode != AttackMode::Peace {
                                    allowed = true;
                                }
                            }
                        } else {
                            // Wild monster is always valid for TaoGuard.
                            allowed = true;
                        }

                        if !allowed {
                            continue;
                        }
                    }

                    let dx = t.x - guard_x;
                    let dy = t.y - guard_y;
                    let dist = dx.abs().max(dy.abs());
                    if dist == 0 || dist > attack_range {
                        continue;
                    }

                    match best_monster {
                        None => best_monster = Some((j, dist)),
                        Some((_, best_dist)) if dist < best_dist => {
                            best_monster = Some((j, dist));
                        }
                        _ => {}
                    }
                }

                // Optionally select the best red-name player target for PK guards.
                let mut best_player: Option<(u32, i32, i32, u8, i32)> = None;
                let can_attack_players = matches!(guard_ai, 6 | 58 | 113);

                if can_attack_players {
                    for (sid, p_map, px, py, p_dir, pk_points) in
                        player_positions.iter().copied()
                    {
                        if p_map != map_index {
                            continue;
                        }

                        if pk_points < 200 {
                            continue;
                        }

                        let dx = px - guard_x;
                        let dy = py - guard_y;
                        let dist = dx.abs().max(dy.abs());
                        if dist == 0 || dist > attack_range {
                            continue;
                        }

                        match best_player {
                            None => best_player = Some((sid, px, py, p_dir, dist)),
                            Some((_, _, _, _, best_dist)) if dist < best_dist => {
                                best_player = Some((sid, px, py, p_dir, dist));
                            }
                            _ => {}
                        }
                    }
                }

                // Choose between a monster or player target based on distance
                // (prefer the closer one; on ties, prefer players to punish PK).
                let mut target_is_player = false;
                let mut target_id: u64 = 0;
                let mut target_sid: u32 = 0;
                let target_x: i32;
                let target_y: i32;
                let target_dir: u8;

                match (best_player, best_monster) {
                    (Some((sid, px, py, p_dir, p_dist)), Some((m_idx, m_dist))) => {
                        if p_dist <= m_dist {
                            target_is_player = true;
                            target_sid = sid;
                            target_x = px;
                            target_y = py;
                            target_dir = p_dir;
                        } else {
                            let t = &monsters[m_idx];
                            target_id = t.id;
                            target_x = t.x;
                            target_y = t.y;
                            target_dir = t.direction;
                        }
                    }
                    (Some((sid, px, py, p_dir, _)), None) => {
                        target_is_player = true;
                        target_sid = sid;
                        target_x = px;
                        target_y = py;
                        target_dir = p_dir;
                    }
                    (None, Some((m_idx, _))) => {
                        let t = &monsters[m_idx];
                        target_id = t.id;
                        target_x = t.x;
                        target_y = t.y;
                        target_dir = t.direction;
                    }
                    (None, None) => {
                        // No valid target this tick; attempt patrol for
                        // stationary guard types that are configured with
                        // a route (Guard/TownArcher/TaoGuard).
                        if matches!(guard_ai, 6 | 57 | 58) {
                            if let Some(respawns) = self.respawns.get(&map_index) {
                                let guard_respawn_index = monsters[i].respawn_index;
                                if let Some(rt) = respawns
                                    .iter()
                                    .find(|rt| rt.info.respawn_index == guard_respawn_index)
                                {
                                    if let Some(route_points) = &rt.route_points {
                                        if !route_points.is_empty() {
                                            let guard = &mut monsters[i];

                                            // Respect per-waypoint delay.
                                            if guard.route_wait_until_ms > 0
                                                && now_ms < guard.route_wait_until_ms
                                            {
                                                // Still waiting at current waypoint.
                                            } else {
                                                let len = route_points.len() as i32;
                                                if guard.route_index < 0
                                                    || guard.route_index >= len
                                                {
                                                    guard.route_index = 0;
                                                }

                                                let idx = guard.route_index as usize;
                                                let point = &route_points[idx];

                                                if guard.x == point.x && guard.y == point.y {
                                                    // Arrived: apply delay and advance to next
                                                    // waypoint.
                                                    if point.delay_ms > 0 {
                                                        guard.route_wait_until_ms = now_ms
                                                            .saturating_add(point.delay_ms);
                                                    } else {
                                                        guard.route_wait_until_ms = 0;
                                                    }

                                                    let mut next = guard.route_index + 1;
                                                    if next >= len {
                                                        next = 0;
                                                    }
                                                    guard.route_index = next;
                                                } else {
                                                    // Step one tile towards the current waypoint,
                                                    // gated by the monster's movement cooldown.
                                                    if guard.next_move_time_ms == 0
                                                        || now_ms >= guard.next_move_time_ms
                                                    {
                                                        let dx = point.x - guard.x;
                                                        let dy = point.y - guard.y;
                                                        let step_x = dx.clamp(-1, 1);
                                                        let step_y = dy.clamp(-1, 1);

                                                        if step_x != 0 || step_y != 0 {
                                                            let new_x = guard.x.saturating_add(step_x);
                                                            let new_y = guard.y.saturating_add(step_y);

                                                            // Reject patrol moves that go
                                                            // outside map bounds or into
                                                            // blocked cells, mirroring the
                                                            // generic monster movement checks.
                                                            if new_x < 0 || new_y < 0 {
                                                                continue;
                                                            }
                                                            if guard.x < 0 || guard.y < 0 {
                                                                continue;
                                                            }

                                                            let from_x = guard.x as u16;
                                                            let from_y = guard.y as u16;
                                                            let to_x = new_x as u16;
                                                            let to_y = new_y as u16;

                                                            if to_x >= map.width || to_y >= map.height {
                                                                continue;
                                                            }

                                                            if !map.can_move(from_x, from_y, to_x, to_y)
                                                            {
                                                                continue;
                                                            }

                                                            let old_x = guard.x;
                                                            let old_y = guard.y;
                                                            guard.x = new_x;
                                                            guard.y = new_y;

                                                            pending_guard_moves.push((
                                                                guard.id,
                                                                map_index,
                                                                old_x,
                                                                old_y,
                                                                new_x,
                                                                new_y,
                                                            ));

                                                            let delay = Self::
                                                                compute_monster_move_delay_ms(
                                                                    guard_move_speed,
                                                                );
                                                            guard.next_move_time_ms = now_ms
                                                                .saturating_add(delay);

                                                            events.push(
                                                                WorldEvent::ObjectLocation {
                                                                    object_id: guard.id,
                                                                    map_index,
                                                                    x: guard.x,
                                                                    y: guard.y,
                                                                    direction: guard.direction,
                                                                },
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        continue;
                    }
                }

                // For chase-type guards (IceGuard, ElementGuard, DemonGuard,
                // KingGuard), move one tile towards the target when it is
                // currently outside their attack range, mirroring the C#
                // MoveTo(Target.CurrentLocation) behaviour in
                // IceGuard/ElementGuard/KingGuard.ProcessTarget and the base
                // MonsterObject logic used by DemonGuard.
                if matches!(guard_ai, 102 | 103 | 104 | 105) {
                    let dist = (target_x - guard_x).abs().max((target_y - guard_y).abs());
                    if dist > attack_range {
                        let guard = &mut monsters[i];

                        if guard.next_move_time_ms == 0 || now_ms >= guard.next_move_time_ms {
                            let dx = target_x - guard.x;
                            let dy = target_y - guard.y;
                            let step_x = dx.clamp(-1, 1);
                            let step_y = dy.clamp(-1, 1);

                            if step_x != 0 || step_y != 0 {
                                let new_x = guard.x.saturating_add(step_x);
                                let new_y = guard.y.saturating_add(step_y);

                                // Respect map bounds and walkability when
                                // chasing, just like generic monsters do.
                                if new_x < 0 || new_y < 0 {
                                    continue;
                                }
                                if guard.x < 0 || guard.y < 0 {
                                    continue;
                                }

                                let from_x = guard.x as u16;
                                let from_y = guard.y as u16;
                                let to_x = new_x as u16;
                                let to_y = new_y as u16;

                                if to_x >= map.width || to_y >= map.height {
                                    continue;
                                }

                                if !map.can_move(from_x, from_y, to_x, to_y)
                                {
                                    continue;
                                }

                                let old_x = guard.x;
                                let old_y = guard.y;
                                guard.x = new_x;
                                guard.y = new_y;

                                pending_guard_moves.push((
                                    guard.id,
                                    map_index,
                                    old_x,
                                    old_y,
                                    new_x,
                                    new_y,
                                ));

                                let delay =
                                    Self::compute_monster_move_delay_ms(guard_move_speed);
                                guard.next_move_time_ms =
                                    now_ms.saturating_add(delay);

                                events.push(WorldEvent::ObjectLocation {
                                    object_id: guard.id,
                                    map_index,
                                    x: guard.x,
                                    y: guard.y,
                                    direction: guard.direction,
                                });
                            }
                        }

                        // Skip attacking this tick; the guard has spent its
                        // action moving towards the target instead.
                        continue;
                    }
                }

                let guard_id;
                let guard_x;
                let guard_y;
                let guard_dir;
                {
                    let guard = &mut monsters[i];
                    guard_id = guard.id;
                    guard_x = guard.x;
                    guard_y = guard.y;

                    let dx = target_x - guard.x;
                    let dy = target_y - guard.y;
                    let sx = dx.clamp(-1, 1);
                    let sy = dy.clamp(-1, 1);
                    guard.direction = match (sx, sy) {
                        (0, -1) => 0,
                        (1, -1) => 1,
                        (1, 0) => 2,
                        (1, 1) => 3,
                        (0, 1) => 4,
                        (-1, 1) => 5,
                        (-1, 0) => 6,
                        (-1, -1) => 7,
                        _ => guard.direction,
                    };
                    guard.next_attack_time_ms = now_ms.saturating_add(attack_delay_ms);
                    guard_dir = guard.direction;
                }

                // Emit the guard's attack using its own position and facing,
                // mirroring C# where ObjectAttack carries the attacker's
                // location and direction rather than the strike tile. The
                // client derives hit cells from this data.
                events.push(WorldEvent::ObjectAttack {
                    session_id: guard_id as u32,
                    map_index,
                    x: guard_x,
                    y: guard_y,
                    direction: guard_dir,
                    spell: 0,
                    level: 0,
                    attack_type: 0,
                });

                if target_is_player {
                    pending_guard_hits.push((
                        guard_id,
                        map_index,
                        target_sid,
                        guard_monster_index,
                    ));
                } else {
                    pending_kills.push((map_index, target_id, target_x, target_y, target_dir));
                }
            }
        }
        // Apply deferred occupancy updates for guards that moved along their
        // patrol routes. These must be done outside the main loop to avoid
        // borrowing self.monsters mutably at the same time as calling
        // occupancy helpers that also take &mut self.
        for (guard_id, map_index, old_x, old_y, new_x, new_y) in pending_guard_moves {
            self.remove_monster_from_occupancy(guard_id, map_index, old_x, old_y);
            self.add_monster_to_occupancy(guard_id, map_index, new_x, new_y);
        }
        for (map_index, target_id, x, y, direction) in pending_kills {
            // For guard-instakilled monsters, mark them as dead and clear
            // their AI/target state, but keep the MonsterInstance in the
            // monsters list so the client can continue to render a corpse
            // after receiving SObjectDied. We also remove dynamic occupancy so
            // that the corpse no longer blocks movement.
            if let Some(monsters) = self.monsters.get_mut(&map_index) {
                if let Some(m) = monsters.iter_mut().find(|m| m.id == target_id) {
                    if m.hp > 0 {
                        m.hp = 0;
                    }
                    m.ai_state = MonsterAiState::Idle;
                    m.target_session_id = None;
                }
            }

            // Use the strike coordinates captured in pending_kills for
            // occupancy removal to avoid needing a second mutable borrow of
            // self.monsters here.
            self.remove_monster_from_occupancy(target_id, map_index, x, y);

            events.push(WorldEvent::MonsterDied {
                object_id: target_id,
                map_index,
                x,
                y,
                direction,
            });
        }

        // Resolve pending guard hits against players using the same
        // MonsterHitPlayer model as generic monster AI.
        for (monster_id, map_index, target_sid, monster_index) in pending_guard_hits {
            let Some(player) = self.players.get_mut(&target_sid) else {
                continue;
            };

            if player.dead || player.hp <= 0 {
                continue;
            }

            let defender_stats: Stats = player.stats.total.clone();
            let max_hp = defender_stats.get(Stat::HP).max(1);
            if max_hp <= 0 {
                continue;
            }

            let Some(info) = self.provider.get_monster_info(monster_index) else {
                continue;
            };

            let mut attacker_stats: Stats = info.stats.clone();
            if let Some(monsters) = self.monsters.get(&map_index) {
                if let Some(m) = monsters.iter().find(|m| m.id == monster_id) {
                    attacker_stats.add(&m.buff_stats);
                }
            }

            let (hit, raw_damage, raw_damage_type) =
                compute_physical_melee_with_crit(&attacker_stats, &defender_stats);

            let (damage, damage_type, new_hp, dead) = if !hit {
                let old_hp = player.hp.max(0);
                (0, 1_u8, old_hp, false)
            } else {
                if raw_damage <= 0 {
                    continue;
                }

                let damage = raw_damage;
                let old_hp = player.hp.max(0);
                let new_hp = old_hp.saturating_sub(damage).max(0);
                if new_hp == old_hp {
                    continue;
                }

                player.hp = new_hp;
                let mut dead = false;
                if new_hp <= 0 {
                    player.dead = true;
                    dead = true;
                }

                (damage, raw_damage_type, new_hp, dead)
            };

            let health_percent = ((new_hp as i64 * 100) / max_hp as i64)
                .clamp(0, 100) as u8;

            events.push(WorldEvent::MonsterHitPlayer {
                attacker_monster_id: monster_id,
                session_id: target_sid,
                map_index,
                x: player.x,
                y: player.y,
                direction: player.direction,
                damage,
                damage_type,
                health_percent,
            });

            if dead {
                // 该分支处理另一类怪物对玩家的物理攻击（例如部分范围技能），
                // 在玩家死亡时同样需要先尝试复活戒指，再触发宠物清理、
                // 死亡 Buff 清理和掉落逻辑。
                if !self.try_revive_with_revival_ring(target_sid, events) {
                    self.remove_all_pets_for_session(target_sid);
                    self.clear_player_buffs_on_death(target_sid, events);
                    self.apply_player_death_drops(target_sid, map_index, events);
                }
            }
        }
    }

    fn update_respawn_tick_counter(&mut self, now_ms: i64) {
        let base_minutes = i64::from(self.respawn_base_spawn_rate_minutes.max(1));
        let tick_ms = base_minutes.saturating_mul(60_000);
        if tick_ms <= 0 {
            return;
        }

        if self.respawn_last_tick_ms == 0 {
            self.respawn_last_tick_ms = now_ms;
            return;
        }

        if now_ms <= self.respawn_last_tick_ms {
            return;
        }

        let elapsed = now_ms.saturating_sub(self.respawn_last_tick_ms);
        if elapsed < tick_ms {
            return;
        }

        let ticks = (elapsed / tick_ms) as u64;
        if ticks == 0 {
            return;
        }

        self.respawn_tick_counter = self.respawn_tick_counter.wrapping_add(ticks);
        let advance = (ticks as i64).saturating_mul(tick_ms);
        self.respawn_last_tick_ms = self
            .respawn_last_tick_ms
            .saturating_add(advance);
    }

    fn compute_respawn_delay_ms(info: &RespawnInfo) -> i64 {
        let base = info.delay as i64;
        let random = info.random_delay as i64;

        let minutes = if random > 0 {
            let mut rng = thread_rng();
            let span = random.saturating_mul(2);
            let delta = rng.gen_range(0..span);
            let total = base.saturating_sub(random).saturating_add(delta);
            if total <= 0 {
                1_i64
            } else {
                total
            }
        } else if base <= 0 {
            1_i64
        } else {
            base
        };

        minutes.saturating_mul(60_000)
    }

    pub(crate) fn compute_monster_move_delay_ms(move_speed: u16) -> i64 {
        let speed = i64::from(move_speed.max(400));
        if speed <= 0 {
            400
        } else {
            speed
        }
    }

    pub(crate) fn compute_monster_attack_delay_ms(attack_speed: u16) -> i64 {
        // AttackSpeed in C# is in milliseconds and clamped to a minimum of
        // 400. We mirror that behaviour here.
        let speed = i64::from(attack_speed.max(400));
        if speed <= 0 {
            400
        } else {
            speed
        }
    }

    fn create_monsters_from_respawn(
        &mut self,
        map: &map::Map,
        respawn: &RespawnInfo,
    ) -> Vec<MonsterInstance> {
        let mut result = Vec::new();

        // Build candidate walkable cells within the configured spread.
        let spread = respawn.spread as i32;
        let mut candidates: Vec<(i32, i32)> = Vec::new();

        for &(wx, wy) in &map.walkable_cells {
            let x = wx as i32;
            let y = wy as i32;
            if x <= respawn.location_x + spread
                && x >= respawn.location_x - spread
                && y <= respawn.location_y + spread
                && y >= respawn.location_y - spread
            {
                candidates.push((x, y));
            }
        }

        // No walkable candidates in range: mirror C# by skipping this respawn.
        if candidates.is_empty() {
            debug!(
                "respawn: no walkable cells for map={} respawn_index={} loc=({}, {}) spread={}",
                map.info.index,
                respawn.respawn_index,
                respawn.location_x,
                respawn.location_y,
                respawn.spread,
            );
            return result;
        }

        let needed = respawn.count as usize;
        let len = candidates.len();
        if len == 0 || needed == 0 {
            return result;
        }

        // Determine base HP for this monster type from MonsterInfo stats.
        let base_hp: i32 = self
            .provider
            .get_monster_info(respawn.monster_index)
            .map(|info| info.stats.get(Stat::HP))
            .unwrap_or(1)
            .max(1);

        let mut rng = thread_rng();

        // For each monster we want to spawn, try a bounded number of times to
        // find a candidate cell that is not currently occupied by any player or
        // monster.
        for _ in 0..needed {
            let mut placed = false;

            for _attempt in 0..8 {
                let idx = rng.gen_range(0..len);
                let (x, y) = candidates[idx];

                if self.is_cell_blocked(map.info.index, x, y) {
                    continue;
                }

                self.next_monster_id = self.next_monster_id.wrapping_add(1);

                result.push(MonsterInstance {
                    id: self.next_monster_id,
                    monster_index: respawn.monster_index,
                    map_index: map.info.index,
                    x,
                    y,
                    home_x: x,
                    home_y: y,
                    direction: 0,
                    hp: base_hp,
                    is_pet: false,
                    owner_session_id: None,
                    pet_kind: None,
                    respawn_index: respawn.respawn_index,
                    ai_state: MonsterAiState::Idle,
                    target_session_id: None,
                    next_move_time_ms: 0,
                    next_attack_time_ms: 0,
                    search_time_ms: 0,
                    roam_time_ms: 0,
                    route_index: -1,
                    route_wait_until_ms: 0,
                    alone: false,
                    alone_time_ms: 0,
                    shock_time_ms: 0,
                    rage_time_ms: 0,
                    hallucination_time_ms: 0,
                    buff_stats: Stats::default(),
                    buffs: Vec::new(),
                    special_mode: false,
                    special_mode_until_ms: 0,
                    special_mode_action_time_ms: 0,
                    dead: false,
                    dead_until_ms: 0,
                    poisons: Vec::new(),
                    current_poison_mask: 0,
                });

                placed = true;
                break;
            }

            if !placed {
                trace!(
                    "respawn: all candidate cells blocked for map={} respawn_index={} loc=({}, {}) spread={}",
                    map.info.index,
                    respawn.respawn_index,
                    respawn.location_x,
                    respawn.location_y,
                    respawn.spread,
                );
            }
        }

        result
    }

    /// Kill the monster closest to the given world position on the specified
    /// map. Returns the monster id and its experience value if a target was
    /// found and removed.
    pub fn kill_nearest_monster(&mut self, map_index: i32, x: i32, y: i32) -> Option<(u64, u32)> {
        let target = match self.monsters.get(&map_index) {
            Some(monsters) => {
                let mut best: Option<(i64, u64, i32)> = None;
                for m in monsters {
                    let dx = m.x - x;
                    let dy = m.y - y;
                    let dist = (dx.abs() + dy.abs()) as i64;
                    match best {
                        None => best = Some((dist, m.id, m.monster_index)),
                        Some((best_dist, _, _)) if dist < best_dist => {
                            best = Some((dist, m.id, m.monster_index));
                        }
                        _ => {}
                    }
                }
                best.map(|(_, id, monster_index)| (id, monster_index))
            }
            None => None,
        };

        if let Some((id, monster_index)) = target {
            self.mark_monster_dead(map_index, id);

            let exp = self
                .provider
                .get_monster_info(monster_index)
                .map(|info| info.experience)
                .unwrap_or(0);

            Some((id, exp))
        } else {
            None
        }
    }

    /// Record that a monster has died. Instead of immediately removing it
    /// from the monsters list, flag it as dead and set a corpse timeout so
    /// that the client can render the corpse for a short period, mirroring C#
    /// MonsterObject.Dead / DeadTime behaviour.
    pub fn mark_monster_dead(&mut self, map_index: i32, monster_id: u64) {
        // First, locate the monster instance and mark it as dead while we
        // hold a mutable borrow to the monsters list. Extract the position
        // and respawn_index we need for subsequent occupancy / respawn
        // updates, then drop the borrow before mutating other fields on self.
        let mut corpse_meta: Option<(i32, i32, i32)> = None;

        if let Some(monsters) = self.monsters.get_mut(&map_index) {
            if let Some(m) = monsters.iter_mut().find(|m| m.id == monster_id) {
                // Mark as dead and set a corpse timeout similar to C#
                // MonsterObject.DeadTime = Envir.Time + DeadDelay. We
                // approximate DeadDelay with a fixed 10 seconds window so
                // that corpses remain visible for a short time before being
                // removed from the world.
                m.hp = 0;
                m.dead = true;
                let base_time = self.time_ms.max(0);
                let corpse_lifetime_ms: i64 = 10_000;
                m.dead_until_ms = base_time.saturating_add(corpse_lifetime_ms);

                corpse_meta = Some((m.x, m.y, m.respawn_index));
            }
        }

        if let Some((x, y, respawn_index)) = corpse_meta {
            // Clear dynamic occupancy so that the cell is no longer blocking
            // for players or other monsters, but keep the monster instance in
            // the list so that visibility can still see it as a corpse.
            self.remove_monster_from_occupancy(monster_id, map_index, x, y);

            // Decrement respawn runtime counts immediately so that spawn logic
            // can consider this monster slot free, matching the C# behaviour
            // where the respawn system reacts to death rather than corpse
            // removal time.
            if let Some(runtimes) = self.respawns.get_mut(&map_index) {
                if let Some(rt) = runtimes
                    .iter_mut()
                    .find(|rt| rt.info.respawn_index == respawn_index)
                {
                    if rt.current_count > 0 {
                        rt.current_count -= 1;
                    }
                }
            }
        }
    }

    /// Remove monsters whose corpse timeout has expired. This is called from
    /// World::update after combat/AI so that dead monsters linger as corpses
    /// for a short time but are eventually cleaned up and no longer sent to
    /// clients via visibility updates.
    fn cleanup_dead_monsters(&mut self, now_ms: i64) {
        let map_indices: Vec<i32> = self.monsters.keys().cloned().collect();

        for map_index in map_indices {
            if let Some(monsters) = self.monsters.get_mut(&map_index) {
                let mut to_remove: Vec<usize> = Vec::new();

                for (idx, m) in monsters.iter().enumerate() {
                    if m.dead && m.dead_until_ms > 0 && now_ms >= m.dead_until_ms {
                        to_remove.push(idx);
                    }
                }

                // Remove in reverse order to keep indices stable.
                for idx in to_remove.into_iter().rev() {
                    monsters.remove(idx);
                }
            }
        }
    }

    /// Process map items: remove expired items and emit MapItemRemoved events.
    /// This mirrors C# ItemObject.Process() behavior where items are removed
    /// when Envir.Time > ExpireTime.
    fn process_map_items(&mut self, now_ms: i64, events: &mut Vec<WorldEvent>) {
        let map_indices: Vec<i32> = self.map_items.keys().cloned().collect();

        for map_index in map_indices {
            if let Some(items) = self.map_items.get_mut(&map_index) {
                let mut to_remove: Vec<usize> = Vec::new();

                for (idx, item) in items.iter().enumerate() {
                    if now_ms > item.expire_time_ms {
                        to_remove.push(idx);
                    }
                }

                // Remove items in reverse order to maintain indices
                for &idx in to_remove.iter().rev() {
                    let removed = items.remove(idx);
                    events.push(WorldEvent::MapItemRemoved {
                        object_id: removed.id,
                        map_index: removed.map_index,
                        x: removed.x,
                        y: removed.y,
                    });
                }
            }
        }
    }
}


