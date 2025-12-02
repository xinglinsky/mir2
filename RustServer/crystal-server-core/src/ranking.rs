use std::collections::HashMap;

use crate::world::Job;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RankType {
    Overall = 0,
    Warrior = 1,
    Wizard = 2,
    Taoist = 3,
    Assassin = 4,
    Archer = 5,
}

impl RankType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(RankType::Overall),
            1 => Some(RankType::Warrior),
            2 => Some(RankType::Wizard),
            3 => Some(RankType::Taoist),
            4 => Some(RankType::Assassin),
            5 => Some(RankType::Archer),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn for_job(job: Job) -> Self {
        match job {
            Job::Warrior => RankType::Warrior,
            Job::Wizard => RankType::Wizard,
            Job::Taoist => RankType::Taoist,
            Job::Assassin => RankType::Assassin,
            Job::Archer => RankType::Archer,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RankCharacterInfo {
    pub player_id: i64,
    pub name: String,
    pub class: Job,
    pub level: u16,
    pub experience: i64,
    pub last_updated_ms: i64,
}

#[derive(Clone, Debug, Default)]
pub struct RankingTables {
    pub rank_top: Vec<RankCharacterInfo>,
    pub rank_by_class: HashMap<Job, Vec<RankCharacterInfo>>,
}

impl RankingTables {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.rank_top.clear();
        self.rank_by_class.clear();
    }

    fn remove_from_list(list: &mut Vec<RankCharacterInfo>, player_id: i64) {
        if let Some(pos) = list.iter().position(|r| r.player_id == player_id) {
            list.remove(pos);
        }
    }

    fn insert_sorted(list: &mut Vec<RankCharacterInfo>, info: RankCharacterInfo) {
        let pos = list
            .iter()
            .position(|r| r.level < info.level || (r.level == info.level && r.experience < info.experience))
            .unwrap_or(list.len());
        list.insert(pos, info);
    }

    pub fn add_or_update(&mut self, info: RankCharacterInfo) {
        let player_id = info.player_id;
        let class = info.class;

        Self::remove_from_list(&mut self.rank_top, player_id);
        if let Some(list) = self.rank_by_class.get_mut(&class) {
            Self::remove_from_list(list, player_id);
        }

        Self::insert_sorted(&mut self.rank_top, info.clone());
        let class_list = self.rank_by_class.entry(class).or_default();
        Self::insert_sorted(class_list, info);
    }

    pub fn remove_player(&mut self, player_id: i64) {
        if let Some(pos) = self.rank_top.iter().position(|r| r.player_id == player_id) {
            let info = self.rank_top.remove(pos);
            if let Some(list) = self.rank_by_class.get_mut(&info.class) {
                if let Some(p) = list.iter().position(|r| r.player_id == player_id) {
                    list.remove(p);
                }
            }
        }
    }

    pub fn overall(&self) -> &[RankCharacterInfo] {
        self.rank_top.as_slice()
    }

    pub fn rankings_for_class(&self, class: Job) -> Option<&[RankCharacterInfo]> {
        self.rank_by_class.get(&class).map(|v| v.as_slice())
    }

    pub fn rankings_for_type(&self, rank_type: RankType) -> Option<&[RankCharacterInfo]> {
        match rank_type {
            RankType::Overall => Some(self.overall()),
            RankType::Warrior => self.rankings_for_class(Job::Warrior),
            RankType::Wizard => self.rankings_for_class(Job::Wizard),
            RankType::Taoist => self.rankings_for_class(Job::Taoist),
            RankType::Assassin => self.rankings_for_class(Job::Assassin),
            RankType::Archer => self.rankings_for_class(Job::Archer),
        }
    }
}
