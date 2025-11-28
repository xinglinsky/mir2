use std::io;

use serde::{Deserialize, Serialize};
use crate::world::magic::UserMagic;
use crate::item::{Inventory, Equipment};
use crate::guild::GuildInfo;

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    Serde(String),
    Sqlite(String),
}

impl From<io::Error> for StoreError {
    fn from(e: io::Error) -> Self {
        StoreError::Io(e)
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
    #[serde(default)]
    pub map_index: i32,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub direction: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterPosition {
    pub map_index: i32,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterStats {
    pub hp: i32,
    pub mp: i32,
    pub experience: i64,
    pub gold: i64,
    pub credit: i64,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountStatus {
    pub id: String,
    pub banned: bool,
    pub ban_reason: String,
    /// Unix timestamp in milliseconds until which the ban is active.
    /// When sending ban-related packets to the legacy C# client, this is
    /// converted to DateTime.ToBinary-compatible ticks.
    pub ban_expires_at: i64,
    pub require_password_change: bool,
    pub wrong_password_count: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(dead_code)]
struct AccountDb {
    accounts: Vec<StoredAccount>,
}

pub trait AccountStore: Send + Sync {
    fn account_exists(&self, id: &str) -> Result<bool, StoreError>;
    fn create_account(&self, id: &str, password: &str) -> Result<(), StoreError>;
    fn verify_password(&self, id: &str, password: &str) -> Result<bool, StoreError>;

    /// Load/save account flags required to mirror the C# AccountInfo login
    /// semantics (banned / ban reason / expiry / RequirePasswordChange /
    /// WrongPasswordCount). Implementations are expected to keep this in the
    /// same backing store as passwords and timestamps.
    fn load_account_status(&self, id: &str) -> Result<Option<AccountStatus>, StoreError>;
    fn save_account_status(&self, status: &AccountStatus) -> Result<(), StoreError>;

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

    /// Load the learned magics for a given character. If no record exists yet,
    /// returns an empty Vec.
    fn load_character_magics(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Vec<UserMagic>, StoreError>;

    /// Persist the learned magics for a given character, replacing any
    /// previously stored record.
    fn save_character_magics(
        &self,
        account_id: &str,
        index: i32,
        magics: &[UserMagic],
    ) -> Result<(), StoreError>;

    /// Load basic stats (HP/MP/experience/gold/credit) for a given character.
    /// Returns Ok(None) if no stats have been stored yet.
    fn load_character_stats(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Option<CharacterStats>, StoreError>;

    /// Persist basic stats (HP/MP/experience/gold/credit) for a given
    /// character.
    fn save_character_stats(
        &self,
        account_id: &str,
        index: i32,
        stats: &CharacterStats,
    ) -> Result<(), StoreError>;

    /// Load the last known position for a given character. Returns Ok(None)
    /// if no position has been stored yet.
    fn load_character_position(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Option<CharacterPosition>, StoreError>;

    /// Persist the last known position for a given character.
    fn save_character_position(
        &self,
        account_id: &str,
        index: i32,
        pos: &CharacterPosition,
    ) -> Result<(), StoreError>;

    /// Load the bind point for a given character (equivalent to BindMapIndex /
    /// BindLocation in the C# server). Returns Ok(None) if no bind has been
    /// stored yet.
    fn load_character_bind(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Option<CharacterPosition>, StoreError>;

    /// Persist the bind point for a given character.
    fn save_character_bind(
        &self,
        account_id: &str,
        index: i32,
        pos: &CharacterPosition,
    ) -> Result<(), StoreError>;

    /// Update the current level for a given character in the summary table,
    /// mirroring the C# server's persistence of CharacterInfo.Level on
    /// level-up or logout.
    fn update_character_level(
        &self,
        account_id: &str,
        index: i32,
        level: u16,
    ) -> Result<(), StoreError>;

    /// Load the full item state (inventory + equipment) for a given
    /// character. Returns Ok(None) if no items have been stored yet.
    fn load_character_items(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Option<(Inventory, Equipment)>, StoreError>;

    /// Persist the full item state (inventory + equipment) for a given
    /// character.
    fn save_character_items(
        &self,
        account_id: &str,
        index: i32,
        inventory: &Inventory,
        equipment: &Equipment,
    ) -> Result<(), StoreError>;

    fn load_character_guild(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Option<(String, u8)>, StoreError>;

    fn save_character_guild(
        &self,
        account_id: &str,
        index: i32,
        guild_name: &str,
        rank_index: u8,
    ) -> Result<(), StoreError>;

    fn load_all_guilds(&self) -> Result<Vec<GuildInfo>, StoreError>;

    fn save_guild(&self, guild: &GuildInfo) -> Result<(), StoreError>;

    fn delete_guild(&self, id: i32) -> Result<(), StoreError>;
}

pub fn hash_password(password: &str) -> String {
    // Simple SHA-256 hex hash, good enough for a local stub.
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

pub fn verify_password_hash(password: &str, hash: &str) -> bool {
    hash_password(password) == hash
}

