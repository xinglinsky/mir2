use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::mpsc::{self, Sender, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Utc;
use crystal_server_core::account::{
    AccountStatus,
    AccountStore,
    AccountStorage,
    StoreError,
    CharacterSummary,
    CharacterStats,
    CharacterPosition,
    StoredMail,
    hash_password,
    verify_password_hash,
};
use crystal_server_core::guild::GuildInfo;
use crystal_server_core::item::{decode_item_slots, encode_item_slots, Inventory, Equipment};
use crystal_server_core::world::magic::UserMagic;
use rusqlite::{self, Connection};
use serde::{Deserialize, Serialize};
use serde_json;

pub struct SqliteAccountStore {
    path: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct StoredItems {
    inventory: Vec<Option<Vec<u8>>>,
    equipment: Vec<Option<Vec<u8>>>,
}

#[derive(Serialize, Deserialize)]
struct StoredAccountStorage {
    slots: Vec<Option<Vec<u8>>>,
    has_expanded_storage: bool,
    expanded_storage_expiry_binary: i64,
}

impl SqliteAccountStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StoreError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let store = SqliteAccountStore { path };
        store.init_db()?;
        Ok(store)
    }

    fn default_stats_for_new_character(class: u8, level: u16) -> CharacterStats {
        // Mirror the C# BaseStats defaults for HP/MP at the given level using
        // the same formulas as BaseStat.Calculate for Health/Mana. For now we
        // only need level 1, but this keeps the logic general.
        let lvl = level as f32;

        let (hp_base, hp_gain, hp_gain_rate) = match class {
            // Warrior
            0 => (14.0_f32, 4.0_f32, 4.5_f32),
            // Wizard
            1 => (14.0_f32, 15.0_f32, 1.8_f32),
            // Taoist
            2 => (14.0_f32, 6.0_f32, 2.5_f32),
            // Assassin
            3 => (14.0_f32, 4.0_f32, 3.25_f32),
            // Archer (default)
            _ => (14.0_f32, 4.0_f32, 3.25_f32),
        };

        // Health formula from C# BaseStat.Calculate (StatFormula.Health).
        let hp_val = if (hp_gain - 0.0).abs() < f32::EPSILON {
            hp_base
        } else if class == 0 {
            // Warrior special-case: Base + (level / Gain + GainRate + level / 20F) * level
            hp_base + ((lvl / hp_gain) + hp_gain_rate + (lvl / 20.0_f32)) * lvl
        } else {
            // Other classes: Base + (level / Gain + GainRate) * level
            hp_base + ((lvl / hp_gain) + hp_gain_rate) * lvl
        };

        let (mp_base, mp_gain, mp_gain_rate) = match class {
            // Warrior
            0 => (11.0_f32, 3.5_f32, 0.0_f32),
            // Wizard
            1 => (13.0_f32, 5.0_f32, 0.0_f32),
            // Taoist
            2 => (13.0_f32, 8.0_f32, 0.0_f32),
            // Assassin
            3 => (11.0_f32, 5.0_f32, 0.0_f32),
            // Archer (default)
            _ => (11.0_f32, 4.0_f32, 0.0_f32),
        };

        // Mana formula from C# BaseStat.Calculate (StatFormula.Mana).
        let mp_val = if (mp_gain - 0.0).abs() < f32::EPSILON {
            mp_base
        } else if class == 1 {
            // Wizard: Base + ((level / Gain + 2F) * 2.2F * level) + (level * GainRate)
            mp_base + (((lvl / mp_gain) + 2.0_f32) * 2.2_f32 * lvl) + (lvl * mp_gain_rate)
        } else if class == 2 {
            // Taoist: (Base + level / Gain * 2.2F * level) + (level * GainRate)
            (mp_base + (lvl / mp_gain) * 2.2_f32 * lvl) + (lvl * mp_gain_rate)
        } else {
            // Other classes: Base + (level * Gain) + (level * GainRate)
            mp_base + (lvl * mp_gain) + (lvl * mp_gain_rate)
        };

        CharacterStats {
            hp: hp_val.max(1.0).round() as i32,
            mp: mp_val.max(0.0).round() as i32,
            experience: 0,
            gold: 0,
            credit: 0,
        }
    }

    fn map_sql_err<T>(res: rusqlite::Result<T>) -> Result<T, StoreError> {
        res.map_err(|e| StoreError::Sqlite(e.to_string()))
    }

    fn with_conn<T, F>(&self, f: F) -> Result<T, StoreError>
    where
        F: FnOnce(&Connection) -> Result<T, StoreError>,
    {
        let conn = Self::map_sql_err(Connection::open(&self.path))?;
        f(&conn)
    }

    fn init_db(&self) -> Result<(), StoreError> {
        self.with_conn(|conn| {
            Self::map_sql_err(conn.execute_batch(
                r#"
                PRAGMA foreign_keys = ON;

                CREATE TABLE IF NOT EXISTS accounts (
                    id              TEXT PRIMARY KEY,
                    password_hash   TEXT NOT NULL,
                    created_at      INTEGER NOT NULL,
                    last_login_at   INTEGER,
                    banned                  INTEGER NOT NULL DEFAULT 0,
                    ban_reason              TEXT NOT NULL DEFAULT '',
                    ban_expires_at          INTEGER NOT NULL DEFAULT 0,
                    require_password_change INTEGER NOT NULL DEFAULT 0,
                    wrong_password_count    INTEGER NOT NULL DEFAULT 0
                );

                CREATE TABLE IF NOT EXISTS characters (
                    account_id          TEXT NOT NULL,
                    idx                 INTEGER NOT NULL,
                    name                TEXT NOT NULL,
                    level               INTEGER NOT NULL,
                    class               INTEGER NOT NULL,
                    gender              INTEGER NOT NULL,
                    last_access_binary  INTEGER NOT NULL,
                    PRIMARY KEY(account_id, idx),
                    FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS character_magics (
                    account_id   TEXT NOT NULL,
                    idx          INTEGER NOT NULL,
                    magics_json  TEXT NOT NULL,
                    PRIMARY KEY(account_id, idx),
                    FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS character_positions (
                    account_id   TEXT NOT NULL,
                    idx          INTEGER NOT NULL,
                    map_index    INTEGER NOT NULL,
                    x            INTEGER NOT NULL,
                    y            INTEGER NOT NULL,
                    direction    INTEGER NOT NULL,
                    PRIMARY KEY(account_id, idx),
                    FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS character_binds (
                    account_id   TEXT NOT NULL,
                    idx          INTEGER NOT NULL,
                    map_index    INTEGER NOT NULL,
                    x            INTEGER NOT NULL,
                    y            INTEGER NOT NULL,
                    direction    INTEGER NOT NULL,
                    PRIMARY KEY(account_id, idx),
                    FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS character_stats (
                    account_id   TEXT NOT NULL,
                    idx          INTEGER NOT NULL,
                    hp           INTEGER NOT NULL,
                    mp           INTEGER NOT NULL,
                    experience   INTEGER NOT NULL,
                    gold         INTEGER NOT NULL,
                    credit       INTEGER NOT NULL,
                    PRIMARY KEY(account_id, idx),
                    FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS character_items (
                    account_id   TEXT NOT NULL,
                    idx          INTEGER NOT NULL,
                    items_json   TEXT NOT NULL,
                    PRIMARY KEY(account_id, idx),
                    FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS character_guilds (
                    account_id   TEXT NOT NULL,
                    idx          INTEGER NOT NULL,
                    guild_name   TEXT NOT NULL,
                    rank_index   INTEGER NOT NULL,
                    PRIMARY KEY(account_id, idx),
                    FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS character_mail (
                    account_id   TEXT NOT NULL,
                    idx          INTEGER NOT NULL,
                    mail_json    TEXT NOT NULL,
                    PRIMARY KEY(account_id, idx),
                    FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS guilds (
                    id          INTEGER PRIMARY KEY,
                    name        TEXT NOT NULL UNIQUE,
                    data_json   TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS account_storage (
                    account_id   TEXT PRIMARY KEY,
                    storage_json TEXT NOT NULL,
                    FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
                );
                "#,
            ))?;
            let _ = conn.execute(
                "ALTER TABLE accounts ADD COLUMN banned INTEGER NOT NULL DEFAULT 0",
                [],
            );
            let _ = conn.execute(
                "ALTER TABLE accounts ADD COLUMN ban_reason TEXT NOT NULL DEFAULT ''",
                [],
            );
            let _ = conn.execute(
                "ALTER TABLE accounts ADD COLUMN ban_expires_at INTEGER NOT NULL DEFAULT 0",
                [],
            );
            let _ = conn.execute(
                "ALTER TABLE accounts ADD COLUMN require_password_change INTEGER NOT NULL DEFAULT 0",
                [],
            );
            let _ = conn.execute(
                "ALTER TABLE accounts ADD COLUMN wrong_password_count INTEGER NOT NULL DEFAULT 0",
                [],
            );
            Ok(())
        })
    }

    fn load_account_storage_inner(&self, account_id: &str) -> Result<Option<AccountStorage>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = Self::map_sql_err(conn.prepare(
                "SELECT storage_json FROM account_storage WHERE account_id = ?1 LIMIT 1",
            ))?;
            let mut rows = Self::map_sql_err(stmt.query([account_id]))?;
            if let Some(row) = Self::map_sql_err(rows.next())? {
                let json: String = Self::map_sql_err(row.get(0))?;
                let stored: StoredAccountStorage = serde_json::from_str(&json)
                    .map_err(|e| StoreError::Serde(e.to_string()))?;

                let slots = decode_item_slots(stored.slots)
                    .map_err(StoreError::Io)?;

                Ok(Some(AccountStorage {
                    slots,
                    has_expanded_storage: stored.has_expanded_storage,
                    expanded_storage_expiry_binary: stored.expanded_storage_expiry_binary,
                }))
            } else {
                Ok(None)
            }
        })
    }

    fn save_account_storage_inner(
        &self,
        account_id: &str,
        storage: &AccountStorage,
    ) -> Result<(), StoreError> {
        let stored = StoredAccountStorage {
            slots: encode_item_slots(&storage.slots)
                .map_err(StoreError::Io)?,
            has_expanded_storage: storage.has_expanded_storage,
            expanded_storage_expiry_binary: storage.expanded_storage_expiry_binary,
        };

        let json = serde_json::to_string(&stored)
            .map_err(|e| StoreError::Serde(e.to_string()))?;

        self.with_conn(|conn| {
            Self::map_sql_err(conn.execute(
                "INSERT INTO account_storage (account_id, storage_json)
                 VALUES (?1, ?2)
                 ON CONFLICT(account_id) DO UPDATE SET storage_json = excluded.storage_json",
                (account_id, &json),
            ))?;
            Ok(())
        })
    }
}

impl AccountStore for SqliteAccountStore {
    fn account_exists(&self, id: &str) -> Result<bool, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = Self::map_sql_err(conn.prepare("SELECT 1 FROM accounts WHERE id = ?1 LIMIT 1"))?;
            let exists = Self::map_sql_err(stmt.exists([id]))?;
            Ok(exists)
        })
    }

    fn create_account(&self, id: &str, password: &str) -> Result<(), StoreError> {
        let hashed = hash_password(password);
        let now = Utc::now().timestamp_millis();

        self.with_conn(|conn| {
            Self::map_sql_err(conn.execute(
                "INSERT OR IGNORE INTO accounts (id, password_hash, created_at, last_login_at) VALUES (?1, ?2, ?3, NULL)",
                (id, &hashed, &now),
            ))?;
            Ok(())
        })
    }

    fn load_account_storage(&self, account_id: &str) -> Result<Option<AccountStorage>, StoreError> {
        self.load_account_storage_inner(account_id)
    }

    fn save_account_storage(
        &self,
        account_id: &str,
        storage: &AccountStorage,
    ) -> Result<(), StoreError> {
        self.save_account_storage_inner(account_id, storage)
    }

    fn verify_password(&self, id: &str, password: &str) -> Result<bool, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = Self::map_sql_err(conn.prepare(
                "SELECT password_hash FROM accounts WHERE id = ?1 LIMIT 1",
            ))?;
            let mut rows = Self::map_sql_err(stmt.query([id]))?;
            if let Some(row) = Self::map_sql_err(rows.next())? {
                let stored: String = Self::map_sql_err(row.get(0))?;
                let ok = verify_password_hash(password, &stored);
                if ok {
                    let now = Utc::now().timestamp_millis();
                    Self::map_sql_err(conn.execute(
                        "UPDATE accounts SET last_login_at = ?2 WHERE id = ?1",
                        (id, &now),
                    ))?;
                }
                Ok(ok)
            } else {
                Ok(false)
            }
        })
    }

    fn load_account_status(&self, id: &str) -> Result<Option<AccountStatus>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = Self::map_sql_err(conn.prepare(
                "SELECT banned, ban_reason, ban_expires_at, require_password_change, wrong_password_count FROM accounts WHERE id = ?1 LIMIT 1",
            ))?;
            let mut rows = Self::map_sql_err(stmt.query([id]))?;
            if let Some(row) = Self::map_sql_err(rows.next())? {
                let banned_int: i64 = Self::map_sql_err(row.get(0))?;
                let banned = banned_int != 0;
                let ban_reason: String = Self::map_sql_err(row.get(1))?;
                let ban_expires_at: i64 = Self::map_sql_err(row.get(2))?;
                let require_int: i64 = Self::map_sql_err(row.get(3))?;
                let require_password_change = require_int != 0;
                let wrong_password_count: i64 = Self::map_sql_err(row.get(4))?;
                Ok(Some(AccountStatus {
                    id: id.to_string(),
                    banned,
                    ban_reason,
                    ban_expires_at,
                    require_password_change,
                    wrong_password_count: wrong_password_count as i32,
                }))
            } else {
                Ok(None)
            }
        })
    }

    fn save_account_status(&self, status: &AccountStatus) -> Result<(), StoreError> {
        self.with_conn(|conn| {
            let banned_val: i64 = if status.banned { 1 } else { 0 };
            let require_val: i64 = if status.require_password_change { 1 } else { 0 };
            Self::map_sql_err(conn.execute(
                "UPDATE accounts SET banned = ?2, ban_reason = ?3, ban_expires_at = ?4, require_password_change = ?5, wrong_password_count = ?6 WHERE id = ?1",
                (
                    &status.id,
                    &banned_val,
                    &status.ban_reason,
                    &status.ban_expires_at,
                    &require_val,
                    &(status.wrong_password_count as i64),
                ),
            ))?;
            Ok(())
        })
    }

    fn list_characters(&self, account_id: &str) -> Result<Vec<CharacterSummary>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = Self::map_sql_err(conn.prepare(
                "SELECT idx, name, level, class, gender, last_access_binary
                 FROM characters WHERE account_id = ?1 ORDER BY idx",
            ))?;
            let rows = Self::map_sql_err(stmt.query_map([account_id], |row| {
                Ok(CharacterSummary {
                    index: row.get::<_, i64>(0)? as i32,
                    name: row.get(1)?,
                    level: row.get::<_, i64>(2)? as u16,
                    class: row.get::<_, i64>(3)? as u8,
                    gender: row.get::<_, i64>(4)? as u8,
                    last_access_binary: row.get(5)?,
                    map_index: 0,
                    x: 0,
                    y: 0,
                    direction: 0,
                })
            }))?;

            let mut chars = Vec::new();
            for ch in rows {
                chars.push(Self::map_sql_err(ch)?);
            }
            Ok(chars)
        })
    }

    fn create_character(
        &self,
        account_id: &str,
        name: String,
        class: u8,
        gender: u8,
    ) -> Result<CharacterSummary, StoreError> {
        self.with_conn(|conn| {
            // Ensure account exists.
            let mut check_stmt = Self::map_sql_err(conn.prepare("SELECT 1 FROM accounts WHERE id = ?1 LIMIT 1"))?;
            if !Self::map_sql_err(check_stmt.exists([account_id]))? {
                return Err(StoreError::Io(io::Error::new(
                    io::ErrorKind::NotFound,
                    "account not found",
                )));
            }

            // Next character index for this account.
            let next_idx: i64 = Self::map_sql_err(conn.query_row(
                "SELECT COALESCE(MAX(idx) + 1, 0) FROM characters WHERE account_id = ?1",
                [account_id],
                |row| row.get(0),
            ))?;

            // For new characters, initialise last_access_binary to 0 so the
            // legacy C# client interprets it as DateTime.MinValue
            // ("Never"), matching the original server's behaviour until the
            // character has logged out at least once.
            let now: i64 = 0;
            Self::map_sql_err(conn.execute(
                "INSERT INTO characters (account_id, idx, name, level, class, gender, last_access_binary)
                 VALUES (?1, ?2, ?3, 1, ?4, ?5, ?6)",
                (account_id, &next_idx, &name, &(class as i64), &(gender as i64), &now),
            ))?;

            // Initialise basic stats (HP/MP/experience/gold/credit) for the
            // new character, mirroring the C# BaseStats defaults at level 1.
            let stats = SqliteAccountStore::default_stats_for_new_character(class, 1);
            Self::map_sql_err(conn.execute(
                "INSERT INTO character_stats (account_id, idx, hp, mp, experience, gold, credit)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(account_id, idx) DO UPDATE SET
                     hp = excluded.hp,
                     mp = excluded.mp,
                     experience = excluded.experience,
                     gold = excluded.gold,
                     credit = excluded.credit",
                (
                    account_id,
                    &next_idx,
                    &stats.hp,
                    &stats.mp,
                    &stats.experience,
                    &stats.gold,
                    &stats.credit,
                ),
            ))?;

            Ok(CharacterSummary {
                index: next_idx as i32,
                name,
                level: 1,
                class,
                gender,
                last_access_binary: now,
                map_index: 0,
                x: 0,
                y: 0,
                direction: 0,
            })
        })
    }

    fn delete_character(&self, account_id: &str, index: i32) -> Result<bool, StoreError> {
        self.with_conn(|conn| {
            Self::map_sql_err(conn.execute(
                "DELETE FROM character_magics WHERE account_id = ?1 AND idx = ?2",
                (account_id, &(index as i64)),
            ))?;
            Self::map_sql_err(conn.execute(
                "DELETE FROM character_positions WHERE account_id = ?1 AND idx = ?2",
                (account_id, &(index as i64)),
            ))?;
            Self::map_sql_err(conn.execute(
                "DELETE FROM character_binds WHERE account_id = ?1 AND idx = ?2",
                (account_id, &(index as i64)),
            ))?;
            Self::map_sql_err(conn.execute(
                "DELETE FROM character_stats WHERE account_id = ?1 AND idx = ?2",
                (account_id, &(index as i64)),
            ))?;
            Self::map_sql_err(conn.execute(
                "DELETE FROM character_items WHERE account_id = ?1 AND idx = ?2",
                (account_id, &(index as i64)),
            ))?;
            Self::map_sql_err(conn.execute(
                "DELETE FROM character_guilds WHERE account_id = ?1 AND idx = ?2",
                (account_id, &(index as i64)),
            ))?;
            let rows = Self::map_sql_err(conn.execute(
                "DELETE FROM characters WHERE account_id = ?1 AND idx = ?2",
                (account_id, &(index as i64)),
            ))?;
            Ok(rows > 0)
        })
    }

    fn set_password(&self, id: &str, new_password: &str) -> Result<bool, StoreError> {
        let hashed = hash_password(new_password);
        self.with_conn(|conn| {
            let rows = Self::map_sql_err(conn.execute(
                "UPDATE accounts SET password_hash = ?2 WHERE id = ?1",
                (id, &hashed),
            ))?;
            Ok(rows > 0)
        })
    }

    fn update_character_level(
        &self,
        account_id: &str,
        index: i32,
        level: u16,
    ) -> Result<(), StoreError> {
        self.with_conn(|conn| {
            let _rows = Self::map_sql_err(conn.execute(
                "UPDATE characters SET level = ?3 WHERE account_id = ?1 AND idx = ?2",
                (account_id, &(index as i64), &(level as i64)),
            ))?;
            Ok(())
        })
    }

    fn update_character_last_access(
        &self,
        account_id: &str,
        index: i32,
        last_access_unix_ms: i64,
    ) -> Result<(), StoreError> {
        self.with_conn(|conn| {
            let _rows = Self::map_sql_err(conn.execute(
                "UPDATE characters SET last_access_binary = ?3 WHERE account_id = ?1 AND idx = ?2",
                (account_id, &(index as i64), &last_access_unix_ms),
            ))?;
            Ok(())
        })
    }

    fn load_character_magics(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Vec<UserMagic>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = Self::map_sql_err(conn.prepare(
                "SELECT magics_json FROM character_magics WHERE account_id = ?1 AND idx = ?2 LIMIT 1",
            ))?;
            let mut rows = Self::map_sql_err(stmt.query((account_id, index)))?;
            if let Some(row) = Self::map_sql_err(rows.next())? {
                let json: String = Self::map_sql_err(row.get(0))?;
                let magics: Vec<UserMagic> = serde_json::from_str(&json)
                    .map_err(|e| StoreError::Serde(e.to_string()))?;
                Ok(magics)
            } else {
                Ok(Vec::new())
            }
        })
    }

    fn save_character_magics(
        &self,
        account_id: &str,
        index: i32,
        magics: &[UserMagic],
    ) -> Result<(), StoreError> {
        let json = serde_json::to_string(magics)
            .map_err(|e| StoreError::Serde(e.to_string()))?;
        self.with_conn(|conn| {
            Self::map_sql_err(conn.execute(
                "INSERT INTO character_magics (account_id, idx, magics_json)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(account_id, idx) DO UPDATE SET magics_json = excluded.magics_json",
                (account_id, &index, &json),
            ))?;
            Ok(())
        })
    }

    fn load_character_stats(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Option<CharacterStats>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = Self::map_sql_err(conn.prepare(
                "SELECT hp, mp, experience, gold, credit FROM character_stats WHERE account_id = ?1 AND idx = ?2 LIMIT 1",
            ))?;
            let mut rows = Self::map_sql_err(stmt.query((account_id, index)))?;
            if let Some(row) = Self::map_sql_err(rows.next())? {
                let hp: i32 = Self::map_sql_err(row.get(0))?;
                let mp: i32 = Self::map_sql_err(row.get(1))?;
                let experience: i64 = Self::map_sql_err(row.get(2))?;
                let gold: i64 = Self::map_sql_err(row.get(3))?;
                let credit: i64 = Self::map_sql_err(row.get(4))?;
                Ok(Some(CharacterStats {
                    hp,
                    mp,
                    experience,
                    gold,
                    credit,
                }))
            } else {
                Ok(None)
            }
        })
    }

    fn save_character_stats(
        &self,
        account_id: &str,
        index: i32,
        stats: &CharacterStats,
    ) -> Result<(), StoreError> {
        self.with_conn(|conn| {
            Self::map_sql_err(conn.execute(
                "INSERT INTO character_stats (account_id, idx, hp, mp, experience, gold, credit)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(account_id, idx) DO UPDATE SET
                     hp = excluded.hp,
                     mp = excluded.mp,
                     experience = excluded.experience,
                     gold = excluded.gold,
                     credit = excluded.credit",
                (
                    account_id,
                    &index,
                    &stats.hp,
                    &stats.mp,
                    &stats.experience,
                    &stats.gold,
                    &stats.credit,
                ),
            ))?;
            Ok(())
        })
    }

    fn load_character_position(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Option<CharacterPosition>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = Self::map_sql_err(conn.prepare(
                "SELECT map_index, x, y, direction FROM character_positions WHERE account_id = ?1 AND idx = ?2 LIMIT 1",
            ))?;
            let mut rows = Self::map_sql_err(stmt.query((account_id, index)))?;
            if let Some(row) = Self::map_sql_err(rows.next())? {
                let map_index: i32 = Self::map_sql_err(row.get(0))?;
                let x: i32 = Self::map_sql_err(row.get(1))?;
                let y: i32 = Self::map_sql_err(row.get(2))?;
                let direction: i64 = Self::map_sql_err(row.get(3))?;
                Ok(Some(CharacterPosition {
                    map_index,
                    x,
                    y,
                    direction: direction as u8,
                }))
            } else {
                Ok(None)
            }
        })
    }

    fn save_character_position(
        &self,
        account_id: &str,
        index: i32,
        pos: &CharacterPosition,
    ) -> Result<(), StoreError> {
        self.with_conn(|conn| {
            Self::map_sql_err(conn.execute(
                "INSERT INTO character_positions (account_id, idx, map_index, x, y, direction)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(account_id, idx) DO UPDATE SET
                     map_index = excluded.map_index,
                     x = excluded.x,
                     y = excluded.y,
                     direction = excluded.direction",
                (
                    account_id,
                    &index,
                    &pos.map_index,
                    &pos.x,
                    &pos.y,
                    &(pos.direction as i64),
                ),
            ))?;
            Ok(())
        })
    }

    fn load_character_bind(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Option<CharacterPosition>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = Self::map_sql_err(conn.prepare(
                "SELECT map_index, x, y, direction FROM character_binds WHERE account_id = ?1 AND idx = ?2 LIMIT 1",
            ))?;
            let mut rows = Self::map_sql_err(stmt.query((account_id, index)))?;
            if let Some(row) = Self::map_sql_err(rows.next())? {
                let map_index: i32 = Self::map_sql_err(row.get(0))?;
                let x: i32 = Self::map_sql_err(row.get(1))?;
                let y: i32 = Self::map_sql_err(row.get(2))?;
                let direction: i64 = Self::map_sql_err(row.get(3))?;
                Ok(Some(CharacterPosition {
                    map_index,
                    x,
                    y,
                    direction: direction as u8,
                }))
            } else {
                Ok(None)
            }
        })
    }

    fn save_character_bind(
        &self,
        account_id: &str,
        index: i32,
        pos: &CharacterPosition,
    ) -> Result<(), StoreError> {
        self.with_conn(|conn| {
            Self::map_sql_err(conn.execute(
                "INSERT INTO character_binds (account_id, idx, map_index, x, y, direction)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(account_id, idx) DO UPDATE SET
                     map_index = excluded.map_index,
                     x = excluded.x,
                     y = excluded.y,
                     direction = excluded.direction",
                (
                    account_id,
                    &index,
                    &pos.map_index,
                    &pos.x,
                    &pos.y,
                    &(pos.direction as i64),
                ),
            ))?;
            Ok(())
        })
    }

    fn load_character_items(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Option<(Inventory, Equipment)>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = Self::map_sql_err(conn.prepare(
                "SELECT items_json FROM character_items WHERE account_id = ?1 AND idx = ?2 LIMIT 1",
            ))?;
            let mut rows = Self::map_sql_err(stmt.query((account_id, index)))?;
            if let Some(row) = Self::map_sql_err(rows.next())? {
                let json: String = Self::map_sql_err(row.get(0))?;
                let stored: StoredItems = serde_json::from_str(&json)
                    .map_err(|e| StoreError::Serde(e.to_string()))?;

                let inv_slots = decode_item_slots(stored.inventory)
                    .map_err(StoreError::Io)?;
                let eq_slots = decode_item_slots(stored.equipment)
                    .map_err(StoreError::Io)?;

                Ok(Some((
                    Inventory { slots: inv_slots },
                    Equipment { slots: eq_slots },
                )))
            } else {
                Ok(None)
            }
        })
    }

    fn save_character_items(
        &self,
        account_id: &str,
        index: i32,
        inventory: &Inventory,
        equipment: &Equipment,
    ) -> Result<(), StoreError> {
        let stored = StoredItems {
            inventory: encode_item_slots(&inventory.slots)
                .map_err(StoreError::Io)?,
            equipment: encode_item_slots(&equipment.slots)
                .map_err(StoreError::Io)?,
        };

        let json = serde_json::to_string(&stored)
            .map_err(|e| StoreError::Serde(e.to_string()))?;

        self.with_conn(|conn| {
            Self::map_sql_err(conn.execute(
                "INSERT INTO character_items (account_id, idx, items_json)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(account_id, idx) DO UPDATE SET items_json = excluded.items_json",
                (account_id, &index, &json),
            ))?;
            Ok(())
        })
    }

    fn load_character_guild(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Option<(String, u8)>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = Self::map_sql_err(conn.prepare(
                "SELECT guild_name, rank_index FROM character_guilds WHERE account_id = ?1 AND idx = ?2 LIMIT 1",
            ))?;
            let mut rows = Self::map_sql_err(stmt.query((account_id, index)))?;
            if let Some(row) = Self::map_sql_err(rows.next())? {
                let guild_name: String = Self::map_sql_err(row.get(0))?;
                let rank_index: i64 = Self::map_sql_err(row.get(1))?;
                Ok(Some((guild_name, rank_index as u8)))
            } else {
                Ok(None)
            }
        })
    }

    fn save_character_guild(
        &self,
        account_id: &str,
        index: i32,
        guild_name: &str,
        rank_index: u8,
    ) -> Result<(), StoreError> {
        self.with_conn(|conn| {
            if guild_name.is_empty() {
                Self::map_sql_err(conn.execute(
                    "DELETE FROM character_guilds WHERE account_id = ?1 AND idx = ?2",
                    (account_id, &index),
                ))?;
                Ok(())
            } else {
                Self::map_sql_err(conn.execute(
                    "INSERT INTO character_guilds (account_id, idx, guild_name, rank_index)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(account_id, idx) DO UPDATE SET guild_name = excluded.guild_name, rank_index = excluded.rank_index",
                    (account_id, &index, &guild_name, &(rank_index as i64)),
                ))?;
                Ok(())
            }
        })
    }

    fn load_character_mail(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Vec<StoredMail>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = Self::map_sql_err(conn.prepare(
                "SELECT mail_json FROM character_mail WHERE account_id = ?1 AND idx = ?2 LIMIT 1",
            ))?;
            let mut rows = Self::map_sql_err(stmt.query((account_id, index)))?;
            if let Some(row) = Self::map_sql_err(rows.next())? {
                let json: String = Self::map_sql_err(row.get(0))?;
                let mails: Vec<StoredMail> = serde_json::from_str(&json)
                    .map_err(|e| StoreError::Serde(e.to_string()))?;
                Ok(mails)
            } else {
                Ok(Vec::new())
            }
        })
    }

    fn save_character_mail(
        &self,
        account_id: &str,
        index: i32,
        mails: &[StoredMail],
    ) -> Result<(), StoreError> {
        let json = serde_json::to_string(mails)
            .map_err(|e| StoreError::Serde(e.to_string()))?;

        self.with_conn(|conn| {
            Self::map_sql_err(conn.execute(
                "INSERT INTO character_mail (account_id, idx, mail_json)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(account_id, idx) DO UPDATE SET mail_json = excluded.mail_json",
                (account_id, &index, &json),
            ))?;
            Ok(())
        })
    }

    fn load_all_guilds(&self) -> Result<Vec<GuildInfo>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = Self::map_sql_err(conn.prepare(
                "SELECT data_json FROM guilds",
            ))?;
            let mut rows = Self::map_sql_err(stmt.query([]))?;
            let mut guilds = Vec::new();

            while let Some(row) = Self::map_sql_err(rows.next())? {
                let json: String = Self::map_sql_err(row.get(0))?;
                let guild: GuildInfo = serde_json::from_str(&json)
                    .map_err(|e| StoreError::Serde(e.to_string()))?;
                guilds.push(guild);
            }

            Ok(guilds)
        })
    }

    fn save_guild(&self, guild: &GuildInfo) -> Result<(), StoreError> {
        let json = serde_json::to_string(guild)
            .map_err(|e| StoreError::Serde(e.to_string()))?;
        self.with_conn(|conn| {
            Self::map_sql_err(conn.execute(
                "INSERT INTO guilds (id, name, data_json)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(id) DO UPDATE SET
                     name = excluded.name,
                     data_json = excluded.data_json",
                (&guild.id.0, &guild.name, &json),
            ))?;
            Ok(())
        })
    }

    fn delete_guild(&self, id: i32) -> Result<(), StoreError> {
        self.with_conn(|conn| {
            Self::map_sql_err(conn.execute(
                "DELETE FROM guilds WHERE id = ?1",
                (&id,),
            ))?;
            Ok(())
        })
    }

    fn find_character_by_name(&self, name: &str) -> Result<Option<(String, i32)>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = Self::map_sql_err(conn.prepare(
                "SELECT account_id, idx FROM characters WHERE lower(name) = lower(?1) LIMIT 1",
            ))?;
            let mut rows = Self::map_sql_err(stmt.query([name]))?;
            if let Some(row) = Self::map_sql_err(rows.next())? {
                let account_id: String = Self::map_sql_err(row.get(0))?;
                let idx: i64 = Self::map_sql_err(row.get(1))?;
                Ok(Some((account_id, idx as i32)))
            } else {
                Ok(None)
            }
        })
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum SaveTask {
    CharacterStats {
        account_id: String,
        idx: i32,
        stats: CharacterStats,
    },
    CharacterLevel {
        account_id: String,
        idx: i32,
        level: u16,
    },
    CharacterItems {
        account_id: String,
        idx: i32,
        inventory: Inventory,
        equipment: Equipment,
    },
    CharacterPosition {
        account_id: String,
        idx: i32,
        pos: CharacterPosition,
    },
    CharacterBind {
        account_id: String,
        idx: i32,
        pos: CharacterPosition,
    },
    CharacterMagics {
        account_id: String,
        idx: i32,
        magics: Vec<UserMagic>,
    },
    CharacterGuild {
        account_id: String,
        idx: i32,
        guild_name: String,
        rank_index: u8,
    },
    CharacterMail {
        account_id: String,
        idx: i32,
        mails: Vec<StoredMail>,
    },
    SaveGuild {
        guild: GuildInfo,
    },
    DeleteGuild {
        guild_id: i32,
    },
}

