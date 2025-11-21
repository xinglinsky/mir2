use crate::item::{Equipment, Inventory};
use crate::stats::Stat;
use crate::stats_util::aggregate_equipment_stats;
use crate::world::magic::UserMagic;
use crate::world::provider::WorldProvider;

use super::{Job, PlayerStats, SessionId, World};

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub session_id: SessionId,
    pub map_index: i32,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub level: u16,
    pub experience: i64,
    pub job: Job,
    pub gender: u8,
    pub magics: Vec<UserMagic>,
    pub stats: PlayerStats,
    pub inventory: Inventory,
    pub equipment: Equipment,
}

impl<P: WorldProvider> World<P> {
    pub(super) fn upsert_player(
        &mut self,
        session_id: SessionId,
        map_index: i32,
        x: i32,
        y: i32,
        direction: u8,
        job: Job,
        gender: u8,
        level: u16,
        experience: i64,
        magics: Vec<UserMagic>,
    ) -> &PlayerState {
        let is_new = !self.players.contains_key(&session_id);

        let start_inventory = if is_new {
            Some(self.build_start_inventory(job, gender))
        } else {
            None
        };

        self.players
            .entry(session_id)
            .and_modify(|p| {
                p.map_index = map_index;
                p.x = x;
                p.y = y;
                p.direction = direction;
                p.level = level;
                p.experience = experience;
                p.job = job;
                p.gender = gender;
                p.stats.set_base_from_level(job, level);
                p.stats.recalc_if_dirty_for_job(job);
            })
            .or_insert_with(|| {
                let mut stats = PlayerStats::default();
                stats.set_base_from_level(job, level);
                stats.recalc_if_dirty_for_job(job);

                let inventory = start_inventory.unwrap_or_else(Inventory::new_default);
                let equipment = Equipment::new_default();

                PlayerState {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                    level,
                    experience,
                    job,
                    gender,
                    magics,
                    stats,
                    inventory,
                    equipment,
                }
            });

        self.players.get(&session_id).unwrap()
    }

    pub fn set_player_level_and_experience(
        &mut self,
        session_id: SessionId,
        level: u16,
        experience: i64,
    ) -> Option<()> {
        let player = self.players.get_mut(&session_id)?;
        player.level = level;
        player.experience = experience;
        let job = player.job;
        player.stats.set_base_from_level(job, level);
        player.stats.recalc_if_dirty_for_job(job);
        Some(())
    }

    /// Learn a new magic for the given player session. If the magic is already
    /// present, this is a no-op and returns None.
    pub fn learn_magic_for_player(&mut self, session_id: SessionId, spell: u8) -> Option<UserMagic> {
        let player = self.players.get_mut(&session_id)?;
        if player.magics.iter().any(|m| m.spell == spell) {
            return None;
        }

        let magic = UserMagic::new(spell);
        player.magics.push(magic.clone());
        Some(magic)
    }

    /// Update the level and experience for an existing magic on the given
    /// player. Returns the updated magic if found.
    pub fn set_magic_level_for_player(
        &mut self,
        session_id: SessionId,
        spell: u8,
        level: u8,
        experience: u16,
    ) -> Option<UserMagic> {
        let player = self.players.get_mut(&session_id)?;
        let magic = player.magics.iter_mut().find(|m| m.spell == spell)?;
        magic.level = level;
        magic.experience = experience;
        Some(magic.clone())
    }

    /// Get a cloned list of learned magics for the given player session.
    pub fn player_magics(&self, session_id: SessionId) -> Vec<UserMagic> {
        self.players
            .get(&session_id)
            .map(|p| p.magics.clone())
            .unwrap_or_default()
    }

    pub fn recalc_player_equipment_stats(&mut self, session_id: SessionId) {
        let (job, equip_stats) = if let Some(player) = self.players.get(&session_id) {
            let mut equipped = Vec::new();
            for slot in &player.equipment.slots {
                if let Some(user_item) = slot {
                    if let Some(info) = self.provider.get_item_info(user_item.item_index) {
                        equipped.push((info.clone(), user_item.clone()));
                    }
                }
            }

            let stats = aggregate_equipment_stats(&equipped);
            (player.job, stats)
        } else {
            return;
        };

        if let Some(player) = self.players.get_mut(&session_id) {
            player.stats.set_equip_stats(&equip_stats);
            player.stats.recalc_if_dirty_for_job(job);
        }
    }

