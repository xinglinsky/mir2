use std::collections::HashMap;
use std::io;

use crystal_shared_proto::io::{write_bool, write_i32_le, write_string, write_u16_le, write_u32_le};
use crystal_shared_proto::item_types::{ItemInfoData, UserItemData};

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

impl QuestInfo {
    pub fn encode_client_info_bytes(
        &self,
        item_infos: &[ItemInfoData],
    ) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();

        write_i32_le(&mut buf, self.id.0)?;
        write_u32_le(&mut buf, self.npc_index)?;
        write_string(&mut buf, &self.name)?;
        write_string(&mut buf, &self.group)?;

        let count = self.description.len().min(i32::MAX as usize) as i32;
        write_i32_le(&mut buf, count)?;
        for s in &self.description {
            write_string(&mut buf, s)?;
        }

        let count = self.task_description.len().min(i32::MAX as usize) as i32;
        write_i32_le(&mut buf, count)?;
        for s in &self.task_description {
            write_string(&mut buf, s)?;
        }

        let count = self.return_description.len().min(i32::MAX as usize) as i32;
        write_i32_le(&mut buf, count)?;
        for s in &self.return_description {
            write_string(&mut buf, s)?;
        }

        let count = self
            .completion_description
            .len()
            .min(i32::MAX as usize) as i32;
        write_i32_le(&mut buf, count)?;
        for s in &self.completion_description {
            write_string(&mut buf, s)?;
        }

        write_i32_le(&mut buf, self.required_min_level)?;
        write_i32_le(&mut buf, self.required_max_level)?;
        write_i32_le(&mut buf, self.required_quest)?;
        buf.push(self.required_class.0);
        buf.push(self.quest_type.0);
        write_i32_le(&mut buf, self.time_limit_seconds)?;
        write_u32_le(&mut buf, self.gold_reward)?;
        write_u32_le(&mut buf, self.exp_reward)?;
        write_u32_le(&mut buf, self.credit_reward)?;

        let fixed: Vec<(&ItemInfoData, u16)> = self
            .fixed_rewards
            .iter()
            .filter_map(|r| {
                item_infos
                    .iter()
                    .find(|i| i.index == r.item_index)
                    .map(|info| (info, r.count))
            })
            .collect();
        let count = fixed.len().min(i32::MAX as usize) as i32;
        write_i32_le(&mut buf, count)?;
        for (info, count) in fixed {
            let bytes = info.encode_to_bytes()?;
            buf.extend_from_slice(&bytes);
            write_u16_le(&mut buf, count)?;
        }

        let select: Vec<(&ItemInfoData, u16)> = self
            .select_rewards
            .iter()
            .filter_map(|r| {
                item_infos
                    .iter()
                    .find(|i| i.index == r.item_index)
                    .map(|info| (info, r.count))
            })
            .collect();
        let count = select.len().min(i32::MAX as usize) as i32;
        write_i32_le(&mut buf, count)?;
        for (info, count) in select {
            let bytes = info.encode_to_bytes()?;
            buf.extend_from_slice(&bytes);
            write_u16_le(&mut buf, count)?;
        }

        write_u32_le(&mut buf, self.finish_npc_index)?;

        Ok(buf)
    }
}

pub fn encode_client_progress_bytes(
    id: i32,
    task_list: &[String],
    taken: bool,
    completed: bool,
    is_new: bool,
) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();

    write_i32_le(&mut buf, id)?;

    let count = task_list.len().min(i32::MAX as usize) as i32;
    write_i32_le(&mut buf, count)?;
    for s in task_list {
        write_string(&mut buf, s)?;
    }

    write_bool(&mut buf, taken)?;
    write_bool(&mut buf, completed)?;
    write_bool(&mut buf, is_new)?;

    Ok(buf)
}

#[derive(Clone, Debug)]
pub struct QuestKillTaskProgress {
    pub monster_index: i32,
    pub count: i32,
    pub required_count: i32,
}

impl QuestKillTaskProgress {
    pub fn is_complete(&self) -> bool {
        self.count >= self.required_count
    }
}

#[derive(Clone, Debug)]
pub struct QuestItemTaskProgress {
    pub item_index: i32,
    pub count: i32,
    pub required_count: i32,
}

impl QuestItemTaskProgress {
    pub fn is_complete(&self) -> bool {
        self.count >= self.required_count
    }
}

