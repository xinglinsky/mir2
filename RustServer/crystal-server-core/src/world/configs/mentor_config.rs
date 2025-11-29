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

fn parse_u8(val: &str, default: u8) -> u8 {
    val.parse::<u8>().unwrap_or(default)
}

#[derive(Clone, Debug)]
pub struct MentorConfig {
    pub level_gap: u8,
    pub mentee_skill_boost: bool,
    pub mentorship_length: u8,
    pub mentor_damage_boost: u8,
    pub mentee_exp_boost: u8,
    pub percent_xp_to_mentor: u8,
}

impl Default for MentorConfig {
    fn default() -> Self {
        MentorConfig {
            level_gap: 10,
            mentee_skill_boost: true,
            mentorship_length: 7,
            mentor_damage_boost: 10,
            mentee_exp_boost: 10,
            percent_xp_to_mentor: 1,
        }
    }
}

pub static MENTOR_CONFIG: Lazy<MentorConfig> =
    Lazy::new(|| load_mentor_config(Path::new("./Configs/MentorSystem.ini")));

pub fn mentor_config() -> &'static MentorConfig {
    &MENTOR_CONFIG
}

fn load_mentor_config(path: &Path) -> MentorConfig {
    let mut cfg = MentorConfig::default();

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
                "LevelGap" => {
                    cfg.level_gap = parse_u8(value, cfg.level_gap);
                }
                "MenteeSkillBoost" => {
                    cfg.mentee_skill_boost =
                        parse_bool(value, cfg.mentee_skill_boost);
                }
                "MentorshipLength" => {
                    cfg.mentorship_length =
                        parse_u8(value, cfg.mentorship_length);
                }
                "MentorDamageBoost" => {
                    cfg.mentor_damage_boost =
                        parse_u8(value, cfg.mentor_damage_boost);
                }
                "MenteeExpBoost" => {
                    cfg.mentee_exp_boost =
                        parse_u8(value, cfg.mentee_exp_boost);
                }
                "PercentXPtoMentor" => {
                    cfg.percent_xp_to_mentor =
                        parse_u8(value, cfg.percent_xp_to_mentor);
                }
                _ => {}
            },
            _ => {}
        }
    }

    cfg
}
