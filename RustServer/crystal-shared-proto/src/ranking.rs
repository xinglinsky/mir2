//! Ranking-related packets (GetRanking / Rankings) compatible with the C# Crystal implementation.
//!
//! C# reference:
//! - ClientPackets.GetRanking
//! - ServerPackets.Rankings
//! - Shared.RankCharacterInfo

#![allow(dead_code)]

use std::io::{self, Cursor, Read};

use crate::io::{
    read_bool,
    read_i32_le,
    read_i64_le,
    read_string,
    write_bool,
    write_i32_le,
    write_i64_le,
    write_string,
};
use crate::login::{ClientPacketId, ServerPacketId};
use crate::packet::RawPacket;

/// Client->Server: request a page of rankings.
#[derive(Clone, Debug)]
pub struct CGetRanking {
    /// RankType: 0 overall, 1–5 per-class; value is forwarded directly from the client.
    pub rank_type: u8,
    /// Starting index within the ranking list (0-based), matches C# RankIndex.
    pub rank_index: i32,
    /// If true, only count and list online players.
    pub online_only: bool,
}

impl CGetRanking {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();
        buf.push(self.rank_type);
        write_i32_le(&mut buf, self.rank_index)?;
        write_bool(&mut buf, self.online_only)?;
        Ok(RawPacket {
            id: ClientPacketId::GetRanking as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);

        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let rank_type = one[0];

        let rank_index = read_i32_le(&mut c)?;
        let online_only = read_bool(&mut c)?;

        Ok(CGetRanking {
            rank_type,
            rank_index,
            online_only,
        })
    }
}

/// Single ranking entry as sent over the network. This mirrors the C#
/// Shared.RankCharacterInfo(BinaryReader) / Save(BinaryWriter).
#[derive(Clone, Debug)]
pub struct SRankCharacterInfo {
    pub player_id: i64,
    pub name: String,
    pub level: i32,
    pub class: u8,
}

impl SRankCharacterInfo {
    pub fn encode_to(&self, buf: &mut Vec<u8>) -> io::Result<()> {
        write_i64_le(buf, self.player_id)?;
        write_string(buf, &self.name)?;
        write_i32_le(buf, self.level)?;
        buf.push(self.class);
        Ok(())
    }

    pub fn decode_from(c: &mut Cursor<&[u8]>) -> io::Result<Self> {
        let player_id = read_i64_le(c)?;
        let name = read_string(c)?;
        let level = read_i32_le(c)?;

        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let class = one[0];

        Ok(SRankCharacterInfo {
            player_id,
            name,
            level,
            class,
        })
    }
}

/// Server->Client: response containing ranking data for a given type.
#[derive(Clone, Debug)]
pub struct SRankings {
    pub rank_type: u8,
    pub my_rank: i32,
    pub listing_details: Vec<SRankCharacterInfo>,
    pub listings: Vec<i64>,
    pub count: i32,
}

impl SRankings {
    pub fn encode(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::new();

        buf.push(self.rank_type);
        write_i32_le(&mut buf, self.my_rank)?;

        // ListingDetails
        let details_len: i32 = self
            .listing_details
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many ranking details"))?;
        write_i32_le(&mut buf, details_len)?;
        for entry in &self.listing_details {
            entry.encode_to(&mut buf)?;
        }

        // Listings (PlayerId list)
        let ids_len: i32 = self
            .listings
            .len()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many ranking ids"))?;
        write_i32_le(&mut buf, ids_len)?;
        for id in &self.listings {
            write_i64_le(&mut buf, *id)?;
        }

        write_i32_le(&mut buf, self.count)?;

        Ok(RawPacket {
            id: ServerPacketId::Rankings as i16,
            payload: buf,
        })
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(payload);

        let mut one = [0u8; 1];
        c.read_exact(&mut one)?;
        let rank_type = one[0];

        let my_rank = read_i32_le(&mut c)?;

        // ListingDetails
        let details_len = read_i32_le(&mut c)?;
        if details_len < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative rankings ListingDetails length",
            ));
        }
        let mut listing_details = Vec::with_capacity(details_len as usize);
        for _ in 0..details_len {
            listing_details.push(SRankCharacterInfo::decode_from(&mut c)?);
        }

        // Listings (PlayerId list)
        let ids_len = read_i32_le(&mut c)?;
        if ids_len < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative rankings Listings length",
            ));
        }
        let mut listings = Vec::with_capacity(ids_len as usize);
        for _ in 0..ids_len {
            listings.push(read_i64_le(&mut c)?);
        }

        let count = read_i32_le(&mut c)?;

        Ok(SRankings {
            rank_type,
            my_rank,
            listing_details,
            listings,
            count,
        })
    }
}
