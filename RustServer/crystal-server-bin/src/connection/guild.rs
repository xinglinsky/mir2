use crystal_shared_proto::guild::{
    SGuildStatus,
    SGuildInvite,
    SGuildMemberChange,
    SGuildNoticeChange,
    SGuildExpGain,
    SGuildStorageGoldChange,
    SGuildStorageItemChange,
    SGuildStorageList,
    CRequestGuildInfo,
    CEditGuildNotice,
    CGuildStorageGoldChange,
    CGuildStorageItemChange,
};
use crystal_shared_proto::io::{write_bool, write_i32_le, write_i64_le, write_string};
use crystal_shared_proto::login::{CGuildInvite, CGuildNameReturn, CEditGuildMember};
use crystal_shared_proto::scene::{
    SObjectGuildNameChanged,
    SGainedGold,
    SLoseGold,
};
use crystal_server_core::guild::{GuildInfo as CoreGuildInfo, GuildRank as CoreGuildRank};
use crystal_server_core::world::world::{GuildGoldChangeError, GuildJoinError, GuildStorageError, SessionId};
use crystal_server_core::world::configs::guild_settings;

use super::{LoginConnection, Stage};

impl LoginConnection {
    const GUILD_RANK_OPT_CAN_CHANGE_RANK: u8 = 1;
    const GUILD_RANK_OPT_CAN_CHANGE_NOTICE: u8 = 64;
    const GUILD_RANK_OPT_CAN_STORE_ITEM: u8 = 8;
    const GUILD_RANK_OPT_CAN_RETRIEVE_ITEM: u8 = 16;
    /// Build a GuildRank/GuildMember list based on the persisted GuildInfo
    /// stored in the world, including offline members when ranks have been
    /// populated (e.g. from a migrated C# database). Returns None if the
    /// guild has no rank data.
    fn build_guild_member_ranks_bytes_from_guildinfo(&self, guild_name: &str) -> Option<Vec<u8>> {
        let guild = {
            let world = self.world.lock().unwrap();
            world.get_guild_info_by_name(guild_name)?
        };

        if guild.ranks.is_empty() {
            return None;
        }

        let mut buf = Vec::new();

        // Rank count
        let _ = write_i32_le(&mut buf, guild.ranks.len() as i32);

        for rank in &guild.ranks {
            // GuildRank.Save: Name, Options(byte), Index(int), MemberCount(int).
            let _ = write_string(&mut buf, &rank.name);
            buf.push(rank.options);
            let _ = write_i32_le(&mut buf, rank.index as i32);
            let _ = write_i32_le(&mut buf, rank.members.len() as i32);

            // GuildMember.Save: Name, Id, LastLoginTicks, HasVoted, Online.
            for member in &rank.members {
                let _ = write_string(&mut buf, &member.name);
                let _ = write_i32_le(&mut buf, member.id);
                let _ = write_i64_le(&mut buf, member.last_login_ticks);
                let _ = write_bool(&mut buf, false); // hasvoted (not tracked in Rust core)
                let _ = write_bool(&mut buf, member.online);
            }
        }

        Some(buf)
    }

    /// Encode a single GuildRank into the wire format used by
    /// GuildMemberChange(Status > 5): [rank_count=1] + GuildRank.Save.
    fn encode_single_rank(rank: &CoreGuildRank) -> Vec<u8> {
        let mut buf = Vec::new();

        // Rank count
        let _ = write_i32_le(&mut buf, 1);

        // GuildRank.Save: Name, Options(byte), Index(int), MemberCount(int).
        let _ = write_string(&mut buf, &rank.name);
        buf.push(rank.options);
        let _ = write_i32_le(&mut buf, rank.index as i32);
        let _ = write_i32_le(&mut buf, rank.members.len() as i32);

        // GuildMember.Save: Name, Id, LastLoginTicks, HasVoted, Online.
        for member in &rank.members {
            let _ = write_string(&mut buf, &member.name);
            let _ = write_i32_le(&mut buf, member.id);
            let _ = write_i64_le(&mut buf, member.last_login_ticks);
            let _ = write_bool(&mut buf, false); // hasvoted (not tracked in Rust core)
            let _ = write_bool(&mut buf, member.online);
        }

        buf
    }

    /// Internal helper to broadcast a GuildMemberChange packet (optionally
    /// carrying a rank payload) to all online members of the specified guild.
    fn broadcast_guild_member_change_with_bytes(
        &self,
        guild_name: &str,
        member_name: &str,
        status: u8,
        exclude: Option<SessionId>,
        ranks_bytes: Vec<u8>,
    ) {
        let pkt = SGuildMemberChange {
            name: member_name.to_string(),
            rank_index: 0,
            status,
            ranks_bytes,
        };

        let Ok(raw) = pkt.encode() else {
            return;
        };
        let encoded = Self::encode_raw(raw);

        let summaries = self.player_summaries.lock().unwrap();
        let mut outboxes = self.outboxes.lock().unwrap();

        for (sid, v) in summaries.iter() {
            if let Some(ex) = exclude {
                if *sid == ex {
                    continue;
                }
            }
            if v.guild_name.eq_ignore_ascii_case(guild_name) {
                outboxes.entry(*sid).or_default().push(encoded.clone());
            }
        }
    }