#[derive(Clone, Debug, Default)]
struct PendingCharacter {
    stats: Option<CharacterStats>,
    level: Option<u16>,
    items: Option<(Inventory, Equipment)>,
    position: Option<CharacterPosition>,
    bind: Option<CharacterPosition>,
    magics: Option<Vec<UserMagic>>,
    guild: Option<(String, u8)>,
    mail: Option<Vec<StoredMail>>,
}

#[derive(Clone, Debug, Default)]
struct PendingGuild {
    guild: Option<GuildInfo>,
    delete: bool,
}

type CharKey = (String, i32);

fn apply_save_task(
    task: SaveTask,
    chars: &mut HashMap<CharKey, PendingCharacter>,
    guilds: &mut HashMap<i32, PendingGuild>,
) {
    match task {
        SaveTask::CharacterStats { account_id, idx, stats } => {
            let entry = chars.entry((account_id, idx)).or_default();
            entry.stats = Some(stats);
        }
        SaveTask::CharacterLevel { account_id, idx, level } => {
            let entry = chars.entry((account_id, idx)).or_default();
            entry.level = Some(level);
        }
        SaveTask::CharacterItems { account_id, idx, inventory, equipment } => {
            let entry = chars.entry((account_id, idx)).or_default();
            entry.items = Some((inventory, equipment));
        }
        SaveTask::CharacterPosition { account_id, idx, pos } => {
            let entry = chars.entry((account_id, idx)).or_default();
            entry.position = Some(pos);
        }
        SaveTask::CharacterBind { account_id, idx, pos } => {
            let entry = chars.entry((account_id, idx)).or_default();
            entry.bind = Some(pos);
        }
        SaveTask::CharacterMagics { account_id, idx, magics } => {
            let entry = chars.entry((account_id, idx)).or_default();
            entry.magics = Some(magics);
        }
        SaveTask::CharacterGuild {
            account_id,
            idx,
            guild_name,
            rank_index,
        } => {
            let entry = chars.entry((account_id, idx)).or_default();
            entry.guild = Some((guild_name, rank_index));
        }
        SaveTask::CharacterMail { account_id, idx, mails } => {
            let entry = chars.entry((account_id, idx)).or_default();
            entry.mail = Some(mails);
        }
        SaveTask::SaveGuild { guild } => {
            let entry = guilds.entry(guild.id.0).or_default();
            entry.guild = Some(guild);
            entry.delete = false;
        }
        SaveTask::DeleteGuild { guild_id } => {
            let entry = guilds.entry(guild_id).or_default();
            entry.guild = None;
            entry.delete = true;
        }
    }
}

