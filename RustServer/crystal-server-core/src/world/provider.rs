use crate::world::map::{GameShopItemRecord, MapInfo};
use crate::world::monster::MonsterInfo;
use crate::world::npc::NpcInfo;
use crate::world::magic::MagicInfo;
use crate::world::buff::BuffInfo;
use crate::world::recipe::RecipeInfo;
use crate::world::types::BuffType;
use crystal_shared_proto::item_types::ItemInfoData;

static EMPTY_BUFF_INFOS: &[BuffInfo] = &[];
static EMPTY_RECIPE_INFOS: &[RecipeInfo] = &[];

#[derive(Clone, Debug, Default)]
pub struct WorldDatabase {
    pub map_infos: Vec<MapInfo>,
    pub item_infos: Vec<ItemInfoData>,
    pub monster_infos: Vec<MonsterInfo>,
    pub npc_infos: Vec<NpcInfo>,
    pub magic_infos: Vec<MagicInfo>,
    pub buff_infos: Vec<BuffInfo>,
    pub recipe_infos: Vec<RecipeInfo>,
    pub game_shop_items: Vec<GameShopItemRecord>,
}

pub trait WorldProvider {
    fn map_infos(&self) -> &[MapInfo];
    fn item_infos(&self) -> &[ItemInfoData];
    fn monster_infos(&self) -> &[MonsterInfo];
    fn npc_infos(&self) -> &[NpcInfo];
    fn magic_infos(&self) -> &[MagicInfo];

    fn buff_infos(&self) -> &[BuffInfo] {
        EMPTY_BUFF_INFOS
    }

    fn recipe_infos(&self) -> &[RecipeInfo] {
        EMPTY_RECIPE_INFOS
    }

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

    fn get_monster_info_by_name(&self, name: &str) -> Option<&MonsterInfo> {
        self
            .monster_infos()
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(name))
    }

    fn get_npc_info(&self, index: i32) -> Option<&NpcInfo> {
        self.npc_infos().iter().find(|n| n.index == index)
    }

    fn get_magic_info(&self, spell: u8) -> Option<&MagicInfo> {
        self.magic_infos().iter().find(|m| m.spell == spell)
    }

    fn get_item_info(&self, index: i32) -> Option<&ItemInfoData> {
        self.item_infos().iter().find(|i| i.index == index)
    }

    fn get_item_info_by_name(&self, name: &str) -> Option<&ItemInfoData> {
        self.item_infos()
            .iter()
            .find(|i| i.name.eq_ignore_ascii_case(name))
    }

    fn get_buff_info(&self, buff_type: BuffType) -> Option<&BuffInfo> {
        self.buff_infos()
            .iter()
            .find(|b| b.buff_type == buff_type)
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

    fn item_infos(&self) -> &[ItemInfoData] {
        &self.item_infos
    }

    fn monster_infos(&self) -> &[MonsterInfo] {
        &self.monster_infos
    }

    fn npc_infos(&self) -> &[NpcInfo] {
        &self.npc_infos
    }

    fn magic_infos(&self) -> &[MagicInfo] {
        &self.magic_infos
    }

    fn buff_infos(&self) -> &[BuffInfo] {
        &self.buff_infos
    }

    fn recipe_infos(&self) -> &[RecipeInfo] {
        &self.recipe_infos
    }
}
