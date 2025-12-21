use std::collections::HashMap;
use rand::Rng;
use crystal_shared_proto::item_types::{UserItemData, ItemInfoData, AwakeData};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum AwakeType {
    None = 0,
    DC = 1,
    MC = 2,
    SC = 3,
    AC = 4,
    MAC = 5,
    HPMP = 6,
}

impl Default for AwakeType {
    fn default() -> Self {
        AwakeType::None
    }
}

impl AwakeType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => AwakeType::DC,
            2 => AwakeType::MC,
            3 => AwakeType::SC,
            4 => AwakeType::AC,
            5 => AwakeType::MAC,
            6 => AwakeType::HPMP,
            _ => AwakeType::None,
        }
    }
}

// Awakening system constants
pub const AWAKE_SUCCESS_RATE: u8 = 70;
pub const AWAKE_HIT_RATE: u8 = 70;
pub const MAX_AWAKE_LEVEL: usize = 5;
pub const AWAKE_WEAPON_RATE: u8 = 1;
pub const AWAKE_HELMET_RATE: u8 = 1;
pub const AWAKE_ARMOR_RATE: u8 = 5;
pub const AWAKE_CHANCE_MIN: u8 = 1;

// Material rate increase by grade (index 0 = Grade 1, etc.)
pub const AWAKE_MATERIAL_RATE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

// Material requirements by type and grade
// Structure: AwakeMaterials[type_index][material_index][grade_index]
pub type MaterialMatrix = Vec<Vec<[u8; 4]>>;

// Default material requirements (can be loaded from config)
pub fn get_default_materials() -> MaterialMatrix {
    vec![
        // DC type materials
        vec![[1, 1, 1, 1], [1, 1, 1, 1]],
        // MC type materials
        vec![[1, 1, 1, 1], [1, 1, 1, 1]],
        // SC type materials
        vec![[1, 1, 1, 1], [1, 1, 1, 1]],
        // AC type materials
        vec![[1, 1, 1, 1], [1, 1, 1, 1]],
        // MAC type materials
        vec![[1, 1, 1, 1], [1, 1, 1, 1]],
        // HPMP type materials
        vec![[1, 1, 1, 1], [1, 1, 1, 1]],
    ]
}

// Awakening chance max by grade
pub const AWAKE_CHANCE_MAX: [u8; 4] = [1, 2, 3, 4];

#[derive(Clone, Debug)]
pub struct Awakening {
    pub awake_type: AwakeType,
    pub values: Vec<u8>,
}

impl Awakening {
    pub fn new() -> Self {
        Self {
            awake_type: AwakeType::None,
            values: Vec::new(),
        }
    }

    pub fn from_data(data: &AwakeData) -> Self {
        Self {
            awake_type: AwakeType::from_u8(data.awake_type),
            values: data.values.clone(),
        }
    }

    pub fn to_data(&self) -> AwakeData {
        AwakeData {
            awake_type: self.awake_type as u8,
            values: self.values.clone(),
        }
    }

    pub fn is_max_level(&self) -> bool {
        self.values.len() >= MAX_AWAKE_LEVEL
    }

    pub fn get_awake_level(&self) -> usize {
        self.values.len()
    }

    pub fn get_awake_value(&self) -> u8 {
        self.values.iter().sum()
    }

    pub fn get_dc(&self) -> u8 {
        if self.awake_type == AwakeType::DC {
            self.get_awake_value()
        } else {
            0
        }
    }

    pub fn get_mc(&self) -> u8 {
        if self.awake_type == AwakeType::MC {
            self.get_awake_value()
        } else {
            0
        }
    }

    pub fn get_sc(&self) -> u8 {
        if self.awake_type == AwakeType::SC {
            self.get_awake_value()
        } else {
            0
        }
    }

    pub fn get_ac(&self) -> u8 {
        if self.awake_type == AwakeType::AC {
            self.get_awake_value()
        } else {
            0
        }
    }

    pub fn get_mac(&self) -> u8 {
        if self.awake_type == AwakeType::MAC {
            self.get_awake_value()
        } else {
            0
        }
    }

    pub fn get_hpmp(&self) -> u8 {
        if self.awake_type == AwakeType::HPMP {
            self.get_awake_value()
        } else {
            0
        }
    }

    pub fn upgrade_awake(&mut self, item: &UserItemData, awake_type: AwakeType) -> (i32, Vec<bool>) {
        // Returns: (-1) condition error, (0) failed, (1) success, and hit results
        if !self.check_awakening(item, awake_type) {
            return (-1, vec![false; 5]);
        }

        let mut rng = rand::thread_rng();
        
        if rng.gen_range(0..100) <= AWAKE_SUCCESS_RATE as i32 {
            let is_hit = self.awakening();
            (1, is_hit)
        } else {
            let is_hit = self.make_hit(1);
            (0, is_hit)
        }
    }

    pub fn remove_awake(&mut self) -> i32 {
        if self.values.is_empty() {
            return 0;
        }
        
        self.values.pop();
        
        if self.values.is_empty() {
            self.awake_type = AwakeType::None;
        }
        
        1
    }

    fn check_awakening(&self, item: &UserItemData, awake_type: AwakeType) -> bool {
        if awake_type == AwakeType::None {
            return false;
        }

        // Check if already max level
        if self.is_max_level() {
            return false;
        }

        // Check if type matches (or first awakening)
        if self.awake_type != AwakeType::None && self.awake_type != awake_type {
            return false;
        }

        true
    }

    fn awakening(&mut self) -> Vec<bool> {
        let is_hit = self.make_hit(AWAKE_CHANCE_MAX[self.values.len().min(3)] as i32);
        
        if self.awake_type == AwakeType::None {
            self.awake_type = AwakeType::DC; // Default type
        }
        
        // Add a random value based on hits
        let value = if is_hit.iter().any(|&h| h) { 1 } else { 0 };
        self.values.push(value);
        is_hit
    }

    fn make_hit(&self, max_value: i32) -> Vec<bool> {
        let step_value = max_value as f32 / 5.0;
        let mut total_value = 0.0;
        let mut is_hit = Vec::with_capacity(5);
        let mut rng = rand::thread_rng();

        for _ in 0..5 {
            if rng.gen_range(0..100) < AWAKE_HIT_RATE as i32 {
                total_value += step_value;
                is_hit.push(true);
            } else {
                is_hit.push(false);
            }
        }

        let final_value = if total_value <= 1.0 { 1 } else { total_value as i32 };
        // We could store the value but the C# version just returns the hits
        is_hit
    }
}

// Calculate awakening price based on item grade and current level
pub fn calculate_awakening_price(grade: u8, level: usize) -> u64 {
    let base_price = match grade {
        1 => 1000,
        2 => 5000,
        3 => 20000,
        4 => 100000,
        _ => 1000,
    };
    
    base_price * (level + 1) as u64
}

// Calculate downgrade price
pub fn calculate_downgrade_price(grade: u8) -> u64 {
    match grade {
        1 => 500,
        2 => 2500,
        3 => 10000,
        4 => 50000,
        _ => 500,
    }
}

// Calculate material requirements
pub fn calculate_material_requirements(
    materials: &MaterialMatrix,
    grade: usize,
    level: usize,
) -> [u8; 2] {
    if materials.is_empty() {
        return [0, 0];
    }

    let type_index = 0; // Would be type-1 in actual implementation
    let material_rate = (AWAKE_MATERIAL_RATE[grade] * level as f32) as u8;
    
    [
        materials[type_index][0][grade] + material_rate,
        materials[type_index][1][grade] + material_rate,
    ]
}
