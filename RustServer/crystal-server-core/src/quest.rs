use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct QuestId(pub i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuestType(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequiredClass(pub u8);

#[derive(Clone, Debug)]
pub struct QuestKillTask {
    pub monster_index: i32,
    pub count: i32,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct QuestItemTask {
    pub item_index: i32,
    pub count: u16,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct QuestFlagTask {
    pub number: i32,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct QuestItemReward {
    pub item_index: i32,
    pub count: u16,
}

#[derive(Clone, Debug)]
pub struct QuestInfo {
    pub id: QuestId,
    pub npc_index: u32,
    pub finish_npc_index: u32,
    pub name: String,
    pub group: String,
    pub file_name: String,
    pub goto_message: String,
    pub kill_message: String,
    pub item_message: String,
    pub flag_message: String,
    pub description: Vec<String>,
    pub task_description: Vec<String>,
    pub return_description: Vec<String>,
    pub completion_description: Vec<String>,
    pub required_min_level: i32,
    pub required_max_level: i32,
    pub required_quest: i32,
    pub required_class: RequiredClass,
    pub quest_type: QuestType,
    pub time_limit_seconds: i32,
    pub carry_items: Vec<QuestItemTask>,
    pub kill_tasks: Vec<QuestKillTask>,
    pub item_tasks: Vec<QuestItemTask>,
    pub flag_tasks: Vec<QuestFlagTask>,
    pub fixed_rewards: Vec<QuestItemReward>,
    pub select_rewards: Vec<QuestItemReward>,
    pub gold_reward: u32,
    pub exp_reward: u32,
    pub credit_reward: u32,
}

#[derive(Default)]
pub struct QuestManager {
    quests: HashMap<QuestId, QuestInfo>,
}

impl QuestManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn quests(&self) -> impl Iterator<Item = &QuestInfo> {
        self.quests.values()
    }

    pub fn get(&self, id: QuestId) -> Option<&QuestInfo> {
        self.quests.get(&id)
    }

    pub fn get_mut(&mut self, id: QuestId) -> Option<&mut QuestInfo> {
        self.quests.get_mut(&id)
    }

    pub fn insert(&mut self, info: QuestInfo) {
        self.quests.insert(info.id, info);
    }

    pub fn remove(&mut self, id: QuestId) -> Option<QuestInfo> {
        self.quests.remove(&id)
    }
}
