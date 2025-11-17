// IO helpers compatible with .NET BinaryReader/BinaryWriter for basic types.
// Especially string encoding using 7-bit encoded length + UTF-8 bytes.

use std::io::{self, Read, Write};

pub fn read_i32_le<R: Read>(r: &mut R) -> io::Result<i32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

pub fn write_i32_le<W: Write>(w: &mut W, value: i32) -> io::Result<()> {
    w.write_all(&value.to_le_bytes())
}

pub fn read_u16_le<R: Read>(r: &mut R) -> io::Result<u16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

pub fn write_u16_le<W: Write>(w: &mut W, value: u16) -> io::Result<()> {
    w.write_all(&value.to_le_bytes())
}

pub fn read_u32_le<R: Read>(r: &mut R) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

pub fn write_u32_le<W: Write>(w: &mut W, value: u32) -> io::Result<()> {
    w.write_all(&value.to_le_bytes())
}

pub fn read_i64_le<R: Read>(r: &mut R) -> io::Result<i64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(i64::from_le_bytes(buf))
}

pub fn write_i64_le<W: Write>(w: &mut W, value: i64) -> io::Result<()> {
    w.write_all(&value.to_le_bytes())
}

pub fn read_bool<R: Read>(r: &mut R) -> io::Result<bool> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b)?;
    Ok(b[0] != 0)
}

pub fn write_bool<W: Write>(w: &mut W, value: bool) -> io::Result<()> {
    w.write_all(&[value as u8])
}

fn read_7bit_encoded_int<R: Read>(r: &mut R) -> io::Result<i32> {
    // Mirrors .NET BinaryReader.Read7BitEncodedInt implementation.
    let mut count: i32 = 0;
    let mut shift = 0;

    loop {
        if shift >= 35 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "7-bit encoded int is too large",
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

fn write_7bit_encoded_int<W: Write>(w: &mut W, value: i32) -> io::Result<()> {
    // Mirrors .NET BinaryWriter.Write7BitEncodedInt implementation.
    let mut v = value as u32;
    while v >= 0x80 {
        w.write_all(&[((v as u8) | 0x80)])?;
        v >>= 7;
    }
    w.write_all(&[v as u8])
}

pub fn read_string<R: Read>(r: &mut R) -> io::Result<String> {
    let byte_count = read_7bit_encoded_int(r)?;
    if byte_count < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "negative string length",
        ));
    }
    let mut buf = vec![0u8; byte_count as usize];
    r.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn write_string<W: Write>(w: &mut W, value: &str) -> io::Result<()> {
    let bytes = value.as_bytes();
    let len: i32 = bytes
        .len()
        .try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "string too long"))?;
    write_7bit_encoded_int(w, len)?;
    w.write_all(bytes)
}