    /// Handle guild storage item changes (store/retrieve/move/list) initiated
    /// by the client. This mirrors the C# PlayerObject.GuildStorageItemChange
    /// logic for Type 0 (store), 1 (retrieve), 2 (move) and 3 (request list),
    /// including safe-zone and rank permission checks.
    pub(crate) fn handle_guild_storage_item_change(
        &mut self,
        msg: CGuildStorageItemChange,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let (guild_name, _player_name) = {
            let map = self.player_summaries.lock().unwrap();
            match map.get(&self.session_id) {
                Some(v) => (v.guild_name.clone(), v.name.clone()),
                None => (String::new(), String::new()),
            }
        };

        let base_type = msg.change_type;
        let from_slot = msg.from_slot;
        let to_slot = msg.to_slot;

        let user_id_i32 = self.current_char_index.unwrap_or(0);

        // Helper: send a single revert packet back to the caller using
        // Type = 3 + base_type and no item payload, mirroring the C#
        // GuildStorageItemChange error behaviour.
        let send_revert = |_conn: &mut LoginConnection, out: &mut Vec<Vec<u8>>| {
            let pkt = SGuildStorageItemChange {
                user: user_id_i32,
                change_type: base_type.saturating_add(3),
                to_slot,
                from_slot,
                item_user_id: None,
                item_bytes: Vec::new(),
            };
            if let Ok(raw) = pkt.encode() {
                out.push(Self::encode_raw(raw));
            }
        };

        if guild_name.is_empty() {
            send_revert(self, out);
            self.send_system_chat("You are not part of a guild.", out);
            return;
        }

        // Enforce safe-zone restriction for guild storage usage for
        // store/retrieve/move. The list request (type 3) is allowed outside
        // safe zones, mirroring the C# behaviour.
        if base_type != 3 {
            let in_safe_zone = {
                let world = self.world.lock().unwrap();
                world.player_in_safe_zone(self.session_id)
            };
            if !in_safe_zone {
                send_revert(self, out);
                self.send_system_chat(
                    "You cannot use guild storage outside safezones.",
                    out,
                );
                return;
            }
        }

        match base_type {
            // Type 0: store item from player inventory into guild storage.
            0 => {
                let (_rank_index, options) = match self
                    .get_kicker_rank_index_and_options(&guild_name)
                {
                    Some(v) => v,
                    None => {
                        send_revert(self, out);
                        self.send_system_chat(
                            "Guild data not found for item operation.",
                            out,
                        );
                        return;
                    }
                };

                if options & Self::GUILD_RANK_OPT_CAN_STORE_ITEM == 0 {
                    send_revert(self, out);
                    self.send_system_chat(
                        "You do not have permission to store items in guild storage.",
                        out,
                    );
                    return;
                }

                if from_slot < 0 || to_slot < 0 {
                    send_revert(self, out);
                    return;
                }
                let from_idx = from_slot as usize;
                let to_idx = to_slot as usize;

                // Snapshot inventory/equipment so we can update it after the
                // guild storage operation succeeds.
                let (mut inv, eq) = {
                    let world = self.world.lock().unwrap();
                    world
                        .player_items(self.session_id)
                        .unwrap_or((
                            crystal_server_core::item::Inventory::new_default(),
                            crystal_server_core::item::Equipment::new_default(),
                        ))
                };

                if from_idx >= inv.slots.len() {
                    send_revert(self, out);
                    return;
                }

                let item = match inv.slots[from_idx].as_ref() {
                    Some(it) => it.clone(),
                    None => {
                        send_revert(self, out);
                        return;
                    }
                };

                let guild_result = {
                    let mut world = self.world.lock().unwrap();
                    world.guild_storage_store_item(
                        &guild_name,
                        to_idx,
                        item.clone(),
                        user_id_i32 as i64,
                    )
                };

                let updated_guild = match guild_result {
                    Ok(g) => g,
                    Err(GuildStorageError::GuildNotFound) => {
                        send_revert(self, out);
                        self.send_system_chat("Guild not found.", out);
                        return;
                    }
                    Err(GuildStorageError::SlotOccupied) => {
                        send_revert(self, out);
                        self.send_system_chat("Target slot not empty.", out);
                        return;
                    }
                    Err(
                        GuildStorageError::InvalidSlot
                        | GuildStorageError::SlotEmpty
                        | GuildStorageError::ItemBound,
                    ) => {
                        // Silent failure like the C# implementation for most
                        // validation errors.
                        send_revert(self, out);
                        return;
                    }
                    Err(GuildStorageError::EncodingFailed | GuildStorageError::DecodingFailed) => {
                        send_revert(self, out);
                        self.send_system_chat("Guild storage operation failed.", out);
                        return;
                    }
                };

                // Persist updated guild snapshot.
                let _ = self.store.save_guild(&updated_guild);

                // Remove the item from the player's inventory and push the
                // new state back into the world.
                inv.slots[from_idx] = None;
                {
                    let mut world = self.world.lock().unwrap();
                    world.set_player_items(self.session_id, inv.clone(), eq.clone());
                }

                // Build the success packet using the stored item snapshot.
                let storage_item = match updated_guild
                    .stored_items
                    .get(to_idx)
                    .and_then(|o| o.as_ref())
                    .cloned()
                {
                    Some(it) => it,
                    None => {
                        return;
                    }
                };

                let pkt = SGuildStorageItemChange {
                    user: user_id_i32,
                    change_type: 0,
                    to_slot,
                    from_slot,
                    item_user_id: Some(storage_item.user_id),
                    item_bytes: storage_item.item_bytes.clone(),
                };

                self.broadcast_guild_storage_item_change(&guild_name, &pkt);
            }
            // Type 1: retrieve item from guild storage into player inventory.
            1 => {
                let (_rank_index, options) = match self
                    .get_kicker_rank_index_and_options(&guild_name)
                {
                    Some(v) => v,
                    None => {
                        self.send_system_chat(
                            "Guild data not found for item operation.",
                            out,
                        );
                        return;
                    }
                };

                if options & Self::GUILD_RANK_OPT_CAN_RETRIEVE_ITEM == 0 {
                    self.send_system_chat(
                        "You do not have permission to retrieve items from guild storage.",
                        out,
                    );
                    return;
                }

                if from_slot < 0 || to_slot < 0 {
                    send_revert(self, out);
                    return;
                }
                let from_idx = from_slot as usize;
                let to_idx = to_slot as usize;

                let (mut inv, eq) = {
                    let world = self.world.lock().unwrap();
                    world
                        .player_items(self.session_id)
                        .unwrap_or((
                            crystal_server_core::item::Inventory::new_default(),
                            crystal_server_core::item::Equipment::new_default(),
                        ))
                };

                if to_idx >= inv.slots.len() {
                    send_revert(self, out);
                    return;
                }

                if inv.slots[to_idx].is_some() {
                    send_revert(self, out);
                    self.send_system_chat("Target slot not empty.", out);
                    return;
                }

                let guild_result = {
                    let mut world = self.world.lock().unwrap();
                    world.guild_storage_retrieve_item(&guild_name, from_idx)
                };

                let (updated_guild, item) = match guild_result {
                    Ok(v) => v,
                    Err(GuildStorageError::GuildNotFound) => {
                        send_revert(self, out);
                        self.send_system_chat("Guild not found.", out);
                        return;
                    }
                    Err(
                        GuildStorageError::InvalidSlot
                        | GuildStorageError::SlotEmpty
                        | GuildStorageError::ItemBound,
                    ) => {
                        send_revert(self, out);
                        return;
                    }
                    Err(GuildStorageError::SlotOccupied) => {
                        // Not expected on retrieve path.
                        send_revert(self, out);
                        return;
                    }
                    Err(GuildStorageError::EncodingFailed | GuildStorageError::DecodingFailed) => {
                        send_revert(self, out);
                        self.send_system_chat("Guild storage operation failed.", out);
                        return;
                    }
                };

                let _ = self.store.save_guild(&updated_guild);

                inv.slots[to_idx] = Some(item.clone());
                {
                    let mut world = self.world.lock().unwrap();
                    world.set_player_items(self.session_id, inv.clone(), eq.clone());
                }

                let pkt = SGuildStorageItemChange {
                    user: user_id_i32,
                    change_type: 1,
                    to_slot,
                    from_slot,
                    item_user_id: None,
                    item_bytes: Vec::new(),
                };

                self.broadcast_guild_storage_item_change(&guild_name, &pkt);
            }
            // Type 2: move or swap items within guild storage.
            2 => {
                let (_rank_index, options) = match self
                    .get_kicker_rank_index_and_options(&guild_name)
                {
                    Some(v) => v,
                    None => {
                        send_revert(self, out);
                        self.send_system_chat(
                            "Guild data not found for item operation.",
                            out,
                        );
                        return;
                    }
                };

                if options & Self::GUILD_RANK_OPT_CAN_STORE_ITEM == 0 {
                    send_revert(self, out);
                    self.send_system_chat(
                        "You do not have permission to move items in guild storage.",
                        out,
                    );
                    return;
                }

                if from_slot < 0 || to_slot < 0 {
                    send_revert(self, out);
                    return;
                }
                let from_idx = from_slot as usize;
                let to_idx = to_slot as usize;

                let guild_result = {
                    let mut world = self.world.lock().unwrap();
                    world.guild_storage_move_item(&guild_name, from_idx, to_idx)
                };

                let updated_guild = match guild_result {
                    Ok(g) => g,
                    Err(GuildStorageError::GuildNotFound) => {
                        send_revert(self, out);
                        self.send_system_chat("Guild not found.", out);
                        return;
                    }
                    Err(
                        GuildStorageError::InvalidSlot
                        | GuildStorageError::SlotEmpty
                        | GuildStorageError::ItemBound,
                    ) => {
                        send_revert(self, out);
                        return;
                    }
                    Err(
                        GuildStorageError::SlotOccupied
                        | GuildStorageError::EncodingFailed
                        | GuildStorageError::DecodingFailed,
                    ) => {
                        send_revert(self, out);
                        self.send_system_chat("Guild storage operation failed.", out);
                        return;
                    }
                };

                let _ = self.store.save_guild(&updated_guild);

                let storage_item = match updated_guild
                    .stored_items
                    .get(to_idx)
                    .and_then(|o| o.as_ref())
                    .cloned()
                {
                    Some(it) => it,
                    None => {
                        return;
                    }
                };

                let pkt = SGuildStorageItemChange {
                    user: user_id_i32,
                    change_type: 2,
                    to_slot,
                    from_slot,
                    item_user_id: Some(storage_item.user_id),
                    item_bytes: storage_item.item_bytes.clone(),
                };

                self.broadcast_guild_storage_item_change(&guild_name, &pkt);
            }
            // Type 3: request full guild storage list.
            3 => {
                let items = {
                    let mut world = self.world.lock().unwrap();
                    match world.guild_storage_list(&guild_name) {
                        Ok(v) => v,
                        Err(GuildStorageError::GuildNotFound) => {
                            self.send_system_chat("Guild not found.", out);
                            return;
                        }
                        Err(_) => {
                            self.send_system_chat(
                                "Guild storage list operation failed.",
                                out,
                            );
                            return;
                        }
                    }
                };

                let mut buf = Vec::new();
                let _ = write_i32_le(&mut buf, items.len() as i32);

                for slot in items {
                    match slot {
                        None => {
                            let _ = write_bool(&mut buf, false);
                        }
                        Some(storage_item) => {
                            let _ = write_bool(&mut buf, true);
                            // GuildStorageItem.Save: Item.Save then UserId.
                            buf.extend_from_slice(&storage_item.item_bytes);
                            let _ = write_i64_le(&mut buf, storage_item.user_id);
                        }
                    }
                }

                let pkt = SGuildStorageList { items_bytes: buf };
                let raw = pkt.encode();
                out.push(Self::encode_raw(raw));
            }
            _ => {
                // Unknown change type; ignore for now.
            }
        }
    }

