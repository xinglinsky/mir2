use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    Serde(serde_json::Error),
    Sqlite(rusqlite::Error),
}

impl From<io::Error> for StoreError {
    fn from(e: io::Error) -> Self {
        StoreError::Io(e)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        StoreError::Serde(e)
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError::Sqlite(e)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterSummary {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: u8,
    pub gender: u8,
    pub last_access_binary: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredAccount {
    pub id: String,
    pub password_hash: String,
    pub created_at: i64,
    pub last_login_at: Option<i64>,
    pub next_char_index: i32,
    pub characters: Vec<CharacterSummary>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct AccountDb {
    accounts: Vec<StoredAccount>,
}

pub trait AccountStore: Send + Sync {
    fn account_exists(&self, id: &str) -> Result<bool, StoreError>;
    fn create_account(&self, id: &str, password: &str) -> Result<(), StoreError>;
    fn verify_password(&self, id: &str, password: &str) -> Result<bool, StoreError>;

    fn list_characters(&self, account_id: &str) -> Result<Vec<CharacterSummary>, StoreError>;
    fn create_character(
        &self,
        account_id: &str,
        name: String,
        class: u8,
        gender: u8,
    ) -> Result<CharacterSummary, StoreError>;
    fn delete_character(&self, account_id: &str, index: i32) -> Result<bool, StoreError>;
    fn set_password(&self, id: &str, new_password: &str) -> Result<bool, StoreError>;
}

fn hash_password(password: &str) -> String {
    // Simple SHA-256 hex hash, good enough for a local stub.
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

fn verify_password_hash(password: &str, hash: &str) -> bool {
    hash_password(password) == hash
}

pub struct FileAccountStore {
    path: PathBuf,
    db: Mutex<AccountDb>,
}

impl FileAccountStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StoreError> {
        let path = path.as_ref().to_path_buf();
        let db = if path.exists() {
            let mut file = fs::File::open(&path)?;
            let mut buf = String::new();
            file.read_to_string(&mut buf)?;
            if buf.trim().is_empty() {
                AccountDb::default()
            } else {
                serde_json::from_str(&buf)?
            }
        } else {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent)?;
                }
            }
            AccountDb::default()
        };

        Ok(FileAccountStore {
            path,
            db: Mutex::new(db),
        })
    }

    fn save_locked(&self, db: &AccountDb) -> Result<(), StoreError> {
        let tmp_path = self.path.with_extension("tmp");
        let json = serde_json::to_string_pretty(db)?;
        {
            let mut f = fs::File::create(&tmp_path)?;
            f.write_all(json.as_bytes())?;
            f.flush()?;
        }
        fs::rename(tmp_path, &self.path)?;
        Ok(())
    }

    fn get_account_mut<'a>(
        db: &'a mut AccountDb,
        id: &str,
    ) -> Option<&'a mut StoredAccount> {
        db.accounts.iter_mut().find(|a| a.id == id)
    }

    fn get_account<'a>(db: &'a AccountDb, id: &str) -> Option<&'a StoredAccount> {
        db.accounts.iter().find(|a| a.id == id)
    }
}

impl AccountStore for FileAccountStore {
    fn account_exists(&self, id: &str) -> Result<bool, StoreError> {
        let db = self.db.lock().unwrap();
        Ok(Self::get_account(&db, id).is_some())
    }

    fn create_account(&self, id: &str, password: &str) -> Result<(), StoreError> {
        let mut db = self.db.lock().unwrap();
        if Self::get_account(&db, id).is_some() {
            return Ok(());
        }

        let now = chrono::Utc::now().timestamp_millis();
        let acc = StoredAccount {
            id: id.to_string(),
            password_hash: hash_password(password),
            created_at: now,
            last_login_at: None,
            next_char_index: 0,
            characters: Vec::new(),
        };
        db.accounts.push(acc);
        self.save_locked(&db)
    }

    fn verify_password(&self, id: &str, password: &str) -> Result<bool, StoreError> {
        let mut db = self.db.lock().unwrap();
        if let Some(acc) = Self::get_account_mut(&mut db, id) {
            let ok = verify_password_hash(password, &acc.password_hash);
            if ok {
                acc.last_login_at = Some(chrono::Utc::now().timestamp_millis());
                self.save_locked(&db)?;
            }
            Ok(ok)
        } else {
            Ok(false)
        }
    }

    fn list_characters(&self, account_id: &str) -> Result<Vec<CharacterSummary>, StoreError> {
        let db = self.db.lock().unwrap();
        if let Some(acc) = Self::get_account(&db, account_id) {
            Ok(acc.characters.clone())
        } else {
            Ok(Vec::new())
        }
    }

    fn create_character(
        &self,
        account_id: &str,
        name: String,
        class: u8,
        gender: u8,
    ) -> Result<CharacterSummary, StoreError> {
        let mut db = self.db.lock().unwrap();
        let acc = Self::get_account_mut(&mut db, account_id)
            .ok_or_else(|| StoreError::Io(io::Error::new(io::ErrorKind::NotFound, "account not found")))?;

        let index = acc.next_char_index;
        acc.next_char_index += 1;

        let now = chrono::Utc::now().timestamp_millis();
        let ch = CharacterSummary {
            index,
            name,
            level: 1,
            class,
            gender,
            last_access_binary: now,
        };
        acc.characters.push(ch.clone());
        self.save_locked(&db)?;
        Ok(ch)
    }

