use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use crate::world::content;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestKillTask {
    pub monster_index: i32,
    pub count: i32,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestItemTask {
    pub item_index: i32,
    pub count: u16,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestFlagTask {
    pub number: i32,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestItemReward {
    pub item_index: i32,
    pub count: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct QuestTextData {
    pub description: Vec<String>,
    pub task_description: Vec<String>,
    pub return_description: Vec<String>,
    pub completion_description: Vec<String>,

    pub carry_items: Vec<QuestItemTask>,
    pub kill_tasks: Vec<QuestKillTask>,
    pub item_tasks: Vec<QuestItemTask>,
    pub flag_tasks: Vec<QuestFlagTask>,

    pub fixed_rewards: Vec<QuestItemReward>,
    pub select_rewards: Vec<QuestItemReward>,

    pub exp_reward: u32,
    pub gold_reward: u32,
    pub credit_reward: u32,
}

fn read_lines(path: &Path) -> io::Result<Vec<String>> {
    let s = content::read_to_string(path)?;
    let s = s.replace("\r\n", "\n");
    let s = s.replace('\r', "\n");
    Ok(s.split_terminator('\n').map(|l| l.to_string()).collect())
}

fn extract_first_quoted(line: &str) -> String {
    let start = match line.find('"') {
        Some(i) => i,
        None => return String::new(),
    };

    let rest = &line[(start + 1)..];
    let end = match rest.find('"') {
        Some(i) => i,
        None => return String::new(),
    };

    rest[..end].to_string()
}

pub fn load_translation_map(csv_path: &Path) -> io::Result<HashMap<(i32, String, i32), String>> {
    let mut out = HashMap::new();

    let raw = match content::read_to_string(csv_path) {
        Ok(s) => s,
        Err(_) => return Ok(out),
    };

    for (i, line) in raw.lines().enumerate() {
        if i == 0 {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split(';').collect();
        if parts.len() < 7 {
            continue;
        }

        let quest_index: i32 = match parts[0].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let typ = parts[3].trim().to_string();

        let line_index: i32 = parts[4].trim().parse().unwrap_or(0);

        let chinese = parts[6].to_string();
        if chinese.trim().is_empty() {
            continue;
        }

        out.insert((quest_index, typ, line_index), chinese);
    }

    Ok(out)
}

fn apply_translations(quest_index: i32, typ: &str, list: &mut [String], map: &HashMap<(i32, String, i32), String>) {
    for (i, s) in list.iter_mut().enumerate() {
        if let Some(v) = map.get(&(quest_index, typ.to_string(), i as i32)) {
            if !v.trim().is_empty() {
                *s = v.clone();
            }
        }
    }
}

fn parse_item_task(
    line: &str,
    item_lookup: &impl Fn(&str) -> Option<i32>,
) -> Option<QuestItemTask> {
    if line.len() < 1 {
        return None;
    }

    let split: Vec<&str> = line.split(' ').collect();
    if split.is_empty() {
        return None;
    }

    let mut count: u16 = 1;
    if split.len() > 1 {
        let _ = split[1].parse::<u16>().map(|v| count = v);
    }

    let idx = item_lookup(split[0])?;
    let message = extract_first_quoted(line);

    Some(QuestItemTask {
        item_index: idx,
        count,
        message,
    })
}

fn parse_kill_task(
    line: &str,
    monster_lookup: &impl Fn(&str) -> Option<i32>,
) -> Option<QuestKillTask> {
    if line.len() < 1 {
        return None;
    }

    let split: Vec<&str> = line.split(' ').collect();
    if split.is_empty() {
        return None;
    }

    let mut count: i32 = 1;
    if split.len() > 1 {
        let _ = split[1].parse::<i32>().map(|v| count = v);
    }

    let idx = monster_lookup(split[0])?;
    let message = extract_first_quoted(line);

    Some(QuestKillTask {
        monster_index: idx,
        count,
        message,
    })
}

fn parse_flag_task(line: &str) -> Option<QuestFlagTask> {
    if line.len() < 1 {
        return None;
    }

    let split: Vec<&str> = line.split(' ').collect();
    if split.is_empty() {
        return None;
    }

    let number: i32 = split[0].parse().unwrap_or(-1);

    // C# check: number < 0 || number > Globals.FlagIndexCount - 1000
    // Globals.FlagIndexCount = 1999
    if number < 0 || number > 999 {
        return None;
    }

    let message = extract_first_quoted(line);

    Some(QuestFlagTask { number, message })
}

fn parse_reward_lines(
    out: &mut Vec<QuestItemReward>,
    line: &str,
    item_lookup: &impl Fn(&str) -> Option<i32>,
) {
    if line.len() < 1 {
        return;
    }

    let split: Vec<&str> = line.split(' ').collect();
    if split.is_empty() {
        return;
    }

    let mut count: u16 = 1;
    if split.len() > 1 {
        let _ = split[1].parse::<u16>().map(|v| count = v);
    }

    let name = split[0];

    if let Some(idx) = item_lookup(name) {
        out.push(QuestItemReward { item_index: idx, count });
        return;
    }

    if let Some(idx) = item_lookup(&format!("{}(M)", name)) {
        out.push(QuestItemReward { item_index: idx, count });
    }

    if let Some(idx) = item_lookup(&format!("{}(F)", name)) {
        out.push(QuestItemReward { item_index: idx, count });
    }
}

fn join_rel(base: &Path, rel: &str) -> PathBuf {
    let rel = rel.replace('\\', &std::path::MAIN_SEPARATOR.to_string());
    base.join(rel)
}

pub fn load_quest_text_data(
    quest_file: &Path,
    quest_index: i32,
    item_lookup: &impl Fn(&str) -> Option<i32>,
    monster_lookup: &impl Fn(&str) -> Option<i32>,
    translation_csv_path: Option<&Path>,
) -> io::Result<QuestTextData> {
    let mut data = QuestTextData::default();

    let lines = read_lines(quest_file)?;

    let description_collect_key = "[@DESCRIPTION]";
    let description_task_key = "[@TASKDESCRIPTION]";
    let description_return_key = "[@RETURNDESCRIPTION]";
    let description_completion_key = "[@COMPLETION]";
    let carry_items_key = "[@CARRYITEMS]";
    let kill_tasks_key = "[@KILLTASKS]";
    let item_tasks_key = "[@ITEMTASKS]";
    let flag_tasks_key = "[@FLAGTASKS]";
    let fixed_rewards_key = "[@FIXEDREWARDS]";
    let select_rewards_key = "[@SELECTREWARDS]";
    let exp_reward_key = "[@EXPREWARD]";
    let gold_reward_key = "[@GOLDREWARD]";
    let credit_reward_key = "[@CREDITREWARD]";

    let headers = vec![
        description_collect_key,
        description_task_key,
        description_completion_key,
        carry_items_key,
        kill_tasks_key,
        item_tasks_key,
        flag_tasks_key,
        fixed_rewards_key,
        select_rewards_key,
        exp_reward_key,
        gold_reward_key,
        credit_reward_key,
        description_return_key,
    ];

    for header in headers {
        for i in 0..lines.len() {
            let line_upper = lines[i].to_ascii_uppercase();
            if line_upper != header.to_ascii_uppercase() {
                continue;
            }

            for j in (i + 1)..lines.len() {
                let inner = &lines[j];

                if inner.starts_with('[') || inner.starts_with("//") {
                    break;
                }
                if inner.is_empty() {
                    continue;
                }

                match header {
                    x if x.eq_ignore_ascii_case(description_collect_key) => data.description.push(inner.clone()),
                    x if x.eq_ignore_ascii_case(description_task_key) => data.task_description.push(inner.clone()),
                    x if x.eq_ignore_ascii_case(description_return_key) => data.return_description.push(inner.clone()),
                    x if x.eq_ignore_ascii_case(description_completion_key) => {
                        data.completion_description.push(inner.clone())
                    }
                    x if x.eq_ignore_ascii_case(carry_items_key) => {
                        if let Some(t) = parse_item_task(inner, item_lookup) {
                            data.carry_items.push(t);
                        }
                    }
                    x if x.eq_ignore_ascii_case(kill_tasks_key) => {
                        if let Some(t) = parse_kill_task(inner, monster_lookup) {
                            data.kill_tasks.push(t);
                        }
                    }
                    x if x.eq_ignore_ascii_case(item_tasks_key) => {
                        if let Some(t) = parse_item_task(inner, item_lookup) {
                            data.item_tasks.push(t);
                        }
                    }
                    x if x.eq_ignore_ascii_case(flag_tasks_key) => {
                        if let Some(t) = parse_flag_task(inner) {
                            data.flag_tasks.push(t);
                        }
                    }
                    x if x.eq_ignore_ascii_case(fixed_rewards_key) => {
                        parse_reward_lines(&mut data.fixed_rewards, inner, item_lookup);
                    }
                    x if x.eq_ignore_ascii_case(select_rewards_key) => {
                        parse_reward_lines(&mut data.select_rewards, inner, item_lookup);
                    }
                    x if x.eq_ignore_ascii_case(exp_reward_key) => {
                        let _ = inner.parse::<u32>().map(|v| data.exp_reward = v);
                    }
                    x if x.eq_ignore_ascii_case(gold_reward_key) => {
                        let _ = inner.parse::<u32>().map(|v| data.gold_reward = v);
                    }
                    x if x.eq_ignore_ascii_case(credit_reward_key) => {
                        let _ = inner.parse::<u32>().map(|v| data.credit_reward = v);
                    }
                    _ => {}
                }
            }
        }
    }

    if let Some(csv) = translation_csv_path {
        let map = load_translation_map(csv)?;
        if !map.is_empty() {
            apply_translations(quest_index, "Description", &mut data.description, &map);
            apply_translations(quest_index, "TaskDescription", &mut data.task_description, &map);
            apply_translations(quest_index, "ReturnDescription", &mut data.return_description, &map);
            apply_translations(
                quest_index,
                "CompletionDescription",
                &mut data.completion_description,
                &map,
            );
        }
    }

    Ok(data)
}

pub fn default_translation_csv_path() -> PathBuf {
    // crystal-server-core/ -> RustServer/ -> mir2/
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = base
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(base);
    join_rel(root, "docs/translations/quest_details_translation.csv")
}
