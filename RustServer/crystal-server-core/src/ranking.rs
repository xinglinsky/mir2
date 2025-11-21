use std::collections::HashMap;

use crate::world::Job;

#[derive(Clone, Debug)]
pub struct RankCharacterInfo {
    pub player_id: i32,
    pub name: String,
    pub class: Job,
    pub level: u16,
    pub experience: i64,
    pub last_updated_ms: i64,
}

#[derive(Default)]
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

    fn remove_from_list(list: &mut Vec<RankCharacterInfo>, player_id: i32) {
        if let Some(pos) = list.iter().position(|r| r.player_id == player_id) {
            list.remove(pos);
        }
    }

    pub fn add_or_update(&mut self, info: RankCharacterInfo) {
        Self::remove_from_list(&mut self.rank_top, info.player_id);
        if let Some(list) = self.rank_by_class.get_mut(&info.class) {
            Self::remove_from_list(list, info.player_id);
        }
        self.rank_top.push(info.clone());
        self.rank_by_class
            .entry(info.class)
            .or_default()
            .push(info);
    }

    pub fn remove_player(&mut self, player_id: i32) {
        if let Some(pos) = self.rank_top.iter().position(|r| r.player_id == player_id) {
            let info = self.rank_top.remove(pos);
            if let Some(list) = self.rank_by_class.get_mut(&info.class) {
                if let Some(p) = list.iter().position(|r| r.player_id == player_id) {
                    list.remove(p);
                }
            }
        }
    }

    pub fn rankings_for_class(&self, class: Job) -> Option<&[RankCharacterInfo]> {
        self.rank_by_class.get(&class).map(|v| v.as_slice())
    }
}
