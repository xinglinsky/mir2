use crate::world::map;
use crate::world::player::PlayerState;
use crate::world::provider::WorldProvider;

use super::{World, WorldEvent};

impl<P: WorldProvider> World<P> {
    pub(super) fn check_map_movement(&mut self, player: &mut PlayerState, events: &mut Vec<WorldEvent>) {
        let current_map_index = player.map_index;
        // Limit the lifetime of the immutable borrow from provider so that we can
        // subsequently borrow &mut self when loading maps and spawning monsters.
        let (dest_map_index, dest_x, dest_y) = {
            let Some(map_info) = self.provider.get_map_info(current_map_index) else {
                return;
            };

            let Some(movement) = map_info
                .movements
                .iter()
                .find(|m| {
                    if m.source_x != player.x || m.source_y != player.y {
                        return false;
                    }

                    // NeedHole: in C# this requires a DigOutZombie/DigOutArmadillo
                    // spell object on the source cell. The Rust core does not yet
                    // track map spell instances, so we conservatively treat this as
                    // unsatisfied and skip such movements for now.
                    if m.need_hole {
                        if !self.cell_has_hole_spell(current_map_index, m.source_x, m.source_y) {
                            tracing::debug!(
                                "[move] Map movement candidate blocked: need_hole at map {} source=({}, {})",
                                current_map_index,
                                m.source_x,
                                m.source_y,
                            );
                            return false;
                        }
                    }

                    // ConquestIndex: in C# this requires the player's guild to own
                    // the specified conquest. Guild/conquest ownership is not yet
                    // wired into the Rust world state, so treat movements with a
                    // positive conquest_index as gated off for now.
                    if m.conquest_index > 0 {
                        tracing::debug!(
                            "[move] Map movement candidate blocked: conquest_index {} at map {} source=({}, {})",
                            m.conquest_index,
                            current_map_index,
                            m.source_x,
                            m.source_y,
                        );
                        return false;
                    }

                    true
                })
            else {
                return;
            };
            tracing::debug!(
                "[move] check_map_movement: from map {} at ({}, {}) using movement source=({}, {}) -> dest map {} at ({}, {})",
                current_map_index,
                player.x,
                player.y,
                movement.source_x,
                movement.source_y,
                movement.dest_map_index,
                movement.dest_x,
                movement.dest_y,
            );

            (movement.dest_map_index, movement.dest_x, movement.dest_y)
        };

        match self.get_or_load_map(dest_map_index) {
            Some(dest_map) => {
                if dest_x < 0 || dest_y < 0 {
                    tracing::debug!(
                        "[move] Map movement blocked: dest out of bounds ({}, {}) on map {}",
                        dest_x, dest_y, dest_map_index
                    );
                    return;
                }

                let ux = dest_x as u16;
                let uy = dest_y as u16;
                if ux >= dest_map.width || uy >= dest_map.height || !dest_map.is_walkable(ux, uy) {
                    tracing::debug!(
                        "[move] Map movement blocked: dest not walkable on map {} at ({}, {})",
                        dest_map_index, dest_x, dest_y
                    );
                    return;
                }

                self.spawn_monsters_for_map(dest_map_index, &dest_map);

                let session_id = player.session_id;
                let old_x = player.x;
                let old_y = player.y;
                player.map_index = dest_map_index;
                player.x = dest_x;
                player.y = dest_y;

                self.remove_player_from_occupancy(session_id, current_map_index, old_x, old_y);
                self.add_player_to_occupancy(session_id, player.map_index, player.x, player.y);

                tracing::debug!(
                    "[move] Map movement success: session {} now on map {} at ({}, {})",
                    player.session_id,
                    player.map_index,
                    player.x,
                    player.y,
                );

                events.push(WorldEvent::MapChanged {
                    session_id: player.session_id,
                    map_index: player.map_index,
                    x: player.x,
                    y: player.y,
                    direction: player.direction,
                });
            }
            None => {
                tracing::debug!(
                    "[move] Map movement failed: could not load destination map {} from ({}, {})",
                    dest_map_index, player.x, player.y
                );
            }
        }
    }

    pub(super) fn apply_step(
        &mut self,
        player: &mut PlayerState,
        map: Option<map::Map>,
        direction: u8,
        distance: i32,
    ) {
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

        // Always update facing direction, even if movement is blocked.
        player.direction = direction;

        if distance <= 0 {
            return;
        }

        let steps = distance;
        let mut cur_x = player.x;
        let mut cur_y = player.y;
        let map_index = player.map_index;
        let session_id = player.session_id;

        for _ in 0..steps {
            let tx = cur_x + dx;
            let ty = cur_y + dy;

            if tx < 0 || ty < 0 || tx > u16::MAX as i32 || ty > u16::MAX as i32 {
                tracing::debug!("[move] blocked: out of bounds ({}, {})", tx, ty);
                break;
            }

            if let Some(ref m) = map {
                let ux = tx as u16;
                let uy = ty as u16;

                if !m.is_walkable(ux, uy) {
                    let attr = m.cell(ux, uy).map(|c| &c.attribute);
                    tracing::debug!(
                        "[move] blocked on map {} from ({}, {}) to ({}, {}), attr={:?}",
                        player.map_index,
                        cur_x,
                        cur_y,
                        tx,
                        ty,
                        attr,
                    );
                    break;
                }
            }

            // Dynamic blocking: prevent walking into cells occupied by other
            // players or monsters, mirroring C# Cell.Objects + Blocking.
            if self.is_cell_blocked(map_index, tx, ty) {
                break;
            }

            // Move one step and update occupancy tracking.
            self.remove_player_from_occupancy(session_id, map_index, cur_x, cur_y);
            self.add_player_to_occupancy(session_id, map_index, tx, ty);
            cur_x = tx;
            cur_y = ty;
        }

        // tracing::debug!(
        //     "[move] apply_step: map {} dir {} dist {} from ({}, {}) to ({}, {})",
        //     player.map_index,
        //     direction,
        //     distance,
        //     player.x,
        //     player.y,
        //     cur_x,
        //     cur_y,
        // );

        player.x = cur_x;
        player.y = cur_y;
    }
}
