use std::io;

use serde::{Deserialize, Serialize};
use crate::world::magic::UserMagic;
use crate::item::{Inventory, Equipment};
use crate::guild::GuildInfo;
use crystal_shared_proto::item_types::UserItemData;

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

/// Lightweight projection of a character suitable for global ranking queries.
/// This is loaded directly from the account database and then converted into
/// world-side RankCharacterInfo entries when building combined online/offline
/// ranking tables.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterRankRow {
    pub account_id: String,
    pub index: i32,
    pub name: String,
    pub class: u8,
    pub level: u16,
    pub experience: i64,
}

/// Account-wide item storage, mirroring the C# AccountInfo.Storage,
/// HasExpandedStorage and ExpandedStorageExpiryDate fields. The slots vector
/// contains the full set of storage cells for the account (including any
/// expanded pages). This type is not serialized directly via serde; the DB
/// layer uses a separate StoredAccountStorage helper that stores item bytes.
#[derive(Clone, Debug)]
pub struct AccountStorage {
    pub slots: Vec<Option<UserItemData>>,
    /// Whether the account currently has expanded storage beyond the base
    /// StorageGridSize.
    pub has_expanded_storage: bool,
    /// Expanded storage expiry time stored as DateTime.ToBinary() ticks so it
    /// can be forwarded directly to the legacy C# client.
    pub expanded_storage_expiry_binary: i64,
}

/// Simplified persisted mail representation for a character, roughly
/// mirroring the C# MailInfo/ClientMail structures. This is stored in the
/// account database and projected into SReceiveMail/ClientMail bytes when
/// sending mail to the legacy C# client.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredMail {
    pub mail_id: u64,
    pub sender: String,
    pub message: String,
    pub gold: u32,
    /// Binary-encoded UserItemData blobs (one per attachment), using the
    /// same layout as UserItemData::encode. This keeps the storage format
    /// stable without requiring serde derives on UserItemData.
    pub items: Vec<Vec<u8>>,
    /// DateSent.ToBinary() from the C# world, represented directly so we
    /// can forward it to the client without losing fidelity.
    pub date_sent_binary: i64,
    pub opened: bool,
    pub locked: bool,
    pub collected: bool,
    pub can_reply: bool,
}

/// Simplified persisted friend representation for a character, roughly
/// mirroring the C# FriendInfo/ClientFriend structures. This is stored in the
/// account database and projected into SFriendUpdate/ClientFriend bytes when
/// sending the friend list to the legacy C# client.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredFriend {
    pub friend_index: i32,
    pub name: String,
    pub memo: String,
    pub blocked: bool,
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

    /// Load the account-wide item storage (personal warehouse) for the given
    /// account. This mirrors the C# AccountInfo.Storage,
    /// AccountInfo.HasExpandedStorage and AccountInfo.ExpandedStorageExpiryDate
    /// fields. Returns Ok(None) when no storage has been created yet.
    fn load_account_storage(&self, account_id: &str) -> Result<Option<AccountStorage>, StoreError>;

    /// Persist the full account-wide item storage for the given account,
    /// replacing any previously stored record.
    fn save_account_storage(
        &self,
        account_id: &str,
        storage: &AccountStorage,
    ) -> Result<(), StoreError>;

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
    ) -> Result<Option<(Inventory, Equipment, Vec<Option<UserItemData>>)>, StoreError>;

    /// Persist the full item state (inventory + equipment + refine_slots) for a given
    /// character.
    fn save_character_items(
        &self,
        account_id: &str,
        index: i32,
        inventory: &Inventory,
        equipment: &Equipment,
        refine_slots: &[Option<UserItemData>],
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

    /// Look up a character by exact name, returning (account_id, idx) if
    /// found. This is used for guild operations such as kicking members by
    /// name, mirroring the C# server's ability to operate on offline
    /// characters.
    fn find_character_by_name(&self, name: &str) -> Result<Option<(String, i32)>, StoreError>;

    /// Update the last-access timestamp for a character in the summary
    /// table. The value is stored as a Unix timestamp in milliseconds and
    /// converted to DateTime.ToBinary-compatible ticks when sending
    /// SelectInfo to the legacy C# client.
    fn update_character_last_access(
        &self,
        account_id: &str,
        index: i32,
        last_access_unix_ms: i64,
    ) -> Result<(), StoreError>;

    /// Load all stored mail for a given character. This is a simplified
    /// projection of the C# MailInfo/ClientMail data used for the legacy
    /// client mailbox. Implementations should return an empty Vec when no
    /// mail exists yet.
    fn load_character_mail(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Vec<StoredMail>, StoreError>;

    /// Persist the full set of stored mail for a given character,
    /// overwriting any previously saved list.
    fn save_character_mail(
        &self,
        account_id: &str,
        index: i32,
        mails: &[StoredMail],
    ) -> Result<(), StoreError>;

    /// Load all stored friends for a given character. This mirrors the C#
    /// CharacterInfo.Friends / FriendInfo list but in a simplified form.
    /// Implementations should return an empty Vec when no friends exist yet.
    fn load_character_friends(
        &self,
        account_id: &str,
        index: i32,
    ) -> Result<Vec<StoredFriend>, StoreError>;

    /// Persist the full set of stored friends for a given character,
    /// overwriting any previously saved list.
    fn save_character_friends(
        &self,
        account_id: &str,
        index: i32,
        friends: &[StoredFriend],
    ) -> Result<(), StoreError>;

    /// Load a page of global character ranking entries for the given RankType.
    ///
    /// RankType mapping mirrors the legacy C# server/client convention:
    ///   0 = overall (all classes)
    ///   1 = warrior
    ///   2 = wizard
    ///   3 = taoist
    ///   4 = assassin
    ///   5 = archer
    /// Any other value returns an empty Vec. Results are ordered by
    /// Level DESC, then Experience DESC, matching the C# InsertRank logic.
    fn load_ranking_page(
        &self,
        rank_type: u8,
        offset: i32,
        limit: i32,
    ) -> Result<Vec<CharacterRankRow>, StoreError>;

    /// Return the total number of characters that participate in the ranking
    /// for the given RankType. This uses the same RankType mapping as
    /// `load_ranking_page` and counts all characters (online and offline).
    fn count_ranking_entries(&self, rank_type: u8) -> Result<i64, StoreError>;
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

