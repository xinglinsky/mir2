use std::fs;
use std::io;
use std::path::Path;
use std::sync::{Arc, RwLock};

use once_cell::sync::Lazy;

use crystal_content_pack::{ContentPack, PackError};

static CONTENT_PACK: Lazy<RwLock<Option<Arc<ContentPack>>>> = Lazy::new(|| RwLock::new(None));

fn pack_error_to_io(e: PackError) -> io::Error {
    io::Error::new(io::ErrorKind::Other, format!("content pack error: {:?}", e))
}

fn normalize_pack_path(path: &Path) -> Option<String> {
    let mut s = path.to_string_lossy().replace('\\', "/");

    if let Some(stripped) = s.strip_prefix("./") {
        s = stripped.to_string();
    }
    if let Some(stripped) = s.strip_prefix("/") {
        s = stripped.to_string();
    }

    let lower = s.to_ascii_lowercase();

    if lower.ends_with("server.mirdb") {
        return Some("Server.MirDB".to_string());
    }
    for marker in ["/configs/", "/envir/", "/maps/"] {
        if let Some(pos) = lower.find(marker) {
            // Keep the directory name (Configs/..., Envir/..., Maps/...).
            let start = pos + 1;
            return Some(s[start..].to_string());
        }
    }

    // For the common case where the caller passes a relative path like
    // "Configs/Setup.ini" or "Maps/0.map".
    if lower.starts_with("configs/") || lower.starts_with("envir/") || lower.starts_with("maps/") {
        return Some(s);
    }

    None
}

pub fn set_content_pack(pack: Option<Arc<ContentPack>>) {
    let mut guard = CONTENT_PACK.write().unwrap();
    *guard = pack;
}

pub fn load_content_pack<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let pack = ContentPack::open(path).map_err(pack_error_to_io)?;
    set_content_pack(Some(Arc::new(pack)));
    Ok(())
}

pub fn has_content_pack() -> bool {
    CONTENT_PACK.read().unwrap().is_some()
}

pub fn list_pack_paths_with_prefix(prefix: &str) -> Vec<String> {
    let mut p = prefix.replace('\\', "/");

    if let Some(stripped) = p.strip_prefix("./") {
        p = stripped.to_string();
    }
    if let Some(stripped) = p.strip_prefix("/") {
        p = stripped.to_string();
    }
    if !p.ends_with('/') {
        p.push('/');
    }

    let p_lower = p.to_ascii_lowercase();

    let guard = CONTENT_PACK.read().unwrap();
    let Some(pack) = guard.as_ref() else {
        return Vec::new();
    };

    let mut out: Vec<String> = pack
        .manifest()
        .entries
        .iter()
        .map(|e| e.path.clone())
        .filter(|path| path.to_ascii_lowercase().starts_with(&p_lower))
        .collect();

    out.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));
    out
 }

pub fn read_bytes(path: &Path) -> io::Result<Vec<u8>> {
    if let Some(pack_path) = normalize_pack_path(path) {
        if let Some(pack) = CONTENT_PACK.read().unwrap().as_ref() {
            if pack.contains(&pack_path) {
                return pack.read_bytes(&pack_path, true).map_err(pack_error_to_io);
            }
        }
    }

    fs::read(path)
}

pub fn read_to_string(path: &Path) -> io::Result<String> {
    if let Some(pack_path) = normalize_pack_path(path) {
        if let Some(pack) = CONTENT_PACK.read().unwrap().as_ref() {
            if pack.contains(&pack_path) {
                return pack
                    .read_text_utf8(&pack_path, true)
                    .map_err(pack_error_to_io);
            }
        }
    }

    fs::read_to_string(path)
}
