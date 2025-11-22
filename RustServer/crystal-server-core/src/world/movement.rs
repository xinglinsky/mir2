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
                .find(|m| m.source_x == player.x && m.source_y == player.y)
            else {
                return;
            };
            println!(
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
                self.spawn_monsters_for_map(dest_map_index, &dest_map);

                player.map_index = dest_map_index;
                player.x = dest_x;
                player.y = dest_y;

                println!(
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
                println!(
                    "[move] Map movement failed: could not load destination map {} from ({}, {})",
                    dest_map_index, player.x, player.y
                );
            }
        }
    }

    pub(super) fn apply_step(player: &mut PlayerState, map: Option<map::Map>, direction: u8, distance: i32) {
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
        let mut new_x = player.x;
        let mut new_y = player.y;

        for _ in 0..steps {
            let tx = new_x + dx;
            let ty = new_y + dy;

            if tx < 0 || ty < 0 || tx > u16::MAX as i32 || ty > u16::MAX as i32 {
                println!("[move] blocked: out of bounds ({}, {})", tx, ty);
                break;
            }

            if let Some(ref m) = map {
                let ux = tx as u16;
                let uy = ty as u16;

                if m.is_walkable(ux, uy) {
                    new_x = tx;
                    new_y = ty;
                } else {
                    let attr = m.cell(ux, uy).map(|c| &c.attribute);
                    println!(
                        "[move] blocked on map {} from ({}, {}) to ({}, {}), attr={:?}",
                        player.map_index,
                        new_x,
                        new_y,
                        tx,
                        ty,
                        attr,
                    );
                    break;
                }
            } else {
                new_x = tx;
                new_y = ty;
            }
        }

        println!(
            "[move] apply_step: map {} dir {} dist {} from ({}, {}) to ({}, {})",
            player.map_index,
            direction,
            distance,
            player.x,
            player.y,
            new_x,
            new_y,
        );

        player.x = new_x;
        player.y = new_y;
    }
}
