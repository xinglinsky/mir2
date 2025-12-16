use once_cell::sync::Lazy;
use std::path::Path;

use crate::world::content;

const AWAKE_TYPE_COUNT: usize = 6; // DC, MC, SC, AC, MAC, HPMP
const GRADE_COUNT: usize = 4; // Common, Rare, Legendary, Mythical

/// Global awakening configuration loaded from Configs/AwakeningSystem.ini.
///
/// This mirrors the fields populated by C# Settings.LoadAwakeAttribute:
/// - Awake.AwakeSuccessRate / Awake.AwakeHitRate
/// - Awake.MaxAwakeLevel
/// - Awake.Awake_WeaponRate / Awake.Awake_HelmetRate / Awake.Awake_ArmorRate
/// - Awake.AwakeChanceMax (per grade)
/// - Awake.AwakeMaterials (per AwakeType x grade x material index)
/// - Awake.AwakeMaterialRate (per grade)
#[derive(Clone, Debug)]
pub struct AwakeningConfig {
    pub success_rate: u8,
    pub hit_rate: u8,
    pub max_level: i32,
    pub weapon_rate: u8,
    pub helmet_rate: u8,
    pub armor_rate: u8,
    /// Maximum roll (inclusive) for awakening chance per item grade.
    pub chance_max: [u8; GRADE_COUNT],
    /// Base material counts indexed as [awake_type][material_index][grade].
    /// awake_type: 0=DC,1=MC,2=SC,3=AC,4=MAC,5=HPMP
    /// material_index: 0=Material1,1=Material2
    /// grade: 0=Common,1=Rare,2=Legendary,3=Mythical
    pub materials_base: [[[u8; GRADE_COUNT]; 2]; AWAKE_TYPE_COUNT],
    /// Material increase multiplier per item grade.
    pub material_rate: [f32; GRADE_COUNT],
}

impl Default for AwakeningConfig {
    fn default() -> Self {
        // Defaults chosen to match the example AwakeningSystem.ini shipped with
        // the legacy C# server. If the ini file is missing or partially
        // invalid, these values act as a reasonable fallback.
        AwakeningConfig {
            success_rate: 70,
            hit_rate: 70,
            max_level: 5,
            weapon_rate: 1,
            helmet_rate: 1,
            armor_rate: 5,
            chance_max: [1, 2, 3, 4],
            materials_base: [[[1; GRADE_COUNT]; 2]; AWAKE_TYPE_COUNT],
            material_rate: [1.0; GRADE_COUNT],
        }
    }
}

pub static AWAKENING_CONFIG: Lazy<AwakeningConfig> = Lazy::new(|| {
    load_awakening_config(Path::new("./Configs/AwakeningSystem.ini"))
});

pub fn awakening_config() -> &'static AwakeningConfig {
    &AWAKENING_CONFIG
}

fn load_awakening_config(path: &Path) -> AwakeningConfig {
    let mut cfg = AwakeningConfig::default();

    let text = match content::read_to_string(path) {
        Ok(t) => t,
        Err(_) => {
            // If the file is missing or unreadable, fall back to defaults.
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
            match section.as_str() {
                "Attribute" => match key {
                    "SuccessRate" => {
                        if let Ok(v) = value.parse::<u8>() {
                            cfg.success_rate = v;
                        }
                    }
                    "HitRate" => {
                        if let Ok(v) = value.parse::<u8>() {
                            cfg.hit_rate = v;
                        }
                    }
                    "MaxUpgradeLevel" => {
                        if let Ok(v) = value.parse::<i32>() {
                            cfg.max_level = v;
                        }
                    }
                    _ => {}
                },
                "IncreaseValue" => match key {
                    "WeaponValue" => {
                        if let Ok(v) = value.parse::<u8>() {
                            cfg.weapon_rate = v;
                        }
                    }
                    "HelmetValue" => {
                        if let Ok(v) = value.parse::<u8>() {
                            cfg.helmet_rate = v;
                        }
                    }
                    "ArmorValue" => {
                        if let Ok(v) = value.parse::<u8>() {
                            cfg.armor_rate = v;
                        }
                    }
                    _ => {}
                },
                "Value" => {
                    if let Some(grade_name) = key.strip_prefix("ChanceMax_") {
                        if let Some(grade_idx) = parse_grade_index(grade_name) {
                            if let Ok(v) = value.parse::<u8>() {
                                cfg.chance_max[grade_idx] = v;
                            }
                        }
                    }
                }
                "Materials_BaseValue" => {
                    // Keys are of the form "DC_Common_Material1".
                    let parts: Vec<&str> = key.split('_').collect();
                    if parts.len() == 3 {
                        if let (Some(at_idx), Some(grade_idx), Some(mat_idx)) = (
                            parse_awake_type_index(parts[0]),
                            parse_grade_index(parts[1]),
                            parse_material_index(parts[2]),
                        ) {
                            if let Ok(v) = value.parse::<u8>() {
                                cfg.materials_base[at_idx][mat_idx][grade_idx] = v;
                            }
                        }
                    }
                }
                "Materials_IncreaseValue" => {
                    if let Some(grade_name) = key.strip_prefix("Materials_") {
                        if let Some(grade_idx) = parse_grade_index(grade_name) {
                            if let Ok(v) = value.parse::<f32>() {
                                cfg.material_rate[grade_idx] = v;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    cfg
}

fn parse_grade_index(name: &str) -> Option<usize> {
    match name {
        "Common" => Some(0),
        "Rare" => Some(1),
        "Legendary" => Some(2),
        "Mythical" => Some(3),
        _ => None,
    }
}

fn parse_awake_type_index(name: &str) -> Option<usize> {
    match name {
        "DC" => Some(0),
        "MC" => Some(1),
        "SC" => Some(2),
        "AC" => Some(3),
        "MAC" => Some(4),
        "HPMP" => Some(5),
        _ => None,
    }
}

fn parse_material_index(name: &str) -> Option<usize> {
    match name {
        "Material1" => Some(0),
        "Material2" => Some(1),
        _ => None,
    }
}
