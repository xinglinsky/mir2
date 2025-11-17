// Packet framing and basic structures will go here.
// C# layout: [length: u16 LE][id: i16 LE][payload...]

pub struct RawPacket {
    pub id: i16,
    pub payload: Vec<u8>,
}

impl RawPacket {
    pub fn encode(&self) -> Vec<u8> {
        let payload_len = self.payload.len();
        let total_len: u16 = (4 + payload_len)
            .try_into()
            .expect("packet too large to encode");

        let mut buf = Vec::with_capacity(4 + payload_len);
        buf.extend_from_slice(&total_len.to_le_bytes());
        buf.extend_from_slice(&self.id.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn decode(raw: &[u8]) -> Option<(Self, &[u8])> {
        if raw.len() < 4 {
            return None;
        }

        let length = u16::from_le_bytes([raw[0], raw[1]]) as usize;
        if length < 4 || length > raw.len() {
            return None;
        }

        let id = i16::from_le_bytes([raw[2], raw[3]]);
        let payload_len = length - 4;
        let payload = raw[4..4 + payload_len].to_vec();

        let extra = &raw[length..];
        Some((
            RawPacket {
                id,
                payload,
            },
            extra,
        ))
    }
}
