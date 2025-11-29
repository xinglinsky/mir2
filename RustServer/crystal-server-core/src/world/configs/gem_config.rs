use once_cell::sync::Lazy;
use std::fs;
use std::path::Path;

fn parse_bool(val: &str, default: bool) -> bool {
    match val {
        "True" | "true" | "1" => true,
        "False" | "false" | "0" => false,
        _ => default,
    }
}

#[derive(Clone, Debug)]
pub struct GemConfig {
    pub gem_stat_independent: bool,
}

impl Default for GemConfig {
    fn default() -> Self {
        GemConfig {
            gem_stat_independent: true,
        }
    }
}

pub static GEM_CONFIG: Lazy<GemConfig> =
    Lazy::new(|| load_gem_config(Path::new("./Configs/GemSystem.ini")));

pub fn gem_config() -> &'static GemConfig {
    &GEM_CONFIG
}

fn load_gem_config(path: &Path) -> GemConfig {
    let mut cfg = GemConfig::default();

    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return cfg,
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
            current_section = Some(line[1..line.len() - 1].to_string());
            continue;
        }

        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();
        if key.is_empty() {
            continue;
        }

        match current_section.as_deref() {
            Some("Config") => match key {
                "GemStatIndependent" => {
                    cfg.gem_stat_independent =
                        parse_bool(value, cfg.gem_stat_independent);
                }
                _ => {}
            },
            _ => {}
        }
    }

    cfg
}
