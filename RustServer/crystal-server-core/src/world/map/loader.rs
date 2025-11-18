use std::{fs, io, path::Path};

use super::data::{CellAttribute, Map, MapCell, MapInfo};

#[derive(Copy, Clone, Debug)]
pub enum MapFormat {
    V0,
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V100,
}

pub fn load_map_from_file(info: MapInfo, map_dir: &Path) -> io::Result<Map> {
    let path = map_dir.join(format!("{}.map", info.file_name));
    let bytes = fs::read(path)?;
    load_map_from_bytes(info, &bytes)
}

pub fn load_map_from_bytes(info: MapInfo, bytes: &[u8]) -> io::Result<Map> {
    let format = detect_format(bytes)?;
    let (width, height, cells) = match format {
        MapFormat::V0 => load_cells_v0(bytes)?,
        MapFormat::V1 => load_cells_v1(bytes)?,
        MapFormat::V2 => load_cells_v2(bytes)?,
        MapFormat::V3 => load_cells_v3(bytes)?,
        MapFormat::V4 => load_cells_v4(bytes)?,
        MapFormat::V5 => load_cells_v5(bytes)?,
        MapFormat::V6 => load_cells_v6(bytes)?,
        MapFormat::V7 => load_cells_v7(bytes)?,
        MapFormat::V100 => load_cells_v100(bytes)?,
    };

    let walkable_cells = compute_walkable_cells(width, height, &cells);

    Ok(Map {
        info,
        width,
        height,
        cells,
        walkable_cells,
    })
}

fn compute_walkable_cells(width: u16, height: u16, cells: &[MapCell]) -> Vec<(u16, u16)> {
    let mut result = Vec::new();
    let w = width as usize;
    for y in 0..height {
        for x in 0..width {
            let idx = y as usize * w + x as usize;
            if let CellAttribute::Walk = cells[idx].attribute {
                result.push((x, y));
            }
        }
    }
    result
}

fn detect_format(bytes: &[u8]) -> io::Result<MapFormat> {
    if bytes.len() < 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "map file too small to detect format",
        ));
    }

    // C# custom map format (header bytes 2,3 == 'C', '#').
    if bytes.len() >= 4 && bytes[2] == 0x43 && bytes[3] == 0x23 {
        return Ok(MapFormat::V100);
    }

    // Wemade Mir3 maps: no title, start with zero.
    if bytes[0] == 0 {
        return Ok(MapFormat::V5);
    }

    // Shanda Mir3 maps: title "(C) SNDA, MIR3." etc.
    if bytes.len() >= 15 && bytes[0] == 0x0F && bytes[5] == 0x53 && bytes[14] == 0x33 {
        return Ok(MapFormat::V6);
    }

    // Wemade anti-hack maps: title starts with "Mir2 AntiHack".
    if bytes.len() >= 20 && bytes[0] == 0x15 && bytes[4] == 0x32 && bytes[6] == 0x41 && bytes[19] == 0x31 {
        return Ok(MapFormat::V4);
    }

    // Wemade 2010 map format: title starts with "Map 2010 Ver 1.0".
    if bytes.len() >= 15 && bytes[0] == 0x10 && bytes[2] == 0x61 && bytes[7] == 0x31 && bytes[14] == 0x31 {
        return Ok(MapFormat::V1);
    }

    // Shanda 2012 / older format, decided by file size vs expected.
    if bytes.len() >= 20 && ((bytes[4] == 0x0F) || (bytes[4] == 0x03) && bytes[18] == 0x0D && bytes[19] == 0x0A)
    {
        let w = (bytes[0] as i32) + ((bytes[1] as i32) << 8);
        let h = (bytes[2] as i32) + ((bytes[3] as i32) << 8);
        let expected = 52i64 + (w as i64 * h as i64 * 14);
        if bytes.len() as i64 > expected {
            return Ok(MapFormat::V3);
        } else {
            return Ok(MapFormat::V2);
        }
    }

    // 3/4 heroes map format.
    if bytes.len() >= 12 && bytes[0] == 0x0D && bytes[1] == 0x4C && bytes[7] == 0x20 && bytes[11] == 0x6D {
        return Ok(MapFormat::V7);
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "unknown map format",
    ))
}

