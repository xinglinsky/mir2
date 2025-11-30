use rand::thread_rng;

use crate::combat::compute_physical_melee_with_crit;
use crate::world::magic::magic_power;
use crate::stats::{Stat, Stats};
use crate::world::monster::MonsterAiState;
use crate::world::player::PlayerState;
use crate::world::provider::WorldProvider;
use crate::world::skills::{
    apply_attack_spell_scaling,
    apply_fatal_sword_and_undead,
    compute_magic_mana_cost,
    compute_pure_magic_attack_damage,
    fatal_sword_level_for_player,
    is_pure_magic_attack,
    resolve_attack_spell_and_level_for_player,
};
use crate::world::skills::assassin::{apply_moon_dark_bonus, resolve_moon_dark_opening};
use crate::world::skills::warrior::{
    cast_counter_attack,
    cast_cross_half_moon,
    cast_flaming_sword,
    cast_half_moon,
    cast_immortal_skin,
    cast_rage,
    is_thrusting_spell,
    resolve_slaying_for_attack,
    thrusting_max_range,
};
use crate::world::skills::wizard::{
    cast_fire_bang_ice_storm,
    cast_fire_wall,
    cast_magic_shield,
    cast_thunder_storm_flame_field,
};
use crate::world::skills::taoist::{
    cast_blessed_armour,
    cast_ultimate_enhancer,
    cast_energy_shield,
    cast_healing,
    cast_mass_healing,
    cast_soul_shield,
    cast_summon_holy_deva,
    cast_summon_shinsu,
    cast_summon_skeleton,
};
use crate::world::types::{AttackMode, BuffType};
use crate::world::{PendingMagicHit, SessionId, World, WorldEvent, Spell};

impl<P: WorldProvider> World<P> {
    fn can_attack(_player: &PlayerState) -> bool {
        true
    }

    /// Determine whether `attacker_sid` is allowed to attack `target_sid`
    /// according to the C# PlayerObject.IsAttackTarget(HumanObject attacker)
    /// rules. This inspects map NoFight, SafeZone membership and the
    /// attacker's AttackMode, together with party and guild membership and
    /// the target's PK status (red/brown).
    fn can_attack_player(&self, attacker_sid: SessionId, target_sid: SessionId) -> bool {
        if attacker_sid == target_sid {
            return false;
        }

        let attacker = match self.players.get(&attacker_sid) {
            Some(p) => p,
            None => return false,
        };
        let target = match self.players.get(&target_sid) {
            Some(p) => p,
            None => return false,
        };

        if attacker.dead || target.dead || attacker.hp <= 0 || target.hp <= 0 {
            return false;
        }

        if attacker.map_index != target.map_index {
            return false;
        }
        let map_index = attacker.map_index;

        let map_info = match self.provider.get_map_info(map_index) {
            Some(info) => info,
            None => return false,
        };

        // Mirror C# CurrentMap.Info.NoFight and InSafeZone checks.
        if map_info.no_fight {
            return false;
        }

        let attacker_in_safe = Self::point_in_safe_zone(map_info, attacker.x, attacker.y);
        let target_in_safe = Self::point_in_safe_zone(map_info, target.x, target.y);
        if attacker_in_safe || target_in_safe {
            return false;
        }

        let mode = AttackMode::from_u8(attacker.attack_mode);

        match mode {
            AttackMode::Peace => false,
            AttackMode::All => true,
            AttackMode::Group => {
                // C#: Group => GroupMembers == null || !GroupMembers.Contains(attacker)
                // Here: only disallow when both are in the same party.
                if let (Some(a_pid), Some(t_pid)) = (attacker.party_id, target.party_id) {
                    a_pid != t_pid
                } else {
                    true
                }
            }
            AttackMode::Guild => {
                // C#: Guild => MyGuild == null || MyGuild != attacker.MyGuild
                // i.e. cannot attack same-guild members when both have guilds.
                if target.guild_name.is_empty() || attacker.guild_name.is_empty() {
                    true
                } else {
                    attacker.guild_name != target.guild_name
                }
            }
            AttackMode::EnemyGuild => {
                // C#: EnemyGuild => MyGuild != null && MyGuild.IsEnemy(attacker.MyGuild)
                // Until full guild war state is implemented, approximate this
                // as: both have non-empty, different guild names.
                if target.guild_name.is_empty() || attacker.guild_name.is_empty() {
                    false
                } else {
                    attacker.guild_name != target.guild_name
                }
            }
            AttackMode::RedBrown => {
                // C#: RedBrown => PKPoints >= 200 || Envir.Time < BrownTime
                target.pk_points >= 200 || self.time_ms < target.brown_time_ms
            }
        }
    }

