use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use super::data::{MapInfo, SafeZoneInfo};

/// Load MapInfo records from a text file using the same basic format as
/// C# Server.MirDatabase.MapInfo.FromText.
///
/// This is intended primarily for tools / stub servers. It focuses on
/// header fields and safe zones; movements/respawns/NPCs are currently
/// skipped but the field layout is kept roughly compatible so we can
/// extend it later without breaking existing data.
pub fn load_map_infos_from_file<P: AsRef<Path>>(path: P) -> io::Result<Vec<MapInfo>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    load_map_infos_from_reader(reader)
}

fn load_map_infos_from_reader<R: BufRead>(reader: R) -> io::Result<Vec<MapInfo>> {
    let mut result = Vec::new();

    for (line_no, line_res) in reader.lines().enumerate() {
        let line = line_res?;
        let line = line.trim();

        if line.is_empty() || line.starts_with(';') || line.starts_with("//") {
            continue;
        }

        match parse_mapinfo_line(line, (result.len() + 1) as i32) {
            Ok(info) => result.push(info),
            Err(msg) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("MapInfo line {}: {}", line_no + 1, msg),
                ));
            }
        }
    }

    Ok(result)
}

fn parse_mapinfo_line(line: &str, index: i32) -> Result<MapInfo, String> {
    let tokens: Vec<&str> = line
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if tokens.len() < 8 {
        return Err(format!("not enough fields (have {}, need >= 8)", tokens.len()));
    }

    let file_name = tokens[0].to_string();
    let title = tokens[1].to_string();

    let mini_map: u16 = tokens[2]
        .parse()
        .map_err(|_| format!("invalid MiniMap value '{}'", tokens[2]))?;

    // For now, LightSetting is parsed leniently: numeric values are used as-is,
    // named values fall back to 0. This can be tightened once the full
    // LightSetting enum mapping is mirrored in Rust.
    let light: u8 = tokens[3].parse().unwrap_or(0);

    let szi_count: usize = tokens[4]
        .parse()
        .map_err(|_| format!("invalid SafeZone count '{}'", tokens[4]))?;
    let mi_count: usize = tokens[5]
        .parse()
        .map_err(|_| format!("invalid Movement count '{}'", tokens[5]))?;
    let ri_count: usize = tokens[6]
        .parse()
        .map_err(|_| format!("invalid Respawn count '{}'", tokens[6]))?;
    let npc_count: usize = tokens[7]
        .parse()
        .map_err(|_| format!("invalid NPC count '{}'", tokens[7]))?;

    let mut safe_zones = Vec::new();

    let mut start = 8usize;

    // Safe zones: each uses 4 fields (x, y, size, startPoint).
    let needed = start + szi_count.saturating_mul(4);
    if tokens.len() < needed {
        return Err(format!(
            "not enough fields for {} safe zones (have {}, need >= {})",
            szi_count,
            tokens.len(),
            needed
        ));
    }

    for i in 0..szi_count {
        let base = start + i * 4;
        let x: i32 = tokens[base]
            .parse()
            .map_err(|_| format!("invalid SafeZone x '{}'", tokens[base]))?;
        let y: i32 = tokens[base + 1]
            .parse()
            .map_err(|_| format!("invalid SafeZone y '{}'", tokens[base + 1]))?;
        let size: u16 = tokens[base + 2]
            .parse()
            .map_err(|_| format!("invalid SafeZone size '{}'", tokens[base + 2]))?;
        let start_point: bool = tokens[base + 3]
            .parse()
            .map_err(|_| format!("invalid SafeZone StartPoint '{}'", tokens[base + 3]))?;

        safe_zones.push(SafeZoneInfo {
            location_x: x,
            location_y: y,
            size,
            start_point,
        });
    }
    start += szi_count * 4;

    // For now, Movements / Respawns / NPCs are skipped but we advance the
    // index in a way that matches the C# layout so we can extend this later
    // without breaking existing data files.
    let movement_fields = mi_count.saturating_mul(5);
    let respawn_fields = ri_count.saturating_mul(10);
    let npc_fields = npc_count.saturating_mul(6);
    let _ = start + movement_fields + respawn_fields + npc_fields;

    Ok(MapInfo {
        index,
        file_name,
        title,
        mini_map,
        big_map: 0,
        light,
        map_dark_light: 0,
        music: 0,
        weather_particles: 0,
        no_teleport: false,
        no_reconnect: false,
        no_random: false,
        no_escape: false,
        no_recall: false,
        no_drug: false,
        no_position: false,
        no_throw_item: false,
        no_drop_player: false,
        no_drop_monster: false,
        no_names: false,
        no_mount: false,
        need_bridle: false,
        no_fight: false,
        fight: false,
        fire: false,
        fire_damage: 0,
        lightning: false,
        lightning_damage: 0,
        no_town_teleport: false,
        no_reincarnation: false,
        no_reconnect_map: String::new(),
        mine_zones: Vec::new(),
        mine_index: 0,
        gt: false,
        gt_index: 0,
        safe_zones,
        respawns: Vec::new(),
        movements: Vec::new(),
    })
}