    /// Broadcast a GuildExpGain packet to all online members of the
    /// specified guild. This mirrors the non-level-up branch of C#
    /// GuildObject.GainExp where GuildExpGain is sent periodically while the
    /// guild is accumulating experience within the same level.
    pub(crate) fn broadcast_guild_exp_gain(&self, guild_name: &str, amount: u32) {
        let pkt = SGuildExpGain { amount };

        let Ok(raw) = pkt.encode() else {
            return;
        };
        let encoded = Self::encode_raw(raw);

        let summaries = self.player_summaries.lock().unwrap();
        let mut outboxes = self.outboxes.lock().unwrap();

        for (sid, v) in summaries.iter() {
            if v.guild_name.eq_ignore_ascii_case(guild_name) {
                outboxes.entry(*sid).or_default().push(encoded.clone());
            }
        }
    }

    /// Broadcast a GuildStorageItemChange packet to all online members of the
    /// specified guild. This mirrors the C# GuildObject.GuildStorageItemChange
    /// behaviour where all members see who stored/retrieved/moved which item.
    fn broadcast_guild_storage_item_change(
        &self,
        guild_name: &str,
        pkt: &SGuildStorageItemChange,
    ) {
        let Ok(raw) = pkt.encode() else {
            return;
        };
        let encoded = Self::encode_raw(raw);

        let summaries = self.player_summaries.lock().unwrap();
        let mut outboxes = self.outboxes.lock().unwrap();

        for (sid, v) in summaries.iter() {
            if v.guild_name.eq_ignore_ascii_case(guild_name) {
                outboxes.entry(*sid).or_default().push(encoded.clone());
            }
        }
    }

    /// Broadcast a GuildStorageGoldChange packet to all online members of the
    /// specified guild. This mirrors the C# GuildObject.GuildStorageGoldChange
    /// behaviour where all members see who donated/withdrew and how much.
    fn broadcast_guild_storage_gold_change(
        &self,
        guild_name: &str,
        change_type: u8,
        amount: u32,
        actor_name: &str,
    ) {
        let pkt = SGuildStorageGoldChange {
            amount,
            change_type,
            name: actor_name.to_string(),
        };

        let Ok(raw) = pkt.encode() else {
            return;
        };
        let encoded = Self::encode_raw(raw);

        let summaries = self.player_summaries.lock().unwrap();
        let mut outboxes = self.outboxes.lock().unwrap();

        for (sid, v) in summaries.iter() {
            if v.guild_name.eq_ignore_ascii_case(guild_name) {
                outboxes.entry(*sid).or_default().push(encoded.clone());
            }
        }
    }

    /// Broadcast an SGuildStatus snapshot to all online members of the given
    /// guild. This is primarily used when a guild levels up so that all
    /// members immediately see the new Level/MaxExperience/MemberCap and
    /// SparePoints in their Guild dialog. Per-recipient rank name and options
    /// are derived from GuildInfo.ranks when possible, falling back to the
    /// cached PlayerVisual rank name otherwise.
    pub(crate) fn broadcast_guild_status_for_guild(&self, guild: &CoreGuildInfo) {
        let summaries = self.player_summaries.lock().unwrap();
        let mut outboxes = self.outboxes.lock().unwrap();

        for (sid, v) in summaries.iter() {
            if !v.guild_name.eq_ignore_ascii_case(&guild.name) {
                continue;
            }

            // Resolve rank information for this member from GuildInfo.
            let mut rank_name = v.guild_rank_name.clone();
            let mut my_options: u8 = 0;
            let mut my_rank_id: u8 = 0;

            for rank in &guild.ranks {
                if rank
                    .members
                    .iter()
                    .any(|m| m.name.eq_ignore_ascii_case(&v.name))
                {
                    rank_name = rank.name.clone();
                    my_options = rank.options;
                    my_rank_id = rank.index;
                    break;
                }
            }

            let status = SGuildStatus {
                guild_name: guild.name.clone(),
                guild_rank_name: rank_name,
                level: guild.level,
                experience: guild.experience,
                max_experience: guild.max_experience,
                gold: guild.gold,
                spare_points: guild.spare_points,
                member_count: guild.member_count,
                max_members: guild.member_cap,
                voting: false,
                item_count: guild.stored_items.len() as u8,
                buff_count: guild.buff_list.len() as u8,
                my_options,
                my_rank_id: my_rank_id as i32,
            };

            if let Ok(raw) = status.encode() {
                let encoded = Self::encode_raw(raw);
                outboxes.entry(*sid).or_default().push(encoded);
            }
        }
    }

    /// Broadcast a GuildNoticeChange(update = -1) to all online members of the
    /// specified guild, mirroring the C# GuildObject.NewNotice behaviour where
    /// this flag causes clients to re-request the full notice.
    fn broadcast_guild_notice_changed(&self, guild_name: &str) {
        let pkt = SGuildNoticeChange {
            update: -1,
            notice: Vec::new(),
        };

        let Ok(raw) = pkt.encode() else {
            return;
        };
        let encoded = Self::encode_raw(raw);

        let summaries = self.player_summaries.lock().unwrap();
        let mut outboxes = self.outboxes.lock().unwrap();

        for (sid, v) in summaries.iter() {
            if v.guild_name.eq_ignore_ascii_case(guild_name) {
                outboxes.entry(*sid).or_default().push(encoded.clone());
            }
        }
    }

