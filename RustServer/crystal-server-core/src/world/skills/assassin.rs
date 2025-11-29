use rand::thread_rng;

use crate::world::magic::magic_power;
use crate::world::player::PlayerState;
use crate::world::provider::WorldProvider;
use crate::world::types::BuffType;
use crate::world::Spell;

/// Resolve MoonLight / DarkBody "opening strike" behaviour for an assassin.
///
/// If the player has either MoonLight or DarkBody buffs active, this helper
/// determines which spell should provide the damage bonus, looks up the
/// learned level and then removes the buffs so that the bonus is one-shot.
///
/// Returns (moon_dark_spell_id, level).
pub fn resolve_moon_dark_opening(player: &mut PlayerState) -> (u8, u8) {
    let mut moon_dark_spell: u8 = 0;
    let mut moon_dark_level: u8 = 0;

    let mut had_moon_light = false;
    let mut had_dark_body = false;
    for buff in &player.active_buffs {
        match buff.buff_type {
            BuffType::MoonLight => had_moon_light = true,
            BuffType::DarkBody => had_dark_body = true,
            _ => {}
        }
    }

    if had_moon_light {
        if let Some(m) = player
            .magics
            .iter()
            .find(|m| m.spell == Spell::MoonLight as u8)
        {
            moon_dark_spell = Spell::MoonLight as u8;
            moon_dark_level = m.level;
        }
    }

    if moon_dark_spell == 0 && had_dark_body {
        if let Some(m) = player
            .magics
            .iter()
            .find(|m| m.spell == Spell::DarkBody as u8)
        {
            moon_dark_spell = Spell::DarkBody as u8;
            moon_dark_level = m.level;
        }
    }

    if had_moon_light || had_dark_body {
        player.active_buffs.retain(|b| {
            b.buff_type != BuffType::MoonLight && b.buff_type != BuffType::DarkBody
        });
    }

    (moon_dark_spell, moon_dark_level)
}

/// Apply the MoonLight/DarkBody opening strike bonus to a physical damage
/// value, mirroring the C# HumanObject.Attack behaviour.
pub fn apply_moon_dark_bonus<P: WorldProvider>(
    provider: &P,
    moon_dark_spell: u8,
    moon_dark_level: u8,
    raw_damage: i32,
) -> i32 {
    if moon_dark_spell == 0 || moon_dark_level == 0 || raw_damage <= 0 {
        return raw_damage;
    }

    if let Some(info) = provider.get_magic_info(moon_dark_spell) {
        let mut rng = thread_rng();
        let bonus = magic_power(info, moon_dark_level, &mut rng);
        if bonus > 0 {
            return raw_damage.saturating_add(bonus);
        }
    }

    raw_damage
}
