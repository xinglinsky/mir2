use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::Utc;
use crystal_server_core::account::{
    AccountStore,
    StoreError,
    CharacterSummary,
    CharacterStats,
    CharacterPosition,
    hash_password,
    verify_password_hash,
};
use crystal_server_core::world::magic::UserMagic;
use rusqlite::{self, Connection};
use serde_json;

pub struct SqliteAccountStore {
    path: PathBuf,
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
                    last_login_at   INTEGER
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
                "#,
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

            let now = Utc::now().timestamp_millis();
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
}
