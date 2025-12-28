//! Combat system module
//! 
//! This module handles all combat-related functionality including:
//! - Attack commands (ranged, melee, magic)
//! - Damage calculation and application
//! - Combat buffs and effects
//! - Negative effects (freezing, poisoning, paralysis)
//! - Elemental damage system
//! - Utility functions

pub mod commands;
pub mod damage;
pub mod buffs;
pub mod effects;
pub mod elemental;
pub mod utils;

// Re-export commonly used functions
pub use damage::{apply_damage_to_player, heal_player, apply_equipment_durability_loss};
pub use buffs::{player_has_buff, remove_player_buff};
pub use effects::{apply_negative_effects, apply_negative_effects_with_level_offset};
pub use utils::can_attack;

// Note: Elemental functions are methods on World<P>, so they are not re-exported here.
// Access them through the World type or import directly from the submodules if needed.

/// Delay in milliseconds for log actions (matches C# Globals.LogDelay)
pub const LOG_DELAY_MS: u64 = 6000;

/// Delay in milliseconds for operate actions (matches C# settings)
pub const OPERATE_DELAY_MS: u64 = 1000;

/// Helper struct for player damage calculation data
pub struct PlayerDamageData {
    pub hp: i32,
    pub stats: crate::stats::Stats,
    pub current_poison_mask: u16,
    pub dead: bool,
}

/// Helper struct for player position data
pub struct PlayerPosition {
    pub x: i32,
    pub y: i32,
    pub direction: u8,
}

