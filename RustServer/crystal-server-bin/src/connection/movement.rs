use crystal_server_core::world;
use crystal_server_core::world::WorldProvider;
use crystal_shared_proto::map::SMapChanged;
use crystal_shared_proto::scene::{
    SObjectAttack,
    SObjectRun,
    SObjectStruck,
    SObjectTurn,
    SObjectTurnWalkRun,
    SObjectWalk,
    SDamageIndicator,
    SObjectHealth,
};
use crystal_shared_proto::user::SUserLocation;

use super::LoginConnection;

impl LoginConnection {
    fn enqueue_for_viewers(
        &self,
        map_index: i32,
        x: i32,
        y: i32,
        payload: Vec<u8>,
    ) {
        let viewers: Vec<world::SessionId> = {
            let world = self.world.lock().unwrap();
            world.sessions_in_range_for_map(map_index, x, y, Self::DATA_RANGE)
        };

        let mut outboxes = self.outboxes.lock().unwrap();
        for sid in viewers {
            if sid == self.session_id {
                continue;
            }
            outboxes.entry(sid).or_default().push(payload.clone());
        }
    }

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

        // Remember the movement type (0=turn,1=walk,2=run) for this command so
        // that handle_world_events can emit the appropriate SObjectTurn/Walk/Run
        // broadcast to nearby players.
        self.last_move_kind = Some(distance as u8);

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

        // Take and clear the last movement kind associated with this batch of
        // world events so we can send a matching ObjectTurn/Walk/Run to
        // observers when processing the UserLocation update.
        let movement_kind = self.last_move_kind.take();

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

                    if let Some(kind) = movement_kind {
                        let base = SObjectTurnWalkRun {
                            object_id: self.session_id,
                            location_x: x,
                            location_y: y,
                            direction,
                        };

                        let pkt_res = match kind {
                            0 => SObjectTurn(base).encode(),
                            1 => SObjectWalk(base).encode(),
                            2 => SObjectRun(base).encode(),
                            _ => return map_changed,
                        };

                        if let Ok(pkt) = pkt_res {
                            let raw = Self::encode_raw(pkt);
                            self.enqueue_for_viewers(map_index, x, y, raw);
                        }
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
                    map_index,
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
                    if let Ok(pkt) = attack.encode() {
                        let raw = Self::encode_raw(pkt);
                        out.push(raw.clone());
                        self.enqueue_for_viewers(map_index, x, y, raw);
                    }
                }
                world::WorldEvent::ObjectStruck {
                    attacker_id,
                    target_id,
                    map_index,
                    x,
                    y,
                    direction,
                    damage,
                    damage_type,
                    health_percent,
                } => {
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
                    if let Ok(pkt) = struck.encode() {
                        let raw = Self::encode_raw(pkt);
                        out.push(raw.clone());
                        self.enqueue_for_viewers(map_index, x, y, raw);
                    }

                    let dmg = SDamageIndicator {
                        damage,
                        damage_type,
                        object_id,
                    };
                    if let Ok(pkt) = dmg.encode() {
                        let raw = Self::encode_raw(pkt);
                        out.push(raw.clone());
                        self.enqueue_for_viewers(map_index, x, y, raw);
                    }

                    let health = SObjectHealth {
                        object_id,
                        percent: health_percent,
                        expire: 2,
                    };
                    if let Ok(pkt) = health.encode() {
                        let raw = Self::encode_raw(pkt);
                        out.push(raw.clone());
                        self.enqueue_for_viewers(map_index, x, y, raw);
                    }
                }
            }
        }

        map_changed
    }
}
