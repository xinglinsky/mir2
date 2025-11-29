use once_cell::sync::Lazy;
use std::collections::HashMap;

use crate::world::types::PetKind;

#[derive(Clone, Debug)]
pub struct PetTemplate {
    pub kind: PetKind,
    pub monster_index: i32,
    pub monster_name: &'static str,
    pub max_count_per_owner: u8,
    pub life_time_ms: i64,
    pub follow_distance: i32,
    pub leash_distance: i32,
    pub persistent: bool,
}

static PET_TEMPLATES: Lazy<HashMap<PetKind, PetTemplate>> = Lazy::new(|| {
    let mut map = HashMap::new();

    map.insert(
        PetKind::TaoistHolyDeva,
        PetTemplate {
            kind: PetKind::TaoistHolyDeva,
            monster_index: 0,
            monster_name: "HolyDeva",
            max_count_per_owner: 1,
            life_time_ms: 0,
            follow_distance: 3,
            leash_distance: 10,
            persistent: false,
        },
    );

    map.insert(
        PetKind::TaoistShinsu,
        PetTemplate {
            kind: PetKind::TaoistShinsu,
            monster_index: 0,
            monster_name: "Shinsu",
            max_count_per_owner: 1,
            life_time_ms: 0,
            follow_distance: 3,
            leash_distance: 10,
            persistent: false,
        },
    );

    map
});

pub fn pet_template(kind: PetKind) -> Option<&'static PetTemplate> {
    PET_TEMPLATES.get(&kind)
}
