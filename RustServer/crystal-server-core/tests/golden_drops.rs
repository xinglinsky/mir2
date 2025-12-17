use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crystal_server_core::world::drop;

#[derive(Debug, Deserialize)]
struct DropsFixture {
    drop_path: String,
    drop_type: u8,
    drops: Vec<DropNode>,
}

#[derive(Debug, Deserialize)]
struct DropNode {
    chance: i32,
    gold: u32,
    item_index: Option<i32>,
    item_name: Option<String>,
    item_type: Option<i32>,
    quest_required: bool,
    drop_type: u8,
    group: Option<GroupNode>,
}

#[derive(Debug, Deserialize)]
struct GroupNode {
    random: bool,
    first: bool,
    drops: Vec<DropNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ComparableDrop {
    chance: i32,
    gold: u32,
    item_index: Option<i32>,
    item_type: Option<i32>,
    quest_required: bool,
    drop_type: u8,
    group: Option<ComparableGroup>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ComparableGroup {
    random: bool,
    first: bool,
    drops: Vec<ComparableDrop>,
}

fn comparable_key(d: &ComparableDrop) -> (i32, i32, i32, i32, u32, u8, String) {
    // Mimic C# sorting intent (Gold last, otherwise grouped by item type), but make it deterministic.
    let gold_flag = if d.gold > 0 { 1 } else { 0 };
    let item_type = d.item_type.unwrap_or(-1);
    let item_index = d.item_index.unwrap_or(-1);
    let chance = d.chance;
    let gold = d.gold;
    let drop_type = d.drop_type;

    // Include group structure to stabilize ordering for group-only markers.
    let group_sig = if let Some(g) = &d.group {
        format!("g:{}:{}:{}", g.random, g.first, g.drops.len())
    } else {
        String::new()
    };

    (gold_flag, item_type, item_index, chance, gold, drop_type, group_sig)
}

fn flatten_fixture_drop(node: &DropNode) -> ComparableDrop {
    ComparableDrop {
        chance: node.chance,
        gold: node.gold,
        item_index: node.item_index,
        item_type: node.item_type,
        quest_required: node.quest_required,
        drop_type: node.drop_type,
        group: node.group.as_ref().map(|g| ComparableGroup {
            random: g.random,
            first: g.first,
            drops: g.drops.iter().map(flatten_fixture_drop).collect(),
        }),
    }
}

fn flatten_actual_drop(node: &drop::DropInfo, item_type_by_index: &std::collections::HashMap<i32, i32>) -> ComparableDrop {
    ComparableDrop {
        chance: node.chance,
        gold: node.gold,
        item_index: node.item_index,
        item_type: node
            .item_index
            .and_then(|idx| item_type_by_index.get(&idx).copied()),
        quest_required: node.quest_required,
        drop_type: node.drop_type,
        group: node.grouped_drop.as_ref().map(|g| ComparableGroup {
            random: g.random,
            first: g.first,
            drops: g.drops.iter().map(|c| flatten_actual_drop(c, item_type_by_index)).collect(),
        }),
    }
}

fn expected_fixture_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel.replace('/', &std::path::MAIN_SEPARATOR.to_string()))
}

#[test]
fn golden_drop_00fishing() {
    // This test expects you to generate fixtures via:
    // dotnet run --project ..\..\..\..\Tools.GoldenFixtures\Tools.GoldenFixtures.csproj -- \
    //   drops --root <JevRoot> --drop 00Fishing.txt --out <this file's dir>/fixtures/drops_00fishing.json
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("drops_00fishing.json");

    let raw = fs::read_to_string(&fixture_path)
        .expect("missing fixture; generate it with Tools.GoldenFixtures (drops)");
    let fixture: DropsFixture = serde_json::from_str(&raw).expect("invalid fixture json");

    // Read the original drop file from Jev root via env var.
    let jev_root = std::env::var("JEV_ROOT").expect("set JEV_ROOT to your Jev directory");
    let drop_file = expected_fixture_path(Path::new(&jev_root), &format!("Envir/Drops/{}", fixture.drop_path));

    // Build name->index and index->type mappings from fixture (resolved by C#).
    let mut name_to_index: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    let mut item_type_by_index: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();

    fn collect_maps(
        node: &DropNode,
        name_to_index: &mut std::collections::HashMap<String, i32>,
        item_type_by_index: &mut std::collections::HashMap<i32, i32>,
    ) {
        if let (Some(name), Some(idx)) = (node.item_name.as_ref(), node.item_index) {
            name_to_index.entry(name.clone()).or_insert(idx);
            if let Some(t) = node.item_type {
                item_type_by_index.entry(idx).or_insert(t);
            }
        }
        if let Some(g) = node.group.as_ref() {
            for c in &g.drops {
                collect_maps(c, name_to_index, item_type_by_index);
            }
        }
    }

    for d in &fixture.drops {
        collect_maps(d, &mut name_to_index, &mut item_type_by_index);
    }

    let item_lookup = |name: &str| name_to_index.get(name).copied();

    let actual = drop::load_drop_file(&drop_file, fixture.drop_type, &item_lookup)
        .expect("failed to parse drop file");

    let mut expected: Vec<ComparableDrop> = fixture.drops.iter().map(flatten_fixture_drop).collect();
    let mut actual: Vec<ComparableDrop> = actual.iter().map(|d| flatten_actual_drop(d, &item_type_by_index)).collect();

    assert_eq!(actual.len(), expected.len(), "drop count mismatch");

    actual.sort_by(|a, b| comparable_key(a).cmp(&comparable_key(b)));
    expected.sort_by(|a, b| comparable_key(a).cmp(&comparable_key(b)));

    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        assert_eq!(a, e, "drop mismatch at normalized index {i}");
    }
}
