use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;

use crystal_server_core::routes_text;

#[derive(Debug, Deserialize)]
struct RoutesFixture {
    route_file: String,
    points: Vec<RoutePointNode>,
}

#[derive(Debug, Deserialize)]
struct RoutePointNode {
    x: i32,
    y: i32,
    delay: i32,
}

#[test]
fn golden_routes_tvpatrol1() {
    fn run_one_fixture(path: &Path) {
        let raw = fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("missing fixture {}; generate it with Tools.GoldenFixtures (routes)", path.display()));
        let fixture: RoutesFixture = serde_json::from_str(&raw).expect("invalid fixture json");

        let jev_root = std::env::var("JEV_ROOT").expect("set JEV_ROOT to your Jev directory");
        let route_path = Path::new(&jev_root)
            .join("Envir")
            .join("Routes")
            .join(fixture.route_file.replace('/', &std::path::MAIN_SEPARATOR.to_string()));

        let actual = routes_text::load_routes_file(&route_path).expect("failed to parse routes file");

        let expected: Vec<routes_text::RoutePoint> = fixture
            .points
            .into_iter()
            .map(|p| routes_text::RoutePoint {
                x: p.x,
                y: p.y,
                delay: p.delay,
            })
            .collect();

        assert_eq!(actual, expected);
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
                .map(|s| s.to_ascii_lowercase().starts_with("routes_") && s.to_ascii_lowercase().ends_with(".json"))
                .unwrap_or(false)
        })
        .collect();

    files.sort_by(|a, b| a.to_string_lossy().to_ascii_lowercase().cmp(&b.to_string_lossy().to_ascii_lowercase()));

    assert!(!files.is_empty(), "no routes_*.json fixtures found");

    for path in files {
        run_one_fixture(&path);
    }
}
