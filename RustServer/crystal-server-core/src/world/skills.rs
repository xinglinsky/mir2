use rand::thread_rng;

pub mod warrior;

use crate::world::magic::magic_damage;
use crate::world::player::PlayerState;
use crate::world::provider::WorldProvider;
use crate::world::Job;

const SPELL_FATAL_SWORD: u8 = crate::world::Spell::FatalSword as u8;
const SPELL_FIRE_BALL: u8 = crate::world::Spell::FireBall as u8;
const SPELL_SOUL_FIRE_BALL: u8 = crate::world::Spell::SoulFireBall as u8;

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
    spell == SPELL_FIRE_BALL || spell == SPELL_SOUL_FIRE_BALL
}

/// Compute damage for a pure magic attack spell using only its MagicInfo
/// parameters and learned level. This mirrors calling apply_attack_spell_scaling
/// with a DamageBase of 0.
pub fn compute_pure_magic_attack_damage<P: WorldProvider>(
    provider: &P,
    spell: u8,
    level: u8,
) -> i32 {
    if spell == 0 {
        return 0;
    }

    apply_attack_spell_scaling(provider, spell, level, 0)
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
