use std::time::{SystemTime, UNIX_EPOCH};

use crystal_server_core::account::StoredMail;
use crystal_server_core::item::create_fresh_user_item;
use crystal_shared_proto::io::{
    write_bool, write_i32_le, write_i64_le, write_string, write_u32_le, write_u64_le,
};
use crystal_shared_proto::item_types::UserItemData;
use crystal_shared_proto::login::CGameshopBuy;
use crystal_shared_proto::mail::SReceiveMail;
use crystal_shared_proto::user::{SLoseCredit, SLoseGold};
use crystal_shared_proto::shop::SGameShopStock;

use super::{LoginConnection, Stage};

impl LoginConnection {
    pub(crate) fn handle_gameshop_buy(
        &mut self,
        msg: CGameshopBuy,
        out: &mut Vec<Vec<u8>>,
    ) {
        tracing::debug!(
            "GameshopBuy: start stage={:?} g_index={} quantity={} p_type={}",
            self.stage,
            msg.g_index,
            msg.quantity,
            msg.p_type,
        );

        if self.stage != Stage::InGame {
            return;
        }

        if msg.quantity == 0 || msg.quantity > 99 {
            self.send_system_chat("购买数量无效。", out);
            return;
        }

        let stats = match self.current_stats.clone() {
            Some(s) => s,
            None => {
                tracing::debug!("GameshopBuy: early-return, no current_stats available");
                self.send_system_chat("当前角色状态不可用，暂时无法购买。", out);
                return;
            }
        };

        let product = match self
            .world_db
            .game_shop_items
            .iter()
            .find(|p| p.g_index == msg.g_index)
        {
            Some(p) => p,
            None => {
                tracing::debug!(
                    "GameshopBuy: early-return, GameShopItem not found g_index={}",
                    msg.g_index
                );
                self.send_system_chat("您尝试购买的商品不存在。", out);
                return;
            }
        };

        let info = match self
            .world_db
            .item_infos
            .iter()
            .find(|i| i.index == product.item_index)
        {
            Some(i) => i,
            None => {
                tracing::debug!(
                    "GameshopBuy: early-return, ItemInfo not found for item_index={} g_index={}",
                    product.item_index,
                    product.g_index
                );
                self.send_system_chat("该商品的物品信息缺失，暂时无法购买。", out);
                return;
            }
        };

        // Check stock availability using the in-memory purchase logs. This
        // mirrors the C# logic where individual-stock items use per-player
        // GSpurchases and server-stock items use Envir.GameshopLog.
        let purchased_before: i32;
        if product.stock != 0 {
            let (per_player, global) = {
                let world = self.world.lock().unwrap();
                let per_player = world.gameshop_purchased_for_player(self.session_id, product.g_index);
                let global = world.gameshop_purchased_global(product.g_index);
                (per_player, global)
            };

            purchased_before = if product.i_stock { per_player } else { global };

            let requested = msg.quantity as i32;
            if product.stock - purchased_before - requested < 0 {
                tracing::debug!(
                    "GameshopBuy: early-return, requested quantity exceeds available stock g_index={} stock={} purchased_before={} requested={}",
                    product.g_index,
                    product.stock,
                    purchased_before,
                    requested,
                );
                self.send_system_chat("您试图购买的数量超过当前可用库存。", out);

                let current_stock = (product.stock - purchased_before).max(0);
                self.send_gameshop_stock_update(product.g_index, current_stock, out);
                return;
            }
        }

        let total_units = (msg.quantity as u32) * (product.count as u32);
        let stack_size = info.stack_size.max(1) as u32;
        let max_full_stacks = (total_units + stack_size - 1) / stack_size;
        if max_full_stacks > 5 {
            tracing::debug!(
                "GameshopBuy: early-return, stack limit exceeded total_units={} stack_size={}",
                total_units,
                stack_size
            );
            self.send_system_chat("一次购买的堆叠数量超过限制。", out);
            return;
        }

        let mut gold_cost: u32 = 0;
        let mut credit_cost: u32 = 0;

        match msg.p_type {
            0 => {
                if !product.can_buy_credit {
                    self.send_system_chat("该商品不能使用点券购买。", out);
                    return;
                }
                let cost = match product.credit_price.checked_mul(msg.quantity as u32) {
                    Some(v) => v,
                    None => {
                        tracing::debug!(
                            "GameshopBuy: early-return, credit cost overflow price={} quantity={}",
                            product.credit_price,
                            msg.quantity
                        );
                        self.send_system_chat("购买价格异常，暂时无法购买。", out);
                        return;
                    }
                };
                if stats.credit < cost as i64 {
                    self.send_system_chat("您的点券不足，无法完成本次购买。", out);
                    return;
                }
                credit_cost = cost;
            }
            1 => {
                if !product.can_buy_gold {
                    self.send_system_chat("该商品不能使用金币购买。", out);
                    return;
                }
                let cost = match product.gold_price.checked_mul(msg.quantity as u32) {
                    Some(v) => v,
                    None => {
                        tracing::debug!(
                            "GameshopBuy: early-return, gold cost overflow price={} quantity={}",
                            product.gold_price,
                            msg.quantity
                        );
                        self.send_system_chat("购买价格异常，暂时无法购买。", out);
                        return;
                    }
                };
                if stats.gold < cost as i64 {
                    self.send_system_chat("您的金币不足，无法完成本次购买。", out);
                    return;
                }
                gold_cost = cost;
            }
            _ => {
                self.send_system_chat("无效的付款方式。", out);
                return;
            }
        }

        let mut new_stats = stats.clone();
        if credit_cost > 0 {
            let diff = credit_cost as i64;
            if new_stats.credit < diff {
                self.send_system_chat("您的点券不足，无法完成本次购买。", out);
                return;
            }
            new_stats.credit = new_stats.credit.saturating_sub(diff);
        }
        if gold_cost > 0 {
            let diff = gold_cost as i64;
            if new_stats.gold < diff {
                self.send_system_chat("您的金币不足，无法完成本次购买。", out);
                return;
            }
            new_stats.gold = new_stats.gold.saturating_sub(diff);
        }

        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let _ = self
                .store
                .save_character_stats(account_id, char_idx, &new_stats);
        }

