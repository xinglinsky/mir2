use crystal_server_core::ranking::RankType;
use crystal_shared_proto::ranking::{CGetRanking, SRankCharacterInfo, SRankings};

use super::{LoginConnection, Stage};

impl LoginConnection {
    pub(crate) fn handle_get_ranking(
        &mut self,
        msg: CGetRanking,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let rank_type_enum = match RankType::from_u8(msg.rank_type) {
            Some(rt) => rt,
            None => {
                tracing::debug!(
                    "[ranking] unknown RankType from client: {} (session_id={})",
                    msg.rank_type,
                    self.session_id,
                );
                return;
            }
        };

        // Build a snapshot of the requested ranking type from the world.
        let (snapshot, online_total) = {
            let world = self.world.lock().unwrap();
            let snapshot = world.ranking_snapshot_for_type(rank_type_enum);
            let online_total = world.online_ranking_count_for_type(rank_type_enum);
            (snapshot, online_total)
        };

        let total = snapshot.len();
        if total == 0 {
            // Still respond with an empty Rankings packet so the client UI can
            // clear its view gracefully.
            let pkt = SRankings {
                rank_type: msg.rank_type,
                my_rank: 0,
                listing_details: Vec::new(),
                listings: Vec::new(),
                count: 0,
            };

            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
            return;
        }

        if msg.rank_index < 0 || (msg.rank_index as usize) >= total {
            // Mirror C# behaviour: invalid index yields no response.
            return;
        }

        let start = msg.rank_index as usize;

        // Determine the caller's own rank within this list, if present.
        let my_rank = if let Some(char_idx) = self.current_char_index {
            let my_id = char_idx as i64;
            snapshot
                .iter()
                .position(|e| e.player_id == my_id)
                .map(|idx| (idx + 1) as i32)
                .unwrap_or(0)
        } else {
            0
        };

        // Slice out up to 20 entries starting from RankIndex, matching the
        // legacy client behaviour.
        let mut listing_details = Vec::new();
        let mut listings = Vec::new();
        let mut sent = 0usize;

        for entry in snapshot.into_iter().skip(start) {
            if sent >= 20 {
                break;
            }

            listings.push(entry.player_id);
            listing_details.push(SRankCharacterInfo {
                player_id: entry.player_id,
                name: entry.name,
                level: entry.level as i32,
                class: entry.class.as_u8(),
            });

            sent += 1;
        }

        // Note: at this stage we only have online players in the world, so the
        // Count field represents the total number of online entries. When DB-
        // backed rankings are implemented, this will be updated to include
        // offline characters as well, and msg.online_only can be honoured.
        let count = if msg.online_only {
            online_total
        } else {
            total.min(i32::MAX as usize) as i32
        };
        let pkt = SRankings {
            rank_type: msg.rank_type,
            my_rank,
            listing_details,
            listings,
            count,
        };

        match pkt.encode() {
            Ok(raw) => out.push(Self::encode_raw(raw)),
            Err(e) => {
                tracing::debug!(
                    "[ranking] failed to encode SRankings for session_id={} err={:?}",
                    self.session_id,
                    e,
                );
            }
        }
    }
}