    /// Query players on a given map within a rectangular view range around the
    /// provided centre, returning (session_id, x, y, direction) tuples. This
    /// mirrors the visibility logic currently used by the connection layer.
    pub fn players_in_view_for_map(
        &self,
        map_index: i32,
        centre_x: i32,
        centre_y: i32,
        range: i32,
        exclude_session: Option<SessionId>,
    ) -> Vec<(SessionId, i32, i32, u8)> {
        self.players
            .iter()
            .filter_map(|(&sid, p)| {
                if let Some(ex) = exclude_session {
                    if sid == ex {
                        return None;
                    }
                }

                if p.map_index != map_index {
                    return None;
                }

                if (p.x - centre_x).abs() > range || (p.y - centre_y).abs() > range {
                    return None;
                }

                Some((sid, p.x, p.y, p.direction))
            })
            .collect()
    }

    /// Query session IDs on a given map within a rectangular view range around
    /// the provided centre. This is used by the connection layer to decide
    /// which clients should receive broadcast events such as attacks.
    pub fn sessions_in_range_for_map(
        &self,
        map_index: i32,
        centre_x: i32,
        centre_y: i32,
        range: i32,
    ) -> Vec<SessionId> {
        self.players
            .iter()
            .filter_map(|(&sid, p)| {
                if p.map_index != map_index {
                    return None;
                }

                if (p.x - centre_x).abs() > range || (p.y - centre_y).abs() > range {
                    return None;
                }

                Some(sid)
            })
            .collect()
    }

    pub fn player_max_hp_mp(&self, session_id: SessionId) -> Option<(i32, i32)> {
        self.players.get(&session_id).map(|p| {
            let hp = p.stats.total.get(Stat::HP).max(0);
            let mp = p.stats.total.get(Stat::MP).max(0);
            (hp, mp)
        })
    }

    pub fn player_items(&self, session_id: SessionId) -> Option<(Inventory, Equipment)> {
        self.players
            .get(&session_id)
            .map(|p| (p.inventory.clone(), p.equipment.clone()))
    }

    fn build_start_inventory(&self, job: Job, gender: u8) -> Inventory {
        let mut inventory = Inventory::new_default();

        let class_bit = match job {
            Job::Warrior => 1,
            Job::Wizard => 2,
            Job::Taoist => 4,
            Job::Assassin => 8,
            Job::Archer => 16,
        };

        let gender_bit = match gender {
            0 => 1,
            1 => 2,
            _ => 1 | 2,
        };

        println!(
            "[world] build_start_inventory: job={} gender={} class_bit={} gender_bit={}",
            job.as_u8(),
            gender,
            class_bit,
            gender_bit
        );

        let mut counter: u64 = 0;

        for info in self.provider.item_infos() {
            let matches_class = (info.required_class & class_bit) != 0;
            let matches_gender = (info.required_gender & gender_bit) != 0;

            if info.start_item {
                println!(
                    "[world]  candidate idx={} name={} start_item={} req_class={} req_gender={} matches_class={} matches_gender={}",
                    info.index,
                    info.name,
                    info.start_item,
                    info.required_class,
                    info.required_gender,
                    matches_class,
                    matches_gender
                );
            }

            if !info.start_item {
                continue;
            }

            // Mirror C# HumanObject.CorrectStartItem: RequiredClass *must* include
            // the current class bit; items with RequiredClass == 0 are never
            // valid start items for any class.
            if !matches_class {
                continue;
            }

            // Same for RequiredGender: it must explicitly include the
            // character's gender flag; items with RequiredGender == 0 are not
            // valid start items for any gender.
            if !matches_gender {
                continue;
            }

            if let Some(slot) = inventory
                .slots
                .iter_mut()
                .skip(6)
                .find(|s| s.is_none())
            {
                counter = counter.saturating_add(1);
                let unique_id = ((job.as_u8() as u64) << 56)
                    | ((gender as u64) << 48)
                    | counter;
                let item = crate::item::create_fresh_user_item(info, unique_id, 1);
                *slot = Some(item);
            } else {
                break;
            }
        }

        for (i, slot) in inventory.slots.iter().enumerate() {
            if let Some(item) = slot {
                println!(
                    "[world]  result slot={} item_index={} for job={} gender={}",
                    i,
                    item.item_index,
                    job.as_u8(),
                    gender
                );
            }
        }

        inventory
    }
}

