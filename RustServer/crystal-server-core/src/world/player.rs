use crate::stats::Stat;
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
    pub magics: Vec<UserMagic>,
    pub stats: PlayerStats,
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
        level: u16,
        experience: i64,
        magics: Vec<UserMagic>,
    ) -> &PlayerState {
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
                p.stats.set_base_from_level(job, level);
                p.stats.recalc_if_dirty_for_job(job);
            })
            .or_insert_with(|| {
                let mut stats = PlayerStats::default();
                stats.set_base_from_level(job, level);
                stats.recalc_if_dirty_for_job(job);

                PlayerState {
                    session_id,
                    map_index,
                    x,
                    y,
                    direction,
                    level,
                    experience,
                    job,
                    magics,
                    stats,
                }
            });

        self.players.get(&session_id).unwrap()
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
}