#[derive(Clone, Debug)]
pub struct QuestFlagTaskProgress {
    pub number: i32,
    pub state: bool,
}

impl QuestFlagTaskProgress {
    pub fn is_complete(&self) -> bool {
        self.state
    }
}

#[derive(Clone, Debug)]
pub struct QuestProgress {
    pub quest_id: QuestId,
    pub start_time_ms: Option<i64>,
    pub end_time_ms: Option<i64>,
    pub kill_tasks: Vec<QuestKillTaskProgress>,
    pub item_tasks: Vec<QuestItemTaskProgress>,
    pub flag_tasks: Vec<QuestFlagTaskProgress>,
    pub task_list: Vec<String>,
}

impl QuestProgress {
    pub fn new_from_quest(info: &QuestInfo) -> Self {
        let kill_tasks = info
            .kill_tasks
            .iter()
            .map(|t| QuestKillTaskProgress {
                monster_index: t.monster_index,
                count: 0,
                required_count: t.count,
            })
            .collect();

        let item_tasks = info
            .item_tasks
            .iter()
            .map(|t| QuestItemTaskProgress {
                item_index: t.item_index,
                count: 0,
                required_count: i32::from(t.count),
            })
            .collect();

        let flag_tasks = info
            .flag_tasks
            .iter()
            .map(|t| QuestFlagTaskProgress {
                number: t.number,
                state: false,
            })
            .collect();

        QuestProgress {
            quest_id: info.id,
            start_time_ms: None,
            end_time_ms: None,
            kill_tasks,
            item_tasks,
            flag_tasks,
            task_list: Vec::new(),
        }
    }

    pub fn taken(&self) -> bool {
        self.start_time_ms.is_some()
    }

    pub fn completed(&self) -> bool {
        self.end_time_ms.is_some()
    }

    pub fn is_new(&self, now_ms: i64) -> bool {
        match self.start_time_ms {
            Some(start) => now_ms.saturating_sub(start) <= 24 * 60 * 60 * 1000,
            None => false,
        }
    }

    pub fn check_completed(
        &mut self,
        info: &QuestInfo,
        item_infos: &[ItemInfoData],
        now_ms: i64,
    ) -> bool {
        self.update_tasks(info, item_infos);

        let mut can_complete = true;

        for t in &self.kill_tasks {
            if !t.is_complete() {
                can_complete = false;
            }
        }

        for t in &self.item_tasks {
            if !t.is_complete() {
                can_complete = false;
            }
        }

        for t in &self.flag_tasks {
            if !t.is_complete() {
                can_complete = false;
            }
        }

        if !can_complete {
            return false;
        }

        if !self.completed() {
            self.end_time_ms = Some(now_ms);
        }

        self.update_tasks(info, item_infos);
        true
    }

    pub fn process_kill(&mut self, monster_index: i32) {
        for task in &mut self.kill_tasks {
            if task.monster_index == monster_index {
                if task.count < task.required_count {
                    task.count = task.count.saturating_add(1);
                }
                break;
            }
        }
    }

    pub fn process_item(&mut self, inventory: &[Option<UserItemData>]) {
        for task in &mut self.item_tasks {
            let mut total: i32 = 0;
            for slot in inventory {
                if let Some(item) = slot {
                    if item.item_index == task.item_index {
                        total = total.saturating_add(i32::from(item.count));
                    }
                }
            }
            task.count = total;
        }
    }

    pub fn process_flag(&mut self, flags: &[bool]) {
        let limit = flags.len().saturating_sub(1000);
        for task in &mut self.flag_tasks {
            let number = task.number;
            if number < 0 {
                continue;
            }
            let idx = number as usize;
            if idx < limit && flags[idx] {
                task.state = true;
            }
        }
    }

    pub fn need_item(&self, item_index: i32) -> bool {
        self.item_tasks
            .iter()
            .any(|t| t.item_index == item_index && !t.is_complete())
    }

    pub fn need_kill(&self, monster_index: i32) -> bool {
        self.kill_tasks
            .iter()
            .any(|t| t.monster_index == monster_index && !t.is_complete())
    }

    pub fn need_flag(&self, flag_number: i32) -> bool {
        self.flag_tasks
            .iter()
            .any(|t| t.number == flag_number && !t.is_complete())
    }

