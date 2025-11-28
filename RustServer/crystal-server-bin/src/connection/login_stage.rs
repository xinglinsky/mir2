use std::time::{SystemTime, UNIX_EPOCH};
use crystal_server_core::account::{AccountStatus, CharacterSummary};
use crystal_shared_proto::login::{
    CChangePassword,
    CClientVersion,
    CLogin,
    CNewAccount,
    SChangePassword,
    SChangePasswordBanned,
    SClientVersion,
    SDisconnect,
    SLogin,
    SLoginBanned,
    SNewAccount,
};
use crystal_shared_proto::select::{SelectInfo, SLoginSuccess};

use super::{LoginConnection, Stage};

impl LoginConnection {
    fn is_valid_account_id(id: &str) -> bool {
        let len = id.chars().count();
        if len < 3 || len > 15 {
            return false;
        }
        id.chars().all(|c| c.is_ascii_alphanumeric())
    }

    fn is_valid_password(pwd: &str) -> bool {
        let len = pwd.chars().count();
        if len < 5 || len > 15 {
            return false;
        }
        pwd.chars().all(|c| c.is_ascii_alphanumeric())
    }

    fn now_millis() -> i64 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(dur) => dur.as_millis() as i64,
            Err(_) => 0,
        }
    }

    fn unix_ms_to_dotnet_binary(ms: i64) -> i64 {
        const DOTNET_TICKS_AT_UNIX_EPOCH: i64 = 621_355_968_000_000_000;
        ms.saturating_mul(10_000).saturating_add(DOTNET_TICKS_AT_UNIX_EPOCH)
    }

    pub(crate) fn handle_new_account(
        &mut self,
        msg: CNewAccount,
        out: &mut Vec<Vec<u8>>,
    ) {
        // Mirror C# AccountIDReg / PasswordReg / EMailReg and related
        // length checks as closely as possible using simple Rust
        // validations. Result codes:
        // 1 = invalid AccountID
        // 2 = invalid Password
        // 3 = invalid EMail
        // 4 = invalid UserName
        // 5 = invalid SecretQuestion
        // 6 = invalid SecretAnswer
        // 7 = account already exists
        // 8 = success
        if !Self::is_valid_account_id(&msg.account_id) {
            out.push(Self::encode_raw(SNewAccount { result: 1 }.encode()));
            return;
        }

        if !Self::is_valid_password(&msg.password) {
            out.push(Self::encode_raw(SNewAccount { result: 2 }.encode()));
            return;
        }

        // EMail: C# uses a regex + length <= 50. We approximate with a
        // simple contains('@') check and the same length bound.
        if !msg.email_address.is_empty() {
            let len = msg.email_address.chars().count();
            let looks_like_email = msg.email_address.contains('@') && msg.email_address.contains('.');
            if len > 50 || !looks_like_email {
                out.push(Self::encode_raw(SNewAccount { result: 3 }.encode()));
                return;
            }
        }

        // UserName: optional, max length 20.
        let user_name = msg.user_name.trim();
        if !user_name.is_empty() && user_name.chars().count() > 20 {
            out.push(Self::encode_raw(SNewAccount { result: 4 }.encode()));
            return;
        }

        // SecretQuestion: optional, max length 30.
        let secret_question = msg.secret_question.trim();
        if !secret_question.is_empty() && secret_question.chars().count() > 30 {
            out.push(Self::encode_raw(SNewAccount { result: 5 }.encode()));
            return;
        }

        // SecretAnswer: optional, max length 30.
        let secret_answer = msg.secret_answer.trim();
        if !secret_answer.is_empty() && secret_answer.chars().count() > 30 {
            out.push(Self::encode_raw(SNewAccount { result: 6 }.encode()));
            return;
        }

        // Finally, check whether the account already exists.
        if self
            .store
            .account_exists(&msg.account_id)
            .unwrap_or(false)
        {
            out.push(Self::encode_raw(SNewAccount { result: 7 }.encode()));
            return;
        }

        let _ = self.store.create_account(&msg.account_id, &msg.password);
        out.push(Self::encode_raw(SNewAccount { result: 8 }.encode()));
    }

    pub(crate) fn handle_client_version(
        &mut self,
        _msg: CClientVersion,
        out: &mut Vec<Vec<u8>>,
    ) {
        out.push(Self::encode_raw(SClientVersion { result: 1 }.encode()));
        self.stage = Stage::VersionChecked;
    }

    pub(crate) fn handle_login(&mut self, msg: CLogin, out: &mut Vec<Vec<u8>>) {
        if !Self::is_valid_account_id(&msg.account_id) {
            out.push(Self::encode_raw(SLogin { result: 1 }.encode()));
            return;
        }
        if !Self::is_valid_password(&msg.password) {
            out.push(Self::encode_raw(SLogin { result: 2 }.encode()));
            return;
        }

        let exists = self
            .store
            .account_exists(&msg.account_id)
            .unwrap_or(false);
        if !exists {
            out.push(Self::encode_raw(SLogin { result: 3 }.encode()));
            return;
        }
        let mut status = match self.store.load_account_status(&msg.account_id) {
            Ok(Some(s)) => s,
            Ok(None) | Err(_) => AccountStatus {
                id: msg.account_id.clone(),
                banned: false,
                ban_reason: String::new(),
                ban_expires_at: 0,
                require_password_change: false,
                wrong_password_count: 0,
            },
        };

        let now_ms = Self::now_millis();
        if status.banned {
            if status.ban_expires_at > now_ms {
                let expiry_binary = Self::unix_ms_to_dotnet_binary(status.ban_expires_at);
                let pkt = SLoginBanned {
                    reason: status.ban_reason.clone(),
                    expiry_date_binary: expiry_binary,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }

            status.banned = false;
            status.ban_reason.clear();
            status.ban_expires_at = 0;
            let _ = self.store.save_account_status(&status);
        }

        match self.store.verify_password(&msg.account_id, &msg.password) {
            Ok(true) => {
                status.wrong_password_count = 0;
                let _ = self.store.save_account_status(&status);

                if status.require_password_change {
                    out.push(Self::encode_raw(SLogin { result: 5 }.encode()));
                    return;
                }

                let account_id = msg.account_id.clone();
                if !account_id.is_empty() {
                    if let Some(old_session) = {
                        let mut map = self.online_accounts.lock().unwrap();
                        map.insert(account_id.clone(), self.session_id)
                    } {
                        if old_session != self.session_id {
                            let pkt = SDisconnect { reason: 1 };
                            let raw = pkt.encode();
                            let mut outboxes = self.outboxes.lock().unwrap();
                            outboxes
                                .entry(old_session)
                                .or_default()
                                .push(Self::encode_raw(raw));
                        }
                    }
                }

                self.account_id = Some(account_id);
                self.stage = Stage::Select;

                let chars: Vec<SelectInfo> = self
                    .store
                    .list_characters(&msg.account_id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|c: CharacterSummary| SelectInfo {
                        index: c.index,
                        name: c.name,
                        level: c.level,
                        class: c.class,
                        gender: c.gender,
                        last_access_binary: c.last_access_binary,
                    })
                    .collect();
                self.characters = chars.clone();

                let resp = SLoginSuccess { characters: chars };
                if let Ok(raw) = resp.encode() {
                    out.push(Self::encode_raw(raw));
                }
            }
            Ok(false) => {
                let prev = status.wrong_password_count;
                if prev >= 5 {
                    status.banned = true;
                    status.ban_reason = "Too many Wrong Login Attempts.".to_string();
                    status.ban_expires_at = now_ms + 2 * 60 * 1000;
                    let _ = self.store.save_account_status(&status);

                    let expiry_binary = Self::unix_ms_to_dotnet_binary(status.ban_expires_at);
                    let pkt = SLoginBanned {
                        reason: status.ban_reason.clone(),
                        expiry_date_binary: expiry_binary,
                    };
                    if let Ok(raw) = pkt.encode() {
                        out.push(Self::encode_raw(raw));
                    }
                    return;
                }

                status.wrong_password_count = prev.saturating_add(1);
                let _ = self.store.save_account_status(&status);
                out.push(Self::encode_raw(SLogin { result: 4 }.encode()));
            }
            Err(_) => {
                out.push(Self::encode_raw(SLogin { result: 4 }.encode()));
            }
        }
    }

    pub(crate) fn handle_change_password(
        &mut self,
        msg: CChangePassword,
        out: &mut Vec<Vec<u8>>,
    ) {
        // C# ChangePassword result codes:
        // 1 = invalid AccountID format
        // 2 = invalid current password format
        // 3 = invalid new password format
        // 4 = account does not exist
        // 5 = current password incorrect
        // 6 = success

        if !Self::is_valid_account_id(&msg.account_id) {
            out.push(Self::encode_raw(SChangePassword { result: 1 }.encode()));
            return;
        }

        if !Self::is_valid_password(&msg.current_password) {
            out.push(Self::encode_raw(SChangePassword { result: 2 }.encode()));
            return;
        }

        if !Self::is_valid_password(&msg.new_password) {
            out.push(Self::encode_raw(SChangePassword { result: 3 }.encode()));
            return;
        }

        let status_opt = self
            .store
            .load_account_status(&msg.account_id)
            .unwrap_or(None);

        let mut status = match status_opt {
            Some(s) => s,
            None => {
                out.push(Self::encode_raw(SChangePassword { result: 4 }.encode()));
                return;
            }
        };

        let now_ms = Self::now_millis();
        if status.banned {
            if status.ban_expires_at > now_ms {
                let expiry_binary = Self::unix_ms_to_dotnet_binary(status.ban_expires_at);
                let pkt = SChangePasswordBanned {
                    reason: status.ban_reason.clone(),
                    expiry_date_binary: expiry_binary,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(Self::encode_raw(raw));
                }
                return;
            }

            status.banned = false;
            status.ban_reason.clear();
            status.ban_expires_at = 0;
            let _ = self.store.save_account_status(&status);
        }

        match self
            .store
            .verify_password(&msg.account_id, &msg.current_password)
        {
            Ok(true) => {
                let _ = self
                    .store
                    .set_password(&msg.account_id, &msg.new_password);
                status.require_password_change = false;
                let _ = self.store.save_account_status(&status);
                out.push(Self::encode_raw(SChangePassword { result: 6 }.encode()));
            }
            Ok(false) | Err(_) => {
                out.push(Self::encode_raw(SChangePassword { result: 5 }.encode()));
            }
        }
    }
}
