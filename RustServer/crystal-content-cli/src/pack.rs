use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
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
pub struct PackEntry {
    pub path: String,
    pub offset: u64,
    pub size: u64,
    pub sha256_raw: [u8; 32],
    pub flags: u8,
}

fn write_u32(w: &mut dyn Write, v: u32) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

fn write_u64(w: &mut dyn Write, v: u64) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

fn read_u32(r: &mut dyn Read) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64(r: &mut dyn Read) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_exact_vec(r: &mut dyn Read, len: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

fn sha256_stream(reader: &mut dyn Read) -> io::Result<[u8; 32]> {
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let out = hasher.finalize();
    let mut raw = [0u8; 32];
    raw.copy_from_slice(&out);
    Ok(raw)
}

pub fn build_pack_from_files(
    out_path: &Path,
    manifest: PackManifest,
    file_bytes: Vec<Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if manifest.entries.len() != file_bytes.len() {
        return Err("manifest entries and file list length mismatch".into());
    }

    let manifest_json = serde_json::to_vec(&manifest)?;

    let mut writer = BufWriter::new(File::create(out_path)?);

    writer.write_all(PACK_MAGIC)?;
    write_u32(&mut writer, PACK_VERSION)?;
    write_u32(&mut writer, manifest_json.len() as u32)?;
    writer.write_all(&manifest_json)?;

    let entry_count = manifest.entries.len() as u32;
    write_u32(&mut writer, entry_count)?;

    let mut entries: Vec<PackEntry> = Vec::with_capacity(entry_count as usize);

    for e in &manifest.entries {
        let mut hasher = Sha256::new();
        hasher.update(e.path.as_bytes());
        let _ = hasher.finalize();
    }

    let mut header_size = 0u64;
    header_size += 8;
    header_size += 4;
    header_size += 4;
    header_size += manifest_json.len() as u64;
    header_size += 4;

    for e in &manifest.entries {
        header_size += 4;
        header_size += e.path.as_bytes().len() as u64;
        header_size += 1;
        header_size += 8;
        header_size += 8;
        header_size += 32;
    }

    let mut cur_offset = header_size;
    for (idx, e) in manifest.entries.iter().enumerate() {
        let bytes = &file_bytes[idx];
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let out = hasher.finalize();
        let mut sha_raw = [0u8; 32];
        sha_raw.copy_from_slice(&out);

        let flags = if e.is_binary { 1u8 } else { 0u8 };

        entries.push(PackEntry {
            path: e.path.clone(),
            offset: cur_offset,
            size: bytes.len() as u64,
            sha256_raw: sha_raw,
            flags,
        });

        cur_offset += bytes.len() as u64;
    }

    for ent in &entries {
        write_u32(&mut writer, ent.path.as_bytes().len() as u32)?;
        writer.write_all(ent.path.as_bytes())?;
        writer.write_all(&[ent.flags])?;
        write_u64(&mut writer, ent.offset)?;
        write_u64(&mut writer, ent.size)?;
        writer.write_all(&ent.sha256_raw)?;
    }

    for bytes in file_bytes {
        writer.write_all(&bytes)?;
    }

    writer.flush()?;
    Ok(())
}

pub fn verify_pack(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut magic = [0u8; 8];
    reader.read_exact(&mut magic)?;
    if &magic != PACK_MAGIC {
        return Err("invalid pack magic".into());
    }

    let version = read_u32(&mut reader)?;
    if version != PACK_VERSION {
        return Err(format!("unsupported pack version: {}", version).into());
    }

    let manifest_len = read_u32(&mut reader)? as usize;
    let manifest_bytes = read_exact_vec(&mut reader, manifest_len)?;
    let manifest: PackManifest = serde_json::from_slice(&manifest_bytes)?;

    let entry_count = read_u32(&mut reader)? as usize;

    let mut entries: Vec<PackEntry> = Vec::with_capacity(entry_count);
    let mut seen = HashSet::new();

    for _ in 0..entry_count {
        let path_len = read_u32(&mut reader)? as usize;
        let path_bytes = read_exact_vec(&mut reader, path_len)?;
        let path = String::from_utf8(path_bytes)?;
        let mut flags_buf = [0u8; 1];
        reader.read_exact(&mut flags_buf)?;
        let flags = flags_buf[0];
        let offset = read_u64(&mut reader)?;
        let size = read_u64(&mut reader)?;
        let mut sha_raw = [0u8; 32];
        reader.read_exact(&mut sha_raw)?;

        let key = path.to_lowercase();
        if !seen.insert(key) {
            return Err(format!("duplicate path in pack: {}", path).into());
        }

        entries.push(PackEntry {
            path,
            offset,
            size,
            sha256_raw: sha_raw,
            flags,
        });
    }

    let data_start = reader.stream_position()?;
    let file_len = reader.get_ref().metadata()?.len();

    for ent in &entries {
        if ent.offset < data_start {
            return Err(format!("entry offset before data region: {}", ent.path).into());
        }
        if ent.offset.checked_add(ent.size).unwrap_or(u64::MAX) > file_len {
            return Err(format!("entry range out of bounds: {}", ent.path).into());
        }

        reader.seek(SeekFrom::Start(ent.offset))?;
        let mut limited = reader.by_ref().take(ent.size);
        let sha = sha256_stream(&mut limited)?;
        if sha != ent.sha256_raw {
            return Err(format!("sha256 mismatch: {}", ent.path).into());
        }
    }

    if manifest.entries.len() != entries.len() {
        return Err("manifest entry count mismatch".into());
    }

    Ok(())
}
