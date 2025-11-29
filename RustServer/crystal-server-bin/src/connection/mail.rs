use crystal_server_core::account::StoredMail;
use crystal_server_core::item::{Equipment, Inventory};
use crystal_shared_proto::item_types::UserItemData;
use crystal_shared_proto::login::{
    CCollectParcel,
    CDeleteMail,
    CLockMail,
    CMailCost,
    CReadMail,
    CSendMail,
};
use crystal_shared_proto::mail::{SParcelCollected, SMailCost, SMailSent, SReceiveMail};
use crystal_shared_proto::item::SDeleteItem;
use crystal_shared_proto::user::{SGainedGold, SLoseGold, SUserSlotsRefresh};

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

    pub(crate) fn handle_read_mail(
        &mut self,
        msg: CReadMail,
        out: &mut Vec<Vec<u8>>,
    ) {
        tracing::debug!("ReadMail: stage={:?} mail_id={}", self.stage, msg.mail_id);

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
                    "ReadMail: failed to load mail for account_id={} idx={} err={:?}",
                    account_id,
                    char_idx,
                    e,
                );
                return;
            }
        };

        if let Some(mail) = mails.iter_mut().find(|m| m.mail_id == msg.mail_id) {
            if !mail.opened {
                mail.opened = true;
                let _ = self
                    .store
                    .save_character_mail(&account_id, char_idx, &mails);
            }
        } else {
            tracing::debug!(
                "ReadMail: mail_id={} not found for account_id={} idx={}",
                msg.mail_id,
                account_id,
                char_idx,
            );
            return;
        }

        // Re-send mailbox, mirroring C# GetMail behaviour.
        self.send_full_mailbox(char_idx, out);
    }

    pub(crate) fn handle_delete_mail(
        &mut self,
        msg: CDeleteMail,
        out: &mut Vec<Vec<u8>>,
    ) {
        tracing::debug!("DeleteMail: stage={:?} mail_id={}", self.stage, msg.mail_id);

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
                    "DeleteMail: failed to load mail for account_id={} idx={} err={:?}",
                    account_id,
                    char_idx,
                    e,
                );
                return;
            }
        };

        let before = mails.len();
        mails.retain(|m| m.mail_id != msg.mail_id);

        if mails.len() == before {
            tracing::debug!(
                "DeleteMail: mail_id={} not found for account_id={} idx={}",
                msg.mail_id,
                account_id,
                char_idx,
            );
            return;
        }

        let _ = self
            .store
            .save_character_mail(&account_id, char_idx, &mails);

        self.send_full_mailbox(char_idx, out);
    }

    pub(crate) fn handle_lock_mail(
        &mut self,
        msg: CLockMail,
        out: &mut Vec<Vec<u8>>,
    ) {
        tracing::debug!(
            "LockMail: stage={:?} mail_id={} lock={}",
            self.stage,
            msg.mail_id,
            msg.lock,
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
                    "LockMail: failed to load mail for account_id={} idx={} err={:?}",
                    account_id,
                    char_idx,
                    e,
                );
                return;
            }
        };

        if let Some(mail) = mails.iter_mut().find(|m| m.mail_id == msg.mail_id) {
            mail.locked = msg.lock;
            let _ = self
                .store
                .save_character_mail(&account_id, char_idx, &mails);
        } else {
            tracing::debug!(
                "LockMail: mail_id={} not found for account_id={} idx={}",
                msg.mail_id,
                account_id,
                char_idx,
            );
            return;
        }

        self.send_full_mailbox(char_idx, out);
    }

    pub(crate) fn handle_mail_cost(
        &mut self,
        msg: CMailCost,
        out: &mut Vec<Vec<u8>>,
    ) {
        tracing::debug!(
            "MailCost: stage={:?} gold={} stamped={} items_idx={:?}",
            self.stage,
            msg.gold,
            msg.stamped,
            msg.items_idx,
        );

        if self.stage != Stage::InGame {
            return;
        }

        // For now, keep mail sending free on the Rust server and simply
        // acknowledge the request with a zero cost. This matches the
        // protocol shape of C# MailCost without replicating all pricing
        // settings yet.
        let pkt = SMailCost { cost: 0 };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_send_mail(
        &mut self,
        msg: CSendMail,
        out: &mut Vec<Vec<u8>>,
    ) {
        tracing::debug!(
            "SendMail: stage={:?} to={} gold={} stamped={}",
            self.stage,
            msg.name,
            msg.gold,
            msg.stamped,
        );

        if self.stage != Stage::InGame {
            return;
        }

        // Basic message length guard mirroring the C# limit of 500 chars.
        if msg.message.chars().count() > 500 {
            self.send_system_chat("邮件内容过长，无法发送。", out);
            let pkt = SMailSent { result: -1 };
            out.push(Self::encode_raw(pkt.encode()));
            return;
        }

        let sender_account_id = match &self.account_id {
            Some(id) => id.clone(),
            None => {
                let pkt = SMailSent { result: -1 };
                out.push(Self::encode_raw(pkt.encode()));
                return;
            }
        };

        let sender_char_idx = match self.current_char_index {
            Some(i) => i,
            None => {
                let pkt = SMailSent { result: -1 };
                out.push(Self::encode_raw(pkt.encode()));
                return;
            }
        };

        // Resolve recipient (account_id, character_index) by name using the
        // shared account store, mirroring the C# Envir.GetCharacterInfo.
        let (recipient_account_id, recipient_char_idx) = match self.store.find_character_by_name(&msg.name) {
            Ok(Some((acc, idx))) => (acc, idx),
            Ok(None) => {
                let m = format!("无法找到玩家 {}。", msg.name);
                self.send_system_chat(&m, out);
                let pkt = SMailSent { result: -1 };
                out.push(Self::encode_raw(pkt.encode()));
                return;
            }
            Err(e) => {
                tracing::debug!("SendMail: find_character_by_name failed name={} err={:?}", msg.name, e);
                self.send_system_chat("查找收件人时发生错误，暂时无法发送邮件。", out);
                let pkt = SMailSent { result: -1 };
                out.push(Self::encode_raw(pkt.encode()));
                return;
            }
        };

        // Enforce a simple mailbox capacity limit of 50, matching the C#
        // PlayerObject.SendMail check.
        let mut recipient_mails = match self
            .store
            .load_character_mail(&recipient_account_id, recipient_char_idx)
        {
            Ok(m) => m,
            Err(e) => {
                tracing::debug!(
                    "SendMail: failed to load recipient mail for account_id={} idx={} err={:?}",
                    recipient_account_id,
                    recipient_char_idx,
                    e,
                );
                self.send_system_chat("读取收件人邮箱失败，暂时无法发送邮件。", out);
                let pkt = SMailSent { result: -1 };
                out.push(Self::encode_raw(pkt.encode()));
                return;
            }
        };

        if recipient_mails.len() >= 50 {
            self.send_system_chat("对方邮箱已满，无法接收更多邮件。", out);
            let pkt = SMailSent { result: -1 };
            out.push(Self::encode_raw(pkt.encode()));
            return;
        }

        // Load sender inventory/equipment from the world.
        let (mut inv, eq) = {
            let world = self.world.lock().unwrap();
            world
                .player_items(self.session_id)
                .unwrap_or((Inventory::new_default(), Equipment::new_default()))
        };

        // Gather attachments by unique_id from the sender's inventory. We
        // honour the stamped flag by allowing up to 5 attachments when
        // stamped, otherwise only the first.
        let max_items = if msg.stamped { 5 } else { 1 };
        let mut gift_items: Vec<UserItemData> = Vec::new();

        for j in 0..max_items {
            let uid = msg.items_idx[j];
            if uid == 0 {
                continue;
            }

            let mut found_slot: Option<usize> = None;
            for (i, slot) in inv.slots.iter().enumerate() {
                if let Some(item) = slot {
                    if item.unique_id == uid {
                        found_slot = Some(i);
                        break;
                    }
                }
            }

            let slot_idx = match found_slot {
                Some(i) => i,
                None => {
                    tracing::debug!(
                        "SendMail: attachment unique_id={} not found in inventory, skipping",
                        uid,
                    );
                    continue;
                }
            };

            if let Some(item) = inv.slots[slot_idx].take() {
                // TODO: enforce Bind / NoMail flags once item binding rules
                // are fully mirrored from C#.
                gift_items.push(item.clone());

                let del = SDeleteItem {
                    unique_id: item.unique_id,
                    count: item.count,
                };
                if let Ok(raw) = del.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
        }

        // Handle gold cost using CharacterStats.gold as the single source of
        // truth on the Rust server. For now we only deduct the user-specified
        // Gold value and keep MailCost free.
        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => {
                self.send_system_chat("当前角色状态不可用，暂时无法发送邮件。", out);
                let pkt = SMailSent { result: -1 };
                out.push(Self::encode_raw(pkt.encode()));
                return;
            }
        };

        let mut new_stats = stats.clone();
        if msg.gold > 0 {
            let needed = msg.gold as i64;
            if new_stats.gold < needed {
                self.send_system_chat("您的金币不足，无法发送包含金币的邮件。", out);
                let pkt = SMailSent { result: -1 };
                out.push(Self::encode_raw(pkt.encode()));
                return;
            }

            new_stats.gold = new_stats.gold.saturating_sub(needed);
        }

        if msg.gold > 0 {
            if let (Some(ref acc_id), Some(cidx)) =
                (self.account_id.as_ref(), self.current_char_index)
            {
                let _ = self
                    .store
                    .save_character_stats(acc_id, cidx, &new_stats);
            }

            self.current_stats = Some(new_stats.clone());

            let lose = SLoseGold { gold: msg.gold };
            if let Ok(raw) = lose.encode() {
                out.push(Self::encode_raw(raw));
            }
        } else {
            self.current_stats = Some(new_stats.clone());
        }

        // Persist updated inventory/equipment for the sender.
        {
            let mut world = self.world.lock().unwrap();
            world.set_player_items(self.session_id, inv.clone(), eq.clone());
        }

        let slots_refresh = SUserSlotsRefresh {
            inventory: inv.slots,
            equipment: eq.slots,
        };
        if let Ok(raw) = slots_refresh.encode() {
            out.push(Self::encode_raw(raw));
        }

        // Build StoredMail for the recipient.
        let sender_name = if let Some(idx) = self.current_char_index {
            self.characters
                .iter()
                .find(|c| c.index == idx)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| String::new())
        } else {
            String::new()
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let mail_id = self.generate_unique_item_id_for_session();
        let date_sent_binary: i64 = 621355968000000000i64
            .saturating_add((now.as_secs() as i64).saturating_mul(10_000_000))
            .saturating_add((now.subsec_nanos() / 100) as i64);

        let mut item_bytes: Vec<Vec<u8>> = Vec::with_capacity(gift_items.len());
        for it in &gift_items {
            match it.encode_to_bytes() {
                Ok(b) => item_bytes.push(b),
                Err(e) => {
                    tracing::debug!(
                        "SendMail: failed to encode UserItemData for recipient mail_id={} err={:?}",
                        mail_id,
                        e,
                    );
                }
            }
        }

        let new_mail = StoredMail {
            mail_id,
            sender: sender_name,
            message: msg.message,
            gold: msg.gold,
            items: item_bytes,
            date_sent_binary,
            opened: false,
            locked: false,
            collected: false,
            can_reply: true,
        };

        recipient_mails.push(new_mail.clone());

        let _ = self
            .store
            .save_character_mail(&recipient_account_id, recipient_char_idx, &recipient_mails);

        // If the recipient is online, enqueue a ReceiveMail packet containing
        // just this new mail into their outbox.
        {
            use crystal_shared_proto::io::{
                write_bool, write_i32_le, write_i64_le, write_string, write_u32_le, write_u64_le,
            };

            let mut online_accounts = self.online_accounts.lock().unwrap();
            if let Some(&other_session) = online_accounts.get(&recipient_account_id) {
                let mut buf = Vec::new();
                if write_i32_le(&mut buf, 1).is_ok()
                    && write_u64_le(&mut buf, new_mail.mail_id).is_ok()
                    && write_string(&mut buf, &new_mail.sender).is_ok()
                    && write_string(&mut buf, &new_mail.message).is_ok()
                    && write_bool(&mut buf, new_mail.opened).is_ok()
                    && write_bool(&mut buf, new_mail.locked).is_ok()
                    && write_bool(&mut buf, new_mail.can_reply).is_ok()
                    && write_bool(&mut buf, new_mail.collected).is_ok()
                    && write_i64_le(&mut buf, new_mail.date_sent_binary).is_ok()
                    && write_u32_le(&mut buf, new_mail.gold).is_ok()
                {
                    let item_count: i32 = new_mail
                        .items
                        .len()
                        .try_into()
                        .unwrap_or(i32::MAX);
                    if write_i32_le(&mut buf, item_count).is_ok() {
                        for b in &new_mail.items {
                            buf.extend_from_slice(b);
                        }

                        let pkt = SReceiveMail { mail_bytes: buf };
                        if let Ok(raw) = pkt.encode() {
                            let mut outboxes = self.outboxes.lock().unwrap();
                            let entry = outboxes.entry(other_session).or_default();
                            entry.push(Self::encode_raw(raw));
                        }
                    }
                }
            }
        }

        let pkt = SMailSent { result: 1 };
        out.push(Self::encode_raw(pkt.encode()));
    }
}
