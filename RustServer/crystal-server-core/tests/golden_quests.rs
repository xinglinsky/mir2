use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;

use crystal_server_core::quest_text;

#[derive(Debug, Deserialize)]
struct QuestFixture {
    quest_index: i32,
    quest_file: String,

    description: Vec<String>,
    task_description: Vec<String>,
    return_description: Vec<String>,
    completion_description: Vec<String>,

    carry_items: Vec<QuestItemTaskNode>,
    kill_tasks: Vec<QuestKillTaskNode>,
    item_tasks: Vec<QuestItemTaskNode>,
    flag_tasks: Vec<QuestFlagTaskNode>,
    fixed_rewards: Vec<QuestItemRewardNode>,
    select_rewards: Vec<QuestItemRewardNode>,

    gold_reward: u32,
    exp_reward: u32,
    credit_reward: u32,
}

#[derive(Debug, Deserialize)]
struct QuestKillTaskNode {
    monster_index: Option<i32>,
    monster_name: Option<String>,
    count: i32,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QuestItemTaskNode {
    item_index: Option<i32>,
    item_name: Option<String>,
    count: u16,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QuestFlagTaskNode {
    number: i32,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QuestItemRewardNode {
    item_index: Option<i32>,
    item_name: Option<String>,
    count: u16,
}

#[test]
fn golden_quests() {
    fn run_one_fixture(fixture_path: &Path) {
        let raw = fs::read_to_string(fixture_path).unwrap_or_else(|_| {
            panic!(
                "missing fixture {}; generate it with Tools.GoldenFixtures (quest)",
                fixture_path.display()
            )
        });
        let fixture: QuestFixture = serde_json::from_str(&raw).expect("invalid fixture json");

        let mut item_name_to_index: HashMap<String, i32> = HashMap::new();
        let mut monster_name_to_index: HashMap<String, i32> = HashMap::new();

        for t in fixture.carry_items.iter().chain(fixture.item_tasks.iter()) {
            if let (Some(name), Some(idx)) = (t.item_name.as_ref(), t.item_index) {
                item_name_to_index.entry(name.clone()).or_insert(idx);
            }
        }
        for r in fixture
            .fixed_rewards
            .iter()
            .chain(fixture.select_rewards.iter())
        {
            if let (Some(name), Some(idx)) = (r.item_name.as_ref(), r.item_index) {
                item_name_to_index.entry(name.clone()).or_insert(idx);
            }
        }
        for t in &fixture.kill_tasks {
            if let (Some(name), Some(idx)) = (t.monster_name.as_ref(), t.monster_index) {
                monster_name_to_index.entry(name.clone()).or_insert(idx);
            }
        }

        let item_lookup = |name: &str| item_name_to_index.get(name).copied();
        let monster_lookup = |name: &str| monster_name_to_index.get(name).copied();

        let jev_root = std::env::var("JEV_ROOT").expect("set JEV_ROOT to your Jev directory");
        let quest_path = Path::new(&jev_root)
            .join("Envir")
            .join("Quests")
            .join(fixture.quest_file.replace('/', &std::path::MAIN_SEPARATOR.to_string()));

        let translation_path = quest_text::default_translation_csv_path();

        let actual = quest_text::load_quest_text_data(
            &quest_path,
            fixture.quest_index,
            &item_lookup,
            &monster_lookup,
            Some(&translation_path),
        )
        .expect("failed to parse quest");

        let expected = quest_text::QuestTextData {
            description: fixture.description,
            task_description: fixture.task_description,
            return_description: fixture.return_description,
            completion_description: fixture.completion_description,
            carry_items: fixture
                .carry_items
                .into_iter()
                .filter_map(|t| {
                    Some(quest_text::QuestItemTask {
                        item_index: t.item_index?,
                        count: t.count,
                        message: t.message.unwrap_or_default(),
                    })
                })
                .collect(),
            kill_tasks: fixture
                .kill_tasks
                .into_iter()
                .filter_map(|t| {
                    Some(quest_text::QuestKillTask {
                        monster_index: t.monster_index?,
                        count: t.count,
                        message: t.message.unwrap_or_default(),
                    })
                })
                .collect(),
            item_tasks: fixture
                .item_tasks
                .into_iter()
                .filter_map(|t| {
                    Some(quest_text::QuestItemTask {
                        item_index: t.item_index?,
                        count: t.count,
                        message: t.message.unwrap_or_default(),
                    })
                })
                .collect(),
            flag_tasks: fixture
                .flag_tasks
                .into_iter()
                .map(|t| quest_text::QuestFlagTask {
                    number: t.number,
                    message: t.message.unwrap_or_default(),
                })
                .collect(),
            fixed_rewards: fixture
                .fixed_rewards
                .into_iter()
                .filter_map(|r| {
                    Some(quest_text::QuestItemReward {
                        item_index: r.item_index?,
                        count: r.count,
                    })
                })
                .collect(),
            select_rewards: fixture
                .select_rewards
                .into_iter()
                .filter_map(|r| {
                    Some(quest_text::QuestItemReward {
                        item_index: r.item_index?,
                        count: r.count,
                    })
                })
                .collect(),
            exp_reward: fixture.exp_reward,
            gold_reward: fixture.gold_reward,
            credit_reward: fixture.credit_reward,
        };

        assert_eq!(actual, expected, "quest mismatch for {}", fixture_path.display());
    }

    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");

    let mut files: Vec<PathBuf> = fs::read_dir(&fixtures_dir)
        .expect("failed to read fixtures dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| {
                    let l = s.to_ascii_lowercase();
                    l.starts_with("quest_") && l.ends_with(".json")
                })
                .unwrap_or(false)
        })
        .collect();

    files.sort_by(|a, b| {
        a.to_string_lossy()
            .to_ascii_lowercase()
            .cmp(&b.to_string_lossy().to_ascii_lowercase())
    });

    assert!(!files.is_empty(), "no quest_*.json fixtures found");

    for path in files {
        run_one_fixture(&path);
    }
}