    fn delete_character(&self, account_id: &str, index: i32) -> Result<bool, StoreError> {
        let mut db = self.db.lock().unwrap();
        let acc = match Self::get_account_mut(&mut db, account_id) {
            Some(a) => a,
            None => return Ok(false),
        };
        if let Some(pos) = acc.characters.iter().position(|c| c.index == index) {
            acc.characters.remove(pos);
            self.save_locked(&db)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn set_password(&self, id: &str, new_password: &str) -> Result<bool, StoreError> {
        let mut db = self.db.lock().unwrap();
        if let Some(acc) = Self::get_account_mut(&mut db, id) {
            acc.password_hash = hash_password(new_password);
            self.save_locked(&db)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// SQLite implementation of AccountStore.
///
/// This store keeps only the database file path in memory and opens a new
/// connection for each operation. This keeps the type `Send + Sync` without
/// relying on rusqlite connection thread guarantees and is sufficient for
/// the current low-throughput use case.
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

    fn with_conn<T, F>(&self, f: F) -> Result<T, StoreError>
    where
        F: FnOnce(&rusqlite::Connection) -> Result<T, StoreError>,
    {
        let conn = rusqlite::Connection::open(&self.path)?;
        f(&conn)
    }

    fn init_db(&self) -> Result<(), StoreError> {
        self.with_conn(|conn| {
            conn.execute_batch(
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
                "#,
            )?;
            Ok(())
        })
    }
}

impl AccountStore for SqliteAccountStore {
    fn account_exists(&self, id: &str) -> Result<bool, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT 1 FROM accounts WHERE id = ?1 LIMIT 1")?;
            let exists = stmt.exists([id])?;
            Ok(exists)
        })
    }

    fn create_account(&self, id: &str, password: &str) -> Result<(), StoreError> {
        let hashed = hash_password(password);
        let now = chrono::Utc::now().timestamp_millis();

        self.with_conn(|conn| {
            conn.execute(
                "INSERT OR IGNORE INTO accounts (id, password_hash, created_at, last_login_at) VALUES (?1, ?2, ?3, NULL)",
                (id, &hashed, &now),
            )?;
            Ok(())
        })
    }

    fn verify_password(&self, id: &str, password: &str) -> Result<bool, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT password_hash FROM accounts WHERE id = ?1 LIMIT 1",
            )?;
            let mut rows = stmt.query([id])?;
            if let Some(row) = rows.next()? {
                let stored: String = row.get(0)?;
                let ok = verify_password_hash(password, &stored);
                if ok {
                    let now = chrono::Utc::now().timestamp_millis();
                    conn.execute(
                        "UPDATE accounts SET last_login_at = ?2 WHERE id = ?1",
                        (id, &now),
                    )?;
                }
                Ok(ok)
            } else {
                Ok(false)
            }
        })
    }

    fn list_characters(&self, account_id: &str) -> Result<Vec<CharacterSummary>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT idx, name, level, class, gender, last_access_binary
                 FROM characters WHERE account_id = ?1 ORDER BY idx",
            )?;
            let rows = stmt.query_map([account_id], |row| {
                Ok(CharacterSummary {
                    index: row.get::<_, i64>(0)? as i32,
                    name: row.get(1)?,
                    level: row.get::<_, i64>(2)? as u16,
                    class: row.get::<_, i64>(3)? as u8,
                    gender: row.get::<_, i64>(4)? as u8,
                    last_access_binary: row.get(5)?,
                })
            })?;

            let mut chars = Vec::new();
            for ch in rows {
                chars.push(ch?);
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
            let mut check_stmt = conn.prepare("SELECT 1 FROM accounts WHERE id = ?1 LIMIT 1")?;
            if !check_stmt.exists([account_id])? {
                return Err(StoreError::Io(io::Error::new(
                    io::ErrorKind::NotFound,
                    "account not found",
                )));
            }

            // Next character index for this account.
            let next_idx: i64 = conn.query_row(
                "SELECT COALESCE(MAX(idx) + 1, 0) FROM characters WHERE account_id = ?1",
                [account_id],
                |row| row.get(0),
            )?;

            let now = chrono::Utc::now().timestamp_millis();
            conn.execute(
                "INSERT INTO characters (account_id, idx, name, level, class, gender, last_access_binary)
                 VALUES (?1, ?2, ?3, 1, ?4, ?5, ?6)",
                (account_id, &next_idx, &name, &(class as i64), &(gender as i64), &now),
            )?;

            Ok(CharacterSummary {
                index: next_idx as i32,
                name,
                level: 1,
                class,
                gender,
                last_access_binary: now,
            })
        })
    }

    fn delete_character(&self, account_id: &str, index: i32) -> Result<bool, StoreError> {
        self.with_conn(|conn| {
            let rows = conn.execute(
                "DELETE FROM characters WHERE account_id = ?1 AND idx = ?2",
                (account_id, &(index as i64)),
            )?;
            Ok(rows > 0)
        })
    }

    fn set_password(&self, id: &str, new_password: &str) -> Result<bool, StoreError> {
        let hashed = hash_password(new_password);
        self.with_conn(|conn| {
            let rows = conn.execute(
                "UPDATE accounts SET password_hash = ?2 WHERE id = ?1",
                (id, &hashed),
            )?;
            Ok(rows > 0)
        })
    }
}
