use rand::thread_rng;

pub mod warrior;

use crate::stats::{Stat, Stats};
use crate::world::magic::magic_damage;
use crate::world::player::PlayerState;
use crate::world::provider::WorldProvider;
use crate::world::{Job, Spell};

const SPELL_FATAL_SWORD: u8 = crate::world::Spell::FatalSword as u8;
const SPELL_FIRE_BALL: u8 = crate::world::Spell::FireBall as u8;
const SPELL_GREAT_FIRE_BALL: u8 = crate::world::Spell::GreatFireBall as u8;
const SPELL_THUNDER_BOLT: u8 = crate::world::Spell::ThunderBolt as u8;
const SPELL_SOUL_FIRE_BALL: u8 = crate::world::Spell::SoulFireBall as u8;

// Approximation of C# Settings.MaxLuck used by MapObject.GetAttackPower when
// sampling between MinMC/MaxMC or MinSC/MaxSC. We reuse the same magnitude as
// the physical melee helper so that Luck biases magic attack power in a
// comparable way.
const MAX_LUCK_FOR_MAGIC: i32 = 10;

/// Resolve the effective attack spell and its learned level for a given
/// player. If the requested spell is not learned, this returns (0, 0)
/// to indicate a plain physical attack.
pub fn resolve_attack_spell_and_level_for_player(
    player: &PlayerState,
    requested_spell: u8,
) -> (u8, u8) {
    if requested_spell == 0 {
        return (0, 0);
    }

    // Only allow attack spells that belong to this player's job. If the
    // client sends a spell outside the allowed range for the job, treat it as
    // a plain physical attack.
    if !job_owns_spell(player.job, requested_spell) {
        return (0, 0);
    }

    if let Some(magic) = player.magics.iter().find(|m| m.spell == requested_spell) {
        (requested_spell, magic.level)
    } else {
        (0, 0)
    }
}

/// Look up the learned level of FatalSword for the given player, if any.
pub fn fatal_sword_level_for_player(player: &PlayerState) -> Option<u8> {
    player
        .magics
        .iter()
        .find(|m| m.spell == SPELL_FATAL_SWORD)
        .map(|m| m.level)
}

/// Return true if the given Job is allowed to learn/use the specified spell
/// based on the class ranges used by the original C# server.
pub fn job_owns_spell(job: Job, spell: u8) -> bool {
    match job {
        // Warrior spells: 1..=17
        Job::Warrior => (1..=17).contains(&spell),
        // Wizard spells: 31..=55
        Job::Wizard => (31..=55).contains(&spell),
        // Taoist spells: 61..=86
        Job::Taoist => (61..=86).contains(&spell),
        // Assassin spells: 91..=107
        Job::Assassin => (91..=107).contains(&spell),
        // Archer spells: 121..=141
        Job::Archer => (121..=141).contains(&spell),
    }
}

/// Convenience helper that mirrors the C# class id mapping used by the
/// legacy server and GM commands: 0=Warrior,1=Wizard,2=Taoist,3=Assassin,4=Archer.
pub fn class_owns_spell(class_id: u8, spell: u8) -> bool {
    if let Some(job) = Job::from_u8(class_id) {
        job_owns_spell(job, spell)
    } else {
        false
    }
}

/// Return true if this spell should be treated as a pure magic attack that
/// does not rely on the physical melee damage helper for its base damage.
pub fn is_pure_magic_attack(spell: u8) -> bool {
    spell == SPELL_FIRE_BALL
        || spell == SPELL_GREAT_FIRE_BALL
        || spell == SPELL_THUNDER_BOLT
        || spell == SPELL_SOUL_FIRE_BALL
}