    pub fn update_tasks(&mut self, info: &QuestInfo, item_infos: &[ItemInfoData]) {
        self.task_list.clear();
        self.update_kill_tasks(info);
        self.update_item_tasks(info, item_infos);
        self.update_flag_tasks(info);
        self.update_goto_task(info);
    }

    fn update_kill_tasks(&mut self, info: &QuestInfo) {
        if !info.kill_message.is_empty() && !self.kill_tasks.is_empty() {
            let all_complete = self.kill_tasks.iter().all(|t| t.is_complete());
            let suffix = if all_complete { " (Completed)" } else { "" };
            let line = format!("{}{}", info.kill_message, suffix);
            self.task_list.push(line);
            return;
        }

        for (i, task) in self.kill_tasks.iter().enumerate() {
            let completed_suffix = if task.is_complete() { " (Completed)" } else { "" };
            let line = if let Some(def) = info.kill_tasks.get(i) {
                if def.message.is_empty() {
                    format!(
                        "Kill {}: {}/{}{}",
                        task.monster_index,
                        task.count,
                        task.required_count,
                        completed_suffix,
                    )
                } else {
                    format!("{}{}", def.message, completed_suffix)
                }
            } else {
                format!(
                    "Kill {}: {}/{}{}",
                    task.monster_index,
                    task.count,
                    task.required_count,
                    completed_suffix,
                )
            };
            self.task_list.push(line);
        }
    }

    fn update_item_tasks(&mut self, info: &QuestInfo, item_infos: &[ItemInfoData]) {
        if !info.item_message.is_empty() && !self.item_tasks.is_empty() {
            let all_complete = self.item_tasks.iter().all(|t| t.is_complete());
            let suffix = if all_complete { " (Completed)" } else { "" };
            let line = format!("{}{}", info.item_message, suffix);
            self.task_list.push(line);
            return;
        }

        for (i, task) in self.item_tasks.iter().enumerate() {
            let completed_suffix = if task.is_complete() { " (Completed)" } else { "" };
            let line = if let Some(def) = info.item_tasks.get(i) {
                if def.message.is_empty() {
                    let name = item_infos
                        .iter()
                        .find(|it| it.index == def.item_index)
                        .map(|it| it.friendly_name())
                        .unwrap_or_else(|| format!("Item{}", def.item_index));

                    format!(
                        "Collect {}: {}/{}{}",
                        name,
                        task.count,
                        i32::from(def.count),
                        completed_suffix,
                    )
                } else {
                    format!("{}{}", def.message, completed_suffix)
                }
            } else {
                format!(
                    "Collect Item{}: {}/{}{}",
                    task.item_index,
                    task.count,
                    task.required_count,
                    completed_suffix,
                )
            };
            self.task_list.push(line);
        }
    }

    fn update_flag_tasks(&mut self, info: &QuestInfo) {
        if !info.flag_message.is_empty() && !self.flag_tasks.is_empty() {
            let all_complete = self.flag_tasks.iter().all(|t| t.is_complete());
            let suffix = if all_complete { " (Completed)" } else { "" };
            let line = format!("{}{}", info.flag_message, suffix);
            self.task_list.push(line);
            return;
        }

        for (i, task) in self.flag_tasks.iter().enumerate() {
            let completed_suffix = if task.is_complete() { " (Completed)" } else { "" };
            let line = if let Some(def) = info.flag_tasks.get(i) {
                if def.message.is_empty() {
                    format!(
                        "Activate Flag {}{}",
                        def.number,
                        completed_suffix,
                    )
                } else {
                    format!("{}{}", def.message, completed_suffix)
                }
            } else {
                format!("Activate Flag {}{}", task.number, completed_suffix)
            };
            self.task_list.push(line);
        }
    }

    fn update_goto_task(&mut self, info: &QuestInfo) {
        if info.goto_message.is_empty() {
            return;
        }
        if !self.completed() {
            return;
        }
        self.task_list.push(info.goto_message.clone());
    }

    pub fn to_client_progress_bytes(&self, now_ms: i64) -> io::Result<Vec<u8>> {
        let taken = self.taken();
        let completed = self.completed();
        let is_new = self.is_new(now_ms);
        encode_client_progress_bytes(self.quest_id.0, &self.task_list, taken, completed, is_new)
    }
}
