use rand::{thread_rng, Rng};
use crate::stats::{Stat, Stats};

pub fn compute_physical_damage(attacker: &Stats, defender: &Stats) -> i32 {
    let min_dc = attacker.get(Stat::MinDC).max(0);
    let max_dc = attacker.get(Stat::MaxDC).max(min_dc);
    let attack_bonus = attacker.get(Stat::AttackBonus).max(0);
    let base_dc = (min_dc + max_dc) / 2 + attack_bonus;

    if base_dc <= 0 {
        return 0;
    }

    let min_ac = defender.get(Stat::MinAC).max(0);
    let max_ac = defender.get(Stat::MaxAC).max(min_ac);
    let ac = (min_ac + max_ac) / 2;

    let mut damage = base_dc - ac;
    if damage <= 0 {
        return 0;
    }

    let dr_percent = defender.get(Stat::DamageReductionPercent);
    if dr_percent > 0 {
        let clamped = dr_percent.clamp(0, 95);
        damage = damage.saturating_mul(100 - clamped) / 100;
    }

    if damage < 0 { 0 } else { damage }
}

/// Compute a physical melee hit using a model closer to the original C#
/// server:
///
/// - Base damage is sampled between MinDC/MaxDC with Luck biasing towards
///   min/max.
/// - A hit check is performed using defender Agility vs attacker Accuracy.
/// - Armour is a random value between MinAC/MaxAC.
/// - DamageReductionPercent is applied on the defender.
/// - CriticalRate/CriticalDamage are applied to scale damage and mark the
///   hit as Critical.
///
/// Returns (hit, damage, damage_type) where damage_type mirrors the C#
/// DamageType enum: 0 = Hit, 1 = Miss, 2 = Critical.
pub fn compute_physical_melee_with_crit(
    attacker: &Stats,
    defender: &Stats,
) -> (bool, i32, u8) {
    const MAX_LUCK: i32 = 10;
    const CRITICAL_RATE_WEIGHT: i32 = 5;
    const CRITICAL_DAMAGE_WEIGHT: i32 = 50;

    let mut rng = thread_rng();

    let min_dc = attacker.get(Stat::MinDC).max(0);
    let max_dc = attacker.get(Stat::MaxDC).max(min_dc);
    if max_dc <= 0 {
        return (false, 0, 1);
    }

    let luck = attacker.get(Stat::Luck);
    let max_luck = MAX_LUCK.max(1);

    let mut base_damage = if luck > 0 {
        let roll = rng.gen_range(0..max_luck);
        if luck > roll {
            max_dc
        } else {
            rng.gen_range(min_dc..=max_dc)
        }
    } else if luck < 0 {
        let roll = rng.gen_range(0..max_luck);
        if luck < -roll {
            min_dc
        } else {
            rng.gen_range(min_dc..=max_dc)
        }
    } else {
        rng.gen_range(min_dc..=max_dc)
    };

    // Flat AttackBonus is applied on top of the MinDC/MaxDC roll, mirroring
    // the C# server where AttackBonus is added after determining base
    // physical damage but before armour and percentage reductions.
    let attack_bonus = attacker.get(Stat::AttackBonus).max(0);
    if attack_bonus > 0 {
        base_damage = base_damage.saturating_add(attack_bonus);
    }

    // Accuracy vs Agility hit check. If this fails we treat the outcome as a
    // miss (DamageType::Miss) at the call site and do not apply any armour or
    // critical logic, matching C#'s ACAgility behaviour.
    let accuracy = attacker.get(Stat::Accuracy).max(0);
    let agility = defender.get(Stat::Agility).max(0);
    if rng.gen_range(0..=agility) > accuracy {
        return (false, 0, 1);
    }

    // Random physical armour from MinAC..=MaxAC.
    let min_ac = defender.get(Stat::MinAC).max(0);
    let max_ac = defender.get(Stat::MaxAC).max(min_ac);
    let armour = if max_ac > 0 {
        rng.gen_range(min_ac..=max_ac)
    } else {
        0
    };

    let mut damage = base_damage.saturating_sub(armour);
    if damage <= 0 {
        // High AC can fully absorb the hit; C# treats this as a Miss
        // (DamageType.Miss) even though HP does not change. We signal this by
        // returning hit=false, damage_type=1 so the caller can emit a Miss
        // indicator while keeping HP unchanged.
        return (false, 0, 1);
    }

    // Apply generic percentage damage reduction (e.g. item-based DR).
    let dr_percent = defender.get(Stat::DamageReductionPercent);
    if dr_percent > 0 {
        let clamped = dr_percent.clamp(0, 95);
        damage = damage.saturating_mul(100 - clamped) / 100;
        if damage <= 0 {
            // Fully absorbed by percentage damage reduction; treat as Miss
            // for visual purposes, mirroring C#'s BroadcastDamageIndicator.
            return (false, 0, 1);
        }
    }

    // Critical hit calculation.
    let crit_rate = attacker.get(Stat::CriticalRate).max(0);
    let crit_damage = attacker.get(Stat::CriticalDamage).max(0);
    let mut damage_type: u8 = 0; // Hit
    if crit_rate > 0 && crit_damage > 0 {
        let threshold = crit_rate.saturating_mul(CRITICAL_RATE_WEIGHT).max(0);
        if threshold > 0 && rng.gen_range(0..100) < threshold {
            let factor = (crit_damage as f64 / CRITICAL_DAMAGE_WEIGHT as f64) * 10.0;
            let bonus = ((damage as f64) * factor).floor() as i32;
            damage = damage.saturating_add(bonus);
            damage_type = 2; // Critical
        }
    }

    (true, damage.max(0), damage_type)
}

