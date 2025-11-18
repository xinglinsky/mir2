use crate::world::map::MapInfo;
use crate::world::monster::MonsterInfo;
use crate::world::npc::NpcInfo;

#[derive(Clone, Debug, Default)]
pub struct WorldDatabase {
    pub map_infos: Vec<MapInfo>,
    pub monster_infos: Vec<MonsterInfo>,
    pub npc_infos: Vec<NpcInfo>,
}

pub trait WorldProvider {
    fn map_infos(&self) -> &[MapInfo];
    fn monster_infos(&self) -> &[MonsterInfo];
    fn npc_infos(&self) -> &[NpcInfo];

    fn get_map_info(&self, index: i32) -> Option<&MapInfo> {
        self.map_infos().iter().find(|m| m.index == index)
    }

    fn get_map_info_by_file_name(&self, file_name: &str) -> Option<&MapInfo> {
        self.map_infos()
            .iter()
            .find(|m| m.file_name.eq_ignore_ascii_case(file_name))
    }

    fn get_monster_info(&self, index: i32) -> Option<&MonsterInfo> {
        self.monster_infos().iter().find(|m| m.index == index)
    }

    fn get_npc_info(&self, index: i32) -> Option<&NpcInfo> {
        self.npc_infos().iter().find(|n| n.index == index)
    }
}

impl WorldDatabase {
    pub fn new() -> Self {
        Self::default()
    }
}

impl WorldProvider for WorldDatabase {
    fn map_infos(&self) -> &[MapInfo] {
        &self.map_infos
    }

    fn monster_infos(&self) -> &[MonsterInfo] {
        &self.monster_infos
    }

    fn npc_infos(&self) -> &[NpcInfo] {
        &self.npc_infos
    }
}
