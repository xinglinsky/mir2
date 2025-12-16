use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use encoding_rs::{GB18030, UTF_16BE, UTF_16LE};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

mod content_db;
mod pack;

use content_db::ContentDb;

#[derive(Serialize)]
struct ExportMetaEntry {
    path: String,
    kind: String,
    revision_id: i64,
    sha256: String,
    original_encoding: String,
    is_binary: bool,
    size: u64,
}

#[derive(Serialize)]
struct ExportMeta {
    snapshot_id: i64,
    exported_at_epoch: i64,
    entries: Vec<ExportMetaEntry>,
}

#[derive(Deserialize)]
struct ImportMetaFile {
    snapshot_id: i64,
    entries: Vec<ImportMetaEntry>,
}

#[derive(Deserialize)]
struct ImportMetaEntry {
    path: String,
    kind: String,
    sha256: String,
    is_binary: bool,
    size: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else {
        return Err("missing command".into());
    };

    match cmd.as_str() {
        "import" => cmd_import(args.collect()),
        "snapshot-create" => cmd_snapshot_create(args.collect()),
        "snapshot-export" => cmd_snapshot_export(args.collect()),
        "pack-build" => cmd_pack_build(args.collect()),
        "pack-verify" => cmd_pack_verify(args.collect()),
        _ => Err(format!("unknown command: {}", cmd).into()),
    }
}

fn now_epoch() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn cmd_import(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut db_path: Option<PathBuf> = None;
    let mut jev_root: Option<PathBuf> = None;
    let mut include_optional_raw = false;

    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--db" => db_path = it.next().map(PathBuf::from),
            "--jev-root" => jev_root = it.next().map(PathBuf::from),
            "--include-raw" => include_optional_raw = true,
            _ => return Err(format!("unknown arg: {}", a).into()),
        }
    }

    let db_path = db_path.ok_or("--db is required")?;
    let jev_root = jev_root.ok_or("--jev-root is required")?;

    let db = ContentDb::open(&db_path)?;

    let mut include_roots: Vec<(PathBuf, &'static str, Vec<&'static str>)> = vec![
        (jev_root.join("Configs"), "config", vec!["ini", "txt"]),
        (jev_root.join("Envir").join("Drops"), "drop", vec!["txt"]),
        (jev_root.join("Envir").join("NPCs"), "npc_script", vec!["txt"]),
        (jev_root.join("Envir").join("SystemScripts"), "npc_system_script", vec!["txt"]),
        (jev_root.join("Envir").join("Quests"), "quest", vec!["txt"]),
        (jev_root.join("Envir").join("Recipe"), "recipe", vec!["txt"]),
    ];

    if include_optional_raw {
        include_roots.push((jev_root.join("Envir").join("Events"), "raw", vec!["txt"]));
        include_roots.push((jev_root.join("Envir").join("Routes"), "raw", vec!["txt"]));
        include_roots.push((jev_root.join("Envir").join("Values"), "raw", vec!["txt", "ini"]));
        include_roots.push((jev_root.join("Envir").join("NameLists"), "raw", vec!["txt"]));
    }

    let created_at = now_epoch();

    for (root, kind, exts) in include_roots {
        if !root.exists() {
            continue;
        }

        for entry in WalkDir::new(&root).follow_links(false) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();

            if is_ignored_file(path) {
                continue;
            }

            let ext = path.extension().and_then(|x| x.to_str()).unwrap_or("");
            if !exts.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                continue;
            }

            let rel = path.strip_prefix(&jev_root)?;
            let rel_norm = normalize_rel_path(rel);
            let rel_lower = rel_norm.to_lowercase();

            let bytes = fs::read(path)?;
            let (normalized, encoding) = decode_to_utf8_normalized(&bytes)?;
            let sha = sha256_hex(normalized.as_bytes());

            let item_id = db.upsert_item(&rel_norm, &rel_lower, kind, created_at)?;
            let latest = db.latest_revision_hash(item_id)?;
            if latest.as_deref() == Some(&sha) {
                continue;
            }

            let _rev_id = db.insert_revision_text(
                item_id,
                &sha,
                &encoding,
                &normalized,
                created_at,
                "import",
            )?;
        }
    }

    // Maps: store as binary blobs (no encoding normalization).
    let maps_root = jev_root.join("Maps");
    if maps_root.exists() {
        for entry in WalkDir::new(&maps_root).follow_links(false) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if is_ignored_file(path) {
                continue;
            }

            let ext = path.extension().and_then(|x| x.to_str()).unwrap_or("");
            if !ext.eq_ignore_ascii_case("map") {
                continue;
            }

            let rel = path.strip_prefix(&jev_root)?;
            let rel_norm = normalize_rel_path(rel);
            let rel_lower = rel_norm.to_lowercase();

            let bytes = fs::read(path)?;
            let sha = sha256_hex(&bytes);

            let item_id = db.upsert_item(&rel_norm, &rel_lower, "map_bin", created_at)?;
            let latest = db.latest_revision_hash(item_id)?;
            if latest.as_deref() == Some(&sha) {
                continue;
            }

            let _rev_id = db.insert_revision_blob(
                item_id,
                &sha,
                "binary",
                &bytes,
                created_at,
                "import",
            )?;
        }
    }

    Ok(())
}

