#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellAttribute {
    Walk,
    HighWall,
    LowWall,
}

#[derive(Clone, Debug)]
pub struct MapCell {
    pub attribute: CellAttribute,
}

#[derive(Clone, Debug)]
pub struct MapInfo {
    pub index: i32,
    pub file_name: String,
    pub title: String,
    pub mini_map: u16,
    pub big_map: u16,
    pub light: u8,
    pub map_dark_light: u8,
    pub music: u16,
    pub weather_particles: u16,
    pub no_teleport: bool,
    pub no_reconnect: bool,
    pub no_random: bool,
    pub no_escape: bool,
    pub no_recall: bool,
    pub no_drug: bool,
    pub no_position: bool,
    pub no_throw_item: bool,
    pub no_drop_player: bool,
    pub no_drop_monster: bool,
    pub no_names: bool,
    pub no_mount: bool,
    pub need_bridle: bool,
    pub no_fight: bool,
    pub fight: bool,
    pub fire: bool,
    pub fire_damage: i32,
    pub lightning: bool,
    pub lightning_damage: i32,
    pub no_town_teleport: bool,
    pub no_reincarnation: bool,
    pub no_reconnect_map: String,
    pub mine_zones: Vec<MineZone>,
    pub mine_index: u8,
    pub gt: bool,
    pub gt_index: u8,
    pub safe_zones: Vec<SafeZoneInfo>,
    pub respawns: Vec<RespawnInfo>,
    pub movements: Vec<MovementInfo>,
}

#[derive(Clone, Debug)]
pub struct Map {
    pub info: MapInfo,
    pub width: u16,
    pub height: u16,
    pub cells: Vec<MapCell>,
    pub walkable_cells: Vec<(u16, u16)>,
}

impl Map {
    #[inline]
    pub fn index_of(&self, x: u16, y: u16) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    #[inline]
    pub fn cell(&self, x: u16, y: u16) -> Option<&MapCell> {
        self.index_of(x, y).and_then(|idx| self.cells.get(idx))
    }
}

#[derive(Clone, Debug)]
pub struct SafeZoneInfo {
    pub location_x: i32,
    pub location_y: i32,
    pub size: u16,
    pub start_point: bool,
}

#[derive(Clone, Debug)]
pub struct MineZone {
    pub mine: u8,
    pub location_x: i32,
    pub location_y: i32,
    pub size: u16,
}

#[derive(Clone, Debug)]
pub struct MovementInfo {
    pub map_index: i32,
    pub source_x: i32,
    pub source_y: i32,
    pub dest_map_index: i32,
    pub dest_x: i32,
    pub dest_y: i32,
    pub need_hole: bool,
    pub need_move: bool,
    pub conquest_index: i32,
    pub show_on_big_map: bool,
    pub icon: i32,
}

#[derive(Clone, Debug)]
pub struct RespawnInfo {
    pub monster_index: i32,
    pub location_x: i32,
    pub location_y: i32,
    pub count: u16,
    pub spread: u16,
    pub delay: u16,
    pub random_delay: u16,
    pub direction: u8,
    pub route_path: String,
    pub respawn_index: i32,
    pub save_respawn_time: bool,
    pub respawn_ticks: u16,
}
