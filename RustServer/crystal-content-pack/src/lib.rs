use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const PACK_MAGIC: &[u8; 8] = b"CRYPACK1";
const PACK_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackManifestEntry {
    pub path: String,
    pub kind: String,
    pub sha256: String,
    pub size: u64,
    pub is_binary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackManifest {
    pub format: String,
    pub version: u32,
    pub snapshot_id: i64,
    pub created_at_epoch: i64,
    pub entries: Vec<PackManifestEntry>,
}

#[derive(Debug, Clone)]
struct IndexEntry {
    offset: u64,
    size: u64,
    sha256_raw: [u8; 32],
    flags: u8,
}

#[derive(Debug)]
pub enum PackError {
    Io(io::Error),
    Json(serde_json::Error),
    Utf8(std::string::FromUtf8Error),
    InvalidMagic,
    UnsupportedVersion(u32),
    Malformed(&'static str),
    NotFound(String),
    ShaMismatch(String),
}

impl From<io::Error> for PackError {
    fn from(e: io::Error) -> Self {
        PackError::Io(e)
    }
}

impl From<serde_json::Error> for PackError {
    fn from(e: serde_json::Error) -> Self {
        PackError::Json(e)
    }
}

impl From<std::string::FromUtf8Error> for PackError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        PackError::Utf8(e)
    }
}

fn read_u32(r: &mut dyn Read) -> Result<u32, PackError> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64(r: &mut dyn Read) -> Result<u64, PackError> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_exact_vec(r: &mut dyn Read, len: usize) -> Result<Vec<u8>, PackError> {
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    let mut raw = [0u8; 32];
    raw.copy_from_slice(&out);
    raw
}

pub struct ContentPack {
    file: File,
    manifest: PackManifest,
    index: HashMap<String, IndexEntry>,
}

impl ContentPack {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, PackError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(&file);

        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        if &magic != PACK_MAGIC {
            return Err(PackError::InvalidMagic);
        }

        let version = read_u32(&mut reader)?;
        if version != PACK_VERSION {
            return Err(PackError::UnsupportedVersion(version));
        }

        let manifest_len = read_u32(&mut reader)? as usize;
        let manifest_bytes = read_exact_vec(&mut reader, manifest_len)?;
        let manifest: PackManifest = serde_json::from_slice(&manifest_bytes)?;

        let entry_count = read_u32(&mut reader)? as usize;

        let mut index = HashMap::with_capacity(entry_count);
        for _ in 0..entry_count {
            let path_len = read_u32(&mut reader)? as usize;
            let path_bytes = read_exact_vec(&mut reader, path_len)?;
            let path = String::from_utf8(path_bytes)?;

            let mut flag_buf = [0u8; 1];
            reader.read_exact(&mut flag_buf)?;
            let flags = flag_buf[0];

            let offset = read_u64(&mut reader)?;
            let size = read_u64(&mut reader)?;
            let mut sha_raw = [0u8; 32];
            reader.read_exact(&mut sha_raw)?;

            index.insert(
                path.to_lowercase(),
                IndexEntry {
                    offset,
                    size,
                    sha256_raw: sha_raw,
                    flags,
                },
            );
        }

        Ok(ContentPack {
            file,
            manifest,
            index,
        })
    }

    pub fn manifest(&self) -> &PackManifest {
        &self.manifest
    }

    pub fn contains(&self, path: &str) -> bool {
        self.index.contains_key(&path.to_lowercase())
    }

    pub fn read_bytes(&self, path: &str, verify_sha: bool) -> Result<Vec<u8>, PackError> {
        let key = path.to_lowercase();
        let ent = self
            .index
            .get(&key)
            .ok_or_else(|| PackError::NotFound(path.to_string()))?;

        let mut f = self.file.try_clone()?;
        let mut reader = BufReader::new(&mut f);
        reader.seek(SeekFrom::Start(ent.offset))?;
        let mut buf = vec![0u8; ent.size as usize];
        reader.read_exact(&mut buf)?;

        if verify_sha {
            let sha = sha256_bytes(&buf);
            if sha != ent.sha256_raw {
                return Err(PackError::ShaMismatch(path.to_string()));
            }
        }

        Ok(buf)
    }

    pub fn read_text_utf8(&self, path: &str, verify_sha: bool) -> Result<String, PackError> {
        let bytes = self.read_bytes(path, verify_sha)?;
        let s = String::from_utf8(bytes).map_err(PackError::Utf8)?;
        Ok(s)
    }

    pub fn is_binary(&self, path: &str) -> Result<bool, PackError> {
        let key = path.to_lowercase();
        let ent = self
            .index
            .get(&key)
            .ok_or_else(|| PackError::NotFound(path.to_string()))?;
        Ok((ent.flags & 1) != 0)
    }
}
