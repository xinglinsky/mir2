use std::collections::HashSet;

use crystal_server_core::world;
use crystal_server_core::world::WorldProvider;
use crystal_shared_proto::scene::{
    SObjectMonster,
    SObjectNpc,
    SObjectRemove,
};
use crystal_shared_proto::user::SObjectPlayer;

use super::{LoginConnection, PlayerVisual};

impl LoginConnection {
    pub(crate) fn send_monsters_for_map(&self, map_index: i32, out: &mut Vec<Vec<u8>>) {
        let monsters = {
            let world = self.world.lock().unwrap();
            world.monsters_for_map(map_index)
        };

        for monster in monsters {
            if let Some(info) = self.world_db.get_monster_info(monster.monster_index) {
                let packet = SObjectMonster {
                    object_id: monster.id as u32,
                    name: info.name.clone(),
                    name_colour_argb: -1,
                    location_x: monster.x,
                    location_y: monster.y,
                    image: info.image,
                    direction: monster.direction,
                    effect: info.effect,
                    ai: info.ai,
                    light: info.light,
                    dead: false,
                    skeleton: false,
                    poison: 0,
                    hidden: false,
                    shock_time: 0,
                    binding_shot_center: false,
                    extra: false,
                    extra_byte: 0,
                    buffs: Vec::new(),
                };
                if let Ok(raw) = packet.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }

    pub(crate) fn send_npcs_for_map(&self, map_index: i32, out: &mut Vec<Vec<u8>>) {
        for npc in self
            .world_db
            .npc_infos
            .iter()
            .filter(|n| n.map_index == map_index)
        {
            let packet = SObjectNpc {
                object_id: npc.index as u32,
                name: npc.name.clone(),
                name_colour_argb: -1,
                image: npc.image,
                colour_argb: -1,
                location_x: npc.location_x,
                location_y: npc.location_y,
                direction: 0,
                quest_ids: npc.collect_quest_indexes.clone(),
            };
            if let Ok(raw) = packet.encode() {
                out.push(Self::encode_raw(raw));
            }
        }
    }

    pub(crate) fn update_visibility(&mut self, out: &mut Vec<Vec<u8>>) {
        if self.current_map_index == 0 {
            return;
        }

        let range = Self::DATA_RANGE;
        let map_index = self.current_map_index;

        // Monsters in view.
        let monsters_in_view = {
            let world = self.world.lock().unwrap();
            world.monsters_in_view_for_map(map_index, self.current_x, self.current_y, range)
        };

        let mut visible_monster_ids: HashSet<u64> = HashSet::new();

        for monster in &monsters_in_view {
            visible_monster_ids.insert(monster.id);

            if !self.known_monsters.contains(&monster.id) {
                if let Some(info) = self.world_db.get_monster_info(monster.monster_index) {
                    let packet = SObjectMonster {
                        object_id: monster.id as u32,
                        name: info.name.clone(),
                        name_colour_argb: -1,
                        location_x: monster.x,
                        location_y: monster.y,
                        image: info.image,
                        direction: monster.direction,
                        effect: info.effect,
                        ai: info.ai,
                        light: info.light,
                        dead: false,
                        skeleton: false,
                        poison: 0,
                        hidden: false,
                        shock_time: 0,
                        binding_shot_center: false,
                        extra: false,
                        extra_byte: 0,
                        buffs: Vec::new(),
                    };
                    if let Ok(raw) = packet.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        }

        // Monsters leaving view.
        let removed_monsters: Vec<u64> = self
            .known_monsters
            .iter()
            .filter(|id| !visible_monster_ids.contains(id))
            .cloned()
            .collect();

        for id in removed_monsters {
            let pkt = SObjectRemove { object_id: id as u32 };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
        }

        self.known_monsters = visible_monster_ids;

        // NPCs in view.
        let mut visible_npc_ids: HashSet<i32> = HashSet::new();

        for npc in self
            .world_db
            .npc_infos
            .iter()
            .filter(|n| n.map_index == map_index)
        {
            if (npc.location_x - self.current_x).abs() <= range
                && (npc.location_y - self.current_y).abs() <= range
            {
                visible_npc_ids.insert(npc.index);

                if !self.known_npcs.contains(&npc.index) {
                    let packet = SObjectNpc {
                        object_id: npc.index as u32,
                        name: npc.name.clone(),
                        name_colour_argb: -1,
                        image: npc.image,
                        colour_argb: -1,
                        location_x: npc.location_x,
                        location_y: npc.location_y,
                        direction: 0,
                        quest_ids: npc.collect_quest_indexes.clone(),
                    };
                    if let Ok(raw) = packet.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        }

        // NPCs leaving view.
        let removed_npcs: Vec<i32> = self
            .known_npcs
            .iter()
            .filter(|id| !visible_npc_ids.contains(id))
            .cloned()
            .collect();

        for id in removed_npcs {
            let pkt = SObjectRemove { object_id: id as u32 };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
        }

        self.known_npcs = visible_npc_ids;

        // Players in view.
        let players_in_view: Vec<(world::SessionId, i32, i32, u8)> = {
            let world = self.world.lock().unwrap();
            world.players_in_view_for_map(
                map_index,
                self.current_x,
                self.current_y,
                range,
                Some(self.session_id),
            )
        };

        let mut visible_player_ids: HashSet<world::SessionId> = HashSet::new();

        for (sid, x, y, direction) in players_in_view {
            visible_player_ids.insert(sid);

            if !self.known_players.contains(&sid) {
                let snapshot = {
                    let map = self.player_summaries.lock().unwrap();
                    map.get(&sid).cloned()
                };

                if let Some(snap) = snapshot {
                    let pkt = SObjectPlayer {
                        object_id: sid,
                        name: snap.name,
                        guild_name: snap.guild_name,
                        guild_rank_name: snap.guild_rank_name,
                        name_colour_argb: snap.name_colour_argb,
                        class: snap.class,
                        gender: snap.gender,
                        level: snap.level,
                        location_x: x,
                        location_y: y,
                        direction,
                        hair: snap.hair,
                        light: 0,
                        weapon: 0,
                        weapon_effect: 0,
                        armour: 0,
                        poison: 0,
                        dead: false,
                        hidden: false,
                        effect: 0,
                        wing_effect: 0,
                        extra: false,
                        mount_type: 0,
                        riding_mount: false,
                        fishing: false,
                        transform_type: 0,
                        element_orb_effect: 0,
                        element_orb_lvl: 0,
                        element_orb_max: 0,
                        buffs: Vec::new(),
                        level_effects: 0,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        }

        // Players leaving view.
        let removed_players: Vec<world::SessionId> = self
            .known_players
            .iter()
            .filter(|sid| !visible_player_ids.contains(sid))
            .cloned()
            .collect();

        for sid in removed_players {
            let pkt = SObjectRemove { object_id: sid };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
        }

        self.known_players = visible_player_ids;
    }
}