        self.current_stats = Some(new_stats.clone());

        if gold_cost > 0 {
            let lose = SLoseGold { gold: gold_cost };
            if let Ok(raw) = lose.encode() {
                out.push(Self::encode_raw(raw));
            }
        }
        if credit_cost > 0 {
            let lose = SLoseCredit {
                credit: credit_cost,
            };
            if let Ok(raw) = lose.encode() {
                out.push(Self::encode_raw(raw));
            }
        }

        // Update in-memory GameShop logs and send an updated stock packet
        // for finite-stock items.
        if product.stock != 0 {
            let quantity_i32 = msg.quantity as i32;

            {
                let mut world = self.world.lock().unwrap();

                if product.i_stock {
                    world.increment_gameshop_purchases_for_player(
                        self.session_id,
                        product.g_index,
                        quantity_i32,
                    );
                }

                world.increment_gameshop_log(product.g_index, quantity_i32);
            }

            let (per_player_after, global_after) = {
                let world = self.world.lock().unwrap();
                let per_player = world.gameshop_purchased_for_player(self.session_id, product.g_index);
                let global = world.gameshop_purchased_global(product.g_index);
                (per_player, global)
            };

            let purchased_after = if product.i_stock {
                per_player_after
            } else {
                global_after
            };

            let stock_level = (product.stock - purchased_after).max(0);
            self.send_gameshop_stock_update(product.g_index, stock_level, out);
        }

        let mut mail_items: Vec<UserItemData> = Vec::new();
        let mut remaining = total_units;

        if info.stack_size <= 1 || remaining == 1 {
            for _ in 0..remaining {
                let unique_id = self.generate_unique_item_id_for_session();
                let mut item = create_fresh_user_item(info, unique_id, 1);
                item.is_shop_item = true;
                mail_items.push(item);
            }
        } else {
            let stack = stack_size;
            while remaining > 0 {
                let take = stack.min(remaining) as u16;
                remaining -= take as u32;
                let unique_id = self.generate_unique_item_id_for_session();
                let mut item = create_fresh_user_item(info, unique_id, take);
                item.is_shop_item = true;
                mail_items.push(item);
            }
        }

