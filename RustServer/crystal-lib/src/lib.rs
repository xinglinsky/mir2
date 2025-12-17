use flate2::read::GzDecoder;
use std::fs;
use std::io::{self, Cursor, Read};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LibError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("invalid lib header")]
    InvalidHeader,
    #[error("unsupported lib version {0}")]
    UnsupportedVersion(i32),
    #[error("index out of range")]
    IndexOutOfRange,
    #[error("bad offset")]
    BadOffset,
    #[error("corrupt image header")]
    CorruptImageHeader,
    #[error("decompress failed")]
    DecompressFailed,
    #[error("unexpected decompressed size")]
    BadDecompressedSize,
}

#[derive(Clone, Debug)]
pub struct LibFile {
    data: Vec<u8>,
    pub version: i32,
    pub count: i32,
    index_list: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct LibImage {
    pub width: u16,
    pub height: u16,
    pub x: i16,
    pub y: i16,
    pub shadow_x: i16,
    pub shadow_y: i16,
    pub shadow: u8,
    pub has_mask: bool,
    pub pixels_bgra: Vec<u8>,
    pub mask_bgra: Option<Vec<u8>>, // same dimensions
}

impl LibImage {
    pub fn pixels_rgba(&self) -> Vec<u8> {
        bgra_to_rgba(&self.pixels_bgra)
    }

    pub fn mask_rgba(&self) -> Option<Vec<u8>> {
        self.mask_bgra.as_ref().map(|v| bgra_to_rgba(v))
    }
}

impl LibFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, LibError> {
        let data = fs::read(path)?;
        let mut cur = Cursor::new(data.as_slice());

        let version = read_i32(&mut cur).ok_or(LibError::InvalidHeader)?;
        if version < 2 {
            return Err(LibError::UnsupportedVersion(version));
        }
        let count = read_i32(&mut cur).ok_or(LibError::InvalidHeader)?;

        if count < 0 {
            return Err(LibError::InvalidHeader);
        }

        if version >= 3 {
            // frameSeek, not required for UI rendering.
            let _frame_seek = read_i32(&mut cur).ok_or(LibError::InvalidHeader)?;
        }

        let mut index_list = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let off = read_i32(&mut cur).ok_or(LibError::InvalidHeader)?;
            if off < 0 {
                return Err(LibError::InvalidHeader);
            }
            index_list.push(off as u32);
        }

        Ok(Self {
            data,
            version,
            count,
            index_list,
        })
    }

    pub fn get_image(&self, index: usize) -> Result<LibImage, LibError> {
        if index >= self.index_list.len() {
            return Err(LibError::IndexOutOfRange);
        }

        let offset = self.index_list[index] as usize;
        if offset + 17 > self.data.len() {
            return Err(LibError::BadOffset);
        }

        let mut cur = Cursor::new(&self.data[offset..]);
        let width = read_i16(&mut cur).ok_or(LibError::CorruptImageHeader)? as i16;
        let height = read_i16(&mut cur).ok_or(LibError::CorruptImageHeader)? as i16;
        let x = read_i16(&mut cur).ok_or(LibError::CorruptImageHeader)?;
        let y = read_i16(&mut cur).ok_or(LibError::CorruptImageHeader)?;
        let shadow_x = read_i16(&mut cur).ok_or(LibError::CorruptImageHeader)?;
        let shadow_y = read_i16(&mut cur).ok_or(LibError::CorruptImageHeader)?;
        let shadow = read_u8(&mut cur).ok_or(LibError::CorruptImageHeader)?;
        let length = read_i32(&mut cur).ok_or(LibError::CorruptImageHeader)?;

        if width <= 0 || height <= 0 {
            return Err(LibError::CorruptImageHeader);
        }
        if length < 0 {
            return Err(LibError::CorruptImageHeader);
        }

        let width_u = width as u16;
        let height_u = height as u16;

        let has_mask = (shadow & 0x80) != 0;
        let len_u = length as usize;

        // C# reads compressed image bytes starting at offset + 17.
        let data_start = offset + 17;
        let data_end = data_start.saturating_add(len_u);
        if data_end > self.data.len() {
            return Err(LibError::BadOffset);
        }

        let pixels_bgra = decompress_gzip_to_vec(&self.data[data_start..data_end])?;
        let expected = (width_u as usize)
            .saturating_mul(height_u as usize)
            .saturating_mul(4);
        if pixels_bgra.len() != expected {
            return Err(LibError::BadDecompressedSize);
        }

        // Mask layer: C# skips 12 bytes (mask header) then reads another `Length` bytes and decompresses.
        let mask_bgra = if has_mask {
            let mask_header_start = data_end;
            let mask_data_start = mask_header_start.saturating_add(12);
            let mask_data_end = mask_data_start.saturating_add(len_u);
            if mask_data_end > self.data.len() {
                return Err(LibError::BadOffset);
            }
            let m = decompress_gzip_to_vec(&self.data[mask_data_start..mask_data_end])?;
            if m.len() != expected {
                return Err(LibError::BadDecompressedSize);
            }
            Some(m)
        } else {
            None
        };

        Ok(LibImage {
            width: width_u,
            height: height_u,
            x,
            y,
            shadow_x,
            shadow_y,
            shadow,
            has_mask,
            pixels_bgra,
            mask_bgra,
        })
    }
}

fn decompress_gzip_to_vec(bytes: &[u8]) -> Result<Vec<u8>, LibError> {
    let mut d = GzDecoder::new(bytes);
    let mut out = Vec::new();
    d.read_to_end(&mut out).map_err(|_| LibError::DecompressFailed)?;
    Ok(out)
}

fn bgra_to_rgba(bgra: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; bgra.len()];
    for (src, dst) in bgra.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
        dst[0] = src[2];
        dst[1] = src[1];
        dst[2] = src[0];
        dst[3] = src[3];
    }
    out
}

fn read_i16(cur: &mut Cursor<&[u8]>) -> Option<i16> {
    let mut b = [0u8; 2];
    cur.read_exact(&mut b).ok()?;
    Some(i16::from_le_bytes(b))
}

fn read_i32(cur: &mut Cursor<&[u8]>) -> Option<i32> {
    let mut b = [0u8; 4];
    cur.read_exact(&mut b).ok()?;
    Some(i32::from_le_bytes(b))
}

fn read_u8(cur: &mut Cursor<&[u8]>) -> Option<u8> {
    let mut b = [0u8; 1];
    cur.read_exact(&mut b).ok()?;
    Some(b[0])
}