    /// Determine whether `attacker_sid` is allowed to attack the specified
    /// monster according to the C# MonsterObject.IsAttackTarget(HumanObject
    /// attacker) rules. This primarily affects pets (monsters with a player
    /// owner) and mirrors the interaction with AttackMode/party/guild/PK
    /// state, while leaving wild monsters unrestricted by AttackMode.
    fn can_attack_monster(&self, attacker_sid: SessionId, map_index: i32, monster_id: u64) -> bool {
        let attacker = match self.players.get(&attacker_sid) {
            Some(p) => p,
            None => return false,
        };

        let monsters = match self.monsters.get(&map_index) {
            Some(ms) => ms,
            None => return false,
        };

        let monster = match monsters.iter().find(|m| m.id == monster_id && m.hp > 0) {
            Some(m) => m,
            None => return false,
        };

        // Wild monsters (no owner) are always valid targets regardless of
        // AttackMode; map-level NoFight/SafeZone rules are handled elsewhere.
        let owner_sid = match monster.owner_session_id {
            Some(sid) => sid,
            None => return true,
        };

        let owner = match self.players.get(&owner_sid) {
            Some(p) => p,
            // If the owner is missing from world state, fall back to treating
            // this as a wild monster.
            None => return true,
        };

        let mode = AttackMode::from_u8(attacker.attack_mode);

        // C#: if (attacker.AMode == AttackMode.Peace) return false; (for pets)
        if mode == AttackMode::Peace {
            return false;
        }

        // C#: if (Master == attacker) return attacker.AMode == AttackMode.All;
        if owner_sid == attacker_sid {
            return mode == AttackMode::All;
        }

        // Approximate the C# safe-zone rule for pets:
        // if (Master.Race == ObjectType.Player && (attacker.InSafeZone || InSafeZone)) return false;
        if let Some(map_info) = self.provider.get_map_info(map_index) {
            let attacker_in_safe =
                Self::point_in_safe_zone(map_info, attacker.x, attacker.y);
            let monster_in_safe =
                Self::point_in_safe_zone(map_info, monster.x, monster.y);
            if attacker_in_safe || monster_in_safe {
                return false;
            }
        }

        match mode {
            AttackMode::All => true,
            AttackMode::Group => {
                // C#: Group => Master.GroupMembers == null || !Master.GroupMembers.Contains(attacker)
                // Approximate via shared PartyId: disallow when both are in
                // the same party.
                if let (Some(a_pid), Some(o_pid)) = (attacker.party_id, owner.party_id) {
                    a_pid != o_pid
                } else {
                    true
                }
            }
            AttackMode::Guild => {
                // C#: Guild => master.MyGuild == null || master.MyGuild != attacker.MyGuild
                if owner.guild_name.is_empty() || attacker.guild_name.is_empty() {
                    true
                } else {
                    owner.guild_name != attacker.guild_name
                }
            }
            AttackMode::EnemyGuild => {
                // C#: EnemyGuild => master.MyGuild != null && attacker.MyGuild != null && master.MyGuild.IsEnemy(attacker.MyGuild)
                // Approximate as: both have non-empty, different guild names.
                if owner.guild_name.is_empty() || attacker.guild_name.is_empty() {
                    false
                } else {
                    owner.guild_name != attacker.guild_name
                }
            }
            AttackMode::RedBrown => {
                // C#: RedBrown => Master.PKPoints >= 200 || Envir.Time < Master.BrownTime
                owner.pk_points >= 200 || self.time_ms < owner.brown_time_ms
            }
            AttackMode::Peace => false, // already handled above
        }
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
        target_id: u32,
        x: i32,
        y: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let (player_map, player_x, player_y, has_magic) = match self.players.get(&session_id) {
            Some(p) => {
                let has_magic = p.magics.iter().any(|m| m.spell == spell);
                (p.map_index, p.x, p.y, has_magic)
            }
            None => return,
        };

        // If the player knows this magic, enforce a simple cooldown based on
        // the MagicInfo delay parameters and the per-magic UserMagic.cast_time
        // field. This mirrors the C# behaviour where repeated CMagic packets
        // while a spell is still on cooldown are ignored server-side.
        //
        // Only after passing the cooldown check do we emit MagicCast so the
        // client can update its per-spell CastTime and log the accepted cast.
        if has_magic {
            if !self.check_and_update_magic_cooldown(session_id, spell) {
                tracing::debug!(
                    "handle_magic_command: session_id={} spell_id={} ignored (on cooldown)",
                    session_id,
                    spell,
                );
                return;
            }

            if let Some(p) = self.players.get(&session_id) {
                tracing::debug!(
                    "handle_magic_command: session_id={} job={:?} spell_id={} accepted",
                    session_id,
                    p.job,
                    spell,
                );
            }

            events.push(WorldEvent::MagicCast {
                session_id,
                spell_id: spell,
            });
        }

        if spell == Spell::FlamingSword as u8 {
            cast_flaming_sword(self, session_id, events);
            return;
        }

        if spell == Spell::Rage as u8 {
            cast_rage(self, session_id, events);
            return;
        }

        if spell == Spell::ImmortalSkin as u8 {
            cast_immortal_skin(self, session_id, events);
            return;
        }

        if spell == Spell::CounterAttack as u8 {
            cast_counter_attack(self, session_id, events);
            return;
        }

        if spell == Spell::FireBang as u8 || spell == Spell::IceStorm as u8 {
            cast_fire_bang_ice_storm(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::FireWall as u8 {
            cast_fire_wall(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::ThunderStorm as u8 || spell == Spell::FlameField as u8 {
            cast_thunder_storm_flame_field(self, session_id, spell, direction, events);
            return;
        }

        if is_pure_magic_attack(spell) {
            // Route wizard single-target attack spells (FireBall, GreatFireBall,
            // ThunderBolt, SoulFireBall, etc.) through the unified attack
            // pipeline so that MP cost, damage, skill training and visual
            // effects are handled consistently. For these spells we honour the
            // client-provided target first, falling back to directional scan
            // when necessary.
            self.handle_attack_command(
                session_id,
                direction,
                spell,
                target_id,
                x,
                y,
                events,
            );
            return;
        }

        if spell == Spell::MagicShield as u8 {
            cast_magic_shield(self, session_id, events);
            return;
        }

        if spell == Spell::SoulShield as u8 {
            cast_soul_shield(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::BlessedArmour as u8 {
            cast_blessed_armour(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::EnergyShield as u8 {
            cast_energy_shield(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::UltimateEnhancer as u8 {
            cast_ultimate_enhancer(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::Healing as u8 {
            cast_healing(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::MassHealing as u8 {
            cast_mass_healing(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::SummonSkeleton as u8 {
            cast_summon_skeleton(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::SummonShinsu as u8 {
            cast_summon_shinsu(self, session_id, spell, direction, x, y, events);
            return;
        }

        if spell == Spell::SummonHolyDeva as u8 {
            cast_summon_holy_deva(self, session_id, spell, direction, x, y, events);
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

    

    pub(super) fn handle_attack_command(
        &mut self,
        session_id: SessionId,
        direction: u8,
        spell: u8,
        packet_target_id: u32,
        packet_target_x: i32,
        packet_target_y: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let (
            map_index,
            x,
            y,
            direction,
            effective_spell,
            level,
            fatal_level,
            attacker_stats,
            flaming_sword_trigger,
            slaying_toggled_on,
            moon_dark_spell,
            moon_dark_level,
        ) =
            match self.players.get_mut(&session_id) {
                Some(p) => {
                    if !Self::can_attack(p) {
                        return;
                    }

                    p.direction = direction;

                    // Check for FlamingSword buff logic BEFORE resolving spell.
                    let mut spell_to_use = spell;
                    let mut consumed_flaming_sword = false;
                    let mut slaying_toggled_on = false;

                    // Mirror the C# HumanObject.Attack behaviour for
                    // MoonLight/DarkBody via the assassin-specific helper: if
                    // the player is emerging from a hidden state with one of
                    // these buffs active, the very next physical attack
                    // receives a DamageBase bonus equal to magic.GetPower()
                    // for the corresponding spell. We approximate Hidden by
                    // the presence of the buff itself and remove the buffs
                    // immediately so the bonus is one-shot.
                    let (moon_dark_spell, moon_dark_level) =
                        resolve_moon_dark_opening(p);

                    // If it's a basic attack (spell 0) and we have FlamingSword buff
                    // active, treat this swing as FlamingSword and consume the buff.
                    if spell == 0 {
                        if let Some(idx) = p
                            .active_buffs
                            .iter()
                            .position(|b| b.buff_type == BuffType::FlamingSword)
                        {
                            p.active_buffs.remove(idx);
                            spell_to_use = Spell::FlamingSword as u8;
                            consumed_flaming_sword = true;
                        }
                    }

                    // Resolve the effective attack spell and its learned level.
                    let (mut effective_spell, mut level) =
                        resolve_attack_spell_and_level_for_player(p, spell_to_use);

                    // Mirror the C# HumanObject.Attack MP gating for certain
                    // warrior attack skills that are driven from the melee
                    // attack loop rather than the magic command path.
                    if level > 0 {
                        if let Some(spell_enum) = Spell::from_u8(effective_spell) {
                            use Spell as S;
                            match spell_enum {
                                // These skills consume MP per attack when used
                                // as part of the melee flow. If there is not
                                // enough MP, the swing still happens but
                                // falls back to a plain physical attack.
                                S::DoubleSlash | S::HalfMoon | S::CrossHalfMoon | S::TwinDrakeBlade => {
                                    if let Some(cost) = compute_magic_mana_cost(
                                        &self.provider,
                                        &p.stats.total,
                                        effective_spell,
                                        level,
                                    ) {
                                        if p.mp < cost {
                                            // Not enough MP: downgrade to
                                            // plain melee for this swing.
                                            effective_spell = 0;
                                            level = 0;
                                        } else {
                                            p.mp -= cost;
                                        }
                                    } else {
                                        // No MagicInfo entry; treat as plain
                                        // melee to avoid inconsistent state.
                                        effective_spell = 0;
                                        level = 0;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    // Handle Slaying activation/consumption and random charge
                    // via the warrior-specific helper, mirroring the C#
                    // HumanObject.Attack flow.
                    resolve_slaying_for_attack(
                        p,
                        &mut effective_spell,
                        &mut level,
                        &mut slaying_toggled_on,
                    );

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
                        consumed_flaming_sword,
                        slaying_toggled_on,
                        moon_dark_spell,
                        moon_dark_level,
                    )
                }
                None => {
                    return;
                }
            };

        if flaming_sword_trigger {
             events.push(WorldEvent::SpellToggle {
                session_id,
                spell_id: Spell::FlamingSword as u8,
                enabled: false,
            });
        }

        if slaying_toggled_on {
            events.push(WorldEvent::SpellToggle {
                session_id,
                spell_id: Spell::Slaying as u8,
                enabled: true,
            });
        }

        let is_half_moon = effective_spell == Spell::HalfMoon as u8;
        let is_cross_half_moon = effective_spell == Spell::CrossHalfMoon as u8;

        if is_half_moon {
            // Train HalfMoon when the attack is actually executed.
            self.level_up_magic_for_player(session_id, Spell::HalfMoon as u8, events);

            cast_half_moon(
                self,
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
            // Train CrossHalfMoon when the attack is actually executed.
            self.level_up_magic_for_player(session_id, Spell::CrossHalfMoon as u8, events);

            cast_cross_half_moon(
                self,
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

        // First try to hit a player standing on the front tile before
        // looking for monsters. This keeps PVP path separate from the
        // monster-targeting logic below and mirrors C# HumanObject.Attack
        // semantics, where player targets are resolved first.
        if !is_pure_magic_attack(effective_spell) {
            let mut player_target: Option<SessionId> = None;
            for (&sid, p) in &self.players {
                if sid == session_id {
                    continue;
                }
                if p.map_index != map_index || p.x != target_x || p.y != target_y {
                    continue;
                }
                if self.can_attack_player(session_id, sid) {
                    player_target = Some(sid);
                    break;
                }
            }

            if let Some(target_session_id) = player_target {
                // Snapshot defender stats for damage calculation.
                let (defender_stats, max_hp) = {
                    if let Some(t) = self.players.get(&target_session_id) {
                        let max_hp = t.stats.total.get(Stat::HP).max(1);
                        (t.stats.total.clone(), max_hp)
                    } else {
                        return;
                    }
                };

                let (hit, mut raw_damage, damage_type) =
                    compute_physical_melee_with_crit(&attacker_stats, &defender_stats);

                if hit && raw_damage > 0 {
                    // Apply MoonLight/DarkBody opening bonus if present.
                    if moon_dark_spell != 0 && moon_dark_level > 0 {
                        if let Some(info) = self.provider.get_magic_info(moon_dark_spell) {
                            let mut rng = thread_rng();
                            let bonus = magic_power(info, moon_dark_level, &mut rng);
                            if bonus > 0 {
                                raw_damage = raw_damage.saturating_add(bonus);
                            }
                        }
                    }

                    // Apply active attack spell scaling except for the
                    // Thrusting extended tile case (not used for adjacent
                    // player hits here).
                    if !is_thrusting_spell(effective_spell) {
                        raw_damage = apply_attack_spell_scaling(
                            &self.provider,
                            effective_spell,
                            level,
                            raw_damage,
                        );
                    }
                }

                // Apply passive FatalSword and undead tweaks; players are not
                // undead so we always pass false for undead.
                raw_damage = apply_fatal_sword_and_undead(
                    &self.provider,
                    fatal_level,
                    false,
                    raw_damage,
                );

                // Approximate multi-hit skills by doubling final damage.
                if effective_spell == Spell::DoubleSlash as u8
                    || effective_spell == Spell::TwinDrakeBlade as u8
                {
                    raw_damage = raw_damage.saturating_mul(2);
                }

                let mut strike_x = target_x;
                let mut strike_y = target_y;
                let mut strike_dir = direction;
                let mut damage_done: i32 = 0;
                let mut health_percent: u8 = 100;
                let mut dead = false;
                let mut old_pk_points = 0;
                let mut old_brown_time = 0;

                if let Some(t) = self.players.get(&target_session_id) {
                    old_pk_points = t.pk_points;
                    old_brown_time = t.brown_time_ms;
                }

                if let Some(t) = self.players.get_mut(&target_session_id) {
                    strike_x = t.x;
                    strike_y = t.y;
                    strike_dir = t.direction;

                    let old_hp = t.hp.max(0);
                    let mut new_hp = old_hp;

                    if hit && raw_damage > 0 {
                        damage_done = raw_damage;

                        if raw_damage >= t.hp {
                            t.hp = 0;
                            dead = true;
                        } else {
                            t.hp -= raw_damage;
                        }

                        new_hp = t.hp.max(0);
                    }

                    if max_hp > 0 {
                        let pct = (new_hp as i64 * 100 / max_hp as i64)
                            .clamp(0, 100) as u8;
                        health_percent = pct;
                    } else {
                        health_percent = 0;
                    }
                }

                if hit {
                    if damage_done > 0 {
                        events.push(WorldEvent::ObjectStruck {
                            attacker_id: session_id,
                            target_id: target_session_id as u64,
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
                        target_id: target_session_id as u64,
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
                    // Mirror the core PK rules: when a player kills another
                    // outside Fight maps, award PKPoints if the victim is not
                    // already red/brown, and always refresh the victim's
                    // BrownTime.
                    if let Some(t) = self.players.get_mut(&target_session_id) {
                        t.brown_time_ms = self.time_ms;
                    }

                    if let Some(map_info) = self.provider.get_map_info(map_index) {
                        if !map_info.fight {
                            let eligible = old_pk_points < 200 && self.time_ms > old_brown_time;
                            if eligible {
                                if let Some(att) = self.players.get_mut(&session_id) {
                                    att.pk_points = att.pk_points.saturating_add(100);
                                }
                            }
                        }
                    }
                }

                // Emit the usual attack animation and position events for the
                // attacker, then stop; monster handling below is skipped.
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
        }

        let target_info = match self.monsters.get(&map_index) {
            Some(monsters) => {
                if is_pure_magic_attack(effective_spell) {
                    // For pure magic attack spells (e.g. FireBall, SoulFireBall,
                    // ThunderBolt), first try to honour the explicit
                    // client-provided target (ID / location) and only fall
                    // back to a directional scan when no suitable monster is
                    // found. This mirrors the C# HumanObject.Fireball /
                    // ThunderBolt behaviour where Magic.Target is preferred
                    // over nearest-in-front.
                    let max_range: i32 = self
                        .provider
                        .get_magic_info(effective_spell)
                        .map(|info| info.range as i32)
                        .unwrap_or_else(|| {
                            // When there is no MagicInfo entry (e.g. an
                            // incomplete MagicInfoList in the DB), fall back
                            // to a reasonable default range so that pure
                            // magic spells like SoulFireBall can still reach
                            // distant monsters and produce ObjectMagic/
                            // damage events instead of silently failing.
                            if effective_spell == Spell::SoulFireBall as u8 {
                                9
                            } else {
                                6
                            }
                        })
                        .max(1);

                    let mut found: Option<(u64, i32, i32, i32)> = None;

                    // 1) Try explicit monster ID from the packet when present.
                    if packet_target_id != 0 {
                        if let Some(m) = monsters
                            .iter()
                            .find(|m| {
                                m.hp > 0
                                    && m.id == packet_target_id as u64
                                    && self.can_attack_monster(session_id, map_index, m.id)
                            })
                        {
                            let dx_t = m.x - x;
                            let dy_t = m.y - y;
                            if dx_t.abs().max(dy_t.abs()) <= max_range {
                                found = Some((m.id, m.monster_index, m.x, m.y));
                            }
                        }
                    }

                    // 2) If no ID match, try the explicit location from the
                    // packet to pick a monster standing exactly on that tile.
                    if found.is_none() && (packet_target_x != 0 || packet_target_y != 0) {
                        if let Some(m) = monsters
                            .iter()
                            .find(|m| {
                                m.hp > 0
                                    && m.x == packet_target_x
                                    && m.y == packet_target_y
                                    && self.can_attack_monster(session_id, map_index, m.id)
                            })
                        {
                            let dx_t = m.x - x;
                            let dy_t = m.y - y;
                            if dx_t.abs().max(dy_t.abs()) <= max_range {
                                found = Some((m.id, m.monster_index, m.x, m.y));
                            }
                        }
                    }

                    // 3) Fallback: scan forward along the attack direction up
                    // to the spell range and select the first monster
                    // encountered, preserving the original behaviour when no
                    // explicit target is usable.
                    if found.is_none() {
                        for step in 1..=max_range {
                            let tx = x + dx * step;
                            let ty = y + dy * step;
                            if let Some(m) = monsters
                                .iter()
                                .find(|m| {
                                    m.hp > 0
                                        && m.x == tx
                                        && m.y == ty
                                        && self.can_attack_monster(session_id, map_index, m.id)
                                })
                            {
                                found = Some((m.id, m.monster_index, tx, ty));
                                break;
                            }
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
                        if let Some(m) = monsters
                            .iter()
                            .find(|m| {
                                m.hp > 0
                                    && m.x == tx
                                    && m.y == ty
                                    && self.can_attack_monster(session_id, map_index, m.id)
                            })
                        {
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
                        .find(|m| {
                            m.hp > 0
                                && m.x == target_x
                                && m.y == target_y
                                && self.can_attack_monster(session_id, map_index, m.id)
                        })
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

            // Train attack-type magic when a valid target is acquired and the
            // effective spell is non-zero (i.e. a learned skill rather than a
            // plain melee swing).
            if effective_spell != 0 {
                self.level_up_magic_for_player(session_id, effective_spell, events);
            }

            let (monster_exp, undead, max_hp, defender_stats, monster_drops): (
                u32,
                bool,
                i32,
                Stats,
                Vec<crate::world::drop::DropInfo>,
            ) = if let Some(info) = self.provider.get_monster_info(monster_index) {
                let max_hp = info.stats.get(Stat::HP).max(1);

                // Base defender stats from MonsterInfo plus any active
                // per-instance buff_stats on this specific monster instance,
                // mirroring C# MonsterObject.RefreshBuffs where Buff.Stats are
                // added on top of base stats.
                let mut defender_stats = info.stats.clone();
                if let Some(monsters) = self.monsters.get(&map_index) {
                    if let Some(m) = monsters.iter().find(|m| m.id == id) {
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

            // Use the unified C#-style physical melee model (including
            // Accuracy/Agility, AC/DR and crit) for player -> monster hits for
            // most attacks. Pure magic spells such as FireBall and
            // SoulFireBall use their MagicInfo parameters directly instead of
            // relying on the melee helper for base damage.
            let use_pure_magic = is_pure_magic_attack(effective_spell);

            // For pure magic attacks, also emit an ObjectMagic-style world
            // event so that the connection layer can send SObjectMagic and
            // drive projectile animations and spell visuals on the client.
            if use_pure_magic {
                events.push(WorldEvent::ObjectMagic {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                    spell: effective_spell,
                    level,
                    target_id: id as u32,
                    target_x,
                    target_y,
                });

                // And notify the caster about the cast using a Magic event,
                // mirroring C# S.Magic. The legacy client uses this to set
                // User.Spell/User.Cast/TargetID/TargetPoint and then, when the
                // MirAction.Spell animation completes, spawn the correct
                // projectile and hit effects (e.g. FireBall explosion,
                // ThunderBolt lightning on the target).
                events.push(WorldEvent::Magic {
                    session_id,
                    spell_id: effective_spell,
                    target_id: id as u32,
                    x: target_x,
                    y: target_y,
                    cast: true,
                    level,
                    secondary_target_ids: Vec::new(),
                });
            }

            if use_pure_magic {
                // Apply MP cost for pure magic attack spells (FireBall,
                // SoulFireBall). If there is no MagicInfo entry, treat the
                // cost as zero rather than aborting the spell so that the
                // cast still animates and can deal damage using the fallback
                // path in compute_pure_magic_attack_damage.
                let cost = compute_magic_mana_cost(
                    &self.provider,
                    &attacker_stats,
                    effective_spell,
                    level,
                )
                .unwrap_or(0);

                if let Some(player) = self.players.get_mut(&session_id) {
                    if player.mp < cost {
                        return;
                    }
                    if cost > 0 {
                        player.mp -= cost;
                    }
                } else {
                    return;
                }
            }

            let (hit, mut raw_damage, damage_type) = if use_pure_magic {
                let dmg = compute_pure_magic_attack_damage(
                    &self.provider,
                    &attacker_stats,
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
                if use_pure_magic {
                    // ThunderBolt deals 1.5x damage to undead targets, mirroring
                    // the C# HumanObject.ThunderBolt implementation.
                    if effective_spell == Spell::ThunderBolt as u8 && undead {
                        let scaled = (raw_damage as f32 * 1.5) as i32;
                        raw_damage = scaled.max(1);
                    }
                } else {
                    // Apply MoonLight / DarkBody opening strike bonus before
                    // any active attack spell scaling.
                    raw_damage = apply_moon_dark_bonus(
                        &self.provider,
                        moon_dark_spell,
                        moon_dark_level,
                        raw_damage,
                    );

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

            if use_pure_magic {
                // For pure magic attacks (FireBall/GreatFireBall/ThunderBolt/SoulFireBall),
                // schedule a delayed hit instead of applying damage
                // immediately so that the damage and visual hit effect
                // (projectile or lightning) stay in sync on the client.
                if hit && raw_damage > 0 {
                    let delay_ms: i64 = if let Some(spell_enum) = Spell::from_u8(effective_spell) {
                        use Spell as S;
                        match spell_enum {
                            // ThunderBolt uses a fixed 500ms delay in C#
                            S::ThunderBolt => 500,
                            // FireBall/GreatFireBall/SoulFireBall use
                            // MaxDistance * 50 + 500ms.
                            S::FireBall | S::GreatFireBall | S::SoulFireBall => {
                                let dx = target_x - x;
                                let dy = target_y - y;
                                let dist = dx.abs().max(dy.abs());
                                (dist as i64) * 50 + 500
                            }
                            _ => 500,
                        }
                    } else {
                        500
                    };

                    let base_time = self.time_ms.max(0);
                    let due_time_ms = base_time.saturating_add(delay_ms);

                    self.pending_magic_hits.push(PendingMagicHit {
                        due_time_ms,
                        attacker_session_id: session_id,
                        map_index,
                        target_monster_id: id,
                        monster_index,
                        spell_id: effective_spell,
                        damage: raw_damage,
                        damage_type,
                    });
                }

                // For pure magic we still emit the usual attacker
                // animation/location events, but skip immediate HP
                // changes and ObjectStruck; those will be handled when
                // the PendingMagicHit fires in World::update.
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

                    tracing::debug!(
                        "[drop-total] monster_index={} gold={} items_len={}",
                        monster_index,
                        total.gold,
                        total.items.len(),
                    );

                    if total.gold > 0 || !total.items.is_empty() {
                        // Set expire time for monster drops: default 5 minutes
                        // (300000 ms) after the current world time, mirroring
                        // the behaviour used for player-dropped items in
                        // WorldCommand::DropItem.
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
                            }
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