        if mail_items.is_empty() {
            tracing::debug!(
                "GameshopBuy: early-return, no mail items generated for g_index={} quantity={}",
                msg.g_index,
                msg.quantity
            );
            self.send_system_chat("生成购买物品失败，暂时无法完成购买。", out);
            return;
        }

        // Generate a MailID and DateSent ticks compatible with the legacy
        // C# server's MailInfo/ClientMail model so that we can both persist
        // the mail and send it to the client.
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let mail_id = self.generate_unique_item_id_for_session();
        let ticks = 621355968000000000i64
            .saturating_add((now.as_secs() as i64).saturating_mul(10_000_000))
            .saturating_add((now.subsec_nanos() / 100) as i64);

        let mail_bytes = match self.encode_gameshop_mail_bytes(mail_id, ticks, &mail_items) {
            Ok(b) => b,
            Err(e) => {
                tracing::debug!(
                    "GameshopBuy: early-return, failed to encode mail bytes err={:?}",
                    e
                );
                self.send_system_chat("发送购买邮件时发生错误。", out);
                return;
            }
        };

        // Persist this mail to the character's mailbox so that it can be
        // retrieved later (e.g. on relogin) via SReceiveMail. Any failure
        // to persist is logged but does not cancel the purchase.
        if let (Some(ref account_id), Some(char_idx)) =
            (self.account_id.as_ref(), self.current_char_index)
        {
            let mut item_bytes: Vec<Vec<u8>> = Vec::with_capacity(mail_items.len());
            for it in &mail_items {
                match it.encode_to_bytes() {
                    Ok(b) => item_bytes.push(b),
                    Err(e) => {
                        tracing::debug!(
                            "GameshopBuy: failed to encode UserItemData for mailbox persistence err={:?}",
                            e
                        );
                        item_bytes.clear();
                        break;
                    }
                }
            }

            if !item_bytes.is_empty() {
                let mut mails = self
                    .store
                    .load_character_mail(account_id, char_idx)
                    .unwrap_or_default();

                mails.push(StoredMail {
                    mail_id,
                    sender: "Gameshop".to_string(),
                    message: "感谢您在商城购物，物品已通过邮件发送，请查收。".to_string(),
                    gold: 0,
                    items: item_bytes,
                    date_sent_binary: ticks,
                    opened: false,
                    locked: false,
                    collected: false,
                    can_reply: false,
                });

                let _ = self
                    .store
                    .save_character_mail(account_id, char_idx, &mails);
            }
        }

        let pkt = SReceiveMail { mail_bytes };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        self.send_system_chat("您的购买已通过邮件发送，请在邮箱中查收。", out);
    }

    fn encode_gameshop_mail_bytes(
        &self,
        mail_id: u64,
        date_sent_binary: i64,
        items: &[UserItemData],
    ) -> std::io::Result<Vec<u8>> {
        let mut buf = Vec::new();

        write_i32_le(&mut buf, 1)?;

        write_u64_le(&mut buf, mail_id)?;
        write_string(&mut buf, "Gameshop")?;
        write_string(&mut buf, "感谢您在商城购物，物品已通过邮件发送，请查收。")?;
        write_bool(&mut buf, false)?;
        write_bool(&mut buf, false)?;
        write_bool(&mut buf, false)?;
        write_bool(&mut buf, false)?;

        write_i64_le(&mut buf, date_sent_binary)?;

        write_u32_le(&mut buf, 0)?;
        write_i32_le(&mut buf, items.len() as i32)?;

        for item in items {
            item.encode(&mut buf)?;
        }

        Ok(buf)
    }

    fn send_gameshop_stock_update(
        &self,
        g_index: i32,
        stock_level: i32,
        out: &mut Vec<Vec<u8>>,
    ) {
        let pkt = SGameShopStock {
            gindex: g_index,
            stock_level,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
}
