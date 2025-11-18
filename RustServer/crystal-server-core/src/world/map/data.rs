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