fn load_cells_v0(bytes: &[u8]) -> io::Result<(u16, u16, Vec<MapCell>)> {
    if bytes.len() < 4 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "map file too small for V0 header",
        ));
    }

    let mut offset = 0usize;
    let width = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u16;
    offset += 2;
    let height = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u16;

    let mut cells = Vec::with_capacity(width as usize * height as usize);
    offset = 52;

    for _x in 0..width {
        for _y in 0..height {
            if offset + 15 > bytes.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "map file truncated in V0 cells",
                ));
            }

            let mut attr = CellAttribute::Walk;

            let v1 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if (v1 & 0x8000) != 0 {
                attr = CellAttribute::HighWall;
            }
            offset += 2;

            let v2 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if (v2 & 0x8000) != 0 {
                attr = CellAttribute::LowWall;
            }
            offset += 2;

            let v3 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if (v3 & 0x8000) != 0 {
                attr = CellAttribute::HighWall;
            }
            offset += 4;

            // Door + padding (ignored, but we must consume bytes in the same way as C#).
            let _door = bytes[offset];
            offset += 3;

            let _light = bytes[offset];
            offset += 1;

            cells.push(MapCell { attribute: attr });
        }
    }

    Ok((width, height, cells))
}

fn load_cells_v1(bytes: &[u8]) -> io::Result<(u16, u16, Vec<MapCell>)> {
    if bytes.len() < 27 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "map file too small for V1 header",
        ));
    }

    let mut offset = 21usize;
    let w = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
    offset += 2;
    let xor = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
    offset += 2;
    let h = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);

    let width = w ^ xor;
    let height = h ^ xor;

    let mut cells = Vec::with_capacity(width as usize * height as usize);
    offset = 54;

    for _x in 0..width {
        for _y in 0..height {
            if offset + 15 > bytes.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "map file truncated in V1 cells",
                ));
            }

            let mut attr = CellAttribute::Walk;

            let v = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            if ((v ^ 0xAA38AA38u32) & 0x2000_0000u32) != 0 {
                attr = CellAttribute::HighWall;
            }
            offset += 6;

            let v2 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if ((v2 ^ xor) & 0x8000) != 0 {
                attr = CellAttribute::LowWall;
            }
            offset += 2;

            let _door = bytes[offset];
            offset += 5;

            let _light = bytes[offset];
            offset += 1;

            cells.push(MapCell { attribute: attr });
        }
    }

    Ok((width, height, cells))
}

fn load_cells_v2(bytes: &[u8]) -> io::Result<(u16, u16, Vec<MapCell>)> {
    if bytes.len() < 4 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "map file too small for V2 header",
        ));
    }

    let mut offset = 0usize;
    let width = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u16;
    offset += 2;
    let height = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u16;

    let mut cells = Vec::with_capacity(width as usize * height as usize);
    offset = 52;

    for _x in 0..width {
        for _y in 0..height {
            if offset + 14 > bytes.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "map file truncated in V2 cells",
                ));
            }

            let mut attr = CellAttribute::Walk;

            let v1 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if (v1 & 0x8000) != 0 {
                attr = CellAttribute::HighWall;
            }
            offset += 2;

            let v2 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if (v2 & 0x8000) != 0 {
                attr = CellAttribute::LowWall;
            }
            offset += 2;

            let v3 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if (v3 & 0x8000) != 0 {
                attr = CellAttribute::HighWall;
            }
            offset += 2;

            let _door = bytes[offset];
            offset += 5;

            let _light = bytes[offset];
            offset += 1;

            offset += 2; // extra padding

            cells.push(MapCell { attribute: attr });
        }
    }

    Ok((width, height, cells))
}

fn load_cells_v3(bytes: &[u8]) -> io::Result<(u16, u16, Vec<MapCell>)> {
    if bytes.len() < 4 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "map file too small for V3 header",
        ));
    }

    let mut offset = 0usize;
    let width = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u16;
    offset += 2;
    let height = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u16;

    let mut cells = Vec::with_capacity(width as usize * height as usize);
    offset = 52;

    for _x in 0..width {
        for _y in 0..height {
            if offset + 36 > bytes.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "map file truncated in V3 cells",
                ));
            }

            let mut attr = CellAttribute::Walk;

            let v1 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if (v1 & 0x8000) != 0 {
                attr = CellAttribute::HighWall;
            }
            offset += 2;

            let v2 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if (v2 & 0x8000) != 0 {
                attr = CellAttribute::LowWall;
            }
            offset += 2;

            let v3 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if (v3 & 0x8000) != 0 {
                attr = CellAttribute::HighWall;
            }

            offset += 2;

            let _door = bytes[offset];
            offset += 12;

            let _light = bytes[offset];
            offset += 1;

            offset += 17;

            cells.push(MapCell { attribute: attr });
        }
    }

    Ok((width, height, cells))
}

