use crystal_server_core::world;
use crystal_server_core::world::WorldProvider;
use crystal_shared_proto::map::SMapChanged;
use crystal_shared_proto::scene::{SObjectAttack, SObjectStruck, SDamageIndicator, SObjectHealth};
use crystal_shared_proto::user::SUserLocation;

use super::LoginConnection;

impl LoginConnection {
    pub(crate) fn apply_step(&mut self, direction: u8, distance: i32, out: &mut Vec<Vec<u8>>) -> bool {
        let cmd = match distance {
            0 => world::WorldCommand::Turn {
                session_id: self.session_id,
                direction,
            },
            1 => world::WorldCommand::Walk {
                session_id: self.session_id,
                direction,
            },
            2 => world::WorldCommand::Run {
                session_id: self.session_id,
                direction,
            },
            _ => return false,
        };

        let events = {
            let mut world = self.world.lock().unwrap();
            world.handle_command(cmd)
        };
        self.handle_world_events(events, out)
    }

    pub(crate) fn handle_world_events(
        &mut self,
        events: Vec<world::WorldEvent>,
        out: &mut Vec<Vec<u8>>,
    ) -> bool {
        let mut map_changed = false;

        for event in events {
            match event {
                world::WorldEvent::UserLocation {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                } => {
                    if session_id != self.session_id {
                        continue;
                    }

                    self.current_map_index = map_index;
                    self.current_x = x;
                    self.current_y = y;
                    self.direction = direction;

                    let loc = SUserLocation {
                        location_x: x,
                        location_y: y,
                        direction,
                    };
                    if let Ok(raw) = loc.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
                world::WorldEvent::MapChanged {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                } => {
                    if session_id != self.session_id {
                        continue;
                    }

                    self.current_map_index = map_index;
                    self.current_x = x;
                    self.current_y = y;
                    self.direction = direction;
                    map_changed = true;

                    if let Some(info) = self.world_db.get_map_info(map_index) {
                        let pkt = SMapChanged {
                            map_index: info.index,
                            file_name: info.file_name.clone(),
                            title: info.title.clone(),
                            mini_map: info.mini_map,
                            big_map: info.big_map,
                            lights: info.light,
                            location_x: x,
                            location_y: y,
                            direction,
                            map_dark_light: info.map_dark_light,
                            music: info.music,
                            weather: 0,
                        };
                        if let Ok(raw) = pkt.encode() {
                            out.push(Self::encode_raw(raw));
                        }
                    }
                }
                world::WorldEvent::ObjectAttack {
                    session_id,
                    map_index: _,
                    x,
                    y,
                    direction,
                    spell,
                    level,
                    attack_type,
                } => {
                    if session_id != self.session_id {
                        continue;
                    }

                    let attack = SObjectAttack {
                        object_id: self.session_id,
                        location_x: x,
                        location_y: y,
                        direction,
                        spell,
                        level,
                        attack_type,
                    };
                    if let Ok(raw) = attack.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
                world::WorldEvent::ObjectStruck {
                    attacker_id,
                    target_id,
                    map_index: _,
                    x,
                    y,
                    direction,
                    damage,
                    damage_type,
                    health_percent,
                } => {
                    // For now we only emit struck/damage/health for actions caused by
                    // this client. Other players will see the same events via their
                    // own connections.
                    if attacker_id != self.session_id {
                        continue;
                    }

                    let object_id = target_id as u32;

                    let struck = SObjectStruck {
                        object_id,
                        attacker_id: attacker_id as u32,
                        location_x: x,
                        location_y: y,
                        direction,
                    };
                    if let Ok(raw) = struck.encode() {
                        out.push(Self::encode_raw(raw));
                    }

                    let dmg = SDamageIndicator {
                        damage,
                        damage_type,
                        object_id,
                    };
                    if let Ok(raw) = dmg.encode() {
                        out.push(Self::encode_raw(raw));
                    }

                    let health = SObjectHealth {
                        object_id,
                        percent: health_percent,
                        expire: 2,
                    };
                    if let Ok(raw) = health.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        }

        map_changed
    }
}
