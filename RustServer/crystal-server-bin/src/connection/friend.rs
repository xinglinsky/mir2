use crystal_server_core::account::StoredFriend;
use crystal_shared_proto::login::{CAddFriend, CAddMemo, CRefreshFriends, CRemoveFriend};
use crystal_shared_proto::social::SFriendUpdate;

use super::{LoginConnection, Stage};

impl LoginConnection {
    fn send_friend_update(&mut self, out: &mut Vec<Vec<u8>>) {
        let friends_bytes = {
            let world = self.world.lock().unwrap();
            world.encode_friends_bytes_for_player(self.session_id)
        };

        let pkt = SFriendUpdate { friends_bytes };
        let raw = pkt.encode();
        out.push(Self::encode_raw(raw));
    }

    fn persist_friends(&mut self) {
        let (account_id, char_idx) = match (&self.account_id, self.current_char_index) {
            (Some(a), Some(idx)) => (a.clone(), idx),
            _ => return,
        };

        let friends = {
            let world = self.world.lock().unwrap();
            world.friends_for_player(self.session_id)
        };

        let stored: Vec<StoredFriend> = friends
            .into_iter()
            .map(|f| StoredFriend {
                friend_index: f.index,
                name: f.name,
                memo: f.memo,
                blocked: f.blocked,
            })
            .collect();

        if let Err(e) = self
            .store
            .save_character_friends(&account_id, char_idx, &stored)
        {
            tracing::debug!(
                "save_character_friends failed for account_id={} idx={} err={:?}",
                account_id,
                char_idx,
                e,
            );
        }
    }

    pub(crate) fn handle_add_friend(
        &mut self,
        msg: CAddFriend,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let name = msg.name.trim();
        if name.is_empty() {
            self.send_system_chat("请输入玩家名字。", out);
            return;
        }

        // Prevent adding self as a friend, mirroring C# "Cannot add yourself".
        if let Some(self_name) = {
            let world = self.world.lock().unwrap();
            world.player_name(self.session_id)
        } {
            if self_name.eq_ignore_ascii_case(name) {
                self.send_system_chat("不能添加自己为好友。", out);
                return;
            }
        }

        // Resolve the target character by name via the account store so we can
        // add friends even when the target is offline, mirroring the legacy C#
        // behaviour of Envir.GetCharacterInfo(name).
        let friend_index_res = self.store.find_character_by_name(name);
        let friend_index = match friend_index_res {
            Ok(Some((_, idx))) => idx,
            Ok(None) => {
                // C#: "Player doesn't exist".
                self.send_system_chat("未找到该玩家。", out);
                return;
            }
            Err(e) => {
                tracing::debug!("find_character_by_name failed: {:?}", e);
                self.send_system_chat("查找玩家失败，请稍后再试。", out);
                return;
            }
        };

        // Check duplicate: if already in friend list, mirror C# "Player already added".
        let already = {
            let world = self.world.lock().unwrap();
            world
                .friends_for_player(self.session_id)
                .iter()
                .any(|f| f.index == friend_index)
        };
        if already {
            self.send_system_chat("该玩家已在你的好友列表中。", out);
            return;
        }

        {
            let mut world = self.world.lock().unwrap();
            let _ = world.add_friend_entry_for_player(self.session_id, friend_index, name, msg.blocked);
        }

        self.persist_friends();
        self.send_friend_update(out);
    }

    pub(crate) fn handle_remove_friend(
        &mut self,
        msg: CRemoveFriend,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        let removed = {
            let mut world = self.world.lock().unwrap();
            world.remove_friend_for_player(self.session_id, msg.character_index)
        };

        if !removed {
            self.send_system_chat("未在好友列表中找到该玩家。", out);
            return;
        }

        self.persist_friends();
        self.send_friend_update(out);
    }

    pub(crate) fn handle_refresh_friends(
        &mut self,
        _msg: CRefreshFriends,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        self.send_friend_update(out);
    }

    pub(crate) fn handle_add_memo(
        &mut self,
        msg: CAddMemo,
        out: &mut Vec<Vec<u8>>,
    ) {
        if self.stage != Stage::InGame {
            return;
        }

        // Validate memo length similarly to C# (non-empty and <= 200 chars).
        let memo = msg.memo.trim();
        let memo_len = memo.chars().count();
        if memo_len == 0 {
            self.send_system_chat("备注不能为空。", out);
            return;
        }
        if memo_len > 200 {
            self.send_system_chat("备注长度不能超过 200 个字符。", out);
            return;
        }

        let updated = {
            let mut world = self.world.lock().unwrap();
            world.set_friend_memo_for_player(
                self.session_id,
                msg.character_index,
                memo.to_string(),
            )
        };

        if !updated {
            self.send_system_chat("未在好友列表中找到该玩家。", out);
            return;
        }

        self.persist_friends();
        self.send_friend_update(out);
    }
}