fn load_cells_v4(bytes: &[u8]) -> io::Result<(u16, u16, Vec<MapCell>)> {
    if bytes.len() < 35 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "map file too small for V4 header",
        ));
    }

    let mut offset = 31usize;
    let w = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
    offset += 2;
    let xor = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
    offset += 2;
    let h = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);

    let width = (w ^ xor) as u16;
    let height = (h ^ xor) as u16;

    let mut cells = Vec::with_capacity(width as usize * height as usize);
    offset = 64;

    for _x in 0..width {
        for _y in 0..height {
            if offset + 12 > bytes.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "map file truncated in V4 cells",
                ));
            }

            let mut attr = CellAttribute::Walk;

            let v1 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if (v1 & 0x8000) != 0 {
                attr = CellAttribute::HighWall;
            }
            offset += 2;

            let v2 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if (v2 & 0x8000) != 0 {
                attr = CellAttribute::LowWall;
            }

            offset += 4;

            let _door = bytes[offset];
            offset += 6;

            cells.push(MapCell { attribute: attr });
        }
    }

    Ok((width, height, cells))
}

fn load_cells_v5(bytes: &[u8]) -> io::Result<(u16, u16, Vec<MapCell>)> {
    if bytes.len() < 26 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "map file too small for V5 header",
        ));
    }

    let mut offset = 22usize;
    let width = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u16;
    offset += 2;
    let height = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u16;

    let mut cells = Vec::with_capacity(width as usize * height as usize);
    let start = 28 + (3 * (((width as usize) / 2) + ((width as usize) % 2)) * ((height as usize) / 2));
    offset = start;

    for _x in 0..width {
        for _y in 0..height {
            if offset + 14 > bytes.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "map file truncated in V5 cells",
                ));
            }

            let b = bytes[offset];
            let attr = if (b & 0x01) != 1 {
                CellAttribute::HighWall
            } else if (b & 0x02) != 2 {
                CellAttribute::LowWall
            } else {
                CellAttribute::Walk
            };
            offset += 13;

            let _light = bytes[offset];
            offset += 1;

            cells.push(MapCell { attribute: attr });
        }
    }

    Ok((width, height, cells))
}

fn load_cells_v6(bytes: &[u8]) -> io::Result<(u16, u16, Vec<MapCell>)> {
    if bytes.len() < 20 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "map file too small for V6 header",
        ));
    }

    let mut offset = 16usize;
    let width = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u16;
    offset += 2;
    let height = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u16;

    let mut cells = Vec::with_capacity(width as usize * height as usize);
    offset = 40;

    for _x in 0..width {
        for _y in 0..height {
            if offset + 20 > bytes.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "map file truncated in V6 cells",
                ));
            }

            let b = bytes[offset];
            let attr = if (b & 0x01) != 1 {
                CellAttribute::HighWall
            } else if (b & 0x02) != 2 {
                CellAttribute::LowWall
            } else {
                CellAttribute::Walk
            };
            offset += 20;

            cells.push(MapCell { attribute: attr });
        }
    }

    Ok((width, height, cells))
}

fn load_cells_v7(bytes: &[u8]) -> io::Result<(u16, u16, Vec<MapCell>)> {
    if bytes.len() < 25 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "map file too small for V7 header",
        ));
    }

    let mut offset = 21usize;
    let width = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u16;
    offset += 4;
    let height = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u16;

    let mut cells = Vec::with_capacity(width as usize * height as usize);
    offset = 54;

    for _x in 0..width {
        for _y in 0..height {
            if offset + 15 > bytes.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "map file truncated in V7 cells",
                ));
            }

            let mut attr = CellAttribute::Walk;

            let v1 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if (v1 & 0x8000) != 0 {
                attr = CellAttribute::HighWall;
            }
            offset += 6;

            let v2 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if (v2 & 0x8000) != 0 {
                attr = CellAttribute::LowWall;
            }
            offset += 2;

            let _door = bytes[offset];
            offset += 4;

            let _light = bytes[offset];
            offset += 1;

            offset += 2;

            cells.push(MapCell { attribute: attr });
        }
    }

    Ok((width, height, cells))
}

fn load_cells_v100(bytes: &[u8]) -> io::Result<(u16, u16, Vec<MapCell>)> {
    if bytes.len() < 8 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "map file too small for V100 header",
        ));
    }

    // Only support version 1 for now, as in the original C# implementation.
    if bytes[0] != 1 || bytes[1] != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported custom map version",
        ));
    }

    let mut offset = 4usize;
    let width = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u16;
    offset += 2;
    let height = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u16;

    let mut cells = Vec::with_capacity(width as usize * height as usize);
    offset = 8;

    for _x in 0..width {
        for _y in 0..height {
            if offset + 20 > bytes.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "map file truncated in V100 cells",
                ));
            }

            offset += 2;

            let v = i32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            let mut attr = CellAttribute::Walk;
            if (v & 0x2000_0000) != 0 {
                attr = CellAttribute::HighWall;
            }
            offset += 10;

            let v2 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if (v2 & 0x8000) != 0 {
                attr = CellAttribute::LowWall;
            }
            offset += 2;

            let _door = bytes[offset];
            offset += 11;

            let _light = bytes[offset];
            offset += 1;

            cells.push(MapCell { attribute: attr });
        }
    }

    Ok((width, height, cells))
}
