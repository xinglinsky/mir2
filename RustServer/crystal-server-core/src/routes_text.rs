use std::io;
use std::path::Path;

use crate::world::content;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoutePoint {
    pub x: i32,
    pub y: i32,
    pub delay: i32,
}

fn read_lines(path: &Path) -> io::Result<Vec<String>> {
    let s = content::read_to_string(path)?;
    let s = s.replace("\r\n", "\n");
    let s = s.replace('\r', "\n");
    Ok(s.split_terminator('\n').map(|l| l.to_string()).collect())
}

pub fn parse_line(line: &str) -> Option<RoutePoint> {
    let parts: Vec<&str> = line.split(',').filter(|p| !p.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }

    let x: i32 = parts[0].trim().parse().ok()?;
    let y: i32 = parts[1].trim().parse().ok()?;

    let mut delay: i32 = 0;
    if parts.len() > 2 {
        if let Ok(d) = parts[2].trim().parse::<i32>() {
            delay = d;
        }
    }

    Some(RoutePoint { x, y, delay })
}

pub fn load_routes_file(path: &Path) -> io::Result<Vec<RoutePoint>> {
    let lines = read_lines(path)?;
    let mut out = Vec::new();

    for line in lines {
        if let Some(p) = parse_line(&line) {
            out.push(p);
        }
    }

    Ok(out)
}