/// Compute damage for DC-vs-MAC attacks with criticals, approximating C#
/// behaviour for cases where an attack uses DC as its base power but MAC as
/// the defending armour (e.g. HolyDeva's ranged bolt via DefenceType.MAC).
///
/// The flow mirrors `compute_physical_melee_with_crit` but samples
/// MinMAC/MaxMAC instead of MinAC/MaxAC.
pub fn compute_dc_vs_mac_with_crit(attacker: &Stats, defender: &Stats) -> (bool, i32, u8) {
    const MAX_LUCK: i32 = 10;
    const CRITICAL_RATE_WEIGHT: i32 = 5;
    const CRITICAL_DAMAGE_WEIGHT: i32 = 50;

    let mut rng = thread_rng();

    let min_dc = attacker.get(Stat::MinDC).max(0);
    let max_dc = attacker.get(Stat::MaxDC).max(min_dc);
    if max_dc <= 0 {
        return (false, 0, 1);
    }

    let luck = attacker.get(Stat::Luck);
    let max_luck = MAX_LUCK.max(1);

    let mut base_damage = if luck > 0 {
        let roll = rng.gen_range(0..max_luck);
        if luck > roll {
            max_dc
        } else {
            rng.gen_range(min_dc..=max_dc)
        }
    } else if luck < 0 {
        let roll = rng.gen_range(0..max_luck);
        if luck < -roll {
            min_dc
        } else {
            rng.gen_range(min_dc..=max_dc)
        }
    } else {
        rng.gen_range(min_dc..=max_dc)
    };

    let attack_bonus = attacker.get(Stat::AttackBonus).max(0);
    if attack_bonus > 0 {
        base_damage = base_damage.saturating_add(attack_bonus);
    }

    let accuracy = attacker.get(Stat::Accuracy).max(0);
    let agility = defender.get(Stat::Agility).max(0);
    if rng.gen_range(0..=agility) > accuracy {
        return (false, 0, 1);
    }

    // Random MAC armour from MinMAC..=MaxMAC.
    let min_mac = defender.get(Stat::MinMAC).max(0);
    let max_mac = defender.get(Stat::MaxMAC).max(min_mac);
    let armour = if max_mac > 0 {
        rng.gen_range(min_mac..=max_mac)
    } else {
        0
    };

    let mut damage = base_damage.saturating_sub(armour);
    if damage <= 0 {
        return (false, 0, 1);
    }

    let dr_percent = defender.get(Stat::DamageReductionPercent);
    if dr_percent > 0 {
        let clamped = dr_percent.clamp(0, 95);
        damage = damage.saturating_mul(100 - clamped) / 100;
        if damage <= 0 {
            return (false, 0, 1);
        }
    }

    let crit_rate = attacker.get(Stat::CriticalRate).max(0);
    let crit_damage = attacker.get(Stat::CriticalDamage).max(0);
    let mut damage_type: u8 = 0; // Hit
    if crit_rate > 0 && crit_damage > 0 {
        let threshold = crit_rate.saturating_mul(CRITICAL_RATE_WEIGHT).max(0);
        if threshold > 0 && rng.gen_range(0..100) < threshold {
            let factor = (crit_damage as f64 / CRITICAL_DAMAGE_WEIGHT as f64) * 10.0;
            let bonus = ((damage as f64) * factor).floor() as i32;
            damage = damage.saturating_add(bonus);
            damage_type = 2; // Critical
        }
    }

    (true, damage.max(0), damage_type)
}