    /// Broadcast a small incremental GuildMemberChange packet (no rank list
    /// payload) to all online members of the specified guild. This is used for
    /// join/leave/kick events so that the client's GuildDialog can update its
    /// member list without requiring an immediate full refresh.
    fn broadcast_guild_member_change(
        &self,
        guild_name: &str,
        member_name: &str,
        status: u8,
        exclude: Option<SessionId>,
    ) {
        self.broadcast_guild_member_change_with_bytes(
            guild_name,
            member_name,
            status,
            exclude,
            Vec::new(),
        );
    }

    /// Build a minimal GuildRank/GuildMember list for the given guild based on
    /// the currently online players. This mirrors the binary layout expected by
    /// the C# client's GuildMemberChange(Status=255) packet, using a single
    /// synthetic rank called "Members".
    fn build_guild_member_ranks_bytes(&self, guild_name: &str) -> Option<Vec<u8>> {
        let map = self.player_summaries.lock().unwrap();
        let mut members: Vec<String> = map
            .values()
            .filter(|v| v.guild_name.eq_ignore_ascii_case(guild_name))
            .map(|v| v.name.clone())
            .collect();

        if members.is_empty() {
            return None;
        }

        // Keep list stable so the client view doesn't jump around.
        members.sort_unstable();

        let mut buf = Vec::new();

        // Rank count
        let _ = write_i32_le(&mut buf, 1);

        // Single rank: Name, Options (byte), Index (int), MemberCount (int).
        let _ = write_string(&mut buf, "Members");
        buf.push(0); // GuildRankOptions = 0 for now; actual permissions come from GuildStatus.MyOptions.
        let _ = write_i32_le(&mut buf, 1); // Rank index
        let _ = write_i32_le(&mut buf, members.len() as i32);

        for name in members {
            // GuildMember.Save: Name, Id, LastLoginTicks, HasVoted, Online.
            let _ = write_string(&mut buf, &name);
            let _ = write_i32_le(&mut buf, 0); // Id (unused by client; keep 0)
            let _ = write_i64_le(&mut buf, 0); // LastLogin ticks (unknown)
            let _ = write_bool(&mut buf, false); // hasvoted
            let _ = write_bool(&mut buf, true); // Online
        }

        Some(buf)
    }
    fn guild_member_exists_for_session(&self) -> bool {
        let map = self.player_summaries.lock().unwrap();
        if let Some(v) = map.get(&self.session_id) {
            !v.guild_name.is_empty()
        } else {
            false
        }
    }

    fn get_kicker_rank_index_and_options(&self, guild_name: &str) -> Option<(u8, u8)> {
        let account_id = self.account_id.as_ref()?;
        let char_idx = self.current_char_index?;

        let (db_guild_name, rank_index) = match self.store.load_character_guild(account_id, char_idx) {
            Ok(Some(v)) => v,
            _ => return None,
        };

        if !db_guild_name.eq_ignore_ascii_case(guild_name) {
            return None;
        }

        let guild = {
            let world = self.world.lock().unwrap();
            world.get_guild_info_by_name(guild_name)?
        };

        let options = guild
            .ranks
            .iter()
            .find(|r| r.index == rank_index)
            .map(|r| r.options)
            .or_else(|| guild.ranks.get(rank_index as usize).map(|r| r.options))
            .unwrap_or(0);

        Some((rank_index, options))
    }

    pub(crate) fn handle_create_guild_command(&mut self, name_raw: &str, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let name = name_raw.trim();
        let len = name.chars().count();
        if len < 3 || len > 20 {
            self.send_system_chat("Guild name must be between 3 and 20 characters.", out);
            return;
        }

        if name.contains('\\') {
            self.send_system_chat("Guild name contains invalid characters.", out);
            return;
        }

        if self.guild_member_exists_for_session() {
            self.send_system_chat("You are already part of a guild.", out);
            return;
        }

        // Enforce the same minimum level requirement as the C# server using
        // GuildSettings.ini (Guilds.MinimumLevel).
        let required_level = guild_settings().required_level as u16;
        let player_level: u16 = {
            let map = self.player_summaries.lock().unwrap();
            map.get(&self.session_id).map(|v| v.level).unwrap_or(1)
        };
        if player_level < required_level {
            let msg = format!(
                "Your level is not high enough to create a guild, required: {}",
                required_level
            );
            self.send_system_chat(&msg, out);
            return;
        }

        let guild_info = {
            let mut world = self.world.lock().unwrap();
            world.create_guild(name)
        };

        let Some(guild) = guild_info else {
            let msg = format!("Guild {} already exists.", name);
            self.send_system_chat(&msg, out);
            return;
        };

        let _ = self.store.save_guild(&guild);

        {
            let mut map = self.player_summaries.lock().unwrap();
            if let Some(v) = map.get_mut(&self.session_id) {
                v.guild_name = guild.name.clone();
                v.guild_rank_name = "Leader".to_string();
            }
        }

        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let _ = self
                .store
                .save_character_guild(account_id, char_idx, &guild.name, 0);
        }

        // Notify self and nearby players that this object's guild name has
        // changed, so clients like B immediately see A's new guild tag
        // without requiring a visibility refresh.
        let guild_name_changed = SObjectGuildNameChanged {
            object_id: self.session_id,
            guild_name: guild.name.clone(),
        };
        if let Ok(pkt) = guild_name_changed.encode() {
            let raw = Self::encode_raw(pkt);
            // Send to self
            out.push(raw.clone());
            // Broadcast to other players in view
            self.enqueue_for_viewers(
                self.current_map_index,
                self.current_x,
                self.current_y,
                raw,
            );
        }

        let status = SGuildStatus {
            guild_name: guild.name.clone(),
            guild_rank_name: "Leader".to_string(),
            level: guild.level,
            experience: guild.experience,
            max_experience: guild.max_experience,
            gold: guild.gold,
            spare_points: guild.spare_points,
            member_count: 1,
            max_members: guild.member_cap,
            voting: false,
            item_count: guild.stored_items.len() as u8,
            buff_count: guild.buff_list.len() as u8,
            my_options: 0xff,
            my_rank_id: 0,
        };
        if let Ok(raw) = status.encode() {
            out.push(Self::encode_raw(raw));
        }

