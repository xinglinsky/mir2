use std::path::Path;

use crystal_shared_proto::item::SSellItem;
use crystal_shared_proto::npc::{CBuyItem, CSellItem};
use crystal_shared_proto::item_types::UserItemData;
use crystal_shared_proto::user::{SGainedGold, SUserSlotsRefresh, SLoseGold};

use super::{LoginConnection, Stage};

impl LoginConnection {
    pub(crate) fn handle_sell_item(&mut self, msg: CSellItem, out: &mut Vec<Vec<u8>>) {
        tracing::debug!(
            "SellItem: start stage={:?} unique_id={} count={}",
            self.stage,
            msg.unique_id,
            msg.count,
        );

        if self.stage != Stage::InGame {
            tracing::debug!("SellItem: early-return, not in-game stage");
            return;
        }

        if msg.count == 0 {
            tracing::debug!("SellItem: early-return, count is 0");
            return;
        }

        let Some(stats) = self.current_stats.clone() else {
            tracing::debug!("SellItem: early-return, no current_stats available");
            return;
        };

        // Locate nearest NPC within DATA_RANGE on current map, mirroring the
        // distance gating in CallNPC. For now we just pick the first matching
        // NPC; later this can be refined to track the active NPC from CallNPC.
        let npc_opt = self
            .world_db
            .npc_infos
            .iter()
            .find(|n| {
                n.map_index == self.current_map_index
                    && (n.location_x - self.current_x).abs() <= Self::DATA_RANGE
                    && (n.location_y - self.current_y).abs() <= Self::DATA_RANGE
            });

        let npc = match npc_opt {
            Some(n) => n,
            None => {
                tracing::debug!("SellItem: early-return, no nearby NPC on map={}", self.current_map_index);
                return;
            }
        };

        // Load the player's current items from the world.
        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((
                    crystal_server_core::item::Inventory::new_default(),
                    crystal_server_core::item::Equipment::new_default(),
                ))
        };

        // Find the item in the inventory by unique_id.
        let mut found_index: Option<usize> = None;
        for (idx, slot) in inv.slots.iter().enumerate() {
            if let Some(item) = slot {
                if item.unique_id == msg.unique_id {
                    found_index = Some(idx);
                    break;
                }
            }
        }

        let idx = match found_index {
            Some(i) => i,
            None => {
                tracing::debug!(
                    "SellItem: early-return, item with unique_id={} not found in inventory",
                    msg.unique_id
                );
                return;
            }
        };

        let item = match inv.slots[idx].clone() {
            Some(it) => it,
            None => {
                tracing::debug!("SellItem: early-return, inventory slot {} is empty", idx);
                return;
            }
        };

        if msg.count as u32 > item.count as u32 {
            tracing::debug!(
                "SellItem: early-return, requested count={} > item.count={} for unique_id={}",
                msg.count,
                item.count,
                msg.unique_id
            );
            return;
        }

        // Look up ItemInfo to compute sale price.
        let info = match self
            .world_db
            .item_infos
            .iter()
            .find(|i| i.index == item.item_index)
        {
            Some(i) => i,
            None => {
                tracing::debug!(
                    "SellItem: early-return, ItemInfo not found for item_index={} (unique_id={})",
                    item.item_index,
                    msg.unique_id
                );
                return;
            }
        };

        // Base sell price is half of the normal price * count, scaled by NPC rate.
        let base_total = match info.price.checked_mul(msg.count as u32) {
            Some(v) => v,
            None => {
                tracing::debug!(
                    "SellItem: early-return, base_total overflow price={} count={}",
                    info.price,
                    msg.count
                );
                return;
            }
        };

        let half = base_total / 2;
        let rate = (npc.rate as f32) / 100.0;
        let mut gold_gain = ((half as f32) * rate).floor() as u32;
        if gold_gain == 0 && half > 0 {
            gold_gain = 1;
        }

        let new_gold_u64 = (stats.gold as i64 as i128) + (gold_gain as i128);
        if new_gold_u64 > i64::MAX as i128 {
            tracing::debug!(
                "SellItem: early-return, gold overflow if adding gain={}, current_gold={}",
                gold_gain,
                stats.gold
            );
            return;
        }

        // Update inventory: either remove the stack or decrement the count.
        // Snapshot of the sold portion of the stack for BuyBack. For a full
        // stack sale this is the entire item; for a partial sale this is a
        // clone with Count = msg.count.
        let mut sold_item: UserItemData = item.clone();
        sold_item.count = msg.count;

        if msg.count as u16 == item.count {
            inv.slots[idx] = None;
        } else {
            let mut updated = item.clone();
            updated.count = updated.count.saturating_sub(msg.count);
            inv.slots[idx] = Some(updated);
        }

