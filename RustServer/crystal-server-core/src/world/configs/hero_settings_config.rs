use once_cell::sync::Lazy;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct HeroSettingsConfig {
    pub allow_new_hero: bool,
    pub minimum_level: u8,
    pub maximum_count: u8,
}

impl Default for HeroSettingsConfig {
    fn default() -> Self {
        HeroSettingsConfig {
            allow_new_hero: true,
            minimum_level: 0,
            maximum_count: 3,
        }
    }
}

pub static HERO_SETTINGS_CONFIG: Lazy<HeroSettingsConfig> = Lazy::new(|| {
    load_hero_settings_config(Path::new("./Configs/HeroSettings.ini"))
});

pub fn hero_settings_config() -> &'static HeroSettingsConfig {
    &HERO_SETTINGS_CONFIG
}

fn load_hero_settings_config(path: &Path) -> HeroSettingsConfig {
    let mut cfg = HeroSettingsConfig::default();

    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => {
            return cfg;
        }
    };

    let mut current_section: Option<String> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let name = &line[1..line.len() - 1];
            current_section = Some(name.to_string());
            continue;
        }

        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();
        if key.is_empty() {
            continue;
        }

        if let Some(section) = &current_section {
            if section == "Hero" {
                match key {
                    "AllowNewHero" => {
                        cfg.allow_new_hero = matches!(value, "True" | "true" | "1");
                    }
                    "MinimumLevel" => {
                        if let Ok(v) = value.parse::<u8>() {
                            cfg.minimum_level = v;
                        }
                    }
                    "MaximumCount" => {
                        if let Ok(v) = value.parse::<u8>() {
                            cfg.maximum_count = v;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    cfg
}
