use std::io::{self, Cursor, Read, Write};

use crate::io::{read_string, write_string};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

/// Notice payload used by the legacy C# UpdateNotice packet.
/// Mirrors Shared/Notice.cs (Title + Message only; LastUpdate is kept
/// server-side and is not serialized).
#[derive(Clone, Debug)]
pub struct NoticeData {
    pub title: String,
    pub message: String,
}

impl NoticeData {
    pub fn decode_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let title = read_string(r)?;
        let message = read_string(r)?;
        Ok(NoticeData { title, message })
    }

    pub fn encode_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write_string(w, &self.title)?;
        write_string(w, &self.message)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct SUpdateNotice {
    pub notice: NoticeData,
}

impl SUpdateNotice {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        self.notice.encode_to(&mut buf)?;
        Ok(RawPacket {
            id: ServerPacketId::UpdateNotice as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let notice = NoticeData::decode_from(&mut c)?;
        Ok(SUpdateNotice { notice })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_notice_roundtrip() {
        let notice = NoticeData {
            title: "Welcome".to_string(),
            message: "Hello from Rust server".to_string(),
        };
        let pkt = SUpdateNotice { notice };
        let raw = pkt.encode().expect("encode SUpdateNotice");
        assert_eq!(raw.id, ServerPacketId::UpdateNotice as i16);

        let decoded = SUpdateNotice::decode(&raw.payload).expect("decode SUpdateNotice");
        assert_eq!(decoded.notice.title, "Welcome");
        assert_eq!(decoded.notice.message, "Hello from Rust server");
    }
}