fn cmd_snapshot_create(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut db_path: Option<PathBuf> = None;
    let mut note = String::from("snapshot");

    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--db" => db_path = it.next().map(PathBuf::from),
            "--note" => note = it.next().unwrap_or(note),
            _ => return Err(format!("unknown arg: {}", a).into()),
        }
    }

    let db_path = db_path.ok_or("--db is required")?;
    let db = ContentDb::open(&db_path)?;

    let snapshot_id = db.create_snapshot(&note, now_epoch())?;

    let include_kinds = [
        "config",
        "drop",
        "npc_script",
        "npc_system_script",
        "quest",
        "recipe",
        "map_bin",
    ];
    db.populate_snapshot_with_latest(snapshot_id, &include_kinds)?;

    println!("{}", snapshot_id);
    Ok(())
}

fn cmd_snapshot_export(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut db_path: Option<PathBuf> = None;
    let mut snapshot_id: Option<i64> = None;
    let mut out_dir: Option<PathBuf> = None;

    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--db" => db_path = it.next().map(PathBuf::from),
            "--snapshot" => {
                let v = it.next().ok_or("--snapshot needs a value")?;
                snapshot_id = Some(v.parse()?);
            }
            "--out" => out_dir = it.next().map(PathBuf::from),
            _ => return Err(format!("unknown arg: {}", a).into()),
        }
    }

    let db_path = db_path.ok_or("--db is required")?;
    let snapshot_id = snapshot_id.ok_or("--snapshot is required")?;
    let out_dir = out_dir.ok_or("--out is required")?;

    let db = ContentDb::open(&db_path)?;

    let files_dir = out_dir.join("files");
    fs::create_dir_all(&files_dir)?;

    let mut stmt = db.conn().prepare(
        "SELECT se.path, se.kind, se.revision_id, se.sha256, r.original_encoding, r.is_binary, r.size, r.content_utf8, r.content_blob\n         FROM snapshot_entries se\n         JOIN content_revisions r ON r.id = se.revision_id\n         WHERE se.snapshot_id = ?1",
    )?;

    let mut entries_out = Vec::new();

    let rows = stmt.query_map([snapshot_id], |row| {
        let path: String = row.get(0)?;
        let kind: String = row.get(1)?;
        let revision_id: i64 = row.get(2)?;
        let sha256: String = row.get(3)?;
        let original_encoding: String = row.get(4)?;
        let is_binary_i: i64 = row.get(5)?;
        let size: i64 = row.get(6)?;
        let content_utf8: String = row.get(7)?;
        let content_blob: Option<Vec<u8>> = row.get(8)?;
        Ok((
            path,
            kind,
            revision_id,
            sha256,
            original_encoding,
            is_binary_i != 0,
            size,
            content_utf8,
            content_blob,
        ))
    })?;

    for row in rows {
        let (rel_path, kind, revision_id, sha256, original_encoding, is_binary, size, content_utf8, content_blob) = row?;
        let dst = files_dir.join(rel_path.replace('/', &std::path::MAIN_SEPARATOR.to_string()));
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }

        if is_binary {
            let blob = content_blob.ok_or("missing content_blob for binary entry")?;
            fs::write(&dst, &blob)?;
        } else {
            fs::write(&dst, content_utf8.as_bytes())?;
        }

        entries_out.push(ExportMetaEntry {
            path: rel_path,
            kind,
            revision_id,
            sha256,
            original_encoding,
            is_binary,
            size: size as u64,
        });
    }

    let meta = ExportMeta {
        snapshot_id,
        exported_at_epoch: now_epoch(),
        entries: entries_out,
    };

    let meta_json = serde_json::to_vec_pretty(&meta)?;
    fs::write(out_dir.join("meta.json"), meta_json)?;

    Ok(())
}