fn flush_pending(
    db_path: &PathBuf,
    chars: &mut HashMap<CharKey, PendingCharacter>,
    guilds: &mut HashMap<i32, PendingGuild>,
) {
    if chars.is_empty() && guilds.is_empty() {
        return;
    }

    let store = match SqliteAccountStore::open(db_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[db] AsyncAccountStore: failed to open sqlite for flush: {:?}", e);
            return;
        }
    };

    for ((account_id, idx), pending) in chars.drain() {
        if let Some(stats) = pending.stats {
            let _ = AccountStore::save_character_stats(&store, &account_id, idx, &stats);
        }

        if let Some(level) = pending.level {
            let _ = AccountStore::update_character_level(&store, &account_id, idx, level);
        }

        if let Some((inventory, equipment)) = pending.items {
            let _ = AccountStore::save_character_items(&store, &account_id, idx, &inventory, &equipment);
        }

        if let Some(pos) = pending.position {
            let _ = AccountStore::save_character_position(&store, &account_id, idx, &pos);
        }

        if let Some(pos) = pending.bind {
            let _ = AccountStore::save_character_bind(&store, &account_id, idx, &pos);
        }

        if let Some(magics) = pending.magics {
            let _ = AccountStore::save_character_magics(&store, &account_id, idx, &magics);
        }

        if let Some((guild_name, rank_index)) = pending.guild {
            let _ = AccountStore::save_character_guild(&store, &account_id, idx, &guild_name, rank_index);
        }

        if let Some(mails) = pending.mail {
            let _ = AccountStore::save_character_mail(&store, &account_id, idx, &mails);
        }
    }

    for (id, pending) in guilds.drain() {
        if pending.delete {
            let _ = AccountStore::delete_guild(&store, id);
        } else if let Some(guild) = pending.guild {
            let _ = AccountStore::save_guild(&store, &guild);
        }
    }
}

