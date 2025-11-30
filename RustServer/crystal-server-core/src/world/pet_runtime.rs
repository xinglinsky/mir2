use crate::world::map;
use crate::world::monster::{MonsterAiState, MonsterInstance};
use crate::world::provider::WorldProvider;
use crate::world::types::PetKind;
use crate::world::configs::pet_template;
use crate::world::{SessionId, World, WorldEvent};
use tracing::debug;

impl<P: WorldProvider> World<P> {
    /// Pet-specific AI: summoned pets stay near their owner and automatically
    /// attack nearby hostile monsters. This function mirrors the inline logic
    /// that previously lived inside `monster_runtime::update` for
    /// `monster.is_pet` and is intended to preserve behaviour exactly while
    /// isolating pet follow/attack code.
    pub(super) fn process_pet_ai_for_monster(
        &mut self,
        now_ms: i64,
        map_index: i32,
        map: &map::Map,
        move_speed: u16,
        attack_speed: u16,
        monster: &mut MonsterInstance,
        hostile_monsters: &[(u64, i32, i32, i32)],
        pending_pet_attacks: &mut Vec<(u64, i32, u64, i32, SessionId, i32)>,
        events: &mut Vec<WorldEvent>,
    ) {
        let owner_sid = match monster.owner_session_id {
            Some(sid) => sid,
            None => {
                monster.ai_state = MonsterAiState::Idle;
                monster.target_session_id = None;
                return;
            }
        };

        let (owner_x, owner_y) = {
            let p = match self.players.get(&owner_sid) {
                Some(p) if !p.dead && p.hp > 0 && p.map_index == map_index => p,
                _ => {
                    monster.ai_state = MonsterAiState::Idle;
                    monster.target_session_id = None;
                    return;
                }
            };
            (p.x, p.y)
        };

        let pet_kind = match monster.pet_kind {
            Some(k) => k,
            None => {
                monster.ai_state = MonsterAiState::Idle;
                monster.target_session_id = None;
                return;
            }
        };

        let template = match pet_template(pet_kind) {
            Some(t) => t,
            None => {
                monster.ai_state = MonsterAiState::Idle;
                monster.target_session_id = None;
                return;
            }
        };

        let mut handled = false;

        // Taoist Shinsu has a custom Mode/attack-range behaviour in C#: when a
        // target exists, it toggles a 30s Mode window during which the pet is
        // visible (ObjectShow) and allowed to attack using a special
        // InAttackRange pattern.
        if pet_kind == PetKind::TaoistShinsu {
            let follow_distance = template.follow_distance.max(1);
            let leash_distance = template.leash_distance.max(follow_distance);

            // debug!(
            //     "pet_ai_shinsu_tick: pet_id={} map={} pos=({}, {}) owner=({}, {}) special_mode={} until_ms={} now_ms={}",
            //     monster.id,
            //     map_index,
            //     monster.x,
            //     monster.y,
            //     owner_x,
            //     owner_y,
            //     monster.special_mode,
            //     monster.special_mode_until_ms,
            //     now_ms,
            // );

            // Separate targets for attack vs. chase. Attack candidates must be
            // inside the Shinsu InAttackRange shape, while chase candidates
            // only need to be within leash distance so Shinsu will move toward
            // nearby threats.
            // InAttackRange pattern from C#:
            //   if (x > 2 || y > 2) return false;
            //   return (x <= 1 && y <= 1) || (x == y || x % 2 == y % 2);
            let mut best_attack: Option<(u64, i32, i32, i32, i32, i32)> = None;
            let mut best_chase: Option<(i32, i32, i32)> = None;

            for &(tid, t_index, tx, ty) in hostile_monsters {
                let dx = tx - monster.x;
                let dy = ty - monster.y;
                let ax = dx.abs();
                let ay = dy.abs();

                if ax == 0 && ay == 0 {
                    continue;
                }

                let dist = ax.max(ay);

                // Attack candidate: inside the Shinsu attack pattern.
                if ax <= 2
                    && ay <= 2
                    && ((ax <= 1 && ay <= 1) || (ax == ay || (ax % 2 == ay % 2)))
                {
                    match best_attack {
                        None => best_attack = Some((tid, t_index, tx, ty, dx, dy)),
                        Some((_, _, _, _, _, _,)) => {
                            // Use distance from center (ax/ay) to prioritise
                            // closer targets.
                            let current_best_dist =
                                best_attack.map(|(_, _, _, _, bdx, bdy)| {
                                    bdx.abs().max(bdy.abs())
                                }).unwrap_or(i32::MAX);
                            if dist < current_best_dist {
                                best_attack = Some((tid, t_index, tx, ty, dx, dy));
                            }
                        }
                    }
                }

                // Chase candidate: any hostile within leash distance of the
                // pet. This approximates C# ProcessSearch/Target behaviour.
                if dist <= leash_distance {
                    match best_chase {
                        None => best_chase = Some((tx, ty, dist)),
                        Some((_, _, best_dist)) if dist < best_dist => {
                            best_chase = Some((tx, ty, dist));
                        }
                        _ => {}
                    }
                }
            }

            let has_target = best_chase.is_some();

            // Update Shinsu Mode timers and emit ObjectShow/ObjectHide,
            // mirroring Shinsu.ProcessAI.
            const MODE_DURATION_MS: i64 = 30_000;
            const MODE_ACTION_INTERVAL_MS: i64 = 1_000;

            if now_ms > monster.special_mode_action_time_ms {
                if has_target {
                    monster.special_mode_until_ms = now_ms.saturating_add(MODE_DURATION_MS);
                }

                if !monster.special_mode && now_ms > 0 && now_ms < monster.special_mode_until_ms {
                    monster.special_mode = true;
                    monster.special_mode_action_time_ms =
                        now_ms.saturating_add(MODE_ACTION_INTERVAL_MS);

                    debug!(
                        "pet_ai_shinsu_mode_on: pet_id={} map={} pos=({}, {}) until_ms={} now_ms={}",
                        monster.id,
                        map_index,
                        monster.x,
                        monster.y,
                        monster.special_mode_until_ms,
                        now_ms,
                    );

                    events.push(WorldEvent::ObjectShow {
                        object_id: monster.id,
                        map_index,
                        x: monster.x,
                        y: monster.y,
                    });
                } else if monster.special_mode
                    && (monster.special_mode_until_ms > 0
                        && now_ms > monster.special_mode_until_ms)
                {
                    // When Mode expires, keep Shinsu visible and only flip the
                    // internal special_mode flag instead of emitting
                    // ObjectHide, to avoid the client playing a full hide
                    // animation that makes the pet appear to disappear.
                    monster.special_mode = false;
                    monster.special_mode_action_time_ms =
                        now_ms.saturating_add(MODE_ACTION_INTERVAL_MS);

                    debug!(
                        "pet_ai_shinsu_mode_off: pet_id={} map={} pos=({}, {}) now_ms={} until_ms={}",
                        monster.id,
                        map_index,
                        monster.x,
                        monster.y,
                        now_ms,
                        monster.special_mode_until_ms,
                    );
                }
            }

            // Only allow Shinsu to attack while Mode is active, approximating
            // the C# CanAttack && Mode property.
            if monster.special_mode && now_ms >= monster.next_attack_time_ms {
                if let Some((
                    target_id,
                    target_monster_index,
                    tx,
                    ty,
                    dx,
                    dy,
                )) = best_attack
                {
                    debug!(
                        "pet_ai_shinsu_attack: pet_id={} map={} pet_pos=({}, {}) target_id={} target_pos=({}, {}) dx={} dy={} now_ms={}",
                        monster.id,
                        map_index,
                        monster.x,
                        monster.y,
                        target_id,
                        tx,
                        ty,
                        dx,
                        dy,
                        now_ms,
                    );

                    let sx = dx.clamp(-1, 1);
                    let sy = dy.clamp(-1, 1);
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

                    pending_pet_attacks.push((
                        monster.id,
                        map_index,
                        target_id,
                        target_monster_index,
                        owner_sid,
                        monster.monster_index,
                    ));

                    let delay_ms = Self::compute_monster_attack_delay_ms(attack_speed);
                    monster.next_attack_time_ms = now_ms.saturating_add(delay_ms);

                    handled = true;
                }
            }

            // When not attacking, chase the closest hostile within leash
            // distance as long as we remain reasonably close to the owner.
            if !handled {
                if let Some((tx, ty, dist_to_target)) = best_chase {
                    let owner_dx = owner_x - monster.x;
                    let owner_dy = owner_y - monster.y;
                    let owner_dist = owner_dx.abs().max(owner_dy.abs());

                    // Only chase while we are strictly inside the leash
                    // distance so that a single step cannot move us outside
                    // Globals.DataRange and out of the owner's view.
                    if owner_dist < leash_distance && dist_to_target > 0 {
                        if monster.next_move_time_ms == 0
                            || now_ms >= monster.next_move_time_ms
                        {
                            let step_x = (tx - monster.x).clamp(-1, 1);
                            let step_y = (ty - monster.y).clamp(-1, 1);

                            let new_x = monster.x.saturating_add(step_x);
                            let new_y = monster.y.saturating_add(step_y);

                            if new_x >= 0 && new_y >= 0 {
                                let from_x = monster.x as u16;
                                let from_y = monster.y as u16;
                                let to_x = new_x as u16;
                                let to_y = new_y as u16;

                                if to_x < map.width
                                    && to_y < map.height
                                    && map.can_move(from_x, from_y, to_x, to_y)
                                    && !self.is_cell_blocked(map_index, new_x, new_y)
                                {
                                    // Update facing based on the chase step,
                                    // mirroring C# Functions.DirectionFromPoint
                                    // and ensuring diagonal movement uses the
                                    // correct MirDirection value.
                                    let dir = match (step_x, step_y) {
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

                                    debug!(
                                        "pet_ai_shinsu_chase: pet_id={} map={} from=({}, {}) to=({}, {}) owner_dist={} target_dist={} now_ms={}",
                                        monster.id,
                                        map_index,
                                        monster.x,
                                        monster.y,
                                        new_x,
                                        new_y,
                                        owner_dist,
                                        dist_to_target,
                                        now_ms,
                                    );

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
                                        direction: dir,
                                    });

                                    handled = true;
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Generic pet behaviour: prefer to attack the closest hostile
            // within a simple Chebyshev range, but also chase nearby targets
            // while staying within leash distance of the owner. HolyDeva in
            // C# has AttackRange=6, so we give it a larger range while
            // keeping other pets at 2.
            let attack_range: i32 = match pet_kind {
                PetKind::TaoistHolyDeva => 6,
                _ => 2,
            };
            let follow_distance = template.follow_distance.max(1);
            let leash_distance = template.leash_distance.max(follow_distance);

            let mut best_attack: Option<(u64, i32, i32, i32, i32)> = None;
            let mut best_chase: Option<(i32, i32, i32)> = None;

            for &(tid, t_index, tx, ty) in hostile_monsters {
                let dx = tx - monster.x;
                let dy = ty - monster.y;

                if dx == 0 && dy == 0 {
                    continue;
                }

                let dist = dx.abs().max(dy.abs());

                // Attack candidates: within attack_range.
                if dx.abs() <= attack_range && dy.abs() <= attack_range {
                    match best_attack {
                        None => best_attack = Some((tid, t_index, tx, ty, dist)),
                        Some((_, _, _, _, best_dist)) if dist < best_dist => {
                            best_attack = Some((tid, t_index, tx, ty, dist));
                        }
                        _ => {}
                    }
                }

                // Chase candidates: within leash_distance of the pet so that
                // we do not wander too far away from the owner.
                if dist <= leash_distance {
                    match best_chase {
                        None => best_chase = Some((tx, ty, dist)),
                        Some((_, _, best_dist)) if dist < best_dist => {
                            best_chase = Some((tx, ty, dist));
                        }
                        _ => {}
                    }
                }
            }

            // Prefer attacking when in range and attack cooldown allows it.
            if let Some((target_id, target_monster_index, tx, ty, _)) = best_attack {
                if now_ms >= monster.next_attack_time_ms {
                    let sx = (tx - monster.x).clamp(-1, 1);
                    let sy = (ty - monster.y).clamp(-1, 1);
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

                    pending_pet_attacks.push((
                        monster.id,
                        map_index,
                        target_id,
                        target_monster_index,
                        owner_sid,
                        monster.monster_index,
                    ));

                    let delay_ms = Self::compute_monster_attack_delay_ms(attack_speed);
                    monster.next_attack_time_ms = now_ms.saturating_add(delay_ms);

                    handled = true;
                }
            }

            // When not attacking, chase the closest hostile within leash
            // distance as long as we remain reasonably close to the owner.
            if !handled {
                if let Some((tx, ty, dist_to_target)) = best_chase {
                    // Only chase freely while we have not strayed beyond our
                    // leash limit from the owner.
                    let owner_dx = owner_x - monster.x;
                    let owner_dy = owner_y - monster.y;
                    let owner_dist = owner_dx.abs().max(owner_dy.abs());

                    // Use a strict < comparison so that a single chase step
                    // cannot push the pet outside the leash/DataRange.
                    if owner_dist < leash_distance && dist_to_target > attack_range {
                        if monster.next_move_time_ms == 0
                            || now_ms >= monster.next_move_time_ms
                        {
                            let step_x = (tx - monster.x).clamp(-1, 1);
                            let step_y = (ty - monster.y).clamp(-1, 1);

                            let new_x = monster.x.saturating_add(step_x);
                            let new_y = monster.y.saturating_add(step_y);

                            if new_x >= 0 && new_y >= 0 {
                                let from_x = monster.x as u16;
                                let from_y = monster.y as u16;
                                let to_x = new_x as u16;
                                let to_y = new_y as u16;

                                if to_x < map.width
                                    && to_y < map.height
                                    && map.can_move(from_x, from_y, to_x, to_y)
                                    && !self.is_cell_blocked(map_index, new_x, new_y)
                                {
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

                                    handled = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        let dx = owner_x - monster.x;
        let dy = owner_y - monster.y;
        let dist = dx.abs().max(dy.abs());

        let follow_distance = template.follow_distance.max(1);
        let leash_distance = template.leash_distance.max(follow_distance);

        // If the pet has strayed too far from its owner, snap it back to the
        // owner's current tile.
        if !handled && dist > leash_distance {
            debug!(
                "pet_ai_recall_to_owner: pet_id={} kind={:?} map={} from=({}, {}) owner=({}, {}) dist={} leash={} now_ms={}",
                monster.id,
                pet_kind,
                map_index,
                monster.x,
                monster.y,
                owner_x,
                owner_y,
                dist,
                leash_distance,
                now_ms,
            );

            self.remove_monster_from_occupancy(monster.id, map_index, monster.x, monster.y);
            monster.x = owner_x;
            monster.y = owner_y;
            self.add_monster_to_occupancy(monster.id, map_index, monster.x, monster.y);

            events.push(WorldEvent::ObjectLocation {
                object_id: monster.id,
                map_index,
                x: monster.x,
                y: monster.y,
                direction: monster.direction,
            });

            handled = true;
        }

        // When outside the preferred follow distance, take a single step
        // toward the owner if movement cooldown and map/occupancy allow it.
        if !handled && dist > follow_distance {
            if monster.next_move_time_ms == 0 || now_ms >= monster.next_move_time_ms {
                let step_x = dx.clamp(-1, 1);
                let step_y = dy.clamp(-1, 1);

                let new_x = monster.x.saturating_add(step_x);
                let new_y = monster.y.saturating_add(step_y);

                if new_x >= 0 && new_y >= 0 {
                    let from_x = monster.x as u16;
                    let from_y = monster.y as u16;
                    let to_x = new_x as u16;
                    let to_y = new_y as u16;

                    if to_x < map.width
                        && to_y < map.height
                        && map.can_move(from_x, from_y, to_x, to_y)
                        && !self.is_cell_blocked(map_index, new_x, new_y)
                    {
                        // Update facing towards the owner based on the
                        // follow step, mirroring C#
                        // Functions.DirectionFromPoint and ensuring
                        // diagonals use the correct MirDirection value.
                        let dir = match (step_x, step_y) {
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

                        self.remove_monster_from_occupancy(
                            monster.id,
                            map_index,
                            monster.x,
                            monster.y,
                        );
                        self.add_monster_to_occupancy(monster.id, map_index, new_x, new_y);

                        monster.x = new_x;
                        monster.y = new_y;

                        let delay_ms = Self::compute_monster_move_delay_ms(move_speed);
                        monster.next_move_time_ms = now_ms.saturating_add(delay_ms);

                        events.push(WorldEvent::ObjectLocation {
                            object_id: monster.id,
                            map_index,
                            x: monster.x,
                            y: monster.y,
                            direction: dir,
                        });
                    }
                }
            }

            handled = true;
        }

        // Within follow distance (or after handling attack/leash): stay near
        // the owner for this tick. Combat resolution for any scheduled
        // attacks will run after the AI loop.
        monster.ai_state = MonsterAiState::Idle;
        monster.target_session_id = None;
    }
}
