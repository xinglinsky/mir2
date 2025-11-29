use crystal_server_core::item::{Equipment, Inventory};
use crystal_shared_proto::item_types::UserItemData;
use crystal_shared_proto::login::CCollectParcel;
use crystal_shared_proto::mail::SParcelCollected;
use crystal_shared_proto::user::{SGainedGold, SUserSlotsRefresh};

use super::{LoginConnection, Stage};

impl LoginConnection {
    pub(crate) fn handle_collect_parcel(
        &mut self,
        msg: CCollectParcel,
        out: &mut Vec<Vec<u8>>,
    ) {
        tracing::debug!(
            "CollectParcel: start stage={:?} mail_id={}",
            self.stage,
            msg.mail_id,
        );

        if self.stage != Stage::InGame {
            return;
        }

        let (account_id, char_idx) = match (&self.account_id, self.current_char_index) {
            (Some(a), Some(i)) => (a.clone(), i),
            _ => return,
        };

        let mut mails = match self.store.load_character_mail(&account_id, char_idx) {
            Ok(m) => m,
            Err(e) => {
                tracing::debug!(
                    "CollectParcel: failed to load mail for account_id={} idx={} err={:?}",
                    account_id,
                    char_idx,
                    e,
                );
                self.send_system_chat("读取邮件数据失败，暂时无法领取包裹。", out);
                return;
            }
        };

        let pos = match mails.iter().position(|m| m.mail_id == msg.mail_id) {
            Some(i) => i,
            None => {
                tracing::debug!(
                    "CollectParcel: mail_id={} not found for account_id={} idx={}",
                    msg.mail_id,
                    account_id,
                    char_idx,
                );
                return;
            }
        };

        let mail = &mut mails[pos];

        if mail.items.is_empty() && mail.gold == 0 {
            let pkt = SParcelCollected { result: 0 };
            out.push(Self::encode_raw(pkt.encode()));
            return;
        }

        // Decode attachment items from stored bytes.
        let mut decoded_items: Vec<UserItemData> = Vec::new();
        for bytes in &mail.items {
            match UserItemData::decode_from_bytes(bytes) {
                Ok(it) => decoded_items.push(it),
                Err(e) => {
                    tracing::debug!(
                        "CollectParcel: failed to decode UserItemData from mail_id={} err={:?}",
                        mail.mail_id,
                        e,
                    );
                    self.send_system_chat("解析邮件附件失败，暂时无法领取包裹。", out);
                    let pkt = SParcelCollected { result: 0 };
                    out.push(Self::encode_raw(pkt.encode()));
                    return;
                }
            }
        }

        // Load the player's current items from the world.
        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((Inventory::new_default(), Equipment::new_default()))
        };

        // Simulate inventory space to ensure all items can fit, mirroring the
        // all-or-nothing behaviour of the C# CanGainItems/GainItem logic.
        let mut sim_inv = inv.clone();
        for it in &decoded_items {
            let slot = match self.find_free_inventory_slot_for_new_item(&sim_inv) {
                Some(s) => s,
                None => {
                    tracing::debug!(
                        "CollectParcel: insufficient inventory space for mail_id={} item_index={}",
                        mail.mail_id,
                        it.item_index,
                    );
                    self.send_system_chat("背包空间不足，无法领取邮件物品。", out);
                    let pkt = SParcelCollected { result: 0 };
                    out.push(Self::encode_raw(pkt.encode()));
                    return;
                }
            };
            sim_inv.slots[slot] = Some(it.clone());
        }

        // Insert items for real.
        for it in decoded_items {
            let slot = match self.find_free_inventory_slot_for_new_item(&inv) {
                Some(s) => s,
                None => {
                    tracing::debug!(
                        "CollectParcel: slot allocation mismatch after simulation for mail_id={}",
                        mail.mail_id,
                    );
                    self.send_system_chat("领取邮件物品时发生错误。", out);
                    let pkt = SParcelCollected { result: 0 };
                    out.push(Self::encode_raw(pkt.encode()));
                    return;
                }
            };
            inv.slots[slot] = Some(it);
        }

        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
        }

        // Handle gold collection via CharacterStats, mirroring GainGold.
        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => {
                tracing::debug!(
                    "CollectParcel: no current_stats available for account_id={} idx={} mail_id={}",
                    account_id,
                    char_idx,
                    mail.mail_id,
                );
                self.send_system_chat("当前角色状态不可用，暂时无法领取包裹金币。", out);
                let pkt = SParcelCollected { result: 0 };
                out.push(Self::encode_raw(pkt.encode()));
                return;
            }
        };

        let mut new_stats = stats.clone();
        let mut gold_gained: u32 = 0;
        if mail.gold > 0 {
            let add = mail.gold as i64;
            let (sum, overflow) = new_stats.gold.overflowing_add(add);
            new_stats.gold = if overflow { i64::MAX } else { sum };
            gold_gained = mail.gold;
        }

        if let (Some(ref acc_id), Some(cidx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let _ = self
                .store
                .save_character_stats(acc_id, cidx, &new_stats);
        }

        self.current_stats = Some(new_stats.clone());

        if gold_gained > 0 {
            let gained = SGainedGold { gold: gold_gained };
            if let Ok(raw) = gained.encode() {
                out.push(Self::encode_raw(raw));
            }
        }

        // Update mail: clear items and gold, mark as collected, and persist.
        mail.items.clear();
        mail.gold = 0;
        mail.collected = true;

        let _ = self
            .store
            .save_character_mail(&account_id, char_idx, &mails);

        let slots_refresh = SUserSlotsRefresh {
            inventory: inv.slots,
            equipment: eq.slots,
        };
        if let Ok(raw) = slots_refresh.encode() {
            out.push(Self::encode_raw(raw));
        }

        let pkt = SParcelCollected { result: 1 };
        out.push(Self::encode_raw(pkt.encode()));
    }
}
