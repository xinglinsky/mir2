use crystal_server_core::account::CharacterSummary;
use crystal_shared_proto::login::{
    CChangePassword,
    CClientVersion,
    CLogin,
    CNewAccount,
    SChangePassword,
    SClientVersion,
    SLogin,
    SNewAccount,
};
use crystal_shared_proto::select::{SelectInfo, SLoginSuccess};

use super::{LoginConnection, Stage};

impl LoginConnection {
    pub(crate) fn handle_new_account(
        &mut self,
        msg: CNewAccount,
        out: &mut Vec<Vec<u8>>,
    ) {
        if msg.account_id.is_empty() {
            out.push(Self::encode_raw(SNewAccount { result: 1 }.encode()));
        } else if msg.password.is_empty() {
            out.push(Self::encode_raw(SNewAccount { result: 2 }.encode()));
        } else if self
            .store
            .account_exists(&msg.account_id)
            .unwrap_or(false)
        {
            out.push(Self::encode_raw(SNewAccount { result: 7 }.encode()));
        } else {
            let _ = self.store.create_account(&msg.account_id, &msg.password);
            out.push(Self::encode_raw(SNewAccount { result: 8 }.encode()));
        }
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
        if msg.account_id.is_empty() {
            out.push(Self::encode_raw(SLogin { result: 1 }.encode()));
            return;
        }
        if msg.password.is_empty() {
            out.push(Self::encode_raw(SLogin { result: 2 }.encode()));
            return;
        }

        match self.store.verify_password(&msg.account_id, &msg.password) {
            Ok(true) => {
                self.account_id = Some(msg.account_id.clone());
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
            Ok(false) | Err(_) => {
                out.push(Self::encode_raw(SLogin { result: 4 }.encode()));
            }
        }
    }

    pub(crate) fn handle_change_password(
        &mut self,
        msg: CChangePassword,
        out: &mut Vec<Vec<u8>>,
    ) {
        if msg.new_password.is_empty() {
            out.push(Self::encode_raw(SChangePassword { result: 3 }.encode()));
        } else {
            let exists = self
                .store
                .account_exists(&msg.account_id)
                .unwrap_or(false);

            if !exists {
                out.push(Self::encode_raw(SChangePassword { result: 4 }.encode()));
            } else {
                match self
                    .store
                    .verify_password(&msg.account_id, &msg.current_password)
                {
                    Ok(true) => {
                        let _ = self
                            .store
                            .set_password(&msg.account_id, &msg.new_password);
                        out.push(Self::encode_raw(SChangePassword { result: 6 }.encode()));
                    }
                    Ok(false) | Err(_) => {
                        out.push(Self::encode_raw(SChangePassword { result: 5 }.encode()));
                    }
                }
            }
        }
    }
}
