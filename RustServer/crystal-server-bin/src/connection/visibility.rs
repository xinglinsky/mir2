use std::collections::HashSet;

use crystal_server_core::world;
use crystal_server_core::world::WorldProvider;
use crystal_server_core::world::types::{Monster, PetKind};
use crystal_shared_proto::scene::{
    SObjectHealth,
    SObjectMonster,
    SObjectNpc,
    SObjectRemove,
};
use crystal_shared_proto::item_types::ItemInfoData;
use tracing::debug;
use crystal_shared_proto::user::{SObjectHero, SObjectPlayer};

use super::{hero_object_id, LoginConnection};

impl LoginConnection {
    fn get_real_item<'a>(
        origin: &'a ItemInfoData,
        level: u16,
        job: u8,
        items: &'a [ItemInfoData],
    ) -> &'a ItemInfoData {
        if origin.class_based && origin.level_based {
            let mut output = origin;
            let required_class = (1u16 << job) as u8;
            for info in items {
                if !info.name.starts_with(&origin.name) {
                    continue;
                }
                if info.required_class != required_class {
                    continue;
                }
                if info.required_type != 0 {
                    continue;
                }
                if info.required_amount as u16 > level {
                    continue;
                }
                if output.required_amount > info.required_amount {
                    continue;
                }
                if origin.required_gender != info.required_gender {
                    continue;
                }
                output = info;
            }
            return output;
        }

        if origin.class_based {
            let required_class = (1u16 << job) as u8;
            for info in items {
                if !info.name.starts_with(&origin.name) {
                    continue;
                }
                if info.required_class != required_class {
                    continue;
                }
                if origin.required_gender != info.required_gender {
                    continue;
                }
                return info;
            }
            return origin;
        }

        if origin.level_based {
            let mut output = origin;
            for info in items {
                if !info.name.starts_with(&origin.name) {
                    continue;
                }
                if info.required_type != 0 {
                    continue;
                }
                if info.required_amount as u16 > level {
                    continue;
                }
                if output.required_amount >= info.required_amount {
                    continue;
                }
                if origin.required_gender != info.required_gender {
                    continue;
                }
                output = info;
            }
            return output;
        }

        origin
    }

    fn player_hp_percent_for_session(&self, sid: world::SessionId) -> u8 {
        let world = self.world.lock().unwrap();
        if let (Some((max_hp, _)), Some((cur_hp, _))) =
            (world.player_max_hp_mp(sid), world.player_current_hp_mp(sid))
        {
            if max_hp > 0 {
                let clamped = cur_hp.max(0).min(max_hp);
                return ((clamped as i64 * 100 / max_hp as i64).clamp(0, 100)) as u8;
            }
        }
        100
    }

    #[allow(dead_code)]
    pub(crate) fn send_monsters_for_map(&self, map_index: i32, out: &mut Vec<Vec<u8>>) {
        let monsters = {
            let world = self.world.lock().unwrap();
            world.monsters_for_map(map_index)
        };

        for monster in monsters {
            if let Some(info) = self.world_db.get_monster_info(monster.monster_index) {
                // Mirror C# MonsterObject.Name and the Extra flag used for
                // summoned pets: when a monster is a pet owned by a player,
                // show its name as "GameName(OwnerName)" and mark
                // extra=true so the client can treat it as a pet.
                let (name, name_colour_argb, extra) = if monster.is_pet {
                    let base_name = info.name.clone();

                    let owner_name = monster.owner_session_id.and_then(|sid| {
                        let map = self.player_summaries.lock().ok()?;
                        map.get(&sid).map(|p| p.name.clone())
                    });

                    let name = match owner_name {
                        Some(owner) if !owner.is_empty() => {
                            format!("{}({})", base_name, owner)
                        }
                        _ => base_name,
                    };

                    let owner_colour = monster.owner_session_id.and_then(|sid| {
                        let map = self.player_summaries.lock().ok()?;
                        map.get(&sid).map(|p| p.name_colour_argb)
                    });

                    let name_colour_argb = owner_colour.unwrap_or(-1);

                    (name, name_colour_argb, true)
                } else {
                    (info.name.clone(), -1, false)
                };

                // For Taoist pets, override the Image field to match the
                // client Monster enum used for visuals instead of relying
                // purely on the DB image. This ensures Shinsu/HolyDeva use the
                // correct sprite IDs for their special animations.
                let image: u16 = if monster.is_pet {
                    match monster.pet_kind {
                        Some(PetKind::TaoistShinsu) => {
                            if monster.special_mode {
                                Monster::Shinsu1.as_u16()
                            } else {
                                Monster::Shinsu.as_u16()
                            }
                        }
                        Some(PetKind::TaoistHolyDeva) => Monster::HolyDeva.as_u16(),
                        Some(PetKind::TaoistSkeleton) => Monster::BoneFamiliar.as_u16(),
                        _ => info.image,
                    }
                } else {
                    info.image
                };

                let packet = SObjectMonster {
                    object_id: monster.id as u32,
                    name,
                    name_colour_argb,
                    location_x: monster.x,
                    location_y: monster.y,
                    image,
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
                    extra,
                    extra_byte: 0,
                    buffs: Vec::new(),
                };
                if let Ok(raw) = packet.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn send_npcs_for_map(&self, map_index: i32, out: &mut Vec<Vec<u8>>) {
        for npc in self
            .world_db
            .npc_infos
            .iter()
            .filter(|n| n.map_index == map_index)
        {
            let npc_object_id = super::npc_object_id(npc.index);
            let packet = SObjectNpc {
                object_id: npc_object_id,
                name: npc.name.clone(),
                // Match C# NPCObject default: NameColour = Color.Lime.
                name_colour_argb: 0xFF00FF00u32 as i32,
                image: npc.image,
                // Match default NPCInfo.Colour.ToArgb(), which is 0 unless
                // explicitly overridden for special NPCs like flags.
                colour_argb: 0,
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

        // Monsters in view. Start with the generic visibility query based on
        // DATA_RANGE around the player.
        let mut monsters_in_view = {
            let world = self.world.lock().unwrap();
            world.monsters_in_view_for_map(map_index, self.current_x, self.current_y, range)
        };

        // Always keep this player's own summoned pets visible regardless of
        // distance, so that pets like Shinsu do not visually disappear while
        // still attacking when they move outside the normal DATA_RANGE
        // rectangle. This mirrors the original server behaviour where your
        // own pets remain tracked even when they momentarily stray.
        {
            let world = self.world.lock().unwrap();
            let all_monsters = world.monsters_for_map(map_index);

            for m in all_monsters.into_iter().filter(|m| {
                m.is_pet && m.owner_session_id == Some(self.session_id)
            }) {
                if !monsters_in_view.iter().any(|existing| existing.id == m.id) {
                    monsters_in_view.push(m);
                }
            }
        }

        let mut visible_monster_ids: HashSet<u64> = HashSet::new();

        for monster in &monsters_in_view {
            visible_monster_ids.insert(monster.id);

            if !self.known_monsters.contains(&monster.id) {
                if let Some(info) = self.world_db.get_monster_info(monster.monster_index) {
                    // As in send_monsters_for_map, mark summoned pets and
                    // include the owner's name in the pet's display name.
                    let (name, name_colour_argb, extra) = if monster.is_pet {
                        let base_name = info.name.clone();

                        let owner_name = monster.owner_session_id.and_then(|sid| {
                            let map = self.player_summaries.lock().ok()?;
                            map.get(&sid).map(|p| p.name.clone())
                        });

                        let name = match owner_name {
                            Some(owner) if !owner.is_empty() => {
                                format!("{}({})", base_name, owner)
                            }
                            _ => base_name,
                        };

                        let owner_colour = monster.owner_session_id.and_then(|sid| {
                            let map = self.player_summaries.lock().ok()?;
                            map.get(&sid).map(|p| p.name_colour_argb)
                        });

                        let name_colour_argb = owner_colour.unwrap_or(-1);

                        (name, name_colour_argb, true)
                    } else {
                        (info.name.clone(), -1, false)
                    };

                    // debug!(
                    //     "update_visibility: session_id={} new monster id={} index={} name={} is_pet={} map={} pos=({}, {})",
                    //     self.session_id,
                    //     monster.id,
                    //     monster.monster_index,
                    //     info.name,
                    //     monster.is_pet,
                    //     map_index,
                    //     monster.x,
                    //     monster.y
                    // );

                    // Mirror the pet image override used in
                    // send_monsters_for_map so that Taoist pets use the
                    // correct Monster enum IDs for client-side animations.
                    let image: u16 = if monster.is_pet {
                        match monster.pet_kind {
                            Some(PetKind::TaoistShinsu) => {
                                if monster.special_mode {
                                    Monster::Shinsu1.as_u16()
                                } else {
                                    Monster::Shinsu.as_u16()
                                }
                            }
                            Some(PetKind::TaoistHolyDeva) => Monster::HolyDeva.as_u16(),
                            Some(PetKind::TaoistSkeleton) => Monster::BoneFamiliar.as_u16(),
                            _ => info.image,
                        }
                    } else {
                        info.image
                    };

                    let packet = SObjectMonster {
                        object_id: monster.id as u32,
                        name,
                        name_colour_argb,
                        location_x: monster.x,
                        location_y: monster.y,
                        image,
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
                        extra,
                        extra_byte: 0,
                        buffs: Vec::new(),
                    };
                    if let Ok(raw) = packet.encode() {
                        out.push(Self::encode_raw(raw));
                    }

                    // For summoned pets, also send an initial ObjectHealth so
                    // the client can render the pet HP bar, approximating the
                    // C# MapObject.BroadcastHealthChange behaviour for
                    // monsters with Master=player.
                    if monster.is_pet {
                        let health = SObjectHealth {
                            object_id: monster.id as u32,
                            percent: 100,
                            // Use a small expire window; the client will
                            // refresh this when the pet actually takes
                            // damage.
                            expire: 5,
                        };
                        if let Ok(pkt) = health.encode() {
                            out.push(Self::encode_raw(pkt));
                        }
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
            // Do not remove this session's own summoned pets purely due to
            // visibility pruning. Pets like Shinsu/HolyDeva should remain in
            // view while they are alive, even if they move outside the normal
            // DATA_RANGE rectangle. We verify existence and ownership via the
            // world state before deciding whether to send SObjectRemove.
            let mut skip_remove = false;
            {
                let world = self.world.lock().unwrap();
                let monsters = world.monsters_for_map(map_index);

                if let Some(m) = monsters.iter().find(|m| m.id == id) {
                    if m.is_pet && m.owner_session_id == Some(self.session_id) {
                        skip_remove = true;
                        // debug!(
                        //     "update_visibility: session_id={} skipping remove for own pet id={} index={} map={} pos=({}, {})",
                        //     self.session_id,
                        //     m.id,
                        //     m.monster_index,
                        //     map_index,
                        //     m.x,
                        //     m.y
                        // );
                    }
                }
            }

            if skip_remove {
                continue;
            }

            // debug!(
            //     "update_visibility: session_id={} removing monster id={} from view on map={}",
            //     self.session_id,
            //     id,
            //     map_index
            // );

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
                    let npc_object_id = super::npc_object_id(npc.index);
                    let packet = SObjectNpc {
                        object_id: npc_object_id,
                        name: npc.name.clone(),
                        name_colour_argb: 0xFF00FF00u32 as i32,
                        image: npc.image,
                        colour_argb: 0,
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
            let pkt = SObjectRemove {
                object_id: super::npc_object_id(id),
            };
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
        let mut next_known_players: HashSet<world::SessionId> = HashSet::new();

        for (sid, x, y, direction) in players_in_view.iter().cloned() {
            visible_player_ids.insert(sid);

            if self.known_players.contains(&sid) {
                next_known_players.insert(sid);
                continue;
            }

            if !self.known_players.contains(&sid) {
                let snapshot = {
                    let map = self.player_summaries.lock().unwrap();
                    map.get(&sid).cloned()
                };

                if let Some(snap) = snapshot {
                    let (weapon, weapon_effect, armour, wing_effect, light, mount_type) = {
                        let items_opt = {
                            let world = self.world.lock().unwrap();
                            world.player_items(sid)
                        };

                        let mut weapon: i16 = -1;
                        let mut weapon_effect: i16 = 0;
                        let mut armour: i16 = 0;
                        let mut wing_effect: u8 = 0;
                        let mut light: u8 = 0;
                        let mut mount_type: i16 = -1;

                        if let Some((_inv, eq)) = items_opt {
                            for slot in eq.slots.iter().flatten() {
                                if let Some(origin_info) = self
                                    .world_db
                                    .item_infos
                                    .iter()
                                    .find(|i| i.index == slot.item_index)
                                {
                                    if slot.current_dura == 0 && origin_info.durability > 0 {
                                        continue;
                                    }

                                    let info = Self::get_real_item(
                                        origin_info,
                                        snap.level,
                                        snap.class,
                                        &self.world_db.item_infos,
                                    );

                                    light = light.max(info.light);

                                    // ItemType mapping mirrors C#:
                                    // Weapon=1, Armour=2, Mount=19.
                                    if info.item_type == 1 {
                                        weapon = info.shape;
                                        weapon_effect = info.effect as i16;
                                    } else if info.item_type == 2 {
                                        armour = info.shape;
                                        wing_effect = info.effect;
                                    } else if info.item_type == 19 {
                                        mount_type = info.shape;
                                    }
                                }
                            }
                        }

                        (weapon, weapon_effect, armour, wing_effect, light, mount_type)
                    };

                    let is_hidden = {
                        let world = self.world.lock().unwrap();
                        world.player_hidden(sid)
                    };

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
                        light,
                        weapon,
                        weapon_effect,
                        armour,
                        poison: 0,
                        dead: false,
                        hidden: is_hidden,
                        effect: 0,
                        wing_effect,
                        extra: false,
                        mount_type,
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
                        debug!(
                            "vis: send SObjectPlayer -> session_id={} target_sid={} map={} pos=({}, {}) hidden={} gender={} class={} level={} weapon={} weapon_effect={} armour={} wing_effect={} light={} mount_type={}",
                            self.session_id,
                            sid,
                            map_index,
                            x,
                            y,
                            is_hidden,
                            snap.gender,
                            snap.class,
                            snap.level,
                            weapon,
                            weapon_effect,
                            armour,
                            wing_effect,
                            light,
                            mount_type,
                        );
                        out.push(Self::encode_raw(raw));
                        next_known_players.insert(sid);
                    }

                    // Also send an initial ObjectHealth packet for this
                    // newly-visible player so the client can render their
                    // head HP bar. This mirrors the C#
                    // MapObject.BroadcastHealthChange behaviour, which sends
                    // S.ObjectHealth on spawn/teleport and HP changes.
                    let percent = self.player_hp_percent_for_session(sid);
                    let hp_pkt = SObjectHealth {
                        object_id: sid,
                        percent,
                        expire: 30,
                    };
                    if let Ok(raw) = hp_pkt.encode() {
                        debug!(
                            "vis: send SObjectHealth -> session_id={} object_id={} percent={} expire={} (reason=new player in view)",
                            self.session_id,
                            sid,
                            percent,
                            30,
                        );
                        out.push(Self::encode_raw(raw));
                    }
                }
            }
        }

        // Refresh ObjectHealth for all visible players so that head HP bars are
        // visible even if the initial spawn-time packet was missed.
        for sid in &visible_player_ids {
            if *sid == self.session_id {
                continue;
            }
            let percent = self.player_hp_percent_for_session(*sid);
            let hp_pkt = SObjectHealth {
                object_id: *sid,
                percent,
                expire: 30,
            };
            if let Ok(raw) = hp_pkt.encode() {
                out.push(Self::encode_raw(raw));
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

        self.known_players = next_known_players;

        // Heroes in view.
        let mut heroes_in_view: Vec<(world::SessionId, i32, i32, u8)> = players_in_view.clone();
        heroes_in_view.push((
            self.session_id,
            self.current_x,
            self.current_y,
            self.direction,
        ));

        let mut visible_hero_owner_ids: HashSet<world::SessionId> = HashSet::new();
        let mut next_known_heroes: HashSet<world::SessionId> = HashSet::new();

        for (sid, x, y, direction) in heroes_in_view {
            visible_hero_owner_ids.insert(sid);

            if self.known_heroes.contains(&sid) {
                next_known_heroes.insert(sid);
                continue;
            }

            let is_new = !self.known_heroes.contains(&sid);

            let snapshot = {
                let map = self.player_summaries.lock().unwrap();
                map.get(&sid).cloned()
            };

            if let Some(snap) = snapshot {
                let (weapon, weapon_effect, armour, wing_effect, light, mount_type) = {
                    let items_opt = {
                        let world = self.world.lock().unwrap();
                        world.player_items(sid)
                    };

                    let mut weapon: i16 = -1;
                    let mut weapon_effect: i16 = 0;
                    let mut armour: i16 = 0;
                    let mut wing_effect: u8 = 0;
                    let mut light: u8 = 0;
                    let mut mount_type: i16 = -1;

                    if let Some((_inv, eq)) = items_opt {
                        for slot in eq.slots.iter().flatten() {
                            if let Some(origin_info) = self
                                .world_db
                                .item_infos
                                .iter()
                                .find(|i| i.index == slot.item_index)
                            {
                                if slot.current_dura == 0 && origin_info.durability > 0 {
                                    continue;
                                }

                                let info = Self::get_real_item(
                                    origin_info,
                                    snap.level,
                                    snap.class,
                                    &self.world_db.item_infos,
                                );

                                light = light.max(info.light);

                                // ItemType mapping mirrors C#:
                                // Weapon=1, Armour=2.
                                if info.item_type == 1 {
                                    weapon = info.shape;
                                    weapon_effect = info.effect as i16;
                                } else if info.item_type == 2 {
                                    armour = info.shape;
                                    wing_effect = info.effect;
                                } else if info.item_type == 19 {
                                    mount_type = info.shape;
                                }
                            }
                        }
                    }

                    (weapon, weapon_effect, armour, wing_effect, light, mount_type)
                };

                let is_hidden = {
                    let world = self.world.lock().unwrap();
                    world.player_hidden(sid)
                };

                let base = SObjectPlayer {
                    object_id: hero_object_id(sid),
                    name: "Hero".to_string(),
                    guild_name: snap.guild_name,
                    guild_rank_name: snap.guild_rank_name,
                    // Use C# HeroObject default name colour: MediumOrchid.
                    name_colour_argb: 0xFFBA55D3u32 as i32,
                    class: snap.class,
                    gender: snap.gender,
                    level: snap.level,
                    location_x: x,
                    location_y: y,
                    direction,
                    hair: snap.hair,
                    light,
                    weapon,
                    weapon_effect,
                    armour,
                    poison: 0,
                    dead: false,
                    hidden: is_hidden,
                    effect: 0,
                    wing_effect,
                    extra: false,
                    mount_type,
                    riding_mount: false,
                    fishing: false,
                    transform_type: 0,
                    element_orb_effect: 0,
                    element_orb_lvl: 0,
                    element_orb_max: 0,
                    buffs: Vec::new(),
                    level_effects: 0,
                };
                let pkt = SObjectHero {
                    base,
                    owner_name: snap.name,
                };
                if is_new {
                    if let Ok(raw) = pkt.encode() {
                        debug!(
                            "vis: send SObjectHero -> session_id={} owner_sid={} map={} pos=({}, {}) hidden={} hero_object_id={} gender={} class={} level={} weapon={} weapon_effect={} armour={} wing_effect={} light={} mount_type={}",
                            self.session_id,
                            sid,
                            map_index,
                            x,
                            y,
                            is_hidden,
                            hero_object_id(sid),
                            snap.gender,
                            snap.class,
                            snap.level,
                            weapon,
                            weapon_effect,
                            armour,
                            wing_effect,
                            light,
                            mount_type,
                        );
                        out.push(Self::encode_raw(raw));
                        next_known_heroes.insert(sid);
                    }
                }
            }
        }

        // Refresh ObjectHealth for all visible heroes so their head HP bars
        // remain visible without recreating the hero object (SObjectHero).
        for sid in &visible_hero_owner_ids {
            let hp_pkt = SObjectHealth {
                object_id: hero_object_id(*sid),
                percent: 100,
                expire: 30,
            };
            if let Ok(raw) = hp_pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
        }

        let removed_heroes: Vec<world::SessionId> = self
            .known_heroes
            .iter()
            .filter(|sid| !visible_hero_owner_ids.contains(sid))
            .cloned()
            .collect();

        for sid in removed_heroes {
            let pkt = SObjectRemove {
                object_id: hero_object_id(sid),
            };
            if let Ok(raw) = pkt.encode() {
                debug!(
                    "vis: send SObjectRemove(hero) -> session_id={} owner_sid={} hero_object_id={}",
                    self.session_id,
                    sid,
                    hero_object_id(sid)
                );
                out.push(Self::encode_raw(raw));
            }
        }

        self.known_heroes = next_known_heroes;
    }
}
