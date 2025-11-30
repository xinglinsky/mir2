use std::io::{self, Cursor, Read, Write};

use crate::io::{
    read_i32_le,
    read_string,
    write_i32_le,
    write_string,
};
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

/// Open a browser on the client with the given URL, mirroring
/// Shared/ServerPackets.OpenBrowser.
#[derive(Clone, Debug)]
pub struct SOpenBrowser {
    pub url: String,
}

impl SOpenBrowser {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.url)?;
        Ok(RawPacket {
            id: ServerPacketId::OpenBrowser as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let url = read_string(&mut c)?;
        Ok(SOpenBrowser { url })
    }
}

/// Play a client-side sound effect, mirroring Shared/ServerPackets.PlaySound.
#[derive(Clone, Debug)]
pub struct SPlaySound {
    pub sound: i32,
}

impl SPlaySound {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, self.sound)?;
        Ok(RawPacket {
            id: ServerPacketId::PlaySound as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let sound = read_i32_le(&mut c)?;
        Ok(SPlaySound { sound })
    }
}

/// Configure a client-side timer entry, mirroring Shared/ServerPackets.SetTimer.
#[derive(Clone, Debug)]
pub struct SSetTimer {
    pub key: String,
    pub type_id: u8,
    pub seconds: i32,
}

impl SSetTimer {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.key)?;
        buf.push(self.type_id);
        write_i32_le(&mut buf, self.seconds)?;
        Ok(RawPacket {
            id: ServerPacketId::SetTimer as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let key = read_string(&mut c)?;
        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let type_id = one[0];
        let seconds = read_i32_le(&mut c)?;
        Ok(SSetTimer {
            key,
            type_id,
            seconds,
        })
    }
}

/// Expire a client-side timer, mirroring Shared/ServerPackets.ExpireTimer.
#[derive(Clone, Debug)]
pub struct SExpireTimer {
    pub key: String,
}

impl SExpireTimer {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_string(&mut buf, &self.key)?;
        Ok(RawPacket {
            id: ServerPacketId::ExpireTimer as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);
        let key = read_string(&mut c)?;
        Ok(SExpireTimer { key })
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
