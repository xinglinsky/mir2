use std::fs;
use std::path::Path;

use serde::Deserialize;

use crystal_server_core::world::npc_script;

#[derive(Debug, Deserialize)]
struct NpcExpandFixture {
    npc_file: String,
    expanded_lines: Vec<String>,
}

#[test]
fn golden_npc_expand_00default() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("npc_expand_00default.json");

    let raw = fs::read_to_string(&fixture_path)
        .expect("missing fixture; generate it with Tools.GoldenFixtures (npc-expand)");
    let fixture: NpcExpandFixture = serde_json::from_str(&raw).expect("invalid fixture json");

    let jev_root = std::env::var("JEV_ROOT").expect("set JEV_ROOT to your Jev directory");
    let envir_root = Path::new(&jev_root).join("Envir");
    let npc_path = envir_root.join("NPCs").join(&fixture.npc_file);

    let actual = npc_script::expand_script(&npc_path, &envir_root).expect("failed to expand npc script");

    assert_eq!(actual, fixture.expanded_lines);
}