fn cmd_pack_build(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut snapshot_dir: Option<PathBuf> = None;
    let mut out_pack: Option<PathBuf> = None;
    let mut created_at_epoch: i64 = now_epoch();

    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--snapshot-dir" => snapshot_dir = it.next().map(PathBuf::from),
            "--out" => out_pack = it.next().map(PathBuf::from),
            "--created-at" => {
                let v = it.next().ok_or("--created-at needs a value")?;
                created_at_epoch = v.parse()?;
            }
            _ => return Err(format!("unknown arg: {}", a).into()),
        }
    }

    let snapshot_dir = snapshot_dir.ok_or("--snapshot-dir is required")?;
    let out_pack = out_pack.ok_or("--out is required")?;

    let meta_path = snapshot_dir.join("meta.json");
    let meta_bytes = fs::read(&meta_path)?;
    let mut meta: ImportMetaFile = serde_json::from_slice(&meta_bytes)?;

    meta.entries.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));

    let mut entries = Vec::with_capacity(meta.entries.len());
    let mut blobs = Vec::with_capacity(meta.entries.len());

    for e in &meta.entries {
        let file_path = snapshot_dir
            .join("files")
            .join(e.path.replace('/', &std::path::MAIN_SEPARATOR.to_string()));

        let bytes = fs::read(&file_path)?;
        let sha = sha256_hex(&bytes);
        if sha != e.sha256 {
            return Err(format!("snapshot file sha mismatch: {}", e.path).into());
        }
        if bytes.len() as u64 != e.size {
            return Err(format!("snapshot file size mismatch: {}", e.path).into());
        }

        entries.push(pack::PackManifestEntry {
            path: e.path.clone(),
            kind: e.kind.clone(),
            sha256: e.sha256.clone(),
            size: e.size,
            is_binary: e.is_binary,
        });
        blobs.push(bytes);
    }

    let manifest = pack::PackManifest {
        format: "crystal-pack".to_string(),
        version: 1,
        snapshot_id: meta.snapshot_id,
        created_at_epoch,
        entries,
    };

    pack::build_pack_from_files(&out_pack, manifest, blobs)?;
    Ok(())
}

fn cmd_pack_verify(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut pack_path: Option<PathBuf> = None;

    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--pack" => pack_path = it.next().map(PathBuf::from),
            _ => return Err(format!("unknown arg: {}", a).into()),
        }
    }

    let pack_path = pack_path.ok_or("--pack is required")?;
    pack::verify_pack(&pack_path)?;
    Ok(())
}

fn is_ignored_file(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|x| x.to_str()) {
        let lower = name.to_lowercase();
        if lower == "desktop.ini" {
            return true;
        }
        if lower.ends_with(".rar")
            || lower.ends_with(".bak")
            || lower.ends_with(".old")
            || lower.ends_with(".tmp")
        {
            return true;
        }
    }
    false
}

fn normalize_rel_path(rel: &Path) -> String {
    let s = rel.to_string_lossy().replace('\\', "/");
    s
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    hex::encode(out)
}

fn decode_to_utf8_normalized(bytes: &[u8]) -> Result<(String, String), Box<dyn std::error::Error>> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let s = std::str::from_utf8(&bytes[3..])?;
        return Ok((normalize_newlines(s), "utf-8-bom".to_string()));
    }

    if bytes.starts_with(&[0xFF, 0xFE]) {
        let (cow, _, _) = UTF_16LE.decode(&bytes[2..]);
        return Ok((normalize_newlines(&cow), "utf-16le".to_string()));
    }

    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (cow, _, _) = UTF_16BE.decode(&bytes[2..]);
        return Ok((normalize_newlines(&cow), "utf-16be".to_string()));
    }

    if let Ok(s) = std::str::from_utf8(bytes) {
        return Ok((normalize_newlines(s), "utf-8".to_string()));
    }

    let (cow, _, _) = GB18030.decode(bytes);
    Ok((normalize_newlines(&cow), "gb18030".to_string()))
}

fn normalize_newlines(s: &str) -> String {
    let s = s.replace("\r\n", "\n");
    let s = s.replace('\r', "\n");
    s
}

fn _io_err(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, msg)
}
