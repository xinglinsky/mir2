use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConquestId(pub i32);

#[derive(Clone, Debug)]
pub struct ConquestArcherInfo {
    pub index: i32,
    pub location_x: i32,
    pub location_y: i32,
    pub mob_index: i32,
    pub name: String,
    pub repair_cost: u32,
}

#[derive(Clone, Debug)]
pub struct ConquestGateInfo {
    pub index: i32,
    pub location_x: i32,
    pub location_y: i32,
    pub mob_index: i32,
    pub name: String,
    pub repair_cost: i32,
}

#[derive(Clone, Debug)]
pub struct ConquestWallInfo {
    pub index: i32,
    pub location_x: i32,
    pub location_y: i32,
    pub mob_index: i32,
    pub name: String,
    pub repair_cost: i32,
}

#[derive(Clone, Debug)]
pub struct ConquestSiegeInfo {
    pub index: i32,
    pub location_x: i32,
    pub location_y: i32,
    pub mob_index: i32,
    pub name: String,
    pub repair_cost: i32,
}

#[derive(Clone, Debug)]
pub struct ConquestFlagInfo {
    pub index: i32,
    pub location_x: i32,
    pub location_y: i32,
    pub name: String,
    pub file_name: String,
}

#[derive(Clone, Debug)]
pub struct ConquestInfo {
    pub id: ConquestId,
    pub full_map: bool,
    pub location_x: i32,
    pub location_y: i32,
    pub size: u16,
    pub name: String,
    pub map_index: i32,
    pub palace_index: i32,
    pub extra_maps: Vec<i32>,
    pub guards: Vec<ConquestArcherInfo>,
    pub gates: Vec<ConquestGateInfo>,
    pub walls: Vec<ConquestWallInfo>,
    pub sieges: Vec<ConquestSiegeInfo>,
    pub flags: Vec<ConquestFlagInfo>,
    pub guard_index: i32,
    pub gate_index: i32,
    pub wall_index: i32,
    pub siege_index: i32,
    pub flag_index: i32,
    pub start_hour: u8,
    pub war_length: i32,
    pub conquest_type: u8,
    pub game: u8,
    pub monday: bool,
    pub tuesday: bool,
    pub wednesday: bool,
    pub thursday: bool,
    pub friday: bool,
    pub saturday: bool,
    pub sunday: bool,
    pub king_location_x: i32,
    pub king_location_y: i32,
    pub king_size: u16,
    pub control_points: Vec<ConquestFlagInfo>,
    pub control_point_index: i32,
}

#[derive(Default)]
pub struct ConquestManager {
    conquests: HashMap<ConquestId, ConquestInfo>,
}

impl ConquestManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn conquests(&self) -> impl Iterator<Item = &ConquestInfo> {
        self.conquests.values()
    }

    pub fn get(&self, id: ConquestId) -> Option<&ConquestInfo> {
        self.conquests.get(&id)
    }

    pub fn get_mut(&mut self, id: ConquestId) -> Option<&mut ConquestInfo> {
        self.conquests.get_mut(&id)
    }

    pub fn insert(&mut self, info: ConquestInfo) {
        self.conquests.insert(info.id, info);
    }

    pub fn remove(&mut self, id: ConquestId) -> Option<ConquestInfo> {
        self.conquests.remove(&id)
    }
}