        let ok = format!("Successfully created guild {}", name);
        self.send_system_chat(&ok, out);
    }

    pub(crate) fn handle_guild_name_return(
        &mut self,
        msg: CGuildNameReturn,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let name = msg.name.trim();
        if name.is_empty() {
            return;
        }

        self.handle_create_guild_command(name, out);
    }

    pub(crate) fn handle_request_guild_info(
        &mut self,
        msg: CRequestGuildInfo,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        // Determine our current guild name from the cached PlayerVisual.
        let guild_name = {
            let map = self.player_summaries.lock().unwrap();
            map.get(&self.session_id)
                .map(|v| v.guild_name.clone())
                .unwrap_or_default()
        };

        if guild_name.is_empty() {
            return;
        }

        match msg.info_type {
            // 0 = notice, 1 = member list in the legacy C# implementation.
            0 => {
                let notice = {
                    let world = self.world.lock().unwrap();
                    match world.get_guild_info_by_name(&guild_name) {
                        Some(g) => g.notice.clone(),
                        None => Vec::new(),
                    }
                };

                let pkt = SGuildNoticeChange {
                    update: notice.len() as i32,
                    notice,
                };

                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            1 => {
                let ranks_bytes = self
                    .build_guild_member_ranks_bytes_from_guildinfo(&guild_name)
                    .or_else(|| self.build_guild_member_ranks_bytes(&guild_name));

                let Some(ranks_bytes) = ranks_bytes else {
                    return;
                };

                // Status = 255 tells the client this is a full member list
                // refresh (GuildDialog.NewMembersList).
                let pkt = SGuildMemberChange {
                    name: String::new(),
                    rank_index: 0,
                    status: 255,
                    ranks_bytes,
                };

                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            _ => {
                // Silently ignore other info types for now.
            }
        }
    }

    pub(crate) fn handle_edit_guild_notice(
        &mut self,
        msg: CEditGuildNotice,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        // Determine our current guild name from the cached PlayerVisual.
        let guild_name = {
            let map = self.player_summaries.lock().unwrap();
            map.get(&self.session_id)
                .map(|v| v.guild_name.clone())
                .unwrap_or_default()
        };

        if guild_name.is_empty() {
            self.send_system_chat("You are not part of a guild.", out);
            return;
        }

        let (_rank_index, options) = match self.get_kicker_rank_index_and_options(&guild_name) {
            Some(v) => v,
            None => {
                self.send_system_chat(
                    "Guild data not found for notice operation.",
                    out,
                );
                return;
            }
        };

        if options & Self::GUILD_RANK_OPT_CAN_CHANGE_NOTICE == 0 {
            self.send_system_chat(
                "You are not allowed to change the guild notice!",
                out,
            );
            return;
        }

        if msg.notice.len() > 200 {
            self.send_system_chat(
                "Guild notice can not be longer then 200 lines!",
                out,
            );
            return;
        }

        let updated_guild = {
            let mut world = self.world.lock().unwrap();
            match world.guild_update_notice(&guild_name, msg.notice.clone()) {
                Some(g) => g,
                None => {
                    self.send_system_chat("Guild not found.", out);
                    return;
                }
            }
        };

        let _ = self.store.save_guild(&updated_guild);

        // Notify all online guild members that the notice has changed. Clients
        // will receive GuildNoticeChange(update=-1) and then request the
        // updated notice via CRequestGuildInfo.
        self.broadcast_guild_notice_changed(&guild_name);
    }

    /// Handle guild storage gold changes (donate/retrieve) initiated by the
    /// client. This mirrors the C# PlayerObject.GuildStorageGoldChange logic
    /// for Type 0 (donate) and Type 1 (retrieve), including safe-zone checks,
    /// leader-only retrieval, and guild/player gold caps.
    pub(crate) fn handle_guild_storage_gold_change(
        &mut self,
        msg: CGuildStorageGoldChange,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        // Resolve current guild and player name from the cached PlayerVisual.
        let (guild_name, player_name) = {
            let map = self.player_summaries.lock().unwrap();
            match map.get(&self.session_id) {
                Some(v) => (v.guild_name.clone(), v.name.clone()),
                None => (String::new(), String::new()),
            }
        };

        if guild_name.is_empty() {
            self.send_system_chat("You are not part of a guild.", out);
            return;
        }

        // Enforce safe-zone restriction for guild storage usage.
        let in_safe_zone = {
            let world = self.world.lock().unwrap();
            world.player_in_safe_zone(self.session_id)
        };
        if !in_safe_zone {
            self.send_system_chat(
                "You cannot use guild storage outside safezones.",
                out,
            );
            return;
        }

        let amount = msg.amount;
        if amount == 0 {
            // Nothing to do; mirror C# by silently ignoring zero changes.
            return;
        }

        match msg.change_type {
            // Type 0 = donate gold from player to guild bank.
            0 => {
                let stats = match self.current_stats.clone() {
                    Some(s) => s,
                    None => {
                        self.send_system_chat(
                            "Character stats not available, cannot change guild gold.",
                            out,
                        );
                        return;
                    }
                };

                let amount_i64 = amount as i64;
                if stats.gold < amount_i64 {
                    self.send_system_chat("Insufficient gold.", out);
                    return;
                }

                let guild_result = {
                    let mut world = self.world.lock().unwrap();
                    world.guild_add_gold(&guild_name, amount)
                };

                let updated_guild = match guild_result {
                    Ok(g) => g,
                    Err(GuildGoldChangeError::NotFound) => {
                        self.send_system_chat("Guild not found.", out);
                        return;
                    }
                    Err(GuildGoldChangeError::Overflow) => {
                        self.send_system_chat("Guild gold limit reached.", out);
                        return;
                    }
                    Err(GuildGoldChangeError::InsufficientGuildGold) => {
                        // Should not occur on add path; treat as generic failure.
                        self.send_system_chat("Guild gold limit reached.", out);
                        return;
                    }
                };

                // Persist updated guild snapshot.
                let _ = self.store.save_guild(&updated_guild);

                // Deduct gold from the player and persist CharacterStats.
                let mut new_stats = stats.clone();
                new_stats.gold = new_stats.gold.saturating_sub(amount_i64);

                if let (Some(ref account_id), Some(char_idx)) =
                    (self.account_id.as_ref(), self.current_char_index)
                {
                    let _ = self
                        .store
                        .save_character_stats(account_id, char_idx, &new_stats);
                }

                self.current_stats = Some(new_stats.clone());

                // Notify the caller about the gold deduction.
                let lose = SLoseGold { gold: amount };
                if let Ok(raw) = lose.encode() {
                    out.push(Self::encode_raw(raw));
                }

                // Broadcast guild storage gold change to all online guild members.
                self.broadcast_guild_storage_gold_change(
                    &guild_name,
                    0,
                    amount,
                    &player_name,
                );
            }
            // Type 1 = retrieve gold from guild bank to player. Leader-only.
            1 => {
                let (rank_index, _options) = match self
                    .get_kicker_rank_index_and_options(&guild_name)
                {
                    Some(v) => v,
                    None => {
                        self.send_system_chat(
                            "Guild data not found for gold operation.",
                            out,
                        );
                        return;
                    }
                };

                // Only the leader (rank index 0) may retrieve guild gold.
                if rank_index != 0 {
                    self.send_system_chat("Insufficient rank.", out);
                    return;
                }

                let stats = match self.current_stats.clone() {
                    Some(s) => s,
                    None => {
                        self.send_system_chat(
                            "Character stats not available, cannot change guild gold.",
                            out,
                        );
                        return;
                    }
                };

                // Mirror C# CanGainGold(uint): ensure resulting gold does not
                // exceed the legacy uint.MaxValue cap.
                let current_gold_u64 = if stats.gold <= 0 {
                    0u64
                } else {
                    stats.gold as u64
                };
                let new_total_u64 = current_gold_u64.saturating_add(amount as u64);
                if new_total_u64 > u32::MAX as u64 {
                    self.send_system_chat("Gold limit reached.", out);
                    return;
                }

                let guild_result = {
                    let mut world = self.world.lock().unwrap();
                    world.guild_subtract_gold(&guild_name, amount)
                };

                let updated_guild = match guild_result {
                    Ok(g) => g,
                    Err(GuildGoldChangeError::NotFound) => {
                        self.send_system_chat("Guild not found.", out);
                        return;
                    }
                    Err(GuildGoldChangeError::InsufficientGuildGold) => {
                        self.send_system_chat("Insufficient gold.", out);
                        return;
                    }
                    Err(GuildGoldChangeError::Overflow) => {
                        // Not expected on subtract path; treat as generic failure.
                        self.send_system_chat("Guild gold operation failed.", out);
                        return;
                    }
                };

                // Persist updated guild snapshot.
                let _ = self.store.save_guild(&updated_guild);

                // Add gold to the player and persist CharacterStats.
                let mut new_stats = stats.clone();
                new_stats.gold = (new_stats.gold.saturating_add(amount as i64))
                    .min(u32::MAX as i64);

                if let (Some(ref account_id), Some(char_idx)) =
                    (self.account_id.as_ref(), self.current_char_index)
                {
                    let _ = self
                        .store
                        .save_character_stats(account_id, char_idx, &new_stats);
                }

                self.current_stats = Some(new_stats.clone());

                // Notify the caller about the gold gain.
                let gain = SGainedGold { gold: amount };
                if let Ok(raw) = gain.encode() {
                    out.push(Self::encode_raw(raw));
                }

                // Broadcast guild storage gold change to all online guild members.
                self.broadcast_guild_storage_gold_change(
                    &guild_name,
                    1,
                    amount,
                    &player_name,
                );
            }
            _ => {
                // Other change types (2,3) are currently driven by server-side
                // guild territory / war flows and are not expected from the
                // client. Ignore them here.
            }
        }
    }

    pub(crate) fn handle_edit_guild_member(
        &mut self,
        msg: CEditGuildMember,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let target_name = msg.name.trim();
        if target_name.is_empty() {
            return;
        }

        // Determine kicker's guild and name from the cached PlayerVisual.
        let (kicker_guild_name, kicker_name) = {
            let map = self.player_summaries.lock().unwrap();
            match map.get(&self.session_id) {
                Some(v) => (v.guild_name.clone(), v.name.clone()),
                None => (String::new(), String::new()),
            }
        };

        if kicker_guild_name.is_empty() {
            self.send_system_chat("You are not in a guild.", out);
            return;
        }

        match msg.change_type {
            0 => {
                // Invite member.

                // Find target session by name (online-only for invite).
                let target_session_id_opt = {
                    let world = self.world.lock().unwrap();
                    world.find_session_by_name(target_name)
                };

                let Some(target_session_id) = target_session_id_opt else {
                    let m = format!("{} is not online!", target_name);
                    self.send_system_chat(&m, out);
                    return;
                };

                // Check if target already has a guild or a pending guild invite.
                let (target_guild_name, has_pending_invite) = {
                    let map = self.player_summaries.lock().unwrap();
                    let guild_name = map
                        .get(&target_session_id)
                        .map(|v| v.guild_name.clone())
                        .unwrap_or_default();

                    let has_pending = {
                        let world = self.world.lock().unwrap();
                        world.pending_guild_invite(target_session_id).is_some()
                    };

                    (guild_name, has_pending)
                };

                if !target_guild_name.is_empty() {
                    let m = format!("{} is already in a guild!", target_name);
                    self.send_system_chat(&m, out);
                    return;
                }

                if has_pending_invite {
                    let m = format!("{} already has a guild invite pending.", target_name);
                    self.send_system_chat(&m, out);
                    return;
                }

                // Record pending invite in world state.
                {
                    let mut world = self.world.lock().unwrap();
                    world.set_pending_guild_invite(target_session_id, &kicker_guild_name);
                }

                // Send SGuildInvite to the target session so the client shows the
                // invite dialog. The C# server sends the guild name in SGuildInvite.Name.
                let invite_pkt = SGuildInvite {
                    name: kicker_guild_name.clone(),
                };
                if let Ok(raw) = invite_pkt.encode() {
                    let encoded = Self::encode_raw(raw);
                    if target_session_id == self.session_id {
                        out.push(encoded);
                    } else {
                        let mut outboxes = self.outboxes.lock().unwrap();
                        outboxes.entry(target_session_id).or_default().push(encoded);
                    }
                }
            }
            1 => {
                // Delete member (kick or leave). Supports offline targets via DB.
                let kicking_self = kicker_name.eq_ignore_ascii_case(target_name);

                // Resolve target character (account_id, idx).
                let target_identity = match self.store.find_character_by_name(target_name) {
                    Ok(Some(info)) => info,
                    Ok(None) => {
                        let m = format!("{} does not exist.", target_name);
                        self.send_system_chat(&m, out);
                        return;
                    }
                    Err(_) => {
                        self.send_system_chat("Failed to resolve character for guild kick.", out);
                        return;
                    }
                };
                let (target_account_id, target_char_idx) = target_identity;

                // Load target's guild from DB and ensure it matches the kicker's guild.
                let target_guild = match self
                    .store
                    .load_character_guild(&target_account_id, target_char_idx)
                {
                    Ok(Some((name, rank_idx))) => (name, rank_idx),
                    Ok(None) => {
                        self.send_system_chat("Target is not in your guild.", out);
                        return;
                    }
                    Err(_) => {
                        self.send_system_chat("Failed to load target guild for kick.", out);
                        return;
                    }
                };

                if !target_guild
                    .0
                    .eq_ignore_ascii_case(&kicker_guild_name)
                {
                    self.send_system_chat("Target is not in your guild.", out);
                    return;
                }

                let target_rank_index = target_guild.1;

                // Load kicker's rank index from DB for basic permission checks.
                let kicker_rank_index = if let (Some(ref acc), Some(char_idx)) =
                    (self.account_id.as_ref(), self.current_char_index)
                {
                    match self.store.load_character_guild(acc, char_idx) {
                        Ok(Some((_name, rank_idx))) => Some(rank_idx),
                        _ => None,
                    }
                } else {
                    None
                };

                let kr = kicker_rank_index.unwrap_or(u8::MAX);
                let tr = target_rank_index;

                if !kicking_self && kr >= tr && kr != 0 {
                    self.send_system_chat("Your rank is not adequate.", out);
                    return;
                }

                // Remove from guild in world state.
                let (updated_guild, _removed_rank) = {
                    let mut world = self.world.lock().unwrap();
                    match world.guild_remove_member_by_name(&kicker_guild_name, target_name) {
                        Some(v) => v,
                        None => {
                            self.send_system_chat(
                                "Target is not recorded as a guild member.",
                                out,
                            );
                            return;
                        }
                    }
                };

                // Persist updated guild snapshot.
                let _ = self.store.save_guild(&updated_guild);

                // Clear target's guild in DB.
                let _ = self
                    .store
                    .save_character_guild(&target_account_id, target_char_idx, "", 0);

                // If target is online, update their visual guild fields and send an
                // empty GuildStatus + nameplate change.
                let target_session_id_opt = {
                    let world = self.world.lock().unwrap();
                    world.find_session_by_name(target_name)
                };

                if let Some(target_sid) = target_session_id_opt {
                    {
                        let mut map = self.player_summaries.lock().unwrap();
                        if let Some(v) = map.get_mut(&target_sid) {
                            v.guild_name.clear();
                            v.guild_rank_name.clear();
                        }
                    }

                    let status = SGuildStatus {
                        guild_name: String::new(),
                        guild_rank_name: String::new(),
                        level: 0,
                        experience: 0,
                        max_experience: 0,
                        gold: 0,
                        spare_points: 0,
                        member_count: updated_guild.member_count,
                        max_members: updated_guild.member_cap,
                        voting: false,
                        item_count: 0,
                        buff_count: 0,
                        my_options: 0,
                        my_rank_id: 0,
                    };
                    if let Ok(raw) = status.encode() {
                        let encoded = Self::encode_raw(raw);
                        let mut outboxes = self.outboxes.lock().unwrap();
                        outboxes.entry(target_sid).or_default().push(encoded);
                    }

                    let name_changed = SObjectGuildNameChanged {
                        object_id: target_sid,
                        guild_name: String::new(),
                    };
                    if let Ok(pkt) = name_changed.encode() {
                        let raw = Self::encode_raw(pkt);
                        let mut outboxes = self.outboxes.lock().unwrap();
                        outboxes.entry(target_sid).or_default().push(raw);
                    }
                }

                // Notify remaining guild members that this member left or was
                // kicked. Match C# semantics where Status=4 means left and
                // Status=3 means kicked.
                let status = if kicking_self { 4 } else { 3 };
                self.broadcast_guild_member_change(&kicker_guild_name, target_name, status, target_session_id_opt);

                // Notify the kicker.
                if kicking_self {
                    self.send_system_chat("You have left your guild.", out);
                } else {
                    let m = format!("{} has been removed from your guild.", target_name);
                    self.send_system_chat(&m, out);
                }
            }
            2 => {
                let (kicker_rank_index, kicker_options) = match self
                    .get_kicker_rank_index_and_options(&kicker_guild_name)
                {
                    Some(v) => v,
                    None => {
                        self.send_system_chat(
                            "Guild data not found for rank operation.",
                            out,
                        );
                        return;
                    }
                };

                if kicker_options & Self::GUILD_RANK_OPT_CAN_CHANGE_RANK == 0 {
                    self.send_system_chat(
                        "You are not allowed to change other members rank!",
                        out,
                    );
                    return;
                }

                let target_identity = match self.store.find_character_by_name(target_name) {
                    Ok(Some(info)) => info,
                    Ok(None) => {
                        let m = format!("{} does not exist.", target_name);
                        self.send_system_chat(&m, out);
                        return;
                    }
                    Err(_) => {
                        self.send_system_chat(
                            "Failed to resolve character for guild rank change.",
                            out,
                        );
                        return;
                    }
                };
                let (target_account_id, target_char_idx) = target_identity;

                let target_guild = match self
                    .store
                    .load_character_guild(&target_account_id, target_char_idx)
                {
                    Ok(Some((name, rank_idx))) => (name, rank_idx),
                    Ok(None) => {
                        self.send_system_chat("Target is not in your guild.", out);
                        return;
                    }
                    Err(_) => {
                        self.send_system_chat(
                            "Failed to load target guild for rank change.",
                            out,
                        );
                        return;
                    }
                };

                if !target_guild
                    .0
                    .eq_ignore_ascii_case(&kicker_guild_name)
                {
                    self.send_system_chat("Target is not in your guild.", out);
                    return;
                }

                let current_rank_index = target_guild.1;

                if !kicker_name.eq_ignore_ascii_case(target_name)
                    && kicker_rank_index >= current_rank_index
                    && kicker_rank_index != 0
                {
                    self.send_system_chat("Your rank is not adequate.", out);
                    return;
                }

                let new_rank_index = msg.rank_index;

                let (updated_guild, new_rank) = {
                    let mut world = self.world.lock().unwrap();
                    let guild = match world
                        .guild_change_member_rank(&kicker_guild_name, target_name, new_rank_index)
                    {
                        Some(g) => g,
                        None => {
                            self.send_system_chat("Rank not found!", out);
                            return;
                        }
                    };

                    let rank = guild
                        .ranks
                        .iter()
                        .find(|r| r.index == new_rank_index)
                        .or_else(|| guild.ranks.get(new_rank_index as usize))
                        .cloned();
                    let Some(rank) = rank else {
                        self.send_system_chat("Rank not found!", out);
                        return;
                    };

                    (guild, rank)
                };

                let _ = self.store.save_guild(&updated_guild);

                let _ = self.store.save_character_guild(
                    &target_account_id,
                    target_char_idx,
                    &kicker_guild_name,
                    new_rank_index,
                );

                let target_session_id_opt = {
                    let world = self.world.lock().unwrap();
                    world.find_session_by_name(target_name)
                };

                if let Some(target_sid) = target_session_id_opt {
                    {
                        let mut map = self.player_summaries.lock().unwrap();
                        if let Some(v) = map.get_mut(&target_sid) {
                            v.guild_rank_name = new_rank.name.clone();
                        }
                    }

                    let status = SGuildStatus {
                        guild_name: kicker_guild_name.clone(),
                        guild_rank_name: new_rank.name.clone(),
                        level: updated_guild.level,
                        experience: updated_guild.experience,
                        max_experience: updated_guild.max_experience,
                        gold: updated_guild.gold,
                        spare_points: updated_guild.spare_points,
                        member_count: updated_guild.member_count,
                        max_members: updated_guild.member_cap,
                        voting: false,
                        item_count: updated_guild.stored_items.len() as u8,
                        buff_count: updated_guild.buff_list.len() as u8,
                        my_options: new_rank.options,
                        my_rank_id: new_rank.index as i32,
                    };
                    if let Ok(raw) = status.encode() {
                        let encoded = Self::encode_raw(raw);
                        let mut outboxes = self.outboxes.lock().unwrap();
                        outboxes.entry(target_sid).or_default().push(encoded);
                    }

                    let ranks_bytes = Self::encode_single_rank(&new_rank);
                    let pkt = SGuildMemberChange {
                        name: kicker_name.clone(),
                        rank_index: 0,
                        status: 8,
                        ranks_bytes,
                    };
                    if let Ok(raw) = pkt.encode() {
                        let encoded = Self::encode_raw(raw);
                        let mut outboxes = self.outboxes.lock().unwrap();
                        outboxes.entry(target_sid).or_default().push(encoded);
                    }
                }

                self.broadcast_guild_member_change(
                    &kicker_guild_name,
                    target_name,
                    5,
                    target_session_id_opt,
                );
            }
            3 => {
                let (kicker_rank_index, kicker_options) = match self
                    .get_kicker_rank_index_and_options(&kicker_guild_name)
                {
                    Some(v) => v,
                    None => {
                        self.send_system_chat(
                            "Guild data not found for rank operation.",
                            out,
                        );
                        return;
                    }
                };

                if kicker_options & Self::GUILD_RANK_OPT_CAN_CHANGE_RANK == 0 {
                    self.send_system_chat("You are not allowed to change ranks!", out);
                    return;
                }

                if msg.rank_name.is_empty() || msg.rank_name.chars().count() < 3 {
                    self.send_system_chat("Rank name to short!", out);
                    return;
                }

                if msg.rank_name.contains('\\') || msg.rank_name.chars().count() > 20 {
                    return;
                }

                if kicker_rank_index > msg.rank_index {
                    self.send_system_chat("Your rank is not adequate.", out);
                    return;
                }

                let (updated_guild, changed_rank) = {
                    let mut world = self.world.lock().unwrap();
                    let guild = match world
                        .guild_change_rank_name(&kicker_guild_name, msg.rank_index, &msg.rank_name)
                    {
                        Some(g) => g,
                        None => {
                            self.send_system_chat("Rank not found!", out);
                            return;
                        }
                    };

                    let idx = msg.rank_index as usize;
                    let rank = match guild.ranks.get(idx) {
                        Some(r) => r.clone(),
                        None => {
                            self.send_system_chat("Rank not found!", out);
                            return;
                        }
                    };

                    (guild, rank)
                };

                let _ = self.store.save_guild(&updated_guild);

                let ranks_bytes = Self::encode_single_rank(&changed_rank);
                self.broadcast_guild_member_change_with_bytes(
                    &kicker_guild_name,
                    &kicker_name,
                    7,
                    None,
                    ranks_bytes,
                );
            }
            4 => {
                let (_kicker_rank_index, kicker_options) = match self
                    .get_kicker_rank_index_and_options(&kicker_guild_name)
                {
                    Some(v) => v,
                    None => {
                        self.send_system_chat(
                            "Guild data not found for rank operation.",
                            out,
                        );
                        return;
                    }
                };

                if kicker_options & Self::GUILD_RANK_OPT_CAN_CHANGE_RANK == 0 {
                    self.send_system_chat("You are not allowed to change ranks!", out);
                    return;
                }

                let (updated_guild, new_rank) = {
                    let mut world = self.world.lock().unwrap();
                    match world.guild_new_rank(&kicker_guild_name) {
                        Some(v) => v,
                        None => {
                            self.send_system_chat(
                                "You cannot have anymore ranks.",
                                out,
                            );
                            return;
                        }
                    }
                };

                let _ = self.store.save_guild(&updated_guild);

                let ranks_bytes = Self::encode_single_rank(&new_rank);
                self.broadcast_guild_member_change_with_bytes(
                    &kicker_guild_name,
                    &kicker_name,
                    6,
                    None,
                    ranks_bytes,
                );
            }
            5 => {
                let (kicker_rank_index, kicker_options) = match self
                    .get_kicker_rank_index_and_options(&kicker_guild_name)
                {
                    Some(v) => v,
                    None => {
                        self.send_system_chat(
                            "Guild data not found for rank operation.",
                            out,
                        );
                        return;
                    }
                };

                if kicker_options & Self::GUILD_RANK_OPT_CAN_CHANGE_RANK == 0 {
                    self.send_system_chat("You are not allowed to change ranks!", out);
                    return;
                }

                let option: u8 = match msg.rank_name.parse() {
                    Ok(v) => v,
                    Err(_) => return,
                };

                if option > 7 {
                    self.send_system_chat("Rank not found!", out);
                    return;
                }

                let enabled_str = msg.name.trim();
                let enabled = if enabled_str == "true" {
                    true
                } else if enabled_str == "false" {
                    false
                } else {
                    return;
                };

                if kicker_rank_index >= msg.rank_index {
                    self.send_system_chat(
                        "You cannot change the options of your own rank!",
                        out,
                    );
                    return;
                }

                let (updated_guild, changed_rank) = {
                    let mut world = self.world.lock().unwrap();
                    let guild = match world.guild_change_rank_option(
                        &kicker_guild_name,
                        msg.rank_index,
                        option,
                        enabled,
                    ) {
                        Some(g) => g,
                        None => {
                            self.send_system_chat("Rank not found!", out);
                            return;
                        }
                    };

                    let idx = msg.rank_index as usize;
                    let rank = match guild.ranks.get(idx) {
                        Some(r) => r.clone(),
                        None => {
                            self.send_system_chat("Rank not found!", out);
                            return;
                        }
                    };

                    (guild, rank)
                };

                let _ = self.store.save_guild(&updated_guild);

                let ranks_bytes = Self::encode_single_rank(&changed_rank);
                self.broadcast_guild_member_change_with_bytes(
                    &kicker_guild_name,
                    &kicker_name,
                    7,
                    None,
                    ranks_bytes,
                );
            }
            _ => {
                self.send_system_chat("Guild member operation not implemented.", out);
            }
        }
    }

    pub(crate) fn handle_guild_invite(
        &mut self,
        msg: CGuildInvite,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }
        if !msg.accept_invite {
            let mut world = self.world.lock().unwrap();
            world.clear_pending_guild_invite(self.session_id);
            return;
        }

        let guild_name_opt = {
            let mut world = self.world.lock().unwrap();
            world.take_pending_guild_invite(self.session_id)
        };

        let Some(guild_name) = guild_name_opt else {
            self.send_system_chat("You have not been invited to a guild.", out);
            return;
        };

        // Try to add this player to the invited guild in the world state.
        let join_result = {
            let mut world = self.world.lock().unwrap();
            world.guild_add_member_by_name(&guild_name)
        };

        match join_result {
            Err(GuildJoinError::NotFound) => {
                let msg = format!("Guild {} no longer exists.", guild_name);
                self.send_system_chat(&msg, out);
                return;
            }
            Err(GuildJoinError::Full) => {
                let msg = format!("Guild {} is full.", guild_name);
                self.send_system_chat(&msg, out);
                return;
            }
            Ok(guild) => {
                // Persist updated guild snapshot.
                let _ = self.store.save_guild(&guild);

                // Update cached visual summary so nameplates show the new guild tag.
                {
                    let mut map = self.player_summaries.lock().unwrap();
                    if let Some(v) = map.get_mut(&self.session_id) {
                        v.guild_name = guild.name.clone();
                        v.guild_rank_name = "Member".to_string();
                    }
                }

                // Persist character -> guild mapping.
                if let (Some(ref account_id), Some(char_idx)) =
                    (self.account_id.as_ref(), self.current_char_index)
                {
                    let _ = self
                        .store
                        .save_character_guild(account_id, char_idx, &guild.name, 1);
                }

                // Notify self and nearby players that this object's guild name has changed.
                let guild_name_changed = SObjectGuildNameChanged {
                    object_id: self.session_id,
                    guild_name: guild.name.clone(),
                };
                if let Ok(pkt) = guild_name_changed.encode() {
                    let raw = Self::encode_raw(pkt);
                    // Send to self
                    out.push(raw.clone());
                    // Broadcast to other players in view
                    self.enqueue_for_viewers(
                        self.current_map_index,
                        self.current_x,
                        self.current_y,
                        raw,
                    );
                }

                // Send updated guild status to the joining player.
                let status = SGuildStatus {
                    guild_name: guild.name.clone(),
                    guild_rank_name: "Member".to_string(),
                    level: guild.level,
                    experience: guild.experience,
                    max_experience: guild.max_experience,
                    gold: guild.gold,
                    spare_points: guild.spare_points,
                    member_count: guild.member_count,
                    max_members: guild.member_cap,
                    voting: false,
                    item_count: guild.stored_items.len() as u8,
                    buff_count: guild.buff_list.len() as u8,
                    my_options: 0,
                    my_rank_id: 1,
                };
                if let Ok(raw) = status.encode() {
                    out.push(Self::encode_raw(raw));
                }

                // Notify other guild members that a new member has joined.
                let joiner_name = {
                    let map = self.player_summaries.lock().unwrap();
                    map.get(&self.session_id)
                        .map(|v| v.name.clone())
                        .unwrap_or_default()
                };
                self.broadcast_guild_member_change(&guild.name, &joiner_name, 2, Some(self.session_id));

                let ok = format!("You have joined guild {}.", guild_name);
                self.send_system_chat(&ok, out);
            }
        }

        let mut world = self.world.lock().unwrap();
        world.clear_pending_guild_invite(self.session_id);
    }
}