/// Compute damage for a pure magic attack spell by combining the caster's
/// magic stats (MC or SC) with the spell's MagicInfo parameters and learned
/// level. This mirrors the C# pattern:
///
///   DamageBase = GetAttackPower(MinMC/MinSC, MaxMC/MaxSC)
///   FinalDamage = UserMagic.GetDamage(DamageBase)
///
/// where GetAttackPower uses Luck to bias towards min/max.
pub fn compute_pure_magic_attack_damage<P: WorldProvider>(
    provider: &P,
    attacker_stats: &Stats,
    spell: u8,
    level: u8,
) -> i32 {
    if spell == 0 {
        return 0;
    }

    // Decide whether this spell scales from MC (wizard) or SC (taoist).
    let (min_stat, max_stat) = if spell == SPELL_SOUL_FIRE_BALL {
        (Stat::MinSC, Stat::MaxSC)
    } else {
        (Stat::MinMC, Stat::MaxMC)
    };

    // Sample a base magic attack power using the same Luck-biased model as
    // C# MapObject.GetAttackPower.
    fn sample_magic_attack_power<R: rand::Rng + ?Sized>(
        min: i32,
        max: i32,
        luck: i32,
        rng: &mut R,
    ) -> i32 {
        let min = min.max(0);
        let max = max.max(min);

        if luck > 0 {
            if luck > rng.gen_range(0..MAX_LUCK_FOR_MAGIC) {
                return max;
            }
        } else if luck < 0 {
            if luck < -rng.gen_range(0..MAX_LUCK_FOR_MAGIC) {
                return min;
            }
        }

        if max <= min {
            min
        } else {
            rng.gen_range(min..=max)
        }
    }

    let min_val = attacker_stats.get(min_stat);
    let max_val = attacker_stats.get(max_stat);
    let luck = attacker_stats.get(Stat::Luck);

    let mut rng = thread_rng();
    let damage_base = sample_magic_attack_power(min_val, max_val, luck, &mut rng);

    if let Some(info) = provider.get_magic_info(spell) {
        let dmg = magic_damage(info, level, damage_base, &mut rng);
        dmg.max(0)
    } else {
        0
    }
}

/// Compute the MP cost for casting a magic, mirroring the C#
/// HumanObject.MagicCost method (BaseCost + LevelCost * level with optional
/// teleport and generic mana penalties, and a Plague override).
pub fn compute_magic_mana_cost<P: WorldProvider>(
    provider: &P,
    stats: &Stats,
    spell: u8,
    level: u8,
) -> Option<i32> {
    let info = provider.get_magic_info(spell)?;

    let mut cost: i32 = info.base_cost as i32 + level as i32 * info.level_cost as i32;

    if let Some(spell_enum) = Spell::from_u8(spell) {
        if matches!(spell_enum, Spell::Teleport | Spell::Blink | Spell::StormEscape) {
            let tele_penalty = stats.get(Stat::TeleportManaPenaltyPercent);
            if tele_penalty > 0 {
                cost += (cost * tele_penalty) / 100;
            }
        }

        let mana_penalty = stats.get(Stat::ManaPenaltyPercent);
        if mana_penalty > 0 {
            cost += (cost * mana_penalty) / 100;
        }

        if spell_enum == Spell::Plague {
            let max_sc = stats.get(Stat::MaxSC);
            let min_sc = stats.get(Stat::MinSC);
            cost = max_sc.saturating_add(min_sc);
        }
    } else {
        let mana_penalty = stats.get(Stat::ManaPenaltyPercent);
        if mana_penalty > 0 {
            cost += (cost * mana_penalty) / 100;
        }
    }

    Some(cost.max(0))
}

/// Apply the active attack spell (if any) as a scalar on top of the
/// physical base damage, using the MagicInfo parameters and the learned
/// level. This mirrors the C# UserMagic.GetDamage pattern where
///   (DamageBase + GetPower()) * GetMultiplier()
/// is applied per spell.
///
/// FatalSword is handled separately as a passive modifier and therefore is
/// excluded here to avoid double-applying its effect.
pub fn apply_attack_spell_scaling<P: WorldProvider>(
    provider: &P,
    effective_spell: u8,
    level: u8,
    base_damage: i32,
) -> i32 {
    if effective_spell == 0 {
        return base_damage;
    }

    if effective_spell == SPELL_FATAL_SWORD {
        return base_damage;
    }

    if let Some(info) = provider.get_magic_info(effective_spell) {
        let mut rng = thread_rng();
        let boosted = magic_damage(info, level, base_damage, &mut rng);
        if boosted > 0 {
            return boosted;
        }
    }

    base_damage
}

/// Apply FatalSword as a passive scalar and any undead-specific tweaks on
/// top of an already computed physical (and active-spell-scaled) damage
/// value. This keeps the world combat loop free from explicit spell IDs.
pub fn apply_fatal_sword_and_undead<P: WorldProvider>(
    provider: &P,
    fatal_level: Option<u8>,
    undead: bool,
    damage: i32,
) -> i32 {
    let mut raw_damage = damage;

    if raw_damage > 0 {
        if let Some(fatal_level) = fatal_level {
            if let Some(info) = provider.get_magic_info(SPELL_FATAL_SWORD) {
                let mut rng = thread_rng();
                let boosted = magic_damage(info, fatal_level, raw_damage, &mut rng);
                if boosted > 0 {
                    raw_damage = boosted;
                }
            }
        }

        if undead {
            let holy_bonus: i32 = 0;
            raw_damage = raw_damage.saturating_add(holy_bonus);
        }
    }

    raw_damage
}
