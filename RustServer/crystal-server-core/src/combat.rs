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

    if damage < 0 {
        0
    } else {
        damage
    }
}
