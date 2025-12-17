use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crystal_server_core::world::map;
use crystal_server_core::world::recipe;

#[derive(Debug, Deserialize)]
struct RecipeFixture {
    recipe_file: String,
    product: RecipeItemNode,
    chance: u8,
    gold: u32,
    tools: Vec<RecipeItemNode>,
    ingredients: Vec<RecipeItemNode>,
    required_flag: Vec<i32>,
    required_level: Option<u16>,
    required_quest: Vec<i32>,
    required_class: Vec<i32>,
    required_gender: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct RecipeItemNode {
    item_index: i32,
    item_name: Option<String>,
    count: u16,
    current_dura: u16,
    max_dura: u16,
}

fn expected_jev_path(jev_root: &Path, rel: &str) -> PathBuf {
    jev_root.join(rel.replace('/', &std::path::MAIN_SEPARATOR.to_string()))
}

#[test]
fn golden_recipe_bonebroth() {
    let jev_root = PathBuf::from(std::env::var("JEV_ROOT").expect("set JEV_ROOT to your Jev directory"));

    let mirdb_path = expected_jev_path(&jev_root, "Server.MirDB");
    let item_infos = map::load_item_infos_from_mirdb(&mirdb_path).expect("failed to load items from Server.MirDB");

    // Use the same name resolution as recipe parser (case-insensitive ItemInfoData.name).
    let name_to_index: HashMap<String, i32> = item_infos
        .iter()
        .map(|i| (i.name.to_ascii_lowercase(), i.index))
        .collect();

    let recipes = recipe::load_recipes_from_dir(jev_root.join("Envir").join("Recipe"), &item_infos)
        .expect("failed to load recipes");

    let parsed_by_product: HashMap<i32, recipe::RecipeInfo> = recipes
        .into_iter()
        .map(|r| (r.item_index, r))
        .collect();

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
                .map(|s| s.to_ascii_lowercase().starts_with("recipe_") && s.to_ascii_lowercase().ends_with(".json"))
                .unwrap_or(false)
        })
        .collect();

    files.sort_by(|a, b| a.to_string_lossy().to_ascii_lowercase().cmp(&b.to_string_lossy().to_ascii_lowercase()));

    assert!(!files.is_empty(), "no recipe_*.json fixtures found");

    for fixture_path in files {
        let raw = fs::read_to_string(&fixture_path)
            .unwrap_or_else(|_| panic!("missing fixture {}; generate it with Tools.GoldenFixtures (recipe)", fixture_path.display()));
        let fixture: RecipeFixture = serde_json::from_str(&raw).expect("invalid fixture json");

        let actual = parsed_by_product
            .get(&fixture.product.item_index)
            .unwrap_or_else(|| panic!("recipe not found for product item_index {} (fixture {})", fixture.product.item_index, fixture_path.display()));

        let expected = recipe::RecipeInfo {
            item_index: fixture.product.item_index,
            amount: fixture.product.count,
            chance: fixture.chance,
            gold: fixture.gold,
            tools: fixture
                .tools
                .into_iter()
                .map(|t| recipe::RecipeItemRequirement {
                    item_index: t.item_index,
                    count: 1,
                    current_dura: None,
                })
                .collect(),
            ingredients: fixture
                .ingredients
                .into_iter()
                .map(|t| recipe::RecipeItemRequirement {
                    item_index: t.item_index,
                    count: t.count,
                    current_dura: if t.current_dura == 0 { None } else { Some(t.current_dura) },
                })
                .collect(),
            required_flag: fixture.required_flag,
            required_level: fixture.required_level,
            required_quest: fixture.required_quest,
            required_class: fixture.required_class,
            required_gender: fixture.required_gender,
        };

        assert_eq!(actual.item_index, expected.item_index);
        assert_eq!(actual.amount, expected.amount);
        assert_eq!(actual.chance, expected.chance);
        assert_eq!(actual.gold, expected.gold);
        assert_eq!(actual.required_level, expected.required_level);
        assert_eq!(actual.required_gender, expected.required_gender);
        assert_eq!(actual.required_flag, expected.required_flag);
        assert_eq!(actual.required_quest, expected.required_quest);

        let mut at = actual.tools.clone();
        let mut et = expected.tools.clone();
        at.sort_by_key(|x| x.item_index);
        et.sort_by_key(|x| x.item_index);
        assert_eq!(at, et, "tools mismatch for {}", fixture_path.display());

        let mut ai = actual.ingredients.clone();
        let mut ei = expected.ingredients.clone();
        ai.sort_by_key(|x| (x.item_index, x.count, x.current_dura.unwrap_or(0)));
        ei.sort_by_key(|x| (x.item_index, x.count, x.current_dura.unwrap_or(0)));
        assert_eq!(ai, ei, "ingredients mismatch for {}", fixture_path.display());
    }
}
