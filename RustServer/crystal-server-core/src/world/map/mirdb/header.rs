use std::io::{self, Read};

use crate::stats::{Stat, Stats};

pub(super) fn read_i32<R: Read>(r: &mut R) -> io::Result<i32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

pub(super) fn read_u32<R: Read>(r: &mut R) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

pub(super) fn read_i64<R: Read>(r: &mut R) -> io::Result<i64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(i64::from_le_bytes(buf))
}

pub(super) fn read_u64<R: Read>(r: &mut R) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

pub(super) fn read_i16<R: Read>(r: &mut R) -> io::Result<i16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(i16::from_le_bytes(buf))
}

pub(super) fn read_u16<R: Read>(r: &mut R) -> io::Result<u16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

pub(super) fn read_u8<R: Read>(r: &mut R) -> io::Result<u8> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

pub(super) fn read_bool<R: Read>(r: &mut R) -> io::Result<bool> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0] != 0)
}

pub(super) fn read_string<R: Read>(r: &mut R) -> io::Result<String> {
    let byte_len = read_7bit_int(r)?;
    if byte_len < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative string length {}", byte_len),
        ));
    }
    let mut buf = vec![0u8; byte_len as usize];
    r.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub(super) fn read_f32<R: Read>(r: &mut R) -> io::Result<f32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
}

pub(super) fn read_f64<R: Read>(r: &mut R) -> io::Result<f64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(f64::from_le_bytes(buf))
}

pub(super) fn read_7bit_int<R: Read>(r: &mut R) -> io::Result<i32> {
    let mut count: i32 = 0;
    let mut shift = 0;

    loop {
        if shift >= 35 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid 7-bit encoded int (too many bytes)",
            ));
        }

        let mut b = [0u8; 1];
        r.read_exact(&mut b)?;
        let byte = b[0];

        count |= ((byte & 0x7F) as i32) << shift;
        if (byte & 0x80) == 0 {
            break;
        }

        shift += 7;
    }

    Ok(count)
}

pub(super) fn read_stats<R: Read>(r: &mut R) -> io::Result<Stats> {
    // Matches Shared/Data/Stat.cs Stats.Save
    let count = read_i32(r)?;
    if count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative Stats count {}", count),
        ));
    }

    let mut stats = Stats::default();
    for _ in 0..count {
        let stat_id = read_u8(r)?;
        let value = read_i32(r)?;
        let stat = Stat::from_u8(stat_id);
        stats.set(stat, value);
    }

    Ok(stats)
}

pub(super) fn skip_stats<R: Read>(r: &mut R) -> io::Result<()> {
    let _ = read_stats(r)?;
    Ok(())
}
