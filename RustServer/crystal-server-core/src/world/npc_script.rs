use std::io;
use std::path::{Path, PathBuf};

use crate::world::content;

fn read_lines(path: &Path) -> io::Result<Vec<String>> {
    let s = content::read_to_string(path)?;
    let s = s.replace("\r\n", "\n");
    let s = s.replace('\r', "\n");
    Ok(s.split_terminator('\n').map(|l| l.to_string()).collect())
}

fn join_rel(base: &Path, rel: &str) -> PathBuf {
    let rel = rel.replace('\\', &std::path::MAIN_SEPARATOR.to_string());
    base.join(rel)
}

pub fn expand_script(npc_file: &Path, envir_root: &Path) -> io::Result<Vec<String>> {
    let mut lines = read_lines(npc_file)?;
    lines = parse_insert(lines, envir_root)?;
    lines = parse_include(lines, envir_root)?;
    Ok(lines)
}

fn parse_insert(mut lines: Vec<String>, envir_root: &Path) -> io::Result<Vec<String>> {
    let mut new_lines: Vec<String> = Vec::new();

    let mut i = 0usize;
    while i < lines.len() {
        let upper = lines[i].to_ascii_uppercase();
        if !upper.starts_with("#INSERT") {
            i += 1;
            continue;
        }

        let split: Vec<&str> = lines[i].split(' ').collect();
        if split.len() < 2 {
            i += 1;
            continue;
        }

        let token = split[1];
        if token.len() < 2 {
            i += 1;
            continue;
        }

        let sub = &token[1..token.len() - 1];
        let path = join_rel(envir_root, sub);

        match read_lines(&path) {
            Ok(n) => new_lines = n,
            Err(_) => {
                // Match C# behaviour: keep prior new_lines and still append it.
            }
        }

        lines.extend(new_lines.clone());
        i += 1;
    }

    lines.retain(|l| !l.to_ascii_uppercase().starts_with("#INSERT"));
    Ok(lines)
}

fn parse_include(mut lines: Vec<String>, envir_root: &Path) -> io::Result<Vec<String>> {
    let mut i = 0usize;
    while i < lines.len() {
        let upper = lines[i].to_ascii_uppercase();
        if !upper.starts_with("#INCLUDE") {
            i += 1;
            continue;
        }

        let split: Vec<&str> = lines[i].split(' ').collect();
        if split.len() < 3 {
            i += 1;
            continue;
        }

        let token = split[1];
        if token.len() < 2 {
            i += 1;
            continue;
        }

        let sub = &token[1..token.len() - 1];
        let path = join_rel(envir_root, sub);
        let page = format!("[{}]", split[2]).to_ascii_uppercase();

        let ext_lines = match read_lines(&path) {
            Ok(v) => v,
            Err(_) => {
                // C# returns empty list if include script missing.
                return Ok(Vec::new());
            }
        };

        let mut start = false;
        let mut finish = false;
        let mut parsed: Vec<String> = Vec::new();

        for j in 0..ext_lines.len() {
            if !ext_lines[j].to_ascii_uppercase().starts_with(&page) {
                continue;
            }

            for x in (j + 1)..ext_lines.len() {
                let t = ext_lines[x].trim();
                if t == "{" {
                    start = true;
                    continue;
                }
                if t == "}" {
                    finish = true;
                    break;
                }
                parsed.push(ext_lines[x].clone());
            }
        }

        if start && finish {
            let insert_at = (i + 1).min(lines.len());
            lines.splice(insert_at..insert_at, parsed);
        }

        i += 1;
    }

    lines.retain(|l| !l.to_ascii_uppercase().starts_with("#INCLUDE"));
    Ok(lines)
}
