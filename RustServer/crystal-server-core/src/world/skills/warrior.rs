use crate::world::provider::WorldProvider;

const SPELL_HALF_MOON: u8 = crate::world::Spell::HalfMoon as u8;
const SPELL_THRUSTING: u8 = crate::world::Spell::Thrusting as u8;
const SPELL_CROSS_HALF_MOON: u8 = crate::world::Spell::CrossHalfMoon as u8;

pub fn is_warrior_aoe_skill(spell: u8) -> bool {
    spell == SPELL_HALF_MOON || spell == SPELL_CROSS_HALF_MOON
}

/// Return true if this spell is the Thrusting skill, which extends melee
/// attack range in a straight line.
pub fn is_thrusting_spell(spell: u8) -> bool {
    spell == SPELL_THRUSTING
}

/// Compute the list of target coordinates for a HalfMoon attack based on
/// the attacker's position and direction.
///
/// This mirrors the C# HumanObject HalfMoon label: starting from the
/// direction immediately to the left of the facing direction, walk 4
/// directions around the player and hit the tiles one step away, skipping
/// the tile directly in front (which is handled by the primary melee
/// attack).
///
/// Returns a vector of (x, y, is_primary_target) tuples; the primary target
/// is the tile directly in front of the attacker.
pub fn compute_half_moon_targets(x: i32, y: i32, direction: u8) -> Vec<(i32, i32, bool)> {
    let mut targets = Vec::new();

    let front_offset = match direction {
        0 => (0, -1),
        1 => (1, -1),
        2 => (1, 0),
        3 => (1, 1),
        4 => (0, 1),
        5 => (-1, 1),
        6 => (-1, 0),
        7 => (-1, -1),
        _ => (0, 0),
    };
    let front_x = x + front_offset.0;
    let front_y = y + front_offset.1;

    // Primary target: one step directly in front.
    targets.push((front_x, front_y, true));

    // Secondary targets: previous direction, facing direction, next, and
    // next-next, skipping the already-handled Front tile. This produces up
    // to three additional tiles around the arc.
    let mut dir = (direction + 7) % 8; // previous dir

    let dir_offset = |d: u8| -> (i32, i32) {
        match d {
            0 => (0, -1),
            1 => (1, -1),
            2 => (1, 0),
            3 => (1, 1),
            4 => (0, 1),
            5 => (-1, 1),
            6 => (-1, 0),
            7 => (-1, -1),
            _ => (0, 0),
        }
    };

    for _ in 0..4 {
        let (dx, dy) = dir_offset(dir);
        let tx = x + dx;
        let ty = y + dy;

        if tx != front_x || ty != front_y {
            targets.push((tx, ty, false));
        }

        dir = (dir + 1) % 8;
    }

    targets
}

/// Calculate damage for a HalfMoon secondary target.
/// Usually it's a percentage of the primary damage.
pub fn compute_half_moon_secondary_damage(primary_damage: i32, level: u8) -> i32 {
    // C# logic often scales AOE damage by level.
    // Example: Base 10% + Level * 10%? 
    // Or just flat divide.
    // Assuming 30% base + level scaling for now or flat ratio.
    // Legacy C# often does: damage / (some_factor).
    
    // Let's use a safe default: 30% of primary damage for secondary targets
    // plus a small bonus per level.
    let percent = 30 + (level as i32 * 5); // Lv0=30%, Lv3=45%
    (primary_damage * percent / 100).max(1)
}

/// Compute the final HalfMoon damage for a single target, given the base
/// physical melee damage, skill level and whether this is the primary target
/// in the arc. This applies the HalfMoon MagicInfo scaling and then, for
/// secondary targets, the additional AoE reduction ratio.
pub fn compute_half_moon_damage_for_target<P: WorldProvider>(
    provider: &P,
    base_physical_damage: i32,
    level: u8,
    _is_primary: bool,
) -> i32 {
    if base_physical_damage <= 0 {
        return 0;
    }

    // Apply the HalfMoon MagicInfo scaling in the same way as C#
    // HumanObject, using the shared magic_damage path via
    // apply_attack_spell_scaling. All HalfMoon targets use the same
    // damage model; we do not apply any additional reduction for
    // secondary tiles.
    let dmg = super::apply_attack_spell_scaling(
        provider,
        SPELL_HALF_MOON,
        level,
        base_physical_damage,
    );
    dmg
}

pub fn compute_cross_half_moon_targets(
    x: i32,
    y: i32,
    direction: u8,
) -> Vec<(i32, i32, bool)> {
    let mut targets = Vec::new();

    let front_offset = match direction {
        0 => (0, -1),
        1 => (1, -1),
        2 => (1, 0),
        3 => (1, 1),
        4 => (0, 1),
        5 => (-1, 1),
        6 => (-1, 0),
        7 => (-1, -1),
        _ => (0, 0),
    };
    let front_x = x + front_offset.0;
    let front_y = y + front_offset.1;

    // Primary tile: directly in front of the attacker.
    targets.push((front_x, front_y, true));

    // CrossHalfMoon matches the C# label: starting from the facing
    // direction, walk all 8 directions around the player and hit the
    // adjacent tiles, skipping the Front tile which is already handled by
    // the primary attack. This effectively covers all surrounding tiles
    // except the one directly in front.
    let mut dir = direction;

    let dir_offset = |d: u8| -> (i32, i32) {
        match d {
            0 => (0, -1),
            1 => (1, -1),
            2 => (1, 0),
            3 => (1, 1),
            4 => (0, 1),
            5 => (-1, 1),
            6 => (-1, 0),
            7 => (-1, -1),
            _ => (0, 0),
        }
    };

    for _ in 0..8 {
        let (dx, dy) = dir_offset(dir);
        let tx = x + dx;
        let ty = y + dy;

        if tx != front_x || ty != front_y {
            targets.push((tx, ty, false));
        }

        dir = (dir + 1) % 8;
    }

    targets
}

pub fn compute_cross_half_moon_damage_for_target<P: WorldProvider>(
    provider: &P,
    base_physical_damage: i32,
    level: u8,
    _is_primary: bool,
) -> i32 {
    if base_physical_damage <= 0 {
        return 0;
    }

    // CrossHalfMoon uses its own MagicInfo multipliers in C#. Here we reuse
    // the shared magic_damage path so that all tiles share the same
    // damage model without extra secondary reductions.
    let dmg = super::apply_attack_spell_scaling(
        provider,
        SPELL_CROSS_HALF_MOON,
        level,
        base_physical_damage,
    );
    dmg
}

/// Determine the effective maximum range (in tiles) for Thrusting. This uses
/// the MagicInfo.range value when available and falls back to a small
/// reasonable default when not.
pub fn thrusting_max_range<P: WorldProvider>(provider: &P, _level: u8) -> i32 {
    let base = provider
        .get_magic_info(SPELL_THRUSTING)
        .map(|info| info.range as i32)
        .unwrap_or(2)
        .max(1);

    // If we ever want to scale Thrusting range by level, we can do so here.
    // For now we keep it at the static MagicInfo range.
    base
}
