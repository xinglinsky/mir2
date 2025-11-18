// Mail-related packets (ReceiveMail, MailCost, MailSendRequest, MailSent).
// Complex mail structures (ClientMail, UserItem) are currently treated as opaque
// byte blobs so that we can get protocol compatibility first and fill in
// strong types later.

use std::io;

use crate::io::{
    read_bool, read_u32_le, read_u64_le, write_bool, write_u32_le, write_u64_le,
};
use crate::login::ServerPacketId;
use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct SReceiveMail {
    /// Raw bytes representing the entire ReceiveMail payload as written by the
    /// C# server: count + repeated ClientMail.Save(writer) blobs.
    pub mail_bytes: Vec<u8>,
}

impl SReceiveMail {
    pub fn encode(&self) -> io::Result<RawPacket> {
        Ok(RawPacket {
            id: ServerPacketId::ReceiveMail as i16,
            payload: self.mail_bytes.clone(),
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        Ok(SReceiveMail {
            mail_bytes: payload.to_vec(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SMailCost {
    pub cost: u32,
}

impl SMailCost {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, self.cost)?;
        Ok(RawPacket {
            id: ServerPacketId::MailCost as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 4 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SMailCost payload must be exactly 4 bytes",
            ));
        }
        let mut c = std::io::Cursor::new(payload);
        let cost = read_u32_le(&mut c)?;
        Ok(SMailCost { cost })
    }
}

#[derive(Clone, Debug)]
pub struct SMailSendRequest;

impl SMailSendRequest {
    pub fn encode(&self) -> RawPacket {
        RawPacket {
            id: ServerPacketId::MailSendRequest as i16,
            payload: Vec::new(),
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if !payload.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SMailSendRequest payload must be empty",
            ));
        }
        Ok(SMailSendRequest)
    }
}

#[derive(Clone, Debug)]
pub struct SMailSent {
    pub result: i8,
}

impl SMailSent {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.result as u8);
        RawPacket {
            id: ServerPacketId::MailSent as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SMailSent payload must be exactly 1 byte",
            ));
        }
        Ok(SMailSent {
            result: payload[0] as i8,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SMailLockedItem {
    pub unique_id: u64,
    pub locked: bool,
}

impl SMailLockedItem {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        write_u64_le(&mut buf, self.unique_id)?;
        write_bool(&mut buf, self.locked)?;
        Ok(RawPacket {
            id: ServerPacketId::MailLockedItem as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = std::io::Cursor::new(payload);
        let unique_id = read_u64_le(&mut c)?;
        let locked = read_bool(&mut c)?;
        Ok(SMailLockedItem { unique_id, locked })
    }
}

#[derive(Clone, Debug)]
pub struct SParcelCollected {
    pub result: i8,
}

impl SParcelCollected {
    pub fn encode(&self) -> RawPacket {
        let mut buf = Vec::with_capacity(1);
        buf.push(self.result as u8);
        RawPacket {
            id: ServerPacketId::ParcelCollected as i16,
            payload: buf,
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SParcelCollected payload must be exactly 1 byte",
            ));
        }
        Ok(SParcelCollected {
            result: payload[0] as i8,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receive_mail_roundtrip() {
        let p = SReceiveMail {
            mail_bytes: vec![1, 2, 3, 4, 5, 6],
        };

        let raw = p.encode().expect("encode SReceiveMail");
        assert_eq!(raw.id, ServerPacketId::ReceiveMail as i16);
        assert_eq!(raw.payload, p.mail_bytes);

        let decoded = SReceiveMail::decode(&raw.payload).expect("decode SReceiveMail");
        assert_eq!(decoded.mail_bytes, p.mail_bytes);
    }

    #[test]
    fn mail_cost_roundtrip() {
        let p = SMailCost { cost: 123456 }; // arbitrary

        let raw = p.encode().expect("encode SMailCost");
        assert_eq!(raw.id, ServerPacketId::MailCost as i16);

        let decoded = SMailCost::decode(&raw.payload).expect("decode SMailCost");
        assert_eq!(decoded.cost, p.cost);
    }

    #[test]
    fn mail_send_request_roundtrip() {
        let p = SMailSendRequest;

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::MailSendRequest as i16);
        assert!(raw.payload.is_empty());

        let _decoded = SMailSendRequest::decode(&raw.payload).expect("decode SMailSendRequest");
    }

    #[test]
    fn mail_sent_roundtrip() {
        let p = SMailSent { result: -1 };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::MailSent as i16);

        let decoded = SMailSent::decode(&raw.payload).expect("decode SMailSent");
        assert_eq!(decoded.result, p.result);
    }

    #[test]
    fn mail_locked_item_roundtrip() {
        let p = SMailLockedItem {
            unique_id: 0xA1B2_C3D4_E5F6_7788,
            locked: true,
        };

        let raw = p.encode().expect("encode SMailLockedItem");
        assert_eq!(raw.id, ServerPacketId::MailLockedItem as i16);

        let decoded = SMailLockedItem::decode(&raw.payload).expect("decode SMailLockedItem");
        assert_eq!(decoded.unique_id, p.unique_id);
        assert_eq!(decoded.locked, p.locked);
    }

    #[test]
    fn parcel_collected_roundtrip() {
        let p = SParcelCollected { result: 1 };

        let raw = p.encode();
        assert_eq!(raw.id, ServerPacketId::ParcelCollected as i16);

        let decoded = SParcelCollected::decode(&raw.payload).expect("decode SParcelCollected");
        assert_eq!(decoded.result, p.result);
    }
}
