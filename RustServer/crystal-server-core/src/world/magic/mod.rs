use std::io;

use rand::Rng;
use serde::{Deserialize, Serialize};
use crystal_shared_proto::io::{write_i64_le, write_string, write_u16_le};

#[derive(Clone, Debug)]
pub struct MagicInfo {
    pub name: String,
    /// Numeric spell identifier matching the C# Spell enum.
    pub spell: u8,
    pub base_cost: u8,
    pub level_cost: u8,
    pub icon: u8,
    pub level1: u8,
    pub level2: u8,
    pub level3: u8,
    pub need1: u16,
    pub need2: u16,
    pub need3: u16,
    pub delay_base: u32,
    pub delay_reduction: u32,
    pub power_base: u16,
    pub power_bonus: u16,
    pub mpower_base: u16,
    pub mpower_bonus: u16,
    pub range: u8,
    pub multiplier_base: f32,
    pub multiplier_bonus: f32,
}

/// Per-character learned magic, mirroring the persistent fields of C# UserMagic.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserMagic {
    /// Numeric spell identifier matching the C# Spell enum.
    pub spell: u8,
    pub level: u8,
    pub key: u8,
    pub experience: u16,
    pub is_temp_spell: bool,
    /// Absolute cast time in server ticks/time units, comparable with `now`.
    pub cast_time: i64,
}

impl UserMagic {
    /// Construct a new UserMagic with default progression fields.
    pub fn new(spell: u8) -> Self {
        Self {
            spell,
            level: 0,
            key: 0,
            experience: 0,
            is_temp_spell: false,
            cast_time: 0,
        }
    }
}

/// Compute the damage multiplier for a magic at the given level, mirroring the
/// C# UserMagic.GetMultiplier implementation.
pub fn magic_multiplier(info: &MagicInfo, level: u8) -> f32 {
    info.multiplier_base + (level as f32 * info.multiplier_bonus)
}

/// Compute the MPower component for a magic, mirroring the C# MPower method.
pub fn magic_mpower<R: Rng + ?Sized>(info: &MagicInfo, rng: &mut R) -> i32 {
    if info.mpower_bonus > 0 {
        let base = info.mpower_base as i32;
        let bonus = info.mpower_bonus as i32;
        // C# Envir.Random.Next(Info.MPowerBase, Info.MPowerBonus + Info.MPowerBase)
        // samples in the half-open interval [base, base + bonus), so we must
        // mirror that using an exclusive upper bound in Rust.
        rng.gen_range(base..base + bonus)
    } else {
        info.mpower_base as i32
    }
}

/// Compute the DefPower component for a magic, mirroring the C# DefPower
/// method.
pub fn magic_def_power<R: Rng + ?Sized>(info: &MagicInfo, rng: &mut R) -> i32 {
    if info.power_bonus > 0 {
        let base = info.power_base as i32;
        let bonus = info.power_bonus as i32;
        // C# Envir.Random.Next(Info.PowerBase, Info.PowerBonus + Info.PowerBase)
        // uses the same half-open range [base, base + bonus).
        rng.gen_range(base..base + bonus)
    } else {
        info.power_base as i32
    }
}

/// Compute the base power added by a magic at the given level, mirroring the
/// C# UserMagic.GetPower implementation.
pub fn magic_power<R: Rng + ?Sized>(info: &MagicInfo, level: u8, rng: &mut R) -> i32 {
    let mpower = magic_mpower(info, rng) as f32;
    let def_power = magic_def_power(info, rng) as f32;
    ((mpower / 4.0) * (level as f32 + 1.0) + def_power).round() as i32
}

/// Compute final damage for a magic given a base physical DamageBase, mirroring
/// the C# UserMagic.GetDamage implementation:
///   (DamageBase + GetPower()) * GetMultiplier()
pub fn magic_damage<R: Rng + ?Sized>(
    info: &MagicInfo,
    level: u8,
    damage_base: i32,
    rng: &mut R,
) -> i32 {
    let power = magic_power(info, level, rng);
    let mult = magic_multiplier(info, level);
    let sum = damage_base.saturating_add(power).max(0) as f32;
    // C# UserMagic.GetDamage casts the floating-point result of
    //   (DamageBase + GetPower()) * GetMultiplier()
    // directly to int, which truncates toward zero. We mirror that
    // behaviour here instead of rounding.
    let raw = (sum * mult) as i32;
    raw
}

/// Encode the exact payload of ClientMagic.Save(writer) given the static MagicInfo
/// from MirDB and the per-character UserMagic state.
///
/// This matches C# ClientMagic.Save(BinaryWriter):
///   Name, Spell, BaseCost, LevelCost, Icon, Level1-3, Need1-3,
///   Level, Key, Experience, Delay, Range, CastTime.
///
/// `now` should be the same time base as `UserMagic.cast_time`, so that
/// `cast_time - now` mirrors C#'s `CastTime - Envir.Time`.
pub fn encode_client_magic_bytes(
    info: &MagicInfo,
    magic: &UserMagic,
    now: i64,
) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();

    // Name
    write_string(&mut buf, &info.name)?;

    // Spell
    buf.push(magic.spell);

    // BaseCost, LevelCost, Icon
    buf.push(info.base_cost);
    buf.push(info.level_cost);
    buf.push(info.icon);

    // Level1, Level2, Level3
    buf.push(info.level1);
    buf.push(info.level2);
    buf.push(info.level3);

    // Need1, Need2, Need3
    write_u16_le(&mut buf, info.need1)?;
    write_u16_le(&mut buf, info.need2)?;
    write_u16_le(&mut buf, info.need3)?;

    // Level, Key, Experience
    buf.push(magic.level);
    buf.push(magic.key);
    write_u16_le(&mut buf, magic.experience)?;

    // Delay = Info.DelayBase - (Level * Info.DelayReduction)
    let delay: i64 = info.delay_base as i64 - (magic.level as i64 * info.delay_reduction as i64);
    write_i64_le(&mut buf, delay)?;

    // Range
    buf.push(info.range);

    // CastTime offset = CastTime - Envir.Time
    let cast_time_offset: i64 = magic.cast_time - now;
    write_i64_le(&mut buf, cast_time_offset)?;

    Ok(buf)
}