        // Persist updated items to the world and append to the per-player
        // BuyBack list for this NPC.
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
            world.add_buyback_item(
                self.session_id,
                self.current_map_index,
                npc.index,
                sold_item,
            );
        }

        // Update gold in current_stats and store.
        let mut new_stats = stats.clone();
        new_stats.gold = new_stats.gold.saturating_add(gold_gain as i64);

        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let _ = self
                .store
                .save_character_stats(account_id, char_idx, &new_stats);
        }

        self.current_stats = Some(new_stats.clone());

        // Notify client about the sale result (SSellItem mirrors C# ServerPackets.SellItem).
        let sell_pkt = SSellItem {
            unique_id: msg.unique_id,
            count: msg.count,
            success: true,
        };
        if let Ok(raw) = sell_pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Also notify client about gained gold and refreshed slots.
        let gained = SGainedGold { gold: gold_gain };
        if let Ok(raw) = gained.encode() {
            out.push(Self::encode_raw(raw));
        }

        let refresh = SUserSlotsRefresh {
            inventory: inv.slots,
            equipment: eq.slots,
        };
        if let Ok(raw) = refresh.encode() {
            out.push(Self::encode_raw(raw));
        }

        tracing::debug!(
            "SellItem: success unique_id={} count={} gold_gain={} gold_after={}",
            msg.unique_id,
            msg.count,
            gold_gain,
            new_stats.gold
        );
    }

    pub(crate) fn handle_buy_item(&mut self, msg: CBuyItem, out: &mut Vec<Vec<u8>>) {
        tracing::debug!(
            "BuyItem: start stage={:?} count={} panel_type={} item_index={}",
            self.stage,
            msg.count,
            msg.panel_type,
            msg.item_index,
        );

        if self.stage != Stage::InGame {
            tracing::debug!("BuyItem: early-return, not in-game stage");
            return;
        }

        if msg.count == 0 {
            tracing::debug!("BuyItem: early-return, count is 0");
            return;
        }

        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => {
                tracing::debug!("BuyItem: early-return, no current_stats available");
                return;
            }
        };

        // Only handle standard Buy panel for now (PanelType.Buy = 0 in C#).
        if msg.panel_type != 0 {
            tracing::debug!(
                "BuyItem: early-return, unsupported panel_type={} (only 0/Buy supported)",
                msg.panel_type
            );
            return;
        }

        // Derive npc_index from the high 32 bits of the unique item_index.
        let npc_index = (msg.item_index >> 32) as i32;
        tracing::debug!(
            "BuyItem: derived npc_index={} from item_index={}",
            npc_index,
            msg.item_index
        );

        let npc_opt = self
            .world_db
            .npc_infos
            .iter()
            .find(|n| n.index == npc_index && n.map_index == self.current_map_index);

        let npc = match npc_opt {
            Some(n) => n,
            None => {
                tracing::debug!(
                    "BuyItem: early-return, npc not found npc_index={} map_index={}",
                    npc_index,
                    self.current_map_index
                );
                return;
            }
        };

        // Range check against the NPC, mirroring CallNPC distance checks.
        let dx = npc.location_x - self.current_x;
        let dy = npc.location_y - self.current_y;
        if dx.abs() > Self::DATA_RANGE || dy.abs() > Self::DATA_RANGE {
            tracing::debug!(
                "BuyItem: early-return, out of range dx={} dy={} data_range={}",
                dx,
                dy,
                Self::DATA_RANGE
            );
            return;
        }

        // Reload the NPC's [TRADE] list to resolve the selected goods line.
        let root_deploy = Path::new("./deploy/Envir/NPCs");
        let root_plain = Path::new("./Envir/NPCs");
        let root = if root_deploy.exists() { root_deploy } else { root_plain };
        if !root.exists() {
            tracing::debug!("BuyItem: early-return, NPC script root not found");
            return;
        }

        let script_path = match Self::find_npc_script_path(root, &npc.file_name) {
            Some(p) => p,
            None => {
                tracing::debug!(
                    "BuyItem: early-return, npc script file not found for file_name='{}'",
                    npc.file_name
                );
                return;
            }
        };
        tracing::debug!("BuyItem: using npc script path {:?}", script_path);

        let specs = match Self::load_npc_trade_goods_from_file(&script_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::debug!(
                    "BuyItem: early-return, failed to load trade goods: {:?}",
                    e
                );
                return;
            }
        };
        tracing::debug!("BuyItem: loaded {} trade goods entries", specs.len());

        // Compute the goods index from the low 32 bits of the unique id.
        let base_uid = (npc.index as u64) << 32;
        if msg.item_index <= base_uid {
            tracing::debug!(
                "BuyItem: early-return, item_index {} <= base_uid {}",
                msg.item_index,
                base_uid
            );
            return;
        }
        let rel = msg.item_index - base_uid;
        if rel == 0 {
            tracing::debug!("BuyItem: early-return, rel computed as 0");
            return;
        }
        let idx = (rel - 1) as usize;
        if idx >= specs.len() {
            tracing::debug!(
                "BuyItem: early-return, computed idx={} out of bounds (len={})",
                idx,
                specs.len()
            );
            return;
        }

        let (ref name, _script_count) = specs[idx];
        tracing::debug!("BuyItem: resolved goods idx={} name='{}'", idx, name);

        // Locate the ItemInfoData by name.
        let info = match self
            .world_db
            .item_infos
            .iter()
            .find(|i| i.name.eq_ignore_ascii_case(name))
        {
            Some(i) => i,
            None => {
                tracing::debug!(
                    "BuyItem: early-return, ItemInfo not found for name='{}'",
                    name
                );
                return;
            }
        };

        // Enforce stack size limit consistent with C# goods.Info.StackSize.
        if msg.count as u32 > info.stack_size as u32 {
            tracing::debug!(
                "BuyItem: early-return, requested count={} exceeds stack_size={} for item='{}'",
                msg.count,
                info.stack_size,
                info.name
            );
            return;
        }

        // Compute base price and apply NPC rate.
        let base_price = match info.price.checked_mul(msg.count as u32) {
            Some(v) => v,
            None => {
                tracing::debug!(
                    "BuyItem: early-return, price overflow price={} count={}",
                    info.price,
                    msg.count
                );
                return;
            }
        };

        let rate = (npc.rate as f32) / 100.0;
        let mut cost = ((base_price as f32) * rate).floor() as u32;
        if cost == 0 && base_price > 0 {
            cost = 1;
        }
        tracing::debug!(
            "BuyItem: pricing item='{}' base_price={} rate={} cost={} player_gold={}",
            info.name,
            base_price,
            rate,
            cost,
            stats.gold
        );

        let cost_i64 = cost as i64;
        if stats.gold < cost_i64 {
            tracing::debug!(
                "BuyItem: early-return, insufficient gold: have={} cost={}",
                stats.gold,
                cost_i64
            );
            return;
        }

        // Deduct gold from the cached CharacterStats and persist to the store.
        let mut new_stats = stats.clone();
        new_stats.gold = new_stats.gold.saturating_sub(cost_i64);

        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let _ = self
                .store
                .save_character_stats(account_id, char_idx, &new_stats);
        }

        self.current_stats = Some(new_stats.clone());

        let lose = SLoseGold { gold: cost };
        if let Ok(raw) = lose.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Insert the purchased item into the first empty inventory slot.
        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((crystal_server_core::item::Inventory::new_default(), crystal_server_core::item::Equipment::new_default()))
        };

        // Mirror C# HumanObject.AddItem behaviour: belt slots occupy the
        // first few inventory indices (BeltSize = 6 by default). Normal
        // items should prefer bag slots starting at index 6, and only fall
        // back to the belt region if the bag is completely full.
        let free_slot = self.find_free_inventory_slot_for_new_item(&inv);

        let slot_index = match free_slot {
            Some(i) => i,
            None => {
                tracing::debug!(
                    "BuyItem: early-return, no free inventory slot for item='{}'",
                    info.name
                );
                return;
            }
        };

        // Generate a per-session unique_id using session_id and time; this is
        // sufficient for inventory operations and mirrors the spirit of the C#
        // UniqueID usage.
        let unique_id = self.generate_unique_item_id_for_session();

        let user_item = crystal_server_core::item::create_fresh_user_item(
            info,
            unique_id,
            msg.count,
        );

        inv.slots[slot_index] = Some(user_item);

        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
        }

        let refresh = SUserSlotsRefresh {
            inventory: inv.slots,
            equipment: eq.slots,
        };
        if let Ok(raw) = refresh.encode() {
            out.push(Self::encode_raw(raw));
        }

        tracing::debug!(
            "BuyItem: success item='{}' count={} cost={} slot={} gold_after={}",
            info.name,
            msg.count,
            cost,
            slot_index,
            new_stats.gold
        );
    }

    fn find_free_inventory_slot_for_new_item(
        &self,
        inv: &crystal_server_core::item::Inventory,
    ) -> Option<usize> {
        let belt_size: usize = 6;
        let inv_len = inv.slots.len();

        let mut free_slot: Option<usize> = None;

        // 1) Prefer non-belt bag area [belt_size .. len)
        if inv_len > belt_size {
            for i in belt_size..inv_len {
                if inv.slots[i].is_none() {
                    free_slot = Some(i);
                    break;
                }
            }
        }

        // 2) If bag is full, fall back to belt area [0 .. belt_size)
        if free_slot.is_none() {
            let upper = belt_size.min(inv_len);
            for i in 0..upper {
                if inv.slots[i].is_none() {
                    free_slot = Some(i);
                    break;
                }
            }
        }

        free_slot
    }

    fn generate_unique_item_id_for_session(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let ts = now.as_nanos() as u64;
        ((self.session_id as u64) << 32) ^ ts
    }
}