fn run_async_worker(db_path: PathBuf, rx: mpsc::Receiver<SaveTask>, flush_interval: Duration) {
    let mut chars: HashMap<CharKey, PendingCharacter> = HashMap::new();
    let mut guilds: HashMap<i32, PendingGuild> = HashMap::new();
    let mut last_flush = Instant::now();

    loop {
        let timeout = flush_interval
            .checked_sub(last_flush.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        match rx.recv_timeout(timeout) {
            Ok(task) => {
                apply_save_task(task, &mut chars, &mut guilds);

                if last_flush.elapsed() >= flush_interval {
                    flush_pending(&db_path, &mut chars, &mut guilds);
                    last_flush = Instant::now();
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                flush_pending(&db_path, &mut chars, &mut guilds);
                last_flush = Instant::now();
            }
            Err(RecvTimeoutError::Disconnected) => {
                flush_pending(&db_path, &mut chars, &mut guilds);
                break;
            }
        }
    }
}

pub struct AsyncAccountStore {
    path: PathBuf,
    tx: Sender<SaveTask>,
}

impl AsyncAccountStore {
    pub fn open<P: AsRef<Path>>(path: P, flush_interval: Duration) -> Result<Self, StoreError> {
        let path_buf = path.as_ref().to_path_buf();

        if let Some(parent) = path_buf.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        // Ensure database and schema exist.
        let _ = SqliteAccountStore::open(&path_buf)?;

        let (tx, rx) = mpsc::channel();
        let worker_path = path_buf.clone();
        thread::spawn(move || {
            run_async_worker(worker_path, rx, flush_interval);
        });

        Ok(AsyncAccountStore { path: path_buf, tx })
    }

    fn sync_store(&self) -> SqliteAccountStore {
        SqliteAccountStore { path: self.path.clone() }
    }
}

impl AccountStore for AsyncAccountStore {
    fn account_exists(&self, id: &str) -> Result<bool, StoreError> {
        let inner = self.sync_store();
        AccountStore::account_exists(&inner, id)
    }

    fn create_account(&self, id: &str, password: &str) -> Result<(), StoreError> {
        let inner = self.sync_store();
        AccountStore::create_account(&inner, id, password)
    }

    fn load_account_storage(&self, account_id: &str) -> Result<Option<AccountStorage>, StoreError> {
        let inner = self.sync_store();
        AccountStore::load_account_storage(&inner, account_id)
    }

    fn save_account_storage(
        &self,
        account_id: &str,
        storage: &AccountStorage,
    ) -> Result<(), StoreError> {
        let inner = self.sync_store();
        AccountStore::save_account_storage(&inner, account_id, storage)
    }

    fn verify_password(&self, id: &str, password: &str) -> Result<bool, StoreError> {
        let inner = self.sync_store();
        AccountStore::verify_password(&inner, id, password)
    }

    fn load_account_status(&self, id: &str) -> Result<Option<AccountStatus>, StoreError> {
        let inner = self.sync_store();
        AccountStore::load_account_status(&inner, id)
    }

    fn save_account_status(&self, status: &AccountStatus) -> Result<(), StoreError> {
        let inner = self.sync_store();
        AccountStore::save_account_status(&inner, status)
    }

    fn list_characters(&self, account_id: &str) -> Result<Vec<CharacterSummary>, StoreError> {
        let inner = self.sync_store();
        AccountStore::list_characters(&inner, account_id)
    }

    fn create_character(
        &self,
        account_id: &str,
        name: String,
        class: u8,
        gender: u8,
    ) -> Result<CharacterSummary, StoreError> {
        let inner = self.sync_store();
        AccountStore::create_character(&inner, account_id, name, class, gender)
    }

    fn delete_character(&self, account_id: &str, index: i32) -> Result<bool, StoreError> {
        let inner = self.sync_store();
        AccountStore::delete_character(&inner, account_id, index)
    }

    fn set_password(&self, id: &str, new_password: &str) -> Result<bool, StoreError> {
        let inner = self.sync_store();
        AccountStore::set_password(&inner, id, new_password)
    }

    fn load_character_magics(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Vec<UserMagic>, StoreError> {
        let inner = self.sync_store();
        AccountStore::load_character_magics(&inner, account_id, index)
    }

    fn save_character_magics(
        &self,
        account_id: &str,
        index: i32,
        magics: &[UserMagic],
    ) -> Result<(), StoreError> {
        let task = SaveTask::CharacterMagics {
            account_id: account_id.to_string(),
            idx: index,
            magics: magics.to_vec(),
        };
        self.tx
            .send(task)
            .map_err(|e| StoreError::Io(io::Error::new(io::ErrorKind::Other, format!("async save_character_magics failed: {}", e))))
    }

    fn load_character_stats(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Option<CharacterStats>, StoreError> {
        let inner = self.sync_store();
        AccountStore::load_character_stats(&inner, account_id, index)
    }

    fn save_character_stats(
        &self,
        account_id: &str,
        index: i32,
        stats: &CharacterStats,
    ) -> Result<(), StoreError> {
        let task = SaveTask::CharacterStats {
            account_id: account_id.to_string(),
            idx: index,
            stats: stats.clone(),
        };
        self.tx
            .send(task)
            .map_err(|e| StoreError::Io(io::Error::new(io::ErrorKind::Other, format!("async save_character_stats failed: {}", e))))
    }

    fn load_character_position(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Option<CharacterPosition>, StoreError> {
        let inner = self.sync_store();
        AccountStore::load_character_position(&inner, account_id, index)
    }

    fn save_character_position(
        &self,
        account_id: &str,
        index: i32,
        pos: &CharacterPosition,
    ) -> Result<(), StoreError> {
        // Position is used immediately by the next StartGame call after
        // logout/character-switch, so write it synchronously to avoid
        // visible rollback when using AsyncAccountStore.
        let inner = self.sync_store();
        AccountStore::save_character_position(&inner, account_id, index, pos)
    }

    fn load_character_bind(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Option<CharacterPosition>, StoreError> {
        let inner = self.sync_store();
        AccountStore::load_character_bind(&inner, account_id, index)
    }

    fn save_character_bind(
        &self,
        account_id: &str,
        index: i32,
        pos: &CharacterPosition,
    ) -> Result<(), StoreError> {
        // Bind position (Town Revive spawn) is also needed with low latency,
        // so keep this write synchronous as well.
        let inner = self.sync_store();
        AccountStore::save_character_bind(&inner, account_id, index, pos)
    }

    fn update_character_level(
        &self,
        account_id: &str,
        index: i32,
        level: u16,
    ) -> Result<(), StoreError> {
        let task = SaveTask::CharacterLevel {
            account_id: account_id.to_string(),
            idx: index,
            level,
        };
        self.tx
            .send(task)
            .map_err(|e| StoreError::Io(io::Error::new(io::ErrorKind::Other, format!("async update_character_level failed: {}", e))))
    }

    fn load_character_items(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Option<(Inventory, Equipment)>, StoreError> {
        let inner = self.sync_store();
        AccountStore::load_character_items(&inner, account_id, index)
    }

    fn save_character_items(
        &self,
        account_id: &str,
        index: i32,
        inventory: &Inventory,
        equipment: &Equipment,
    ) -> Result<(), StoreError> {
        let task = SaveTask::CharacterItems {
            account_id: account_id.to_string(),
            idx: index,
            inventory: inventory.clone(),
            equipment: equipment.clone(),
        };
        self.tx
            .send(task)
            .map_err(|e| StoreError::Io(io::Error::new(io::ErrorKind::Other, format!("async save_character_items failed: {}", e))))
    }

    fn load_character_guild(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Option<(String, u8)>, StoreError> {
        let inner = self.sync_store();
        AccountStore::load_character_guild(&inner, account_id, index)
    }

    fn save_character_guild(
        &self,
        account_id: &str,
        index: i32,
        guild_name: &str,
        rank_index: u8,
    ) -> Result<(), StoreError> {
        let task = SaveTask::CharacterGuild {
            account_id: account_id.to_string(),
            idx: index,
            guild_name: guild_name.to_string(),
            rank_index,
        };
        self.tx
            .send(task)
            .map_err(|e| StoreError::Io(io::Error::new(io::ErrorKind::Other, format!("async save_character_guild failed: {}", e))))
    }

    fn load_all_guilds(&self) -> Result<Vec<GuildInfo>, StoreError> {
        let inner = self.sync_store();
        AccountStore::load_all_guilds(&inner)
    }

    fn save_guild(&self, guild: &GuildInfo) -> Result<(), StoreError> {
        let task = SaveTask::SaveGuild { guild: guild.clone() };
        self.tx
            .send(task)
            .map_err(|e| StoreError::Io(io::Error::new(io::ErrorKind::Other, format!("async save_guild failed: {}", e))))
    }

    fn delete_guild(&self, id: i32) -> Result<(), StoreError> {
        let task = SaveTask::DeleteGuild { guild_id: id };
        self.tx
            .send(task)
            .map_err(|e| StoreError::Io(io::Error::new(io::ErrorKind::Other, format!("async delete_guild failed: {}", e))))
    }

    fn find_character_by_name(&self, name: &str) -> Result<Option<(String, i32)>, StoreError> {
        let inner = self.sync_store();
        AccountStore::find_character_by_name(&inner, name)
    }

    fn update_character_last_access(
        &self,
        account_id: &str,
        index: i32,
        last_access_unix_ms: i64,
    ) -> Result<(), StoreError> {
        let inner = self.sync_store();
        AccountStore::update_character_last_access(&inner, account_id, index, last_access_unix_ms)
    }

    fn load_character_mail(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Vec<StoredMail>, StoreError> {
        let inner = self.sync_store();
        AccountStore::load_character_mail(&inner, account_id, index)
    }

    fn save_character_mail(
        &self,
        account_id: &str,
        index: i32,
        mails: &[StoredMail],
    ) -> Result<(), StoreError> {
        let task = SaveTask::CharacterMail {
            account_id: account_id.to_string(),
            idx: index,
            mails: mails.to_vec(),
        };
        self.tx
            .send(task)
            .map_err(|e| StoreError::Io(io::Error::new(io::ErrorKind::Other, format!("async save_character_mail failed: {}", e))))
    }
}
