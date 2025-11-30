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
            // Use the concrete MonsterInfo index for HolyDeva from the
            // original DB so we do not depend on name-based lookup here.
            // See monsters_translation.csv: Monster_HolyDeva;12;HolyDeva;...
            monster_index: 12,
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
            // Use the concrete MonsterInfo index for Shinsu from the
            // original DB so we do not depend on name-based lookup here.
            // See monsters_translation.csv: Monster_Shinsu;10;Shinsu;...
            monster_index: 10,
            monster_name: "Shinsu",
            max_count_per_owner: 1,
            life_time_ms: 0,
            follow_distance: 3,
            leash_distance: 10,
            persistent: false,
        },
    );

    // Taoist Skeleton pet, mirrors C# Settings.SkeletonName = "BoneFamiliar".
    // monsters_translation.csv: Monster_BoneFamiliar;9;BoneFamiliar;...
    map.insert(
        PetKind::TaoistSkeleton,
        PetTemplate {
            kind: PetKind::TaoistSkeleton,
            monster_index: 9,
            monster_name: "BoneFamiliar",
            // 单人最多 2 个骷髅（同时还受 World::spawn_pet_for_player 中
            // MAX_MONSTER_PETS_PER_OWNER = 2 的总宠物数限制）
            max_count_per_owner: 2,
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
