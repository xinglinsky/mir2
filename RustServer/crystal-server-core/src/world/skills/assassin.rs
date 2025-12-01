use rand::thread_rng;

use crate::stats::{Stat, Stats};
use crate::world::magic::magic_power;
use crate::world::player::PlayerState;
use crate::world::provider::WorldProvider;
use crate::world::skills::compute_magic_mana_cost;
use crate::world::types::BuffType;
use crate::world::{SessionId, World, WorldEvent};
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

pub fn cast_swift_feet<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    events: &mut Vec<WorldEvent>,
) {
    let spell_id = Spell::SwiftFeet as u8;

    let (map_index, x, y, direction, level, duration_ms) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell_id) {
            Some(m) => m,
            None => return,
        };

        let level = magic.level;
        let cost = match compute_magic_mana_cost(&world.provider, &player.stats.total, spell_id, level)
        {
            Some(c) => c,
            None => return,
        };

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        let duration_ms = 25_000_i64.saturating_add((level as i64).saturating_mul(5_000));

        (
            player.map_index,
            player.x,
            player.y,
            player.direction,
            level,
            duration_ms,
        )
    };

    world.add_player_buff(
        session_id,
        BuffType::SwiftFeet,
        duration_ms,
        Stats::default(),
        Vec::new(),
        events,
    );

    world.level_up_magic_for_player(session_id, spell_id, events);

    events.push(WorldEvent::ObjectAttack {
        session_id,
        map_index,
        x,
        y,
        direction,
        spell: spell_id,
        level,
        attack_type: 0,
    });
}

pub fn cast_haste<P: WorldProvider>(
    world: &mut World<P>,
    session_id: SessionId,
    events: &mut Vec<WorldEvent>,
) {
    let spell_id = Spell::Haste as u8;

    let (duration_ms, attack_speed_bonus) = {
        let player = match world.players.get_mut(&session_id) {
            Some(p) => p,
            None => return,
        };

        let magic = match player.magics.iter().find(|m| m.spell == spell_id) {
            Some(m) => m,
            None => return,
        };

        let level = magic.level;
        let cost = match compute_magic_mana_cost(&world.provider, &player.stats.total, spell_id, level)
        {
            Some(c) => c,
            None => return,
        };

        if player.mp < cost {
            return;
        }

        player.mp -= cost;

        // Duration mirrors C#: (Settings.Second * 25) + (Settings.Second * magic.Level * 15)
        let duration_sec = 25_i64.saturating_add(15_i64.saturating_mul(level as i64));
        let duration_ms = duration_sec.saturating_mul(1_000);

        let attack_speed_bonus = (level as i32).saturating_mul(2).saturating_add(2);

        (duration_ms, attack_speed_bonus)
    };

    let mut stats = Stats::default();
    stats.set(Stat::AttackSpeed, attack_speed_bonus);

    world.add_player_buff(
        session_id,
        BuffType::Haste,
        duration_ms,
        stats,
        Vec::new(),
        events,
    );

    world.level_up_magic_for_player(session_id, spell_id, events);
}
