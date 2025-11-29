use once_cell::sync::Lazy;
use crystal_shared_proto::map_types::{WorldMapIconData, WorldMapSetupData};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub static WORLD_MAP_SETUP: Lazy<WorldMapSetupData> =
    Lazy::new(|| load_world_map_setup(Path::new("./Configs/WorldMap.ini")));

pub fn world_map_setup() -> &'static WorldMapSetupData {
    &WORLD_MAP_SETUP
}

fn load_world_map_setup(path: &Path) -> WorldMapSetupData {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(
                "WorldMap.ini not found or unreadable at {:?}: {:?}; world map will be disabled",
                path,
                e
            );
            return WorldMapSetupData {
                enabled: false,
                icons: Vec::new(),
            };
        }
    };

    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_section = String::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("//")
            || line.starts_with(';')
        {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].trim().to_string();
            continue;
        }

        if let Some(eq) = line.find('=') {
            let (k, v) = line.split_at(eq);
            let key = k.trim().to_string();
            let val = v[1..].trim().to_string();
            sections
                .entry(current_section.clone())
                .or_default()
                .insert(key, val);
        }
    }

    let get = |section: &str, key: &str| -> Option<String> {
        sections
            .get(section)
            .and_then(|m| m.get(key))
            .cloned()
    };

    let enabled = get("Setup", "Enabled")
        .map(|s| {
            let lower = s.to_ascii_lowercase();
            matches!(lower.as_str(), "true" | "1" | "yes" | "on")
        })
        .unwrap_or(false);

    let mut icons: Vec<WorldMapIconData> = Vec::new();
    for idx in 0.. {
        let image_key = format!("Button{}ImageIndex", idx);
        let image_str = match get("Layout", &image_key) {
            Some(s) => s,
            None => break,
        };

        let image_index: i32 = match image_str.parse() {
            Ok(v) => v,
            Err(_) => break,
        };

        let title = get("Layout", &format!("Button{}Title", idx)).unwrap_or_default();
        let map_index: i32 = get("Layout", &format!("Button{}MapIndex", idx))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        icons.push(WorldMapIconData {
            image_index,
            title,
            map_index,
        });
    }

    WorldMapSetupData { enabled, icons }
}
